use super::{
    Assignment,
    a03::{check_transitions, state_labels},
};
use crate::{
    AnswerLine, Format, Report, Section, Severity, edge_trim, normalize_state_pair,
    parse_state_pair,
};
use std::collections::BTreeSet;

// Sources: 06_answer-sheet.txt; autolang第6回.pdf pp.5–7.
// Pairs are unordered, written (X,Y); normalize both their sides and line order.
// The user confirmed an empty 6-1.2 is a zero-element list, not unanswered.
// Never check membership in D, self-pair eligibility, totality, added Z,
// minimization, equivalence classes, or which class a q-number should name.
// 6-1.3 checks only q1,q2,... spelling and no gaps among the labels present.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "06",
    title: "演習06：DFAの最小化",
    note: "6-1.1・6-1.3 は最終状態と遷移をそれぞれ辞書順に列挙。6-1.2 は (X,Y) を1行に1組、組の左右も辞書順にし、0件なら空欄にできます。6-1.3 の状態名は q1,q2,…。注意書きは %%% で始めます。最小化の正しさは検査しません。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "6-1.1",
            marker: "6-1.1=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_transitions),
        },
        Section {
            id: "6-1.2",
            marker: "6-1.2=====",
            ascii: true,
            format: Format::StatePairs { allow_empty: true },
            validate: Some(check_pairs),
        },
        Section {
            id: "6-1.3",
            marker: "6-1.3=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_numbered_automaton),
        },
    ],
};

fn check_pairs(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    let mut canonical = Vec::new();
    for row in rows {
        let value = edge_trim(row.text);
        match parse_state_pair(value) {
            None => r.issue(Severity::Error, "state-pair", Some(row.number), "6-1.2 は (状態,状態) を空白なしで1行に1組記載してください。例：(A,B)。括弧内の空要素や3要素以上は使えません。"),
            Some((left, right)) => {
                if left > right {
                    r.issue(Severity::Error, "pair-order", Some(row.number), "組の左右が ASCII の辞書順ではありません。修正案では左右を入れ替えます。例：(B,A) → (A,B)。");
                }
                canonical.push((normalize_state_pair(value), row.number));
            }
        }
    }
    if let Some(pair) = canonical.windows(2).find(|pair| pair[0].0 > pair[1].0) {
        r.issue(Severity::Error, "pair-sort", Some(pair[1].1), "6-1.2 の組の行が ASCII の辞書順ではありません。修正案では組の左右を整えてから各行を並べ替えます。");
    }
    canonical.sort_unstable();
    if canonical.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        r.issue(Severity::Warning, "duplicate-pair", None, "同じ状態の組が重複しています。左右の順が逆の組も同一として扱います。修正案でも削除せず保持します。");
    }
}

fn check_numbered_automaton(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_transitions(r, rows);
    let mut seen = BTreeSet::new();
    let mut numbered = BTreeSet::new();
    for (state, line) in state_labels(rows) {
        if !seen.insert(state) {
            continue;
        }
        let valid = state.strip_prefix('q').is_some_and(|number| {
            number.starts_with(|c: char| matches!(c, '1'..='9'))
                && number.bytes().all(|c| c.is_ascii_digit())
        });
        if valid {
            numbered.insert(state);
        } else {
            r.issue(Severity::Error, "numbered-state", Some(line), format!("6-1.3 の状態名 {state} は q1,q2,… の形式ではありません。小文字 q と1以上の整数を用い、先頭に余分な0を付けません。状態名は自動変更しません。"));
        }
    }
    if (1..=numbered.len()).any(|i| !numbered.contains(format!("q{i}").as_str())) {
        r.issue(Severity::Error, "numbered-state-sequence", None, "6-1.3 の状態名は q1 から番号を飛ばさず q1,q2,… としてください。どの元の状態集合に各番号を割り当てるかは検査しません。");
    }
}
