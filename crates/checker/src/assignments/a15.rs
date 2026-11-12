use super::{Assignment, a14, pda::Syntax};
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};
use std::collections::BTreeSet;

mod construction;

// Sources: 15_answer-sheet.txt; autolang第15回.pdf pp.5–7 (CFG -> NPDA).
// The user authorized local conditions, declaration/trace checks and warning-only
// comparison with the lecture construction. No language equivalence, redundancy
// search, stored solved NPDA/trace, or automatic transition correction.
// Unlike assignment 14, q1 may rewrite/push and epsilon edges may change stack.
const STACK_SYMBOLS: &str = "ABSZab";
// Original problem grammar, not a computed answer. Reference tuples are created
// transiently by the generic construction and never displayed or normalized in.
const PRODUCTIONS: &[(&str, &str)] = &[
    ("S", "aA"),
    ("S", "aBB"),
    ("A", "Ba"),
    ("A", "Sb"),
    ("B", "bAS"),
    ("B", "$"),
];

pub const ASSIGNMENT: Assignment = Assignment {
    id: "15",
    title: "演習15：文脈自由文法からNPDAへの構成",
    note: "15-1 は最初に M の7要素の定義、続けて5つ組を辞書順に記載。ε は $、底記号は Z、空白・タブは禁止です。状態・スタックの局所条件、q0 からの遷移数、宣言との対応、15-2 の abaaaa の受理過程を1段ずつ検査します。講義の CFG→NPDA 構成手順との不一致は warning です。一般の言語等価性・不要な遷移の判定は行いません。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "15-1",
            marker: "15-1=====",
            ascii: true,
            format: Format::PdaMachine { sort_sets: true },
            validate: Some(check_machine),
        },
        Section {
            id: "15-2",
            marker: "15-2=====",
            ascii: true,
            format: Format::StrictCompact,
            validate: Some(check_configurations),
        },
    ],
};

fn check_machine(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    let Some((definition, transitions)) = rows.split_first() else {
        return;
    };
    a14::check_definition_rows(r, &[*definition], STACK_SYMBOLS);
    a14::check_transition_rows(r, transitions, STACK_SYMBOLS, None);
    // A transition relation is a set: identical repeated tuples are one edge,
    // but the common duplicate warning and output preserve every submitted row.
    let mut q0 = BTreeSet::new();
    let mut valid = true;
    for row in transitions {
        match Syntax::Alphanumeric.transition(edge_trim(row.text)) {
            Some(t) if t[0] == "q0" => {
                q0.insert(t);
            }
            Some(_) => {}
            None => valid = false,
        }
    }
    if valid && q0.len() != 1 {
        r.issue(Severity::Error, "mechanical-pda-q0-count", Some(definition.number), "q0 から出る遷移はちょうど1つにしてください。同一の5つ組の重複は、遷移数の検査では1つとして数えます。");
    } else if !valid {
        r.issue(
            Severity::Warning,
            "pda-q0-count-unchecked",
            None,
            "15-1 の遷移に構文エラーがあるため、q0 からの遷移数は未検査です。",
        );
    }
}

fn check_configurations(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    a14::check_configuration_rows(r, rows, STACK_SYMBOLS);
}

pub(super) fn check_related(r: &mut Report, bodies: &[Vec<AnswerLine<'_>>]) {
    let [machine, trace] = bodies else {
        return;
    };
    let rows: Vec<_> = machine
        .iter()
        .filter(|row| !row.text.trim().is_empty())
        .collect();
    let (definitions, transitions) = rows.split_at(usize::from(!rows.is_empty()));
    a14::traces::check_machine(
        r,
        definitions,
        transitions,
        &[(trace, "abaaaa")],
        &a14::traces::Spec {
            alphabet: STACK_SYMBOLS,
            definition_section: "15-1",
            transition_section: "15-1",
        },
    );
    compare_construction(r, definitions, transitions);
}

fn compare_construction(r: &mut Report, definitions: &[&AnswerLine<'_>], rows: &[&AnswerLine<'_>]) {
    let parsed: Option<BTreeSet<_>> = rows
        .iter()
        .map(|row| Syntax::Alphanumeric.transition(edge_trim(row.text)))
        .collect();
    let Some(submitted) = parsed.filter(|set| !set.is_empty()) else {
        r.issue(
            Severity::Warning,
            "pda-construction-unchecked",
            None,
            "15-1 の遷移が空欄または構文エラーのため、CFG→NPDA の構成手順との照合は未検査です。",
        );
        return;
    };
    if definitions
        .first()
        .and_then(|row| Syntax::Alphanumeric.definition(edge_trim(row.text)))
        .is_none()
    {
        r.issue(
            Severity::Warning,
            "pda-construction-unchecked",
            None,
            "15-1 の定義を読み取れないため、CFG→NPDA の構成手順との照合は未検査です。",
        );
        return;
    }
    let Some(reference) = construction::construct("S", "ab", PRODUCTIONS) else {
        r.issue(
            Severity::Warning,
            "pda-construction-unchecked",
            None,
            "構成の計算上限または元の文法の制約により、CFG→NPDA の構成手順との照合は未検査です。",
        );
        return;
    };
    if !reference
        .iter()
        .map(|t| t.each_ref().map(String::as_str))
        .eq(submitted)
    {
        r.issue(Severity::Warning, "pda-construction-mismatch", None, "15-1 の遷移集合は、講義の CFG→NPDA 構成手順で計算した集合との一致を確認できませんでした。別の等価な構成もあり得るため、この不一致だけで非等価・不正解とは判定しません。正解の遷移は提示・自動修正しません。");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedure_comparison_accepts_computed_relation_and_duplicates_but_warns_on_missing_edge() {
        // Compute transiently from the original problem data; no solved tuple
        // list/answer sheet is stored, displayed or inserted into Report output.
        let reference = construction::construct("S", "ab", PRODUCTIONS).unwrap();
        let mut text: Vec<_> = reference
            .iter()
            .map(|t| format!("({})", t.join(",")))
            .collect();
        text.push(text[0].clone());
        let rows: Vec<_> = text
            .iter()
            .enumerate()
            .map(|(i, s)| AnswerLine {
                text: s,
                number: i + 2,
            })
            .collect();
        let definition = AnswerLine {
            text: "M=({q0,q1,qF},{a,b},{A,B,S,Z,a,b},p,q0,Z,{qF})",
            number: 1,
        };
        let mut r = Report::default();
        compare_construction(&mut r, &[&definition], &rows.iter().collect::<Vec<_>>());
        assert!(r.issues.is_empty());
        compare_construction(
            &mut r,
            &[&definition],
            &rows[1..rows.len() - 1].iter().collect::<Vec<_>>(),
        );
        assert_eq!(r.issues.len(), 1);
        assert_eq!(r.issues[0].code, "pda-construction-mismatch");
        assert_eq!(r.issues[0].severity, Severity::Warning);
        assert!(!r.issues[0].message.contains("(q"));
        assert!(r.normalized.is_none() && r.hash_input.is_none() && r.hash.is_none());
        assert!(r.changes.is_empty());
    }
}
