use answer_checker::{Severity, check, sha256};

// Deliberately invalid tiny examples, NOT a solved assignment. They exercise
// public validator wiring and preservation of user data when errors are found.
const BODY: &str = "8-1.1=====\n1\n2\n\n(1,a,2)\n========\n\n8-1.2.1=====\nlf(a)={<b,1>}\nlf(1)={}\ni=0,K={},Delta={a}\ni=1,K={a},Delta={1}\ni=2,K={1,a},Delta={}\n========\n\n8-1.2.2=====\na\n1\n\n(a,b,1)\n========\n";
fn text() -> String {
    format!("学籍番号: 00X00000\n氏名: ダミー\n\n{BODY}")
}
fn has(text: &str, code: &str) -> bool {
    check("08", text.as_bytes())
        .issues
        .iter()
        .any(|i| i.code == code)
}

#[test]
fn public_checker_reports_construction_and_calculation_errors_without_rewriting_answers() {
    let text = text();
    let r = check("08", text.as_bytes());
    for code in [
        "mechanical-bottom-up",
        "mechanical-linear-form",
        "mechanical-stage-initial",
        "mechanical-top-down-start",
        "mechanical-top-down-transitions",
    ] {
        assert!(
            r.issues.iter().any(|i| i.code == code),
            "{code}: {:?}",
            r.issues
        );
    }
    assert_eq!(r.normalized.as_deref(), Some(text.as_str()));
    assert_eq!(r.hash_input.as_deref(), Some(BODY));
    assert_eq!(r.hash, Some(sha256(BODY)));
    let again = check("08", r.normalized.as_ref().unwrap().as_bytes());
    assert_eq!(again.normalized, r.normalized);
    assert_eq!(again.hash, r.hash);
}

#[test]
fn alphabet_is_checked_in_transitions_forms_states_and_iteration_sets() {
    for (before, after) in [
        ("(1,a,2)", "(1,x,2)"),
        ("<b,1>", "<x,1>"),
        ("K={a},Delta={1}", "K={x},Delta={1}"),
        ("(a,b,1)", "(a,b,z)"),
    ] {
        assert!(has(&text().replace(before, after), "mechanical-alphabet"));
    }
    let expanded = text().replace("\na\n1\n", "\n(a+b+c)*(bc+ac+baa)\n1\n");
    assert!(has(&expanded, "mechanical-alias"));
}

#[test]
fn deep_valid_syntax_is_marked_unchecked_instead_of_overflowing_the_stack() {
    let deep = format!("{}a{}", "(".repeat(2000), ")".repeat(2000));
    let text = text().replace("\na\n1\n", &format!("\n{deep}\n1\n"));
    let r = check("08", text.as_bytes());
    assert!(
        r.issues
            .iter()
            .any(|i| i.code == "mechanical-limit" && i.severity == Severity::Warning)
    );
    assert!(!r.issues.iter().any(|i| i.code == "state-notation"));
    assert_eq!(r.normalized.as_deref(), Some(text.as_str()));
}
