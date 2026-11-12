use super::{
    Assignment,
    a07::check_set_proof_notation,
    pda::{self, Declared, Syntax, Transition},
};
use crate::{AnswerLine, Format, Report, Section, Severity};
use std::collections::BTreeMap;

// 18_answer-sheet.txt specifies the state limits and row order.
// autolang第18回.pdf pp.4–5 defines DPDA determinism and final-state acceptance.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "18",
    title: "演習18：決定性プッシュダウンオートマトン",
    note: "18-1 は M の定義に続けて5つ組を辞書順に記載します。状態数の上限は順に4・4・3です。宣言との整合性とDPDAの決定性を検査します。入力・スタック記号はASCIIの1文字、ε は $、底記号は Z です。18-2は証明文の ^{…}、~L、and / or、in / not in などの数式表記を検査します。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "18-1.1",
            marker: "18-1.1=====",
            ascii: true,
            format: Format::PdaMachine { sort_sets: false },
            validate: Some(check_machine::<4>),
        },
        Section {
            id: "18-1.2",
            marker: "18-1.2=====",
            ascii: true,
            format: Format::PdaMachine { sort_sets: false },
            validate: Some(check_machine::<4>),
        },
        Section {
            id: "18-1.3",
            marker: "18-1.3=====",
            ascii: true,
            format: Format::PdaMachine { sort_sets: false },
            validate: Some(check_machine::<3>),
        },
        Section {
            id: "18-2",
            marker: "18-2=====",
            ascii: false,
            format: Format::Prose,
            validate: Some(check_set_proof_notation),
        },
    ],
};

fn check_machine<const MAX_STATES: usize>(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    let Some((definition, transitions)) = rows.split_first() else {
        return;
    };

    let declared = check_definition::<MAX_STATES>(r, definition);
    let mut choices = BTreeMap::new();

    pda::check_transitions(r, transitions, Syntax::Ascii, |r, row, rule, repeated| {
        let [_, _, pop, _, push] = rule;

        if pop == "$" || (push != "$" && push.contains('$')) {
            r.issue(
                Severity::Error,
                "mechanical-pda-stack",
                Some(row.number),
                "取り除く記号は Γ の1文字です。何も積まない場合は置換欄全体を $ とします。",
            );
            return;
        }

        if declared
            .as_ref()
            .is_some_and(|sets| !sets.contains_transition(rule))
        {
            r.issue(
                Severity::Error,
                "mechanical-pda-declaration",
                Some(row.number),
                "遷移の状態・入力記号・スタック記号を、先頭の定義の K・Σ・Γ に宣言してください。",
            );
            return;
        }

        if !repeated {
            check_determinism(r, row.number, rule, &mut choices);
        }
    });
}

fn check_definition<'a, const MAX_STATES: usize>(
    r: &mut Report,
    row: &AnswerLine<'a>,
) -> Option<Declared<'a>> {
    let d = pda::check_definition(r, row, Syntax::Ascii, false)?;
    let declared = Declared::from(&d);

    if declared.states.len() > MAX_STATES {
        r.issue(
            Severity::Error,
            "mechanical-pda-states",
            Some(row.number),
            format!(
                "この欄の状態数の上限は {MAX_STATES} です。K には異なる状態が {} 個あります。",
                declared.states.len()
            ),
        );
    }

    if d.function != "p" || d.bottom != "Z" {
        r.issue(
            Severity::Error,
            "mechanical-pda-definition",
            Some(row.number),
            "遷移関数名は p、底記号は Z として記載してください。",
        );
    }

    if !declared.states.contains(d.start) || !declared.finals.is_subset(&declared.states) {
        r.issue(
            Severity::Error,
            "mechanical-pda-declaration",
            Some(row.number),
            "出発状態と F の各最終状態を、状態集合 K に含めてください。",
        );
    }

    let alphabet = |set: &[&str]| set.iter().all(|s| s.len() == 1 && *s != "$");
    if !alphabet(&d.input) {
        r.issue(
            Severity::Error,
            "mechanical-pda-alphabet",
            Some(row.number),
            "Σ は入力記号をASCIIの1文字ずつ宣言します。$ はε遷移の入力欄に使います。",
        );
    }

    if !alphabet(&d.stack) || !declared.stack_contains("Z") {
        r.issue(
            Severity::Error,
            "mechanical-pda-stack-alphabet",
            Some(row.number),
            "Γ は Z を含むASCIIの1文字ずつの集合です。$ は空の置換を表す遷移欄に使います。",
        );
    }

    Some(declared)
}

struct Outcome<'a> {
    target: &'a str,
    replacement: &'a str,
    line: usize,
}

type Choices<'a> = BTreeMap<(&'a str, &'a str), BTreeMap<&'a str, Outcome<'a>>>;

fn check_determinism<'a>(
    r: &mut Report,
    line: usize,
    [from, input, pop, to, push]: Transition<'a>,
    choices: &mut Choices<'a>,
) {
    let by_input = choices.entry((from, pop)).or_default();

    if let Some(prior) = by_input.get(input) {
        if prior.target != to || prior.replacement != push {
            r.issue(Severity::Error, "dpda-conflicting-transition", Some(line), format!("{}行目と状態・入力・スタック先頭が同じですが、遷移先または置換列が異なります。各組に対して結果を1つにしてください。", prior.line));
        }

        return;
    }

    let conflict = if input == "$" {
        by_input.values().next()
    } else {
        by_input.get("$")
    };

    if let Some(prior) = conflict {
        r.issue(Severity::Error, "dpda-epsilon-conflict", Some(line), format!("{}行目と状態・スタック先頭が同じです。ε遷移と入力を読む遷移の一方を選んでください。", prior.line));
    }

    by_input.insert(
        input,
        Outcome {
            target: to,
            replacement: push,
            line,
        },
    );
}
