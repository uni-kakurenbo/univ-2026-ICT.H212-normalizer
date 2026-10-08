use super::Assignment;
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};

// Source: 02_answer-sheet.txt common instructions for 2-1 / 2-2.
// Symbols and grammar equivalence are answer content, so they are not checked.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "02",
    hash_prefixed_finals: false,
    title: "演習02：正規文法",
    note: "ASCII・1行1規則・辞書順。生成規則は ->、空列は $ を使います。文法の正しさは判定しません。",
    sections: &[
        Section {
            id: "2-1.1",
            marker: "2-1.1===",
            ascii: true,
            format: Format::Productions,
            validate: Some(check_productions),
        },
        Section {
            id: "2-1.2",
            marker: "2-1.2===",
            ascii: true,
            format: Format::Productions,
            validate: Some(check_productions),
        },
        Section {
            id: "2-2",
            marker: "2-2=====",
            ascii: true,
            format: Format::Productions,
            validate: Some(check_productions),
        },
    ],
};

pub(super) fn check_productions(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    use Severity::*;
    for row in rows {
        let value = edge_trim(row.text);
        let sides = value.split("->").collect::<Vec<_>>();
        if sides.len() != 2
            || sides.iter().any(|s| s.is_empty())
            || value.contains('|')
            || value.chars().any(char::is_whitespace)
        {
            r.issue(Error, "production", Some(row.number), "生成規則は空白なしの 左辺->右辺 を1行に1規則記載してください。| は使えません。空列は $ で表します。");
        }
        if value.contains('ε')
            || value.contains('ϵ')
            || sides
                .get(1)
                .is_some_and(|rhs| rhs.contains('$') && *rhs != "$")
        {
            r.issue(
                Error,
                "epsilon-notation",
                Some(row.number),
                "空列は右辺全体を $ で表します。ε や、他の文字と組み合わせた $ は使いません。",
            );
        }
    }
    if let Some(pair) = rows
        .windows(2)
        .find(|pair| edge_trim(pair[0].text) > edge_trim(pair[1].text))
    {
        r.issue(
            Error,
            "sort",
            Some(pair[1].number),
            "生成規則が ASCII の辞書順に並んでいません。修正案では並べ替えます。",
        );
    }
    let mut values = rows
        .iter()
        .map(|row| edge_trim(row.text))
        .collect::<Vec<_>>();
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        r.issue(
            Warning,
            "duplicate-rule",
            None,
            "同じ生成規則の行が重複しています。修正案でも削除せず保持します。",
        );
    }
}
