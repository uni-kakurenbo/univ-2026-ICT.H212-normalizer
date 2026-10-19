use super::{AnswerLine, edge_trim, triple};
use crate::{Report, Severity};

const MAX_SEARCH_STEPS: usize = 100_000;

#[derive(Clone, Copy)]
struct Frame {
    state: usize,
    position: usize,
    next_edge: usize,
}

#[derive(Clone, Copy)]
struct Edge {
    target: usize,
    input: u8,
    line: usize,
}

struct Graph {
    start: usize,
    final_state: usize,
    outgoing: Vec<Vec<Edge>>,
    incoming: Vec<Vec<Edge>>,
}

impl Graph {
    // Called only after state, sequence, final-state and triple validation.
    fn from_rows(rows: &[&AnswerLine<'_>]) -> Self {
        let number = |text: &str| edge_trim(text).parse::<usize>().expect("validated number");
        let n = number(rows[1].text);
        let mut outgoing = vec![Vec::new(); n];
        for row in &rows[2..] {
            let [source, input, target] = triple(edge_trim(row.text)).expect("validated triple");
            outgoing[number(source) - 1].push(Edge {
                target: number(target) - 1,
                input: input.as_bytes()[0],
                line: row.number,
            });
        }
        let mut incoming = vec![Vec::new(); n];
        for (source, edges) in outgoing.iter_mut().enumerate() {
            // Repeated identical triples are one logical edge. Keep the input
            // and its normalization intact; do not invent distinct NFA paths.
            edges.sort_by_key(|edge| (edge.input, edge.target));
            edges.dedup_by_key(|edge| (edge.input, edge.target));
            for edge in edges {
                incoming[edge.target].push(Edge {
                    target: source,
                    ..*edge
                });
            }
        }
        Self {
            start: number(rows[0].text) - 1,
            final_state: n - 1,
            outgoing,
            incoming,
        }
    }

    fn accepting_path(&self, word: &[u8]) -> Path {
        let n = self.outgoing.len();
        // Product nodes are (consumed input length, state). Backward reachability
        // marks nodes from which this word can still reach the final state.
        // This preprocessing is O((word.len()+1)*(states+edges)), even with
        // epsilon cycles. The bounded simple-path search below uses no recursion.
        let finish = word.len() * n + self.final_state;
        let mut productive = vec![false; n * (word.len() + 1)];
        productive[finish] = true;
        let mut pending = vec![finish];
        while let Some(node) = pending.pop() {
            let (position, state) = (node / n, node % n);
            for edge in &self.incoming[state] {
                let previous = if edge.input == b'$' {
                    position
                } else if position > 0 && word[position - 1] == edge.input {
                    position - 1
                } else {
                    continue;
                };
                let previous = previous * n + edge.target;
                if !productive[previous] {
                    productive[previous] = true;
                    pending.push(previous);
                }
            }
        }
        if !productive[self.start] {
            return Path::Missing;
        }
        let mut states = vec![self.start + 1];
        let mut lines = Vec::new();
        let mut visited = vec![false; n];
        visited[self.start] = true;
        let mut frames = vec![Frame {
            state: self.start,
            position: 0,
            next_edge: 0,
        }];
        let mut found = None;
        let mut examined = 0;
        while let Some(frame) = frames.last().copied() {
            let at_finish = frame.state == self.final_state && frame.position == word.len();
            if at_finish || frame.next_edge == self.outgoing[frame.state].len() {
                if at_finish {
                    // Account for whole-sequence comparisons too, so many
                    // epsilon placements sharing a long prefix stay bounded.
                    examined += states.len();
                    if examined > MAX_SEARCH_STEPS {
                        return Path::Limit;
                    }
                    if let Some(Path::Unique {
                        states: previous, ..
                    }) = &found
                    {
                        // Different placements of epsilon edges can yield the
                        // same T(x). Numbering compares state sequences only.
                        if previous != &states {
                            return Path::Multiple;
                        }
                    } else {
                        found = Some(Path::Unique {
                            states: states.clone(),
                            lines: lines.clone(),
                        });
                    }
                }
                visited[frame.state] = false;
                frames.pop();
                states.pop();
                lines.pop();
                continue;
            }
            frames.last_mut().expect("current frame").next_edge += 1;
            examined += 1;
            if examined > MAX_SEARCH_STEPS {
                return Path::Limit;
            }
            let edge = self.outgoing[frame.state][frame.next_edge];
            // "Ignore returning paths" is interpreted structurally: no repeated
            // state. Do NOT ignore descending labels, which must be diagnosed.
            if visited[edge.target] {
                continue;
            }
            let position = if edge.input == b'$' {
                frame.position
            } else if word.get(frame.position) == Some(&edge.input) {
                frame.position + 1
            } else {
                continue;
            };
            if productive[position * n + edge.target] {
                visited[edge.target] = true;
                states.push(edge.target + 1);
                lines.push(edge.line);
                frames.push(Frame {
                    state: edge.target,
                    position,
                    next_edge: 0,
                });
            }
        }
        found.unwrap_or(Path::Missing)
    }
}

enum Path {
    Missing,
    Multiple,
    Limit,
    Unique {
        states: Vec<usize>,
        lines: Vec<usize>,
    },
}

pub(super) fn check(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    let graph = Graph::from_rows(rows);
    let words = ["aac", "bbc", "cbaa"];
    let paths = words.map(|word| graph.accepting_path(word.as_bytes()));
    for (word, path) in words.iter().zip(&paths) {
        match path {
            Path::Unique { states, lines } => check_increase(r, word, states, lines),
            Path::Missing | Path::Multiple | Path::Limit => {
                let (code, reason) = match path {
                    Path::Missing => (
                        "path-numbering-missing",
                        "状態を再訪しない受理経路が見つかりません",
                    ),
                    Path::Multiple => (
                        "path-numbering-ambiguous",
                        "状態を再訪しない受理経路が複数あり、一意に決められません",
                    ),
                    _ => (
                        "path-numbering-limit",
                        "探索上限に達し、経路を確定できません",
                    ),
                };
                r.issue(
                    Severity::Warning,
                    code,
                    None,
                    format!("8-1.1 の T({word}) は{reason}。この語の番号の単調増加と、この語を含む経路間比較は未検査です。"),
                );
            }
        }
    }
    if let [
        Path::Unique { states: a, .. },
        Path::Unique { states: b, .. },
        _,
    ] = &paths
    {
        compare(r, a, b, false);
    }
    if let [
        _,
        Path::Unique { states: b, .. },
        Path::Unique { states: c, .. },
    ] = &paths
    {
        compare(r, b, c, true);
    }
}

fn check_increase(r: &mut Report, word: &str, states: &[usize], lines: &[usize]) {
    if let Some((index, pair)) = states
        .windows(2)
        .enumerate()
        .find(|(_, pair)| pair[0] >= pair[1])
    {
        r.issue(
            Severity::Error,
            "path-number-increase",
            Some(lines[index]),
            format!(
                "8-1.1 の T({word}) が厳密な単調増加ではありません。{} 番目からの遷移で {} >= {} です（ε遷移も含みます）。番号は自動変更しません。",
                index + 1, pair[0], pair[1]
            ),
        );
    }
}

fn compare(r: &mut Report, left: &[usize], right: &[usize], prefix_only: bool) {
    let (left_word, right_word, order_code, length_code) = if prefix_only {
        ("bbc", "cbaa", "path-order-bbc-cbaa", "path-length-bbc-cbaa")
    } else {
        ("aac", "bbc", "path-order-aac-bbc", "path-length-aac-bbc")
    };
    let (n, m) = (left.len(), right.len());
    if if prefix_only { n >= m } else { n != m } {
        r.issue(
            Severity::Error,
            length_code,
            None,
            format!(
                "8-1.1 の T({left_word}) は {n} 状態、T({right_word}) は {m} 状態です。{}（ε遷移・出発状態・最終状態も数えます）。",
                if prefix_only { "n < m を満たしません" } else { "同じ経路長の条件を満たしません" }
            ),
        );
        return;
    }
    let count = if prefix_only { n - 1 } else { n };
    if let Some((index, (a, b))) = left
        .iter()
        .zip(right)
        .take(count)
        .enumerate()
        .find(|(_, (a, b))| a > b)
    {
        r.issue(
            Severity::Error,
            order_code,
            None,
            format!(
                "8-1.1 の T({left_word}) と T({right_word}) は i={} で Si <= S'i を満たしません（{a} > {b}）。番号は自動変更しません。",
                index + 1
            ),
        );
    }
}
