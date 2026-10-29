use super::Assignment;
use crate::{AnswerLine, Format, Report, Section, Severity};
use std::collections::BTreeMap;

// Sources: 07_answer-sheet.txt; autolang第7回.pdf pp.5–6.
// No delimiters separate Japanese prose from formulas. The user chose automatic,
// optimistic detection: explicit notation violations are errors; uncertain math
// candidates are warnings. Do not add a marking UI or submission delimiters.
// Keep prose, whitespace, paragraphs and order. Never inspect proof validity,
// pumping-lemma conditions, chosen words/exponents or the languages proved.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "07",
    title: "演習07：正規言語の反復補題",
    note: "証明文は日本語で記載できます。指数は ^{k}、空語は eps、長さは |u|、所属は in / not in。数式らしい部分を自動検出し、確定できない候補は warning で示します。文章内の空白・段落と数式の文字は保持します。証明の正しさは判定しません。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "7-1",
            marker: "7-1=====",
            ascii: false,
            format: Format::Prose,
            validate: Some(check_notation),
        },
        Section {
            id: "7-2",
            marker: "7-2=====",
            ascii: false,
            format: Format::Prose,
            validate: Some(check_notation),
        },
    ],
};

fn check_notation(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_proof_notation(r, rows, Some("eps"));
}

// Reused by assignment 13, whose template has the same math notation but no
// specified ASCII spelling for the empty word. None must never require eps.
pub(super) fn check_proof_notation(
    r: &mut Report,
    rows: &[&AnswerLine<'_>],
    empty_word: Option<&str>,
) {
    for row in rows {
        let text = row.text;
        if text.contains(['≤', '≥', '≦', '≧', '≠', '＝', '＜', '＞']) {
            r.issue(Severity::Error, "math-comparison", Some(row.number), "数式の比較記号は ASCII の <=、>=、<、>、=、!= で記載してください。記号は自動変更しません。");
        }
        if text.contains(['∈', '∉']) {
            r.issue(
                Severity::Error,
                "math-membership",
                Some(row.number),
                "所属の記法は u in L または u not in L を使ってください。∈・∉ は使いません。",
            );
        }
        if let Some(notation) = empty_word
            && text.contains(['ε', 'ϵ'])
        {
            r.issue(
                Severity::Error,
                "proof-empty-word",
                Some(row.number),
                format!("この課題の空語は {notation} で表します。ε・ϵ は使いません。"),
            );
        }
        // Match braces once, so nested or unfinished exponents cannot trigger
        // quadratic rescans. At most one candidate warning per source line.
        let mut brace_stack = Vec::new();
        let mut closes = BTreeMap::new();
        if text.contains('^') {
            for (i, c) in text.bytes().enumerate() {
                if c == b'{' {
                    brace_stack.push(i);
                } else if c == b'}'
                    && let Some(open) = brace_stack.pop()
                {
                    closes.insert(open, i);
                }
            }
        }
        let non_ascii: Vec<_> = text
            .char_indices()
            .filter_map(|(i, c)| (!c.is_ascii()).then_some(i))
            .collect();
        let mut exponent_problem = None;
        let mut candidate = false;
        for (index, _) in text.match_indices('^') {
            let tail = &text[index + 1..];
            let clear_base = text[..index]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, ')' | '}' | '|'));
            if !clear_base {
                candidate = true;
                continue;
            }
            if !tail.starts_with('{') {
                exponent_problem.get_or_insert(
                    "指数は必ず ^{...} の形で {} に囲んでください。例：a^{k}、u^{k+3}。",
                );
                continue;
            }
            match closes.get(&(index + 1)) {
                None => {
                    exponent_problem.get_or_insert(
                        "指数の { に対応する } がありません。指数は ^{...} の形で囲んでください。",
                    );
                }
                Some(&end) => {
                    if text[index + 2..end].trim().is_empty() {
                        exponent_problem
                            .get_or_insert("指数の {} が空です。指数を記載してください。");
                    }
                    let first = non_ascii.partition_point(|&i| i < index + 2);
                    candidate |= non_ascii.get(first).is_some_and(|&i| i < end);
                }
            }
        }
        if let Some(message) = exponent_problem {
            r.issue(
                Severity::Error,
                "exponent-braces",
                Some(row.number),
                message,
            );
        }
        let unusual = text.chars().any(|c| {
            !matches!(c, 'ε' | 'ϵ')
                && matches!(c,
            'α'..='ω' | 'Α'..='Ω' | '⁰'..='ₜ' | '²' | '³' | '¹' | '\u{1d400}'..='\u{1d7ff}' |
            '∀' | '∃' | '∅' | '∞' | '∧' | '∨' | '⇒' | '→' | '∪' | '∩' | '⊆' | '⊂' |
            '−' | '×' | '｜' | '＾' | '０'..='９' | 'Ａ'..='Ｚ' | 'ａ'..='ｚ')
        });
        let command = text
            .as_bytes()
            .windows(2)
            .any(|pair| pair[0] == b'\\' && pair[1].is_ascii_alphabetic());
        let mut bars = text.split('|').skip(1);
        let mut non_ascii_length = false;
        while let Some(inner) = bars.next() {
            if bars.next().is_some() && !inner.is_ascii() {
                non_ascii_length = true;
            }
        }
        let unspecified_empty = empty_word.is_none() && text.contains(['ε', 'ϵ']);
        if candidate
            || unusual
            || command
            || non_ascii_length
            || text.contains('$')
            || unspecified_empty
        {
            let notation = empty_word.map_or_else(
                || "数式は ASCII という指定を確認してください。".to_owned(),
                |word| format!("数式は ASCII、空語は {word} という指定を確認してください。"),
            );
            r.issue(Severity::Warning, "math-possible", Some(row.number), format!("数式の誤表記である可能性があります。数式らしい記号・表記を検出しました。{notation}文章中の記号なら変更は不要です。"));
        }
    }
}
