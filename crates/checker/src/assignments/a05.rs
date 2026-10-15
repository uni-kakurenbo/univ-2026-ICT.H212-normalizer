use super::{
    Assignment,
    a03::{check_transitions, parse_transition, state_labels},
};
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};

// Sources: 05_answer-sheet.txt; autolang第5回.pdf pp.5–7, 9, 13–15.
// Instructions use %%%, not #. '$' is the epsilon spelling, only as a whole
// input token. Do not check the construction, epsilon removal or reachability.
// The user confirmed sorting subset labels by q0..q3/r0..r4 TOKENS, not chars.
// Names of added states and the ASCII spelling of an empty subset are not
// specified. If the entire label cannot be split into these known tokens,
// keep it without guessing token boundaries or enforcing an internal order.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "05",
    title: "演習05：ε遷移を含むオートマトン",
    note: "5-1・5-2 は最終状態と三つ組の遷移をそれぞれ辞書順に列挙。ε は $。5-2 の集合状態名は q0q2r1 のように元の状態名を辞書順に連結します。状態名は自動変更しません。注意書きは %%% で始めます。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "5-1",
            marker: "5-1=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_epsilon_automaton),
        },
        Section {
            id: "5-2",
            marker: "5-2=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_subset_automaton),
        },
    ],
};

fn check_epsilon_automaton(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_transitions(r, rows);
    for row in rows.iter().skip(1) {
        if let Some([_, symbol, _]) = parse_transition(edge_trim(row.text))
            && (symbol.contains(['ε', 'ϵ']) || (symbol.contains('$') && symbol != "$"))
        {
            r.issue(Severity::Error, "epsilon-notation", Some(row.number), "演習05の ε は入力記号の欄全体を $ として表してください。ε・ϵ や、他の文字と組み合わせた $ は使いません。記号は自動変更しません。");
        }
    }
}

fn known_subset_tokens(label: &str) -> Option<Vec<&str>> {
    if label.is_empty() || !label.is_ascii() || !label.len().is_multiple_of(2) {
        return None;
    }
    let tokens: Vec<_> = (0..label.len())
        .step_by(2)
        .map(|i| &label[i..i + 2])
        .collect();
    tokens
        .iter()
        .all(|token| {
            matches!(
                *token,
                "q0" | "q1" | "q2" | "q3" | "r0" | "r1" | "r2" | "r3" | "r4"
            )
        })
        .then_some(tokens)
}

fn check_subset_automaton(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_epsilon_automaton(r, rows);
    for (label, line) in state_labels(rows) {
        if let Some(tokens) = known_subset_tokens(label)
            && tokens.windows(2).any(|pair| pair[0] > pair[1])
        {
            r.issue(Severity::Error, "subset-label-sort", Some(line), format!("5-2 の状態名 {label} が元の状態名単位の辞書順ではありません。例：r1q0q2 ではなく q0q2r1。文字単位では並べ替えません。状態名は自動変更しません。"));
        }
    }
}
