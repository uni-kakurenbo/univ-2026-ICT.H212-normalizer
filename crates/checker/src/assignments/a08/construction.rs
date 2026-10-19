use super::expressions::{Expr, charge, given, limit};
use super::{AnswerLine, edge_trim, triple};
use crate::{Report, Severity};
use std::collections::{BTreeMap, BTreeSet};

// Anonymous ports/edges encode the lecture's rules, without assigning answer
// state numbers or storing a solved automaton. State-number permutations and
// permutations of sum terms are accepted by bounded graph isomorphism.
#[derive(Default)]
struct Shape {
    nodes: usize,
    edges: BTreeSet<(usize, u8, usize)>,
}

impl Shape {
    fn pair(&mut self) -> (usize, usize) {
        let s = self.nodes;
        self.nodes += 2;
        (s, s + 1)
    }
    fn build(&mut self, e: &Expr) -> (usize, usize) {
        match e {
            Expr::Zero | Expr::One | Expr::Letter(_) => {
                let (s, t) = self.pair();
                match e {
                    Expr::One => {
                        self.edges.insert((s, b'$', t));
                    }
                    Expr::Letter(c) => {
                        self.edges.insert((s, *c, t));
                    }
                    _ => {}
                }
                (s, t)
            }
            Expr::Cat(v) => {
                let (s, mut t) = self.build(&v[0]);
                for e in &v[1..] {
                    let (next, end) = self.build(e);
                    self.edges.insert((t, b'$', next));
                    t = end;
                }
                (s, t)
            }
            Expr::Sum(v) => {
                let (s, t) = self.pair();
                // One shared entry/exit for ALL terms, not a binary fold.
                for e in v {
                    let (a, b) = self.build(e);
                    self.edges.extend([(s, b'$', a), (b, b'$', t)]);
                }
                (s, t)
            }
            Expr::Star(e) => {
                let (s, t) = self.pair();
                let (a, b) = self.build(e);
                self.edges
                    .extend([(s, b'$', a), (s, b'$', t), (b, b'$', a), (b, b'$', t)]);
                (s, t)
            }
        }
    }
}

struct Graph {
    matrix: Vec<Vec<Vec<u8>>>,
    signature: Vec<Vec<(bool, u8, usize)>>,
}

impl Graph {
    fn new(n: usize, edges: &BTreeSet<(usize, u8, usize)>) -> Self {
        let mut matrix = vec![vec![Vec::new(); n]; n];
        let mut counts = vec![BTreeMap::new(); n];
        for &(s, c, t) in edges {
            matrix[s][t].push(c);
            *counts[s].entry((false, c)).or_insert(0) += 1;
            *counts[t].entry((true, c)).or_insert(0) += 1;
        }
        let signature = counts
            .into_iter()
            .map(|m| {
                m.into_iter()
                    .map(|((incoming, c), n)| (incoming, c, n))
                    .collect()
            })
            .collect();
        Self { matrix, signature }
    }
}

struct Matcher<'a> {
    pattern: &'a Graph,
    input: &'a Graph,
    candidates: Vec<Vec<usize>>,
    map: Vec<Option<usize>>,
    used: Vec<bool>,
    budget: usize,
}

impl Matcher<'_> {
    fn consistent(&mut self, p: usize, a: usize) -> Result<bool, ()> {
        charge(&mut self.budget, self.map.len() + 1)?;
        if self.pattern.matrix[p][p] != self.input.matrix[a][a] {
            return Ok(false);
        }
        for (q, b) in self.map.iter().enumerate() {
            if let Some(b) = b
                && (self.pattern.matrix[p][q] != self.input.matrix[a][*b]
                    || self.pattern.matrix[q][p] != self.input.matrix[*b][a])
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn search(&mut self) -> Result<bool, ()> {
        let mut best: Option<(usize, Vec<usize>)> = None;
        for p in 0..self.map.len() {
            if self.map[p].is_some() {
                continue;
            }
            let mut choices = Vec::new();
            for a in self.candidates[p].clone() {
                if !self.used[a] && self.consistent(p, a)? {
                    choices.push(a);
                }
            }
            if choices.is_empty() {
                return Ok(false);
            }
            if best.as_ref().is_none_or(|(_, v)| choices.len() < v.len()) {
                best = Some((p, choices));
            }
        }
        let Some((p, choices)) = best else {
            return Ok(true);
        };
        for a in choices {
            self.map[p] = Some(a);
            self.used[a] = true;
            if self.search()? {
                return Ok(true);
            }
            self.used[a] = false;
            self.map[p] = None;
        }
        Ok(false)
    }
}

fn recognizes(
    e: &Expr,
    n: usize,
    edges: &BTreeSet<(usize, u8, usize)>,
    start: usize,
    end: usize,
) -> Result<bool, ()> {
    let mut shape = Shape::default();
    let (s, t) = shape.build(e);
    // These are cheap necessary conditions, followed by full edge matching.
    // Never allocate an input n*n matrix for an arbitrarily large answer.
    if shape.nodes != n || shape.edges.len() != edges.len() || start == end {
        return Ok(false);
    }
    let pattern = Graph::new(shape.nodes, &shape.edges);
    let input = Graph::new(n, edges);
    let candidates: Vec<Vec<_>> = pattern
        .signature
        .iter()
        .map(|sig| {
            input
                .signature
                .iter()
                .enumerate()
                .filter_map(|(i, a)| (sig == a).then_some(i))
                .collect()
        })
        .collect();
    if !candidates[s].contains(&start) || !candidates[t].contains(&end) {
        return Ok(false);
    }
    let mut matcher = Matcher {
        pattern: &pattern,
        input: &input,
        candidates,
        map: vec![None; n],
        used: vec![false; n],
        budget: 100_000,
    };
    matcher.map[s] = Some(start);
    matcher.map[t] = Some(end);
    matcher.used[start] = true;
    matcher.used[end] = true;
    if !matcher.consistent(s, start)? || !matcher.consistent(t, end)? {
        return Ok(false);
    }
    matcher.search()
}

pub(super) fn check(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    let number = |v: &str| {
        edge_trim(v)
            .parse::<usize>()
            .expect("validated state number")
            - 1
    };
    let start = number(rows[0].text);
    let end = number(rows[1].text);
    let mut edges = BTreeSet::new();
    for row in &rows[2..] {
        let [s, c, t] = triple(edge_trim(row.text)).expect("validated triple");
        edges.insert((number(s), c.as_bytes()[0], number(t)));
    }
    match recognizes(&given(),end+1,&edges,start,end) {
        Ok(true) => {},
        Ok(false) => r.issue(Severity::Error,"mechanical-bottom-up",Some(rows[0].number),"8-1.1 の構造が講義のボトムアップ構成に一致しません。3項以上の和を一度に分岐・合流させる指定、積の ε 接続、* の分岐・戻り辺、余分・欠落した状態や遷移を確認してください。状態数・分岐数だけでなく遷移全体を照合しています。"),
        Err(()) => limit(r,Some(rows[0].number)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn edges(v: &[(usize, u8, usize)]) -> BTreeSet<(usize, u8, usize)> {
        v.iter().copied().collect()
    }
    #[test]
    fn flat_three_way_sum_accepts_permutations_but_rejects_binary_nesting() {
        let flat = edges(&[
            (0, b'$', 1),
            (0, b'$', 3),
            (0, b'$', 5),
            (1, b'a', 2),
            (3, b'b', 4),
            (5, b'c', 6),
            (2, b'$', 7),
            (4, b'$', 7),
            (6, b'$', 7),
        ]);
        let e = Expr::parse("a+b+c").unwrap();
        assert_eq!(recognizes(&e, 8, &flat, 0, 7), Ok(true));
        let permuted = flat.iter().map(|&(s, c, t)| (7 - s, c, 7 - t)).collect();
        assert_eq!(recognizes(&e, 8, &permuted, 7, 0), Ok(true));
        let binary = edges(&[
            (0, b'$', 1),
            (0, b'$', 7),
            (1, b'$', 2),
            (1, b'$', 4),
            (2, b'a', 3),
            (4, b'b', 5),
            (3, b'$', 6),
            (5, b'$', 6),
            (7, b'c', 8),
            (6, b'$', 9),
            (8, b'$', 9),
        ]);
        assert_eq!(recognizes(&e, 10, &binary, 0, 9), Ok(false));
        assert_eq!(
            recognizes(&Expr::parse("(a+b)+c").unwrap(), 10, &binary, 0, 9),
            Ok(true)
        );
    }
    #[test]
    fn matching_counts_cannot_hide_wrong_wiring() {
        let mut star = edges(&[
            (0, b'$', 1),
            (0, b'$', 3),
            (1, b'a', 2),
            (2, b'$', 1),
            (2, b'$', 3),
        ]);
        let e = Expr::parse("a*").unwrap();
        assert_eq!(recognizes(&e, 4, &star, 0, 3), Ok(true));
        star.remove(&(2, b'$', 1));
        star.insert((1, b'$', 2));
        assert_eq!(recognizes(&e, 4, &star, 0, 3), Ok(false));
        assert_eq!(
            recognizes(
                &Expr::parse("ab").unwrap(),
                4,
                &edges(&[(0, b'a', 1), (1, b'$', 2), (2, b'b', 3)]),
                0,
                3
            ),
            Ok(true)
        );

        // Keep every labelled in/out degree unchanged and swap just the two
        // product connectors. A degree-count-only checker would miss this.
        let e = Expr::parse("ab+ca").unwrap();
        let mut sum = edges(&[
            (0, b'$', 1),
            (0, b'$', 5),
            (1, b'a', 2),
            (2, b'$', 3),
            (3, b'b', 4),
            (4, b'$', 9),
            (5, b'c', 6),
            (6, b'$', 7),
            (7, b'a', 8),
            (8, b'$', 9),
        ]);
        assert_eq!(recognizes(&e, 10, &sum, 0, 9), Ok(true));
        sum.remove(&(2, b'$', 3));
        sum.remove(&(6, b'$', 7));
        sum.insert((2, b'$', 7));
        sum.insert((6, b'$', 3));
        assert_eq!(recognizes(&e, 10, &sum, 0, 9), Ok(false));
    }

    #[test]
    fn bounded_matcher_handles_the_problem_size_under_a_different_numbering() {
        // Generated anonymously from the rule engine only for the search-budget
        // regression. No solved state numbers or answer sheet are stored.
        let expression = given();
        let mut shape = Shape::default();
        let (s, t) = shape.build(&expression);
        let edges = shape
            .edges
            .iter()
            .map(|&(a, c, b)| (shape.nodes - 1 - a, c, shape.nodes - 1 - b))
            .collect();
        assert_eq!(
            recognizes(
                &expression,
                shape.nodes,
                &edges,
                shape.nodes - 1 - s,
                shape.nodes - 1 - t
            ),
            Ok(true)
        );
    }
}
