use super::Assignment;
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};

// Sources: 03_answer-sheet.txt and the user's supplementary triple-notation
// slide: (S,a,A), (A,a,#). The slide explicitly does not encode start/final
// state markers in triples. The user additionally confirmed comma-separated
// final states, and requested checking/sorting their list as well.
// Do not check transition semantics, state choices, minimality or image contents.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "03",
    hash_prefixed_finals: false,
    title: "演習03：有限オートマトン",
    note: "最初の解答行に最終状態をカンマ区切り・辞書順で記載。以降は (S,a,A) 形式の遷移を辞書順に列挙します。3-3 は PNG ファイル名。",
    sections: &[
        Section {
            id: "3-1",
            marker: "3-1===",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_transitions),
        },
        Section {
            id: "3-2",
            marker: "3-2=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_transitions),
        },
        Section {
            id: "3-3",
            marker: "3-3=====",
            ascii: true,
            format: Format::PngFilename,
            validate: Some(check_png),
        },
    ],
};

pub(super) fn check_transitions(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    if let Some(finals) = rows.first() {
        if !edge_trim(finals.text).split(',').all(valid_token) {
            r.issue(Severity::Error, "final-states", Some(finals.number), "最初の解答行には最終状態をカンマ区切りで記載してください。例：A,B。括弧・空要素・空白は使いません。状態の存在や受理条件は検査しません。");
        }
        let states: Vec<_> = edge_trim(finals.text).split(',').collect();
        if states.windows(2).any(|pair| pair[0] > pair[1]) {
            r.issue(Severity::Error, "final-state-sort", Some(finals.number), "最終状態が ASCII の辞書順に並んでいません。修正案ではカンマ区切りのリストを並べ替えます。");
        }
    }
    // The first non-comment answer row is the separate final-state list.
    // A graph example is not a solution, and '#' inside a triple is a token.
    let transitions = rows.get(1..).unwrap_or_default();
    for row in transitions {
        let value = edge_trim(row.text);
        let valid = value
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .is_some_and(|inner| {
                let fields: Vec<_> = inner.split(',').collect();
                fields.len() == 3 && fields.iter().all(|field| valid_token(field))
            });
        if !valid {
            r.issue(Severity::Error, "transition", Some(row.number), "遷移は (状態,入力記号,状態) の三つ組を空白なしで1行に1つ記載してください。例：(S,a,A)。記号の意味は検査しません。");
        }
    }
    if let Some(pair) = transitions
        .windows(2)
        .find(|pair| edge_trim(pair[0].text) > edge_trim(pair[1].text))
    {
        r.issue(Severity::Error, "sort", Some(pair[1].number), "遷移行が ASCII の辞書順に並んでいません。最終状態のリストとは分けて、遷移行を並べ替えます。");
    }
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && !value
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '(' | ')' | '{' | '}' | '[' | ']'))
}

fn check_png(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    if rows.len() != 1
        || rows.first().is_some_and(|row| {
            !edge_trim(row.text).to_ascii_lowercase().ends_with(".png")
                || edge_trim(row.text).len() <= 4
        })
    {
        r.issue(Severity::Error, "png-filename", rows.first().map(|row| row.number), "3-3 には PNG 画像のファイル名を1行で記載してください。画像の添付・内容は検査しません。");
    }
}
