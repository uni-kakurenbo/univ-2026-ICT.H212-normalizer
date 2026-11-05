use super::{
    Assignment,
    a03::{check_transitions, parse_transition, state_labels},
    a05::two_byte_subset_tokens,
    a06::check_state_numbers,
};
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};
use std::collections::BTreeSet;

mod procedure;
use procedure::{Machine, Unavailable};

// Sources: ex_answer-sheet.txt; autolang演習（8回以降）.pdf pp.3–4;
// lecture 05 pp.6,9–10 and lecture 06 p.6. User authorized four-stage procedure
// comparisons as warnings, including ex-only minimization and p-label mapping.
// Compare ex-1 with original problem data, subsequent stages with the previous
// SUBMITTED section. No solved sheet, generic equivalence check or correction.
// User tentatively accepted sorting whole rendered classes {X,Y} in ASCII,
// including braces/commas; this interpretation is explicit in the UI/docs.
const NAMES: &[&str] = &[
    "q0", "q1", "q2", "q3", "r0", "r1", "r2", "r3", "r4", "s1", "s2",
];

pub const ASSIGNMENT: Assignment = Assignment {
    id: "ex",
    title: "演習ex：オートマトンの構成と最小化",
    note: "ex-1〜ex-4 は最初に最終状態をカンマ区切り・辞書順、続けて三つ組を1行1遷移・辞書順で記載。ε は $、空白・タブは禁止です。ex-3 は元の状態名を辞書順に連結し、ex-4 は p1,p2,…。構成・ε除去・DFA化・最小化は warning で照合し、ex-2以降は直前の提出欄を参照します。ex-4のクラスは要素を整列し、{…}とカンマを含む表記全体のASCII辞書順で番号を割り当てる解釈です。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "ex-1",
            marker: "ex-1=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_first),
        },
        Section {
            id: "ex-2",
            marker: "ex-2=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_second),
        },
        Section {
            id: "ex-3",
            marker: "ex-3=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_third),
        },
        Section {
            id: "ex-4",
            marker: "ex-4=====",
            ascii: true,
            format: Format::DfaTransitions,
            validate: Some(check_fourth),
        },
    ],
};

#[derive(Clone, Copy)]
enum Stage {
    Epsilon,
    Nfa,
    Dfa,
    Minimal,
}
fn check_first(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_rows(r, rows, Stage::Epsilon);
}
fn check_second(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_rows(r, rows, Stage::Nfa);
}
fn check_third(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_rows(r, rows, Stage::Dfa);
}
fn check_fourth(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_rows(r, rows, Stage::Minimal);
}

fn numbered(label: &str) -> bool {
    label.strip_prefix('p').is_some_and(|s| {
        s.starts_with(|c: char| matches!(c, '1'..='9')) && s.bytes().all(|c| c.is_ascii_digit())
    })
}
fn label_valid(label: &str, stage: Stage) -> bool {
    match stage {
        Stage::Epsilon | Stage::Nfa => NAMES.contains(&label),
        Stage::Dfa => two_byte_subset_tokens(label, NAMES).is_some(),
        Stage::Minimal => numbered(label),
    }
}
fn input_valid(symbol: &str, stage: Stage) -> bool {
    matches!(symbol, "a" | "b") || (matches!(stage, Stage::Epsilon) && symbol == "$")
}

fn check_rows(r: &mut Report, rows: &[&AnswerLine<'_>], stage: Stage) {
    check_transitions(r, rows);
    let mut triples = BTreeSet::new();
    let mut duplicate = rows.first().is_some_and(|row| {
        let finals: Vec<_> = edge_trim(row.text).split(',').collect();
        finals.iter().copied().collect::<BTreeSet<_>>().len() != finals.len()
    });
    for (i, row) in rows.iter().enumerate() {
        if row.text.contains([' ', '\t']) {
            r.issue(Severity::Error, "forbidden-space", Some(row.number), "演習exの解答にはスペース・タブを入れません。修正案では行の端だけを整え、行中の空白は保持します。");
        }
        if row.text.contains(['ε', 'ϵ']) {
            r.issue(Severity::Error, "epsilon-notation", Some(row.number), "演習exの ε は $ で表します。ex-2以降はε遷移を記載しません。記号は自動変更しません。");
        }
        if i > 0
            && let Some(t @ [_, symbol, _]) = parse_transition(edge_trim(row.text))
        {
            duplicate |= !triples.insert(t);
            if !input_valid(symbol, stage) {
                r.issue(
                    Severity::Error,
                    "ex-input-symbol",
                    Some(row.number),
                    if matches!(stage, Stage::Epsilon) {
                        "ex-1の入力記号は a/b またはεの $ を1つだけ記載してください。"
                    } else {
                        "ex-2〜ex-4の入力記号は a/b の1記号です。ε遷移や複数記号は記載しません。"
                    },
                );
            }
        }
    }
    let mut seen = BTreeSet::new();
    for (label, line) in state_labels(rows) {
        if !seen.insert(label) {
            continue;
        }
        if !label_valid(label, stage) && !matches!(stage, Stage::Minimal) {
            r.issue(Severity::Error, "ex-state-label", Some(line), if matches!(stage, Stage::Dfa) { "ex-3の状態名は q0〜q3・r0〜r4・s1・s2 を単位として連結してください。集合の括弧や区切り文字は付けません。" } else { "ex-1・ex-2の状態名は q0〜q3・r0〜r4・s1・s2 です。元の状態名を保持し、追加状態には指定された s1・s2 を使ってください。" });
        }
        if matches!(stage, Stage::Dfa)
            && let Some(tokens) = two_byte_subset_tokens(label, NAMES)
        {
            if tokens.windows(2).any(|p| p[0] > p[1]) {
                r.issue(Severity::Error, "subset-label-sort", Some(line), "ex-3の集合状態名は、元の状態名を単位としてASCII辞書順に連結してください。内部の並べ替えや名前の変更は自動では行いません。");
            }
            if tokens.iter().copied().collect::<BTreeSet<_>>().len() != tokens.len() {
                r.issue(Severity::Warning, "ex-subset-duplicate", Some(line), "ex-3の集合状態名の中で同じ元の状態名が重複しています。この名前を使う手順照合は未検査とし、修正案では名前を保持します。");
            }
        }
    }
    if matches!(stage, Stage::Minimal) {
        check_state_numbers(r, rows, 'p', "ex-4");
    }
    if duplicate {
        r.issue(Severity::Warning, "ex-duplicate", None, "同じ最終状態または三つ組が重複しています。手順照合では集合として扱いますが、修正案とハッシュには重複を保持します。");
    }
}

fn parse(rows: &[AnswerLine<'_>], stage: Stage) -> procedure::Result<Machine> {
    let mut rows = rows.iter().filter(|row| !row.text.trim().is_empty());
    let first = rows.next().ok_or(Unavailable::Syntax)?;
    // Start fields are implicit in this template. No start state can be inferred
    // from ex-4, so use no artificial p1 start when comparing its relation.
    let mut machine = Machine::new(if matches!(stage, Stage::Minimal) {
        ""
    } else {
        "s2"
    });
    if matches!(stage, Stage::Minimal) {
        machine.states.clear();
    }
    for label in edge_trim(first.text).split(',') {
        if !label_valid(label, stage) {
            return Err(Unavailable::Syntax);
        }
        machine.states.insert(label.to_owned());
        machine.finals.insert(label.to_owned());
        machine.bound()?;
    }
    for row in rows {
        let [from, symbol, to] =
            parse_transition(edge_trim(row.text)).ok_or(Unavailable::Syntax)?;
        if !label_valid(from, stage) || !label_valid(to, stage) || !input_valid(symbol, stage) {
            return Err(Unavailable::Syntax);
        }
        machine.edge(from, symbol.as_bytes()[0], to)?;
    }
    if matches!(stage, Stage::Dfa)
        && machine.states.iter().any(|s| {
            two_byte_subset_tokens(s, NAMES)
                .is_none_or(|tokens| tokens.windows(2).any(|p| p[0] >= p[1]))
        })
    {
        return Err(Unavailable::Syntax);
    }
    Ok(machine)
}

fn original(
    start: &str,
    finals: &[&str],
    edges: &[(&str, u8, &str)],
) -> procedure::Result<Machine> {
    let mut m = Machine::new(start);
    for &q in finals {
        m.states.insert(q.to_owned());
        m.finals.insert(q.to_owned());
    }
    for &(from, symbol, to) in edges {
        m.edge(from, symbol, to)?;
    }
    m.bound()?;
    Ok(m)
}
fn construct_problem() -> procedure::Result<Machine> {
    // Only ORIGINAL problem automata are stored, never any of the four answers.
    let first = original(
        "q0",
        &["q2", "q3"],
        &[("q0", b'b', "q1"), ("q1", b'a', "q2"), ("q1", b'b', "q3")],
    )?;
    let second = original(
        "r0",
        &["r3", "r4"],
        &[
            ("r0", b'a', "r2"),
            ("r0", b'b', "r1"),
            ("r1", b'a', "r3"),
            ("r2", b'b', "r4"),
        ],
    )?;
    procedure::union(&first, &procedure::complement(&second, "s1")?, "s2")
}

pub(super) fn check_related(r: &mut Report, bodies: &[Vec<AnswerLine<'_>>]) {
    if bodies.len() != 4 {
        return;
    }
    let parsed: Vec<_> = bodies
        .iter()
        .zip([Stage::Epsilon, Stage::Nfa, Stage::Dfa, Stage::Minimal])
        .map(|(rows, stage)| parse(rows, stage))
        .collect();
    for stage in 0..4 {
        let expected = if stage == 0 {
            construct_problem()
        } else {
            parsed[stage - 1]
                .as_ref()
                .map_err(|e| *e)
                .and_then(match stage {
                    1 => procedure::remove_epsilon,
                    2 => procedure::determinize,
                    _ => procedure::minimize,
                })
        };
        match (expected.as_ref(), &parsed[stage]) {
            (Ok(expected), Ok(submitted)) => {
                if !expected.same_relation(submitted) {
                    r.issue(Severity::Warning, "ex-procedure-mismatch", None, format!("ex-{}は{}から講義の手順で計算した状態・遷移・最終状態との一致を確認できませんでした。この不一致だけで言語の非等価・不正解とは判定しません。計算した解答は提示・自動修正しません。", stage+1, if stage == 0 { "問題文のM1・M2".to_owned() } else { format!("直前の提出欄ex-{stage}") }));
                }
            }
            (Err(reason), _) | (_, Err(reason)) => {
                let explanation = match reason {
                    Unavailable::Syntax => {
                        "対象欄または参照欄が空欄・構文エラー・解釈できない状態名です"
                    }
                    Unavailable::Limit => "状態数・遷移数または計算ステップの上限に達しました",
                    Unavailable::NeedsDfa => {
                        "参照するDFAが非決定的、または最小化に必要な遷移が未定義です"
                    }
                    Unavailable::Epsilon => "参照欄にε遷移が残っています",
                    Unavailable::EmptySubset => {
                        "空の部分集合に到達しますが、その状態名のASCII表記は資料に指定がありません"
                    }
                    Unavailable::LabelCollision => {
                        "異なる状態集合を連結した名前が衝突するなど、命名を一意に決められません"
                    }
                    Unavailable::NoAcceptingState => {
                        "出発状態から最終状態へ到達できず、講義の整理後の機械をこの提出形式で表現できません"
                    }
                };
                r.issue(Severity::Warning, "ex-procedure-unchecked", None, format!("ex-{}の手順照合は未検査です。{explanation}。ex-2以降は直前の提出欄を参照します。", stage+1));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_problem_procedures_finish_and_compare_without_storing_or_displaying_solutions() {
        // All computed relations stay transient; no model answer fixture.
        let first = construct_problem().unwrap();
        let second = procedure::remove_epsilon(&first).unwrap();
        let third = procedure::determinize(&second).unwrap();
        let fourth = procedure::minimize(&third).unwrap();
        let relations = [first, second, third, fourth];
        let text: Vec<Vec<_>> =
            relations
                .iter()
                .map(|m| {
                    std::iter::once(m.finals.iter().cloned().collect::<Vec<_>>().join(","))
                        .chain(m.edges.iter().map(|(from, symbol, to)| {
                            format!("({from},{},{to})", char::from(*symbol))
                        }))
                        .collect()
                })
                .collect();
        let mut bodies: Vec<Vec<_>> = text
            .iter()
            .map(|rows| {
                rows.iter()
                    .enumerate()
                    .map(|(i, s)| AnswerLine {
                        text: s,
                        number: i + 1,
                    })
                    .collect()
            })
            .collect();
        let mut r = Report::default();
        check_related(&mut r, &bodies);
        assert!(
            r.issues.is_empty(),
            "computed pipeline should match without diagnostics"
        );
        let first_duplicate = bodies[0][1].clone();
        let third_duplicate = bodies[2][1].clone();
        bodies[0].push(first_duplicate);
        bodies[2].push(third_duplicate);
        check_related(&mut r, &bodies);
        assert!(
            r.issues.is_empty(),
            "logical duplicates do not alter comparisons"
        );
        bodies[3].pop();
        check_related(&mut r, &bodies);
        assert_eq!(r.issues.len(), 1);
        assert_eq!(r.issues[0].severity, Severity::Warning);
        assert!(r.issues[0].message.starts_with("ex-4"));
        assert!(!r.issues[0].message.contains("(p"));
        assert!(r.normalized.is_none() && r.hash_input.is_none() && r.hash.is_none());
    }
}
