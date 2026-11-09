use answer_checker::{Report, Severity, check, sha256};

// These small grammars exercise the formats independently of the target languages.
const FIRST: &str = "G=({A^{1,2},S,Z^{1,1}},{a,b},P,S)\nA^{1,2}->b\nS->Z^{1,1}\nZ^{1,1}->aA^{1,2}";
const SECOND: &str = "G=({S,X^{1,2},Z^{2,2}},{0,1},P,S)\nS->Z^{2,2}\nX^{1,2}->$\nZ^{2,2}->1X^{1,2}";
const PROOF: &str = "  L1 and L2 と ~L を考える。\n\n  a^{k} in L1 or L2 と書く。";

fn sheet(sections: &[(&str, &str)]) -> String {
    let sections: Vec<_> = sections
        .iter()
        .map(|(id, body)| format!("{id}=====\n{body}\n========"))
        .collect();
    format!(
        "学籍番号: 00X00000\n氏名: ダミー\n\n{}\n",
        sections.join("\n\n")
    )
}

fn sheet16(first: &str) -> String {
    sheet(&[("16-1", first), ("16-2", SECOND)])
}

fn sheet17(proof: &str) -> String {
    sheet(&[
        ("17-1", "G=({S,A},{b,a},P,S)\nS->bA\nA->a\nA->$"),
        ("17-2", proof),
        ("17-3.1", "G1=({S},{0},P1,S)\nS->0"),
        ("17-3.2", "G2=({S},{c},P2,S)\nS->c"),
    ])
}

fn has(report: &Report, code: &str, severity: Severity) -> bool {
    report
        .issues
        .iter()
        .any(|issue| issue.code == code && issue.severity == severity)
}

#[test]
fn indexed_members_and_productions_sort_separately_and_hash_idempotently() {
    let unsorted = "G=({Z^{1,1},S,A^{1,2},A^{1,2}},{b,a},P,S)\nZ^{1,1}->aA^{1,2}\nS->Z^{1,1}\nA^{1,2}->b\nA^{1,2}->b";
    let expected = sheet16(
        &FIRST
            .replace("{A^{1,2},S", "{A^{1,2},A^{1,2},S")
            .replace("A^{1,2}->b\n", "A^{1,2}->b\nA^{1,2}->b\n"),
    );
    let input = sheet16(unsorted)
        .replace("16-1=====\n", "16-1=====\n%%% 注意書き\n")
        .replace('\n', "\r\n");
    let report = check("16", input.as_bytes());

    assert!(has(&report, "cfg-set-sort", Severity::Error));
    assert!(has(&report, "sort", Severity::Error));
    assert!(has(&report, "cfg-set-duplicate", Severity::Warning));
    assert_eq!(report.normalized.as_deref(), Some(expected.as_str()));
    assert_eq!(
        report.hash,
        Some(sha256(expected.split_once("\n\n").unwrap().1))
    );

    let again = check("16", expected.as_bytes());
    assert_eq!(again.errors(), 0, "{:?}", again.issues);
    assert_eq!(again.hash, report.hash);
    assert_eq!(again.normalized, report.normalized);
    assert!(has(
        &again,
        "mechanical-npda-cfg-procedure",
        Severity::Warning
    ));
}

#[test]
fn indexed_declarations_check_bounds_membership_and_exact_variable_set() {
    for (first, code) in [
        (FIRST.replace("1,2", "1,4"), "cfg-nonterminal"),
        (FIRST.replace("A^{1,2},S", "S"), "cfg-undeclared"),
        (
            FIRST.replace("A^{1,2},S", "A^{1,1},A^{1,2},S"),
            "cfg-variable-set",
        ),
        (FIRST.replace("{a,b}", "{a}"), "cfg-terminal"),
        (FIRST.replace("aA^{1,2}", "a$A^{1,2}"), "cfg-production"),
        (FIRST.replace("},P,S)", "},Q,S)"), "cfg-declaration-symbols"),
    ] {
        let report = check("16", sheet16(&first).as_bytes());
        assert!(
            has(&report, code, Severity::Error),
            "{code}: {:?}",
            report.issues
        );
        assert!(has(&report, "npda-cfg-unchecked", Severity::Warning));
    }

    let report = check("16", sheet16("G=({S},{a,b},P,S)").as_bytes());
    assert!(has(&report, "cfg-missing-rules", Severity::Error));
}

#[test]
fn assignment17_preserves_rule_and_set_order_and_proof_layout() {
    let input = sheet17(PROOF);
    let report = check("17", input.as_bytes());
    assert_eq!(report.errors(), 0, "{:?}", report.issues);
    assert_eq!(report.normalized.as_deref(), Some(input.as_str()));
    assert_eq!(
        report.hash,
        Some(sha256(input.split_once("\n\n").unwrap().1))
    );
}

#[test]
fn assignment17_checks_declaration_names_and_single_letter_rules() {
    let base = sheet17(PROOF);
    for (input, code) in [
        (
            base.replace("G1=({S},{0},P1,S)", "G1=({S},{0},P,S)"),
            "cfg-declaration-symbols",
        ),
        (base.replace("G2=({S}", "G2=({S,AB}"), "cfg-nonterminal"),
        (base.replace("S->bA", "S->bB"), "cfg-undeclared"),
        (base.replace("S->bA", "S->bA|a"), "production"),
        (base.replace("S->bA", "S ->bA"), "forbidden-space"),
    ] {
        assert!(
            has(&check("17", input.as_bytes()), code, Severity::Error),
            "{code}"
        );
    }
}

#[test]
fn assignment17_distinguishes_clear_set_notation_violations_from_candidates() {
    for proof in ["L1 ∩ L2", "L1 ∪ L2", "∁L", "L1\u{0305}"] {
        let input = sheet17(proof);
        let report = check("17", input.as_bytes());
        assert!(has(&report, "math-set-operator", Severity::Error));
        assert_eq!(report.normalized.as_deref(), Some(input.as_str()));
    }

    let report = check("17", sheet17("∼L と ε の記号を考える。").as_bytes());
    assert_eq!(report.errors(), 0);
    assert!(has(&report, "math-possible", Severity::Warning));

    let report = check("17", sheet17("a^k ∈ L、k ≥ 1").as_bytes());
    for code in ["exponent-braces", "math-membership", "math-comparison"] {
        assert!(has(&report, code, Severity::Error));
    }
}
