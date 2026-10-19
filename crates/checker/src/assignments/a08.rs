use super::Assignment;
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};
use std::collections::BTreeSet;

// Sources: 08_answer-sheet.txt; autolang第8回.pdf pp.5, 7–12.
// Explicit start row, final CSV row, then triples. Preserve the start separately.
// The user confirmed 8-1.1 positive consecutive labels and the unique maximum
// final label. Count labels present, without evaluating any accepting paths.
// 8-1.2 labels are regexes with nested (), concatenation, +, *, constants 0/1
// and alias e. Never compute lf, K/Delta membership, equivalence or correctness.
// No ordering of lf rows/set members is specified: preserve them and step order.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "08",
    title: "演習08：正規表現とオートマトン",
    note: "オートマトン欄は出発状態、最終状態、三つ組の順。最終状態と遷移は文字の辞書順（11 は 2 より前）。8-1.1 は1からの連続番号と最大番号の最終状態を検査。8-1.2.1 は lf(p)={...} と i=0,K={...},Delta={...}。導出の順序は保持し、空白・タブはエラーです。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "8-1.1",
            marker: "8-1.1=====",
            ascii: true,
            format: Format::StartedTransitions,
            validate: Some(check_numbered),
        },
        Section {
            id: "8-1.2.1",
            marker: "8-1.2.1=====",
            ascii: true,
            format: Format::StrictCompact,
            validate: Some(check_derivation),
        },
        Section {
            id: "8-1.2.2",
            marker: "8-1.2.2=====",
            ascii: true,
            format: Format::StartedTransitions,
            validate: Some(check_regex_automaton),
        },
    ],
};

fn positive_number(value: &str) -> bool {
    value.starts_with(|c: char| matches!(c, '1'..='9')) && value.bytes().all(|c| c.is_ascii_digit())
}

// Iterative parser: deeply nested input must not overflow the WASM call stack.
fn regex(value: &str) -> bool {
    let mut depth = 0usize;
    let mut need_atom = true;
    for c in value.bytes() {
        match c {
            b'(' => {
                depth += 1;
                need_atom = true;
            }
            b')' if depth > 0 && !need_atom => {
                depth -= 1;
                need_atom = false;
            }
            b'+' if !need_atom => need_atom = true,
            b'*' if !need_atom => {}
            c if c.is_ascii_alphanumeric() => need_atom = false,
            _ => return false,
        }
    }
    depth == 0 && !need_atom
}

fn triple(value: &str) -> Option<[&str; 3]> {
    let inner = value.strip_prefix('(')?.strip_suffix(')')?;
    let mut parts = inner.split(',');
    let fields = [parts.next()?, parts.next()?, parts.next()?];
    (parts.next().is_none()
        && fields
            .iter()
            .all(|value| !value.is_empty() && !value.chars().any(char::is_whitespace)))
    .then_some(fields)
}

fn symbol(value: &str) -> bool {
    value.len() == 1
        && value.as_bytes()[0].is_ascii_graphic()
        && !value.contains([',', '<', '>', '{', '}'])
}

fn check_numbered(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_automaton(r, rows, true);
}
fn check_regex_automaton(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_automaton(r, rows, false);
}

fn check_automaton(r: &mut Report, rows: &[&AnswerLine<'_>], numbered: bool) {
    if rows.is_empty() {
        return;
    }
    if rows.len() < 2 {
        r.issue(
            Severity::Error,
            "automaton-headers",
            rows.first().map(|row| row.number),
            "最初の解答行に出発状態1つ、次の解答行に最終状態のリストを記載してください。",
        );
        return;
    }
    let valid_state = if numbered { positive_number } else { regex };
    let mut states = vec![(edge_trim(rows[0].text), rows[0].number)];
    let finals: Vec<_> = edge_trim(rows[1].text).split(',').collect();
    states.extend(finals.iter().map(|state| (*state, rows[1].number)));
    if finals.windows(2).any(|pair| pair[0] > pair[1]) {
        r.issue(Severity::Error, "final-state-sort", Some(rows[1].number), "最終状態が文字の辞書順ではありません。修正案では出発状態と分けて並べ替えます。11 は 2 より前です。");
    }
    for row in rows.iter().skip(2) {
        if let Some([source, input, target]) = triple(edge_trim(row.text)) {
            states.extend([(source, row.number), (target, row.number)]);
            if !symbol(input) {
                r.issue(Severity::Error, "transition", Some(row.number), "入力記号は ASCII の1記号（ε は $）で記載してください。三つ組は (出発状態,入力記号,到着状態) です。");
            }
            if input.contains(['ε', 'ϵ']) || (input.contains('$') && input != "$") {
                r.issue(Severity::Error, "epsilon-notation", Some(row.number), "演習08の ε 遷移は入力記号の欄全体を $ として表してください。正規表現中の空列のみの言語は 1 です。");
            }
        } else {
            r.issue(Severity::Error, "transition", Some(row.number), "遷移は (出発状態,入力記号,到着状態) を空白なしで1行に1つ記載してください。正規表現の状態名の半角括弧は使用できます。");
        }
    }
    if let Some(pair) = rows[2..]
        .windows(2)
        .find(|pair| edge_trim(pair[0].text) > edge_trim(pair[1].text))
    {
        r.issue(
            Severity::Error,
            "sort",
            Some(pair[1].number),
            "遷移行が文字の辞書順ではありません。出発状態・最終状態とは分けて並べ替えます。",
        );
    }
    let mut valid_labels = BTreeSet::new();
    let mut invalid = false;
    for (state, line) in states {
        if !valid_state(state) {
            invalid = true;
            r.issue(Severity::Error, "state-notation", Some(line), if numbered {
                "8-1.1 の状態名は1以上の整数で記載し、先頭に余分な0を付けません。状態名は自動変更しません。"
            } else {
                "8-1.2 の状態名は講義の正規表現の記法（英数字、+、*、半角括弧）を使います。全体の別名 e と、空列のみの言語 1 を使用できます。"
            });
        } else {
            valid_labels.insert(state);
        }
    }
    if numbered && !invalid {
        let n = valid_labels.len();
        if (1..=n).any(|i| !valid_labels.contains(i.to_string().as_str())) {
            r.issue(Severity::Error, "state-number-sequence", None, "8-1.1 の状態名は1から欠番なく番号を付けてください。番号の変更は自動では行いません。");
        } else if finals.len() != 1 || finals[0] != n.to_string() {
            r.issue(Severity::Error, "maximum-final-state", Some(rows[1].number), "8-1.1 の最終状態は最大番号の1状態としてください。番号や最終状態は自動変更しません。");
        }
    }
}

fn regex_set(value: &str) -> bool {
    value
        .strip_prefix('{')
        .and_then(|v| v.strip_suffix('}'))
        .is_some_and(|inner| inner.is_empty() || inner.split(',').all(regex))
}

fn linear_form(value: &str) -> bool {
    let Some((expression, set)) = value.strip_prefix("lf(").and_then(|v| v.split_once(")=")) else {
        return false;
    };
    if !regex(expression) {
        return false;
    }
    let Some(inner) = set.strip_prefix('{').and_then(|v| v.strip_suffix('}')) else {
        return false;
    };
    if inner.is_empty() {
        return true;
    }
    inner
        .strip_prefix('<')
        .and_then(|v| v.strip_suffix('>'))
        .is_some_and(|pairs| {
            pairs.split(">,<").all(|pair| {
                pair.split_once(',')
                    .is_some_and(|(input, target)| symbol(input) && regex(target))
            })
        })
}

fn stage(value: &str) -> Option<&str> {
    let (index, tail) = value.strip_prefix("i=")?.split_once(",K=")?;
    let (known, delta) = tail.split_once(",Delta=")?;
    if index.is_empty()
        || !index.bytes().all(|c| c.is_ascii_digit())
        || !regex_set(known)
        || !regex_set(delta)
    {
        return None;
    }
    Some(index)
}

fn check_derivation(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    let mut forms = 0;
    let mut stages = 0;
    let mut stage_started = false;
    for row in rows {
        let value = edge_trim(row.text);
        if value.starts_with("lf(") {
            forms += 1;
            if !linear_form(value) {
                r.issue(Severity::Error, "linear-form", Some(row.number), "linear form は lf(p)={<x,q>,<y,r>} の形で1行に1つ記載してください。空集合は lf(p)={}、組の括弧は半角 < > です。正規表現中の空列のみの言語は 1 と書きます。");
            }
            if stage_started {
                r.issue(Severity::Error, "derivation-order", Some(row.number), "lf(p) の行を先に記載し、その後に i=0 からの各段階を続けてください。導出の行は自動で並べ替えません。");
            }
        } else if value.starts_with("i=") {
            stage_started = true;
            match stage(value) {
                None => r.issue(Severity::Error, "derivation-stage", Some(row.number), "各段階は i=0,K={...},Delta={...} の形で1行に1段階記載してください。空集合は {}、Delta は ASCII で書きます。"),
                Some(index) if index != stages.to_string() => r.issue(Severity::Error, "stage-index", Some(row.number), "反復の段階番号は0から1ずつ増やして記載してください。段階の行は自動で並べ替えません。"),
                _ => {}
            }
            stages += 1;
        } else {
            r.issue(Severity::Error, "derivation-line", Some(row.number), "8-1.2.1 は lf(p)={...} または i=0,K={...},Delta={...} の行で記載してください。途中式の正しさは検査しません。");
        }
    }
    if !rows.is_empty() && (forms == 0 || stages == 0) {
        r.issue(
            Severity::Error,
            "derivation-parts",
            None,
            "8-1.2.1 には linear form の行と反復の各段階の行を記載してください。",
        );
    }
}
