use super::{Assignment, a02::check_productions};
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};
use std::collections::{BTreeSet, VecDeque};

// Sources: 12_answer-sheet.txt; autolang第12回.pdf pp.4–8.
// CNF syntax is A->BC or A->a, with single uppercase nonterminals and a/b.
// User-confirmed naming checks: C->a, D->b when present, consecutive added
// letters, then the originating LHS order S,A,B. Infer origins only from the
// submitted auxiliary references, never from a computed solution grammar.
// Shared/missing/untraceable or cyclic helper origins are warnings, explicitly
// unavailable for that helper. No order among helpers of the same origin is
// specified. Preserve names and duplicates; only production ROWS are sorted.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "12",
    title: "演習12：チョムスキーの標準形",
    note: "12-1・12-2 は CNF の A->BC または A->a（終端 a/b）を1行1規則、ASCII 辞書順で列挙。追加記号は C->a、D->b、以降は欠番なし・元の左辺 S,A,B の順を検査します。共有などで補助記号の由来が不明なら warning。記号の自動変更や元の文法との等価性の判定は行いません。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "12-1",
            marker: "12-1=====",
            ascii: true,
            format: Format::Productions,
            validate: Some(check_cnf),
        },
        Section {
            id: "12-2",
            marker: "12-2=====",
            ascii: true,
            format: Format::Productions,
            validate: Some(check_cnf),
        },
    ],
};

#[derive(Clone, Copy)]
enum Rhs {
    Terminal(u8),
    Binary([u8; 2]),
}

struct Rule {
    left: u8,
    right: Rhs,
    line: usize,
}

fn parse(value: &str, line: usize) -> Option<Rule> {
    let (left, right) = value.split_once("->")?;
    let [left] = left.as_bytes() else {
        return None;
    };
    if !left.is_ascii_uppercase() {
        return None;
    }
    let right = match right.as_bytes() {
        [terminal @ (b'a' | b'b')] => Rhs::Terminal(*terminal),
        [a, b] if a.is_ascii_uppercase() && b.is_ascii_uppercase() => Rhs::Binary([*a, *b]),
        _ => return None,
    };
    Some(Rule {
        left: *left,
        right,
        line,
    })
}

fn helper(label: u8) -> bool {
    !matches!(label, b'S' | b'A' | b'B' | b'C' | b'D')
}

fn index(label: u8) -> usize {
    usize::from(label - b'A')
}

fn original_rank(label: u8) -> Option<u8> {
    match label {
        b'S' => Some(0),
        b'A' => Some(1),
        b'B' => Some(2),
        _ => None,
    }
}

fn check_cnf(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_productions(r, rows);
    let mut rules = Vec::new();
    let mut invalid = false;
    for row in rows {
        if let Some(rule) = parse(edge_trim(row.text), row.number) {
            rules.push(rule);
        } else {
            r.issue(Severity::Error, "cnf-production", Some(row.number), "CNF の規則は大文字1文字->大文字2文字、または大文字1文字->a/b です。単位規則、3記号以上の右辺、終端と非終端の混在、$、|、添字付きの名前は使いません。");
            invalid = true;
        }
    }
    if invalid || rules.is_empty() {
        return;
    }
    check_names(r, &rules);
}

fn check_names(r: &mut Report, rules: &[Rule]) {
    let mut labels = BTreeSet::new();
    let mut first_line = [None; 26];
    let mut definition_line = [None; 26];
    let mut defined = [false; 26];
    let mut terminal_only = [true; 26];
    let mut terminal_values = [0u8; 26];
    let mut edges: [BTreeSet<u8>; 26] = std::array::from_fn(|_| BTreeSet::new());
    let mut origins = [0u8; 26];
    for rule in rules {
        labels.insert(rule.left);
        first_line[index(rule.left)].get_or_insert(rule.line);
        defined[index(rule.left)] = true;
        definition_line[index(rule.left)].get_or_insert(rule.line);
        match rule.right {
            Rhs::Terminal(c) => terminal_values[index(rule.left)] |= if c == b'a' { 1 } else { 2 },
            Rhs::Binary(children) => {
                terminal_only[index(rule.left)] = false;
                for child in children {
                    labels.insert(child);
                    first_line[index(child)].get_or_insert(rule.line);
                    if helper(child) {
                        if let Some(rank) = original_rank(rule.left) {
                            origins[index(child)] |= 1 << rank;
                        } else if helper(rule.left) {
                            edges[index(rule.left)].insert(child);
                        }
                    }
                }
            }
        }
    }
    let added: Vec<_> = labels
        .into_iter()
        .filter(|&c| original_rank(c).is_none())
        .collect();
    // S is already reserved; it can never be allocated again as a new symbol.
    if !added
        .iter()
        .copied()
        .eq((b'C'..=b'Z').filter(|&c| c != b'S').take(added.len()))
    {
        r.issue(Severity::Error, "cnf-label-sequence", added.first().and_then(|&c| first_line[index(c)]), "新たな非終端記号は C,D,E,… を欠番なしで使ってください。元の S は予約済みです。記号の付け替えは自動で行いません。");
    }
    for (label, expected) in [(b'C', 1), (b'D', 2)] {
        let i = index(label);
        if first_line[i].is_some()
            && (!defined[i] || !terminal_only[i] || terminal_values[i] != expected)
        {
            r.issue(Severity::Error, "cnf-terminal-label", first_line[i], "終端記号専用の追加規則は C->a、D->b の順に割り当ててください。C/D に別の生成規則を混在させたり、自動で名前を入れ替えたりはしません。");
        }
    }
    for &label in added.iter().filter(|&&c| helper(c)) {
        if defined[index(label)] && terminal_values[index(label)] != 0 {
            r.issue(Severity::Error, "cnf-helper-role", first_line[index(label)], "C/D より後の追加記号は、長い右辺を2記号ずつに分ける補助記号です。終端記号専用の規則を割り当てる場合の名前を確認してください。");
        }
    }
    // Propagate only through newly added helpers. Crossing S/A/B would mix a
    // nonterminal's language dependencies with the provenance of a fresh rule.
    let mut pending: VecDeque<_> = added
        .iter()
        .copied()
        .filter(|&c| origins[index(c)] != 0)
        .collect();
    while let Some(source) = pending.pop_front() {
        for &child in &edges[index(source)] {
            let combined = origins[index(child)] | origins[index(source)];
            if combined != origins[index(child)] {
                origins[index(child)] = combined;
                pending.push_back(child);
            }
        }
    }
    // Kahn's scan marks cycles and their downstream helpers as untraceable.
    // At most 26 labels and 676 distinct edges, independent of duplicate rows.
    let mut incoming = [0usize; 26];
    for targets in &edges {
        for &child in targets {
            incoming[index(child)] += 1;
        }
    }
    let mut acyclic = [false; 26];
    let mut pending: VecDeque<_> = added
        .iter()
        .copied()
        .filter(|&c| helper(c) && incoming[index(c)] == 0)
        .collect();
    while let Some(source) = pending.pop_front() {
        acyclic[index(source)] = true;
        for &child in &edges[index(source)] {
            incoming[index(child)] -= 1;
            if incoming[index(child)] == 0 {
                pending.push_back(child);
            }
        }
    }
    let mut previous_rank: Option<u32> = None;
    for &label in added.iter().filter(|&&c| helper(c)) {
        let i = index(label);
        if !defined[i] || !acyclic[i] || origins[i].count_ones() != 1 {
            r.issue(Severity::Warning, "cnf-label-origin", first_line[i], format!("補助記号 {} は共有・由来不明・定義不足・循環のため、元の左辺を一意に特定できません。この記号の S,A,B に従う命名順は未検査です。記号と規則は保持します。", label as char));
            continue;
        }
        if terminal_values[i] != 0 {
            continue;
        }
        let rank = origins[i].trailing_zeros();
        if previous_rank.is_some_and(|previous| previous > rank) {
            r.issue(Severity::Error, "cnf-label-order", definition_line[i], "右辺の分割用の追加記号が、元の規則の左辺 S,A,B の順に割り当てられていません。同じ左辺に由来する補助記号どうしの順序は指定がないため比較しません。記号は自動変更しません。");
        }
        previous_rank = Some(previous_rank.map_or(rank, |previous| previous.max(rank)));
    }
}
