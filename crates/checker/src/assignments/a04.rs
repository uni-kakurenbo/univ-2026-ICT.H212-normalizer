use super::{Assignment, a02::check_productions, a03::check_transitions};
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};
use std::collections::BTreeSet;

// Sources: 04_answer-sheet.txt; autolang第4回.pdf pp.5–9; the user's
// supplementary (source,symbol,target) notation and comma/sort confirmations.
// 4-1 and 4-2 use the same final-state/triple syntax for NFA and DFA: no
// determinism, subset membership, reachability or language equivalence checks.
// The template explicitly spells {A,B,#} as #AB, unlike lecture figures.
// A first final-state row '#' or '#AB' must not be removed as a comment.
// ASCII spelling for an empty subset is unspecified, so no spelling is invented.
// The user confirmed checking 4-2 character order and 4-3 label conventions.
// Compare xY order only among rules with the same left-hand nonterminal.
// The supplied production set is not compared with a model grammar or DFA.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "04",
    title: "演習04：正規文法と有限オートマトン",
    note: "4-1・4-2 は最終状態と遷移をそれぞれ辞書順に列挙。4-2 の状態名内の文字も辞書順（#AB）。# で始まる最終状態は保持し、注意書きには ### を使ってください。4-3 は S と A,B,C,… を使用し、同じ左辺の xY の順序が交差しないかも検査します。状態名は自動変更しません。",
    hash_prefixed_finals: true,
    sections: &[
        Section {
            id: "4-1",
            marker: "4-1=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_transitions),
        },
        Section {
            id: "4-2",
            marker: "4-2=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_subset_labels),
        },
        Section {
            id: "4-3",
            marker: "4-3=====",
            ascii: true,
            format: Format::Productions,
            validate: Some(check_renamed_productions),
        },
    ],
};

fn check_subset_labels(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_transitions(r, rows);
    for (index, row) in rows.iter().enumerate() {
        let value = edge_trim(row.text);
        let labels: Vec<_> = if index == 0 {
            value.split(',').collect()
        } else if let Some(inner) = value.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
            let fields: Vec<_> = inner.split(',').collect();
            if fields.len() != 3 {
                continue;
            }
            vec![fields[0], fields[2]]
        } else {
            continue;
        };
        for label in labels {
            if !label.is_empty()
                && label.is_ascii()
                && !label
                    .chars()
                    .any(|c| c.is_whitespace() || matches!(c, '(' | ')' | '{' | '}' | '[' | ']'))
                && label.as_bytes().windows(2).any(|pair| pair[0] > pair[1])
            {
                r.issue(Severity::Error, "subset-label-sort", Some(row.number), format!("4-2 の状態名 {label} の文字が ASCII の辞書順ではありません。例：AB# ではなく #AB。状態名は自動変更しません。"));
            }
        }
    }
}

fn check_renamed_productions(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_productions(r, rows);
    let mut states = BTreeSet::new();
    let mut start_seen = false;
    let mut links = Vec::new();
    for row in rows {
        let value = edge_trim(row.text);
        let Some((lhs, rhs)) = value.split_once("->") else {
            continue;
        };
        if lhs.len() != 1 || !lhs.as_bytes()[0].is_ascii_uppercase() {
            r.issue(Severity::Error, "state-label", Some(row.number), "4-3 の左辺は S または A,B,C,… のラテン大文字1文字で記載してください。状態名は自動変更しません。");
            continue;
        }
        let source = lhs.as_bytes()[0];
        states.insert(source);
        start_seen |= source == b'S';
        // Inspect notation only: the first RHS character is a terminal in xY.
        // This notation follows xY in the template and aB in lecture p.9.
        // A single-symbol RHS is not graded for whether it is a terminal or unit rule.
        if rhs.is_ascii() && !rhs.contains("->") && !rhs.contains('|') {
            let tail = rhs.get(1..).unwrap_or_default();
            if rhs.len() > 1 {
                if tail.len() != 1 || !tail.as_bytes()[0].is_ascii_uppercase() {
                    r.issue(Severity::Error, "state-label", Some(row.number), "4-3 の右辺の非終端記号は S または A,B,C,… のラテン大文字1文字で記載してください。# や複数文字の状態名は使いません。");
                    continue;
                }
                let target = tail.as_bytes()[0];
                states.insert(target);
                links.push((source, rhs.as_bytes()[0], target, row.number));
            } else if rhs.len() == 1 && rhs.as_bytes()[0].is_ascii_uppercase() {
                states.insert(rhs.as_bytes()[0]);
            }
        }
    }
    if !rows.is_empty() && !start_seen {
        r.issue(Severity::Error, "start-label", None, "4-3 の出発記号は S です。S を左辺とする生成規則がありません。出発記号の意味や正しさは検査しません。");
    }
    let others: Vec<_> = states.into_iter().filter(|&state| state != b'S').collect();
    if !others.iter().copied().eq((b'A'..=b'Z')
        .filter(|&state| state != b'S')
        .take(others.len()))
    {
        r.issue(Severity::Error, "state-label-sequence", None, "4-3 の S 以外の非終端記号は A,B,C,… を飛ばさず順に使ってください（S は出発記号として予約）。状態名は自動変更しません。");
    }
    // Sorting avoids comparing every pair on potentially large input. Equal
    // terminals are grouped: their target order is not constrained by the rule.
    links.sort_unstable();
    let mut previous: Option<(u8, u8, u8)> = None;
    for group in links.chunk_by(|a, b| a.0 == b.0 && a.1 == b.1) {
        let &(source, symbol, min_target, line) = group.first().expect("nonempty group");
        let max_target = group.last().expect("nonempty group").2;
        if let Some((old_source, old_symbol, old_target)) = previous
            && old_source == source
            && old_target > min_target
        {
            r.issue(Severity::Error, "state-order-cross", Some(line), format!("4-3 の同じ左辺 {} で、右辺 {}{} と {}{} の終端記号・非終端記号の順序が交差しています。記号の付け替えは自動で行いません。", source as char, old_symbol as char, old_target as char, symbol as char, min_target as char));
        }
        if previous.is_none_or(|(old_source, _, old_target)| {
            old_source != source || old_target < max_target
        }) {
            previous = Some((source, symbol, max_target));
        }
    }
}
