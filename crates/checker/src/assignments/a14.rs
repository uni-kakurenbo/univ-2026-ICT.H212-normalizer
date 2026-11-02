use super::Assignment;
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};
use std::collections::BTreeSet;

pub(super) mod traces;

pub(super) const STACK_SYMBOLS: &str = "ABZ";

// Sources: 14_answer-sheet.txt; autolang第14回.pdf pp.6–10.
// The user authorized local stack/state restrictions, declaration consistency,
// and exact ONE-step checks of the submitted aa/aba traces, not language
// equivalence or transition redundancy. No model NPDA or solved traces.
// Replacement pops one symbol and writes gamma with the left end on top:
// q0 preserves the old top as a suffix; q1 can remove it or keep it unchanged.
// Do not invent phase-order constraints or require every qF edge to be epsilon.
// T(M) is final-state acceptance with the whole input consumed (lecture p.10).
pub const ASSIGNMENT: Assignment = Assignment {
    id: "14",
    title: "演習14：非決定性プッシュダウンオートマトン",
    note: "14-1 は5つ組を辞書順、14-2 は M=(K,Σ,Γ,p,q0,Z,F) の集合を辞書順に記載。ε は $、底記号は Z、空白・タブは禁止です。状態・スタックの局所条件と、aa/aba の時点表示が記載した遷移に1段ずつ一致するかも検査します。時点表示は順序を保持。回文言語全体との一致・不要な遷移の有無は未検査です。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "14-1",
            marker: "14-1=====",
            ascii: true,
            format: Format::SortedTuples,
            validate: Some(check_transitions),
        },
        Section {
            id: "14-2",
            marker: "14-2=====",
            ascii: true,
            format: Format::PdaDefinition,
            validate: Some(check_definition),
        },
        Section {
            id: "14-3",
            marker: "14-3=====",
            ascii: true,
            format: Format::StrictCompact,
            validate: Some(check_configurations),
        },
        Section {
            id: "14-4",
            marker: "14-4=====",
            ascii: true,
            format: Format::StrictCompact,
            validate: Some(check_configurations),
        },
    ],
};

fn token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'$')
}

fn tuple<const N: usize>(value: &str) -> Option<[&str; N]> {
    let inner = value.strip_prefix('(')?.strip_suffix(')')?;
    let mut fields = inner.split(',');
    let mut result = [""; N];
    for field in &mut result {
        *field = fields.next()?;
    }
    (fields.next().is_none() && result.iter().all(|s| token(s))).then_some(result)
}

pub(super) fn transition(value: &str) -> Option<[&str; 5]> {
    let fields = tuple::<5>(value)?;
    (fields[1].len() == 1 && fields[2].len() == 1).then_some(fields)
}

pub(super) fn state(value: &str) -> bool {
    matches!(value, "q0" | "q1" | "qF")
}
fn stack_word(value: &str, alphabet: &str) -> bool {
    value == "$" || (!value.is_empty() && value.bytes().all(|c| alphabet.as_bytes().contains(&c)))
}
fn input_word(value: &str) -> bool {
    value == "$" || (!value.is_empty() && value.bytes().all(|c| matches!(c, b'a' | b'b')))
}
pub(super) fn decode(value: &str) -> &str {
    if value == "$" { "" } else { value }
}

pub(super) fn configuration<'a>(value: &'a str, alphabet: &str) -> Option<[&'a str; 3]> {
    let fields = tuple::<3>(value)?;
    (input_word(fields[1]) && stack_word(fields[2], alphabet)).then_some([
        fields[0],
        decode(fields[1]),
        decode(fields[2]),
    ])
}

fn check_notation(r: &mut Report, row: &AnswerLine<'_>) {
    if row.text.contains(['ε', 'ϵ']) {
        r.issue(Severity::Error, "pda-epsilon", Some(row.number), "この課題の ε は $ で記載してください。空の入力・積まない記号列・空の残り入力・空スタックは欄全体を $ とします。");
    }
    if row.text.contains('⊥') {
        r.issue(
            Severity::Error,
            "pda-bottom",
            Some(row.number),
            "スタックの底記号は ASCII の Z で記載してください。⊥ は使いません。",
        );
    }
}

fn check_transitions(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_transition_rows(r, rows, STACK_SYMBOLS, Some(check_local_transition));
}

type LocalCheck = fn(&mut Report, &AnswerLine<'_>, [&str; 5]);

pub(super) fn check_transition_rows(
    r: &mut Report,
    rows: &[&AnswerLine<'_>],
    alphabet: &str,
    local: Option<LocalCheck>,
) {
    let mut previous = None;
    let mut seen = BTreeSet::new();
    let mut duplicate = false;
    for row in rows {
        check_notation(r, row);
        let Some(t @ [from, input, pop, to, push]) = transition(edge_trim(row.text)) else {
            r.issue(Severity::Error, "pda-transition", Some(row.number), "遷移は (状態,入力1記号,取り除く1記号,状態,積む記号列) の5つ組を空白なしで1行に1つ記載してください。空要素や集合表記は使わず、ε・空の記号列は $ とします。");
            continue;
        };
        if previous.is_some_and(|old| old > t) {
            r.issue(Severity::Error, "pda-transition-sort", Some(row.number), "5つ組を左の要素から ASCII 辞書順で比べて並べてください。$・数字は英字より前、大文字は小文字より前です。修正案は行順だけを整えます。");
        }
        previous = Some(t);
        duplicate |= !seen.insert(t);
        if !state(from) || !state(to) {
            r.issue(
                Severity::Error,
                "mechanical-pda-state",
                Some(row.number),
                "この課題の状態名は q0・q1・qF だけです。記号名は自動変更しません。",
            );
        }
        if !matches!(input, "$" | "a" | "b") {
            r.issue(
                Severity::Error,
                "mechanical-pda-input",
                Some(row.number),
                "読む入力記号は a/b の1記号、または読まない場合の $ です。",
            );
        }
        if !alphabet.contains(pop) || !stack_word(push, alphabet) {
            r.issue(Severity::Error, "mechanical-pda-stack", Some(row.number), format!("取り除く記号は {alphabet} のいずれか1記号です。積む列はこれらの記号、または何も積まない $ を使ってください。$ と他の記号は混在させません。"));
            continue;
        }
        let replacement = decode(push);
        if replacement.contains('Z')
            && !(pop == "Z"
                && replacement.strip_suffix('Z').is_some_and(|prefix| {
                    prefix
                        .bytes()
                        .all(|c| c != b'Z' && alphabet.as_bytes().contains(&c))
                }))
        {
            r.issue(Severity::Error, "mechanical-pda-bottom-stack", Some(row.number), "Z は元の先頭が Z のときに、置換列の末尾に1個だけ保持できます。新しい Z は積みません。他の記号は課題で許されたものだけを使ってください。");
        }
        if to == "qF" && (from != "q1" || pop != "Z" || push != "$") {
            r.issue(Severity::Error, "mechanical-pda-final-edge", Some(row.number), "qF へは q1 からだけ移り、そのとき先頭の Z を取り除きます。取り除く欄は Z、積む欄は $ です。");
        }
        if let Some(check) = local {
            check(r, row, t);
        }
    }
    if duplicate {
        r.issue(Severity::Warning, "pda-transition-duplicate", None, "同じ5つ組の遷移が重複しています。時点表示の検査では同じ1遷移と扱い、修正案からは削除しません。");
    }
}

fn check_local_transition(
    r: &mut Report,
    row: &AnswerLine<'_>,
    [from, input, pop, to, push]: [&str; 5],
) {
    let replacement = decode(push);
    if from == "q0"
        && !replacement
            .strip_suffix(pop)
            .is_some_and(|added| added.bytes().all(|c| matches!(c, b'A' | b'B')))
    {
        r.issue(Severity::Error, "mechanical-pda-q0-stack", Some(row.number), "q0 からは元のスタック先頭を保持し、その左に A/B だけを追加できます。置換列の末尾が取り除く欄の記号と一致するか確認してください。");
    }
    if from == "q1" && push != "$" && push != pop {
        r.issue(Severity::Error, "mechanical-pda-q1-stack", Some(row.number), "q1 からは新たな記号を積みません。置換列は $（先頭を取り除く）または元の先頭記号そのもの（変化なし）です。");
    }
    if input == "$" && to != "qF" && push != pop {
        r.issue(Severity::Error, "mechanical-pda-epsilon-stack", Some(row.number), "qF に移る場合を除き、ε遷移ではスタックを変化させません。取り除く欄と積む欄を同じ1記号にしてください。");
    }
}

pub(super) struct Definition<'a> {
    pub sets: [Vec<&'a str>; 4], // K, Sigma, Gamma, F
    function: &'a str,
    pub start: &'a str,
    pub bottom: &'a str,
}

fn parse_set(value: &str) -> Option<Vec<&str>> {
    let inner = value.strip_prefix('{')?.strip_suffix('}')?;
    if inner.is_empty() {
        return Some(Vec::new());
    }
    inner.split(',').map(|s| token(s).then_some(s)).collect()
}

pub(super) fn definition(value: &str) -> Option<Definition<'_>> {
    let inner = value.strip_prefix("M=(")?.strip_suffix(')')?;
    let mut fields = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in inner.bytes().enumerate() {
        match c {
            b'{' if depth == 0 => depth = 1,
            b'}' if depth == 1 => depth = 0,
            b'{' | b'}' => return None,
            b',' if depth == 0 => {
                fields.push(&inner[start..i]);
                if fields.len() >= 7 {
                    return None;
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    fields.push(&inner[start..]);
    let [k, sigma, gamma, function, start, bottom, finals] = fields.as_slice() else {
        return None;
    };
    if !token(function) || !token(start) || !token(bottom) {
        return None;
    }
    Some(Definition {
        sets: [
            parse_set(k)?,
            parse_set(sigma)?,
            parse_set(gamma)?,
            parse_set(finals)?,
        ],
        function,
        start,
        bottom,
    })
}

pub(crate) fn normalize_definition(value: &str) -> String {
    let Some(mut d) = definition(value) else {
        return value.to_owned();
    };
    for set in &mut d.sets {
        set.sort_unstable();
    }
    format!(
        "M=({{{}}},{{{}}},{{{}}},{},{},{},{{{}}})",
        d.sets[0].join(","),
        d.sets[1].join(","),
        d.sets[2].join(","),
        d.function,
        d.start,
        d.bottom,
        d.sets[3].join(",")
    )
}

fn check_definition(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    if rows.is_empty() {
        return;
    }
    if rows.len() != 1 {
        r.issue(Severity::Error, "pda-definition-lines", rows.first().map(|row| row.number), "14-2 には M=(K,Σ,Γ,p,q0,Z,F) を1行だけ記載してください。複数の定義は自動で選びません。");
    }
    check_definition_rows(r, rows, STACK_SYMBOLS);
}

pub(super) fn check_definition_rows(r: &mut Report, rows: &[&AnswerLine<'_>], alphabet: &str) {
    for row in rows {
        check_notation(r, row);
        let Some(d) = definition(edge_trim(row.text)) else {
            r.issue(Severity::Error, "pda-definition", Some(row.number), "M=({状態の集合},{入力記号の集合},{スタック記号の集合},p,出発状態,底記号,{最終状態の集合}) の7要素を空白なしで記載してください。集合には半角 {} を使います。");
            continue;
        };
        for (name, set) in ["K", "Σ", "Γ", "F"].into_iter().zip(&d.sets) {
            if set.windows(2).any(|pair| pair[0] > pair[1]) {
                r.issue(Severity::Error, "pda-set-sort", Some(row.number), format!("集合 {name} の要素が ASCII 辞書順ではありません。修正案ではこの集合の中だけを並べ替えます。"));
            }
            let unique: BTreeSet<_> = set.iter().copied().collect();
            if unique.len() != set.len() {
                r.issue(
                    Severity::Warning,
                    "pda-set-duplicate",
                    Some(row.number),
                    format!("集合 {name} の要素が重複しています。修正案からは削除しません。"),
                );
            }
        }
        let sets: Vec<BTreeSet<_>> = d.sets.iter().map(|s| s.iter().copied().collect()).collect();
        if sets[0] != BTreeSet::from(["q0", "q1", "qF"]) {
            r.issue(
                Severity::Error,
                "mechanical-pda-states",
                Some(row.number),
                "K は q0・q1・qF の3状態だけを宣言してください。",
            );
        }
        if sets[1] != BTreeSet::from(["a", "b"]) {
            r.issue(
                Severity::Error,
                "mechanical-pda-alphabet",
                Some(row.number),
                "入力記号の集合 Σ は a/b です。ε は入力アルファベットに加えません。",
            );
        }
        if !sets[2].contains("Z")
            || sets[2]
                .iter()
                .any(|s| s.len() != 1 || !alphabet.contains(*s))
        {
            r.issue(Severity::Error, "mechanical-pda-stack-alphabet", Some(row.number), format!("Γ は底記号 Z を含み、記号は {alphabet} に制限してください。未使用の記号を必ず含めることは要求しません。"));
        }
        if d.function != "p"
            || d.start != "q0"
            || d.bottom != "Z"
            || sets[3] != BTreeSet::from(["qF"])
        {
            r.issue(Severity::Error, "mechanical-pda-definition", Some(row.number), "遷移関数は p、出発状態は q0、底記号は Z、最終状態の集合は {qF} として記載してください。7要素の順番も確認してください。");
        }
    }
}

fn check_configurations(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_configuration_rows(r, rows, STACK_SYMBOLS);
}

pub(super) fn check_configuration_rows(r: &mut Report, rows: &[&AnswerLine<'_>], alphabet: &str) {
    for row in rows {
        check_notation(r, row);
        if row.text.contains('⊢') {
            r.issue(
                Severity::Error,
                "pda-turnstile",
                Some(row.number),
                "時点表示は1行に1つ記載し、⊢ は書かないでください。",
            );
        }
        match configuration(edge_trim(row.text), alphabet) {
            Some([q, _, _]) if state(q) => {}
            _ => r.issue(Severity::Error, "pda-configuration", Some(row.number), format!("時点表示は (q0/q1/qF,残りの入力,スタック全体) を空白なしで1行に1つ記載してください。残りの入力は a/b、スタックは {alphabet} の列で、空の欄はそれぞれ $ です。先頭は左です。")),
        }
    }
}

pub(super) fn check_related(r: &mut Report, bodies: &[Vec<AnswerLine<'_>>]) {
    traces::check_related(r, bodies);
}
