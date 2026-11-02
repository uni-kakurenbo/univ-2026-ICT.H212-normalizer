use answer_checker::{HASH_VERSION, Report, Severity, assignments, check, sha256};

// Unrelated toy machine accepting all binary words. It is deliberately NOT the
// NPDA for the supplied CFG; its procedure mismatch must be warning-only.
const DEF: &str = "M=({q0,q1,qF},{a,b},{Z},p,q0,Z,{qF})";
const FULL_DEF: &str = "M=({q0,q1,qF},{a,b},{A,B,S,Z,a,b},p,q0,Z,{qF})";
const RULES: &str = "(q0,$,Z,q1,Z)\n(q1,$,Z,qF,$)\n(q1,a,Z,q1,Z)\n(q1,b,Z,q1,Z)";
// An explicit run of the unrelated toy, not a solution of the assignment.
const TRACE: &str = "(q0,abaaaa,Z)\n(q1,abaaaa,Z)\n(q1,baaaa,Z)\n(q1,aaaa,Z)\n(q1,aaa,Z)\n(q1,aa,Z)\n(q1,a,Z)\n(q1,$,Z)\n(qF,$,$)";

fn sheet(definition: &str, rules: &str, trace: &str) -> String {
    format!(
        "学籍番号: 00X00000\n氏名: ダミー\n\n15-1=====\n{definition}\n{rules}\n========\n\n15-2=====\n{trace}\n========\n"
    )
}
fn base() -> String {
    sheet(DEF, RULES, TRACE)
}
fn has(r: &Report, code: &str, severity: Severity) -> bool {
    r.issues
        .iter()
        .any(|i| i.code == code && i.severity == severity)
}
fn sorted(rules: &str) -> String {
    let mut lines: Vec<_> = rules.lines().collect();
    lines.sort_unstable();
    lines.join("\n")
}

#[test]
fn registers_and_checks_a_toy_trace_with_procedure_mismatch_only_a_warning() {
    let spec = assignments::get("15").unwrap();
    assert_eq!(spec.comment_prefix, "%%%");
    assert_eq!(
        spec.sections.iter().map(|s| s.marker).collect::<Vec<_>>(),
        ["15-1=====", "15-2====="]
    );
    let input = base();
    let r = check("15", input.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.warnings(), 1, "{:?}", r.issues);
    assert!(has(&r, "pda-construction-mismatch", Severity::Warning));
    assert!(
        r.issues
            .iter()
            .any(|i| i.message.contains("非等価・不正解とは判定しません"))
    );
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
}

#[test]
fn definition_stays_first_while_its_sets_and_following_tuples_sort_independently() {
    let def = "M=({qF,q1,q0},{b,a},{b,a,Z,S,B,A,A},p,q0,Z,{qF})";
    let rules = RULES.lines().rev().collect::<Vec<_>>().join("\n");
    let r = check("15", sheet(def, &rules, TRACE).as_bytes());
    assert!(has(&r, "pda-set-sort", Severity::Error));
    assert!(has(&r, "pda-transition-sort", Severity::Error));
    assert!(has(&r, "pda-set-duplicate", Severity::Warning));
    let expected = sheet(
        "M=({q0,q1,qF},{a,b},{A,A,B,S,Z,a,b},p,q0,Z,{qF})",
        RULES,
        TRACE,
    );
    assert_eq!(r.normalized.as_deref(), Some(expected.as_str()));
    assert_eq!(check("15", expected.as_bytes()).hash, r.hash);
    let bad = "(q2,a,Z,q1,Z)\n(q11,a,Z,q1,Z)\n(q$,a,Z,q1,Z)\n(q,a,Z,q1,Z)";
    let r = check("15", sheet(DEF, bad, TRACE).as_bytes());
    assert!(
        r.normalized
            .unwrap()
            .contains("(q,a,Z,q1,Z)\n(q$,a,Z,q1,Z)\n(q11,a,Z,q1,Z)\n(q2,a,Z,q1,Z)")
    );
}

#[test]
fn one_unique_outgoing_q0_transition_is_required_and_duplicates_are_preserved() {
    for rules in [
        RULES.replace("(q0,$,Z,q1,Z)\n", ""),
        sorted(&format!("{RULES}\n(q0,a,Z,q1,Z)")),
    ] {
        let r = check("15", sheet(DEF, &rules, TRACE).as_bytes());
        assert!(has(&r, "mechanical-pda-q0-count", Severity::Error));
    }
    let rules = RULES.replace("(q0,$,Z,q1,Z)", "(q0,$,Z,q1,Z)\n(q0,$,Z,q1,Z)");
    let input = sheet(DEF, &rules, TRACE);
    let r = check("15", input.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert!(has(&r, "pda-transition-duplicate", Severity::Warning));
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
}

#[test]
fn local_checks_reject_wrong_symbols_new_bottoms_and_invalid_entry_to_final() {
    for (rule, code) in [
        ("(q2,a,Z,q1,Z)", "mechanical-pda-state"),
        ("(q1,c,Z,q1,Z)", "mechanical-pda-input"),
        ("(q1,a,C,q1,C)", "mechanical-pda-stack"),
        ("(q1,a,Z,q1,A$Z)", "mechanical-pda-stack"),
        ("(q1,$,A,q1,AZ)", "mechanical-pda-bottom-stack"),
        ("(q1,$,Z,q1,ZZ)", "mechanical-pda-bottom-stack"),
        ("(q1,$,Z,q1,Za)", "mechanical-pda-bottom-stack"),
        ("(q0,a,Z,qF,$)", "mechanical-pda-final-edge"),
        ("(q1,$,A,qF,$)", "mechanical-pda-final-edge"),
        ("(q1,$,Z,qF,Z)", "mechanical-pda-final-edge"),
    ] {
        let rules = sorted(&format!("{RULES}\n{rule}"));
        let r = check("15", sheet(FULL_DEF, &rules, TRACE).as_bytes());
        assert!(has(&r, code, Severity::Error), "{rule}: {:?}", r.issues);
    }
}

#[test]
fn q1_rewriting_lowercase_stack_and_stack_changing_epsilon_are_allowed_only_in_15() {
    for rule in [
        "(q1,$,A,q1,aBS)",
        "(q1,$,B,q1,$)",
        "(q1,a,a,q1,$)",
        "(q1,$,Z,q1,bAZ)",
        "(q1,a,Z,qF,$)",
    ] {
        let rules = sorted(&format!("{RULES}\n{rule}"));
        let r = check("15", sheet(FULL_DEF, &rules, TRACE).as_bytes());
        assert_eq!(r.errors(), 0, "{rule}: {:?}", r.issues);
        assert!(!r.issues.iter().any(|i| matches!(
            i.code,
            "mechanical-pda-q1-stack" | "mechanical-pda-epsilon-stack"
        )));
    }
}

#[test]
fn definition_components_and_used_symbols_must_agree() {
    for (def, code) in [
        (DEF.replace("q0,q1,qF", "q0,q1"), "mechanical-pda-states"),
        (DEF.replace("{a,b}", "{a}"), "mechanical-pda-alphabet"),
        (DEF.replace("{Z}", "{A}"), "mechanical-pda-stack-alphabet"),
        (DEF.replace("{Z}", "{C,Z}"), "mechanical-pda-stack-alphabet"),
        (
            DEF.replace(",p,q0,Z,", ",t,q0,Z,"),
            "mechanical-pda-definition",
        ),
        (DEF.replace("{qF}", "{q1}"), "mechanical-pda-definition"),
    ] {
        assert!(
            has(
                &check("15", sheet(&def, RULES, TRACE).as_bytes()),
                code,
                Severity::Error
            ),
            "{def}"
        );
    }
    let rules = sorted(&format!("{RULES}\n(q1,$,A,q1,aB)"));
    assert!(has(
        &check("15", sheet(DEF, &rules, TRACE).as_bytes()),
        "mechanical-pda-declaration",
        Severity::Error
    ));
    let trace = TRACE.replace("(q1,a,Z)", "(q1,a,aZ)");
    assert!(has(
        &check("15", sheet(DEF, RULES, &trace).as_bytes()),
        "mechanical-pda-config-declaration",
        Severity::Error
    ));
}

#[test]
fn trace_checks_the_target_word_endpoints_each_step_and_untouched_stack_tail() {
    for (trace, code) in [
        (
            TRACE.replace("(q0,abaaaa,Z)", "(q0,aaaa,Z)"),
            "mechanical-pda-trace-start",
        ),
        (
            TRACE.replace("(q0,abaaaa,Z)", "(q1,abaaaa,Z)"),
            "mechanical-pda-trace-start",
        ),
        (
            TRACE.replace("(qF,$,$)", "(q1,$,Z)"),
            "mechanical-pda-trace-end",
        ),
        (
            TRACE.replace("(qF,$,$)", "(qF,a,$)"),
            "mechanical-pda-trace-end",
        ),
        (
            TRACE.replace("(q1,aaaa,Z)\n", ""),
            "mechanical-pda-trace-step",
        ),
        (
            TRACE.replace("(q1,aaa,Z)", "(q1,aaa,AZ)"),
            "mechanical-pda-trace-step",
        ),
    ] {
        let input = sheet(FULL_DEF, RULES, &trace);
        let r = check("15", input.as_bytes());
        assert!(has(&r, code, Severity::Error), "{:?}", r.issues);
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
    // Rewrite with lowercase terminals, then pop them, without searching a run.
    let rules = "(q0,$,Z,q1,aZ)\n(q1,$,Z,qF,$)\n(q1,a,a,q1,$)\n(q1,b,Z,q1,aaaZ)";
    let trace = "(q0,abaaaa,Z)\n(q1,abaaaa,aZ)\n(q1,baaaa,Z)\n(q1,aaaa,aaaZ)\n(q1,aaa,aaZ)\n(q1,aa,aZ)\n(q1,a,Z)";
    let r = check("15", sheet(FULL_DEF, rules, trace).as_bytes());
    assert!(
        !has(&r, "mechanical-pda-trace-step", Severity::Error),
        "{:?}",
        r.issues
    );
    assert!(has(&r, "mechanical-pda-trace-end", Severity::Error));
    let r = check(
        "15",
        sheet(
            FULL_DEF,
            rules,
            &trace.replace("(q1,aaa,aaZ)", "(q1,aaa,aZ)"),
        )
        .as_bytes(),
    );
    assert!(has(&r, "mechanical-pda-trace-step", Severity::Error));
}

#[test]
fn syntax_errors_mark_dependent_checks_unavailable_without_guessing_a_definition() {
    for rule in [
        "(q0,,Z,q1,Z)",
        "(q0,aa,Z,q1,Z)",
        "(q0,$,Z,q1)",
        "(q0,$,Z,q1,Z,Z)",
        "p(q0,$,Z)={(q1,Z)}",
    ] {
        let r = check("15", sheet(DEF, rule, TRACE).as_bytes());
        assert!(has(&r, "pda-transition", Severity::Error));
        assert!(has(&r, "pda-trace-unchecked", Severity::Warning));
        assert!(has(&r, "pda-construction-unchecked", Severity::Warning));
        assert!(has(&r, "pda-q0-count-unchecked", Severity::Warning));
    }
    let input = sheet("(q0,$,Z,q1,Z)", DEF, TRACE);
    let r = check("15", input.as_bytes());
    assert!(has(&r, "pda-definition", Severity::Error));
    assert!(has(&r, "pda-trace-unchecked", Severity::Warning));
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    for trace in [
        "(q0,,Z)",
        "(q0,abaaaa,a$Z)",
        "(q0,abaaaa,C)",
        "(q0,abaaaa,Z)⊢(q1,abaaaa,Z)",
    ] {
        assert!(has(
            &check("15", sheet(FULL_DEF, RULES, trace).as_bytes()),
            "pda-configuration",
            Severity::Error
        ));
    }
}

#[test]
fn comments_identity_line_endings_and_sorting_keep_hash_v1_idempotent() {
    let input = base();
    let baseline = check("15", input.as_bytes());
    assert_eq!(HASH_VERSION, "answer-text-v1");
    assert_eq!(
        baseline.hash,
        Some(sha256(input.split_once("\n\n").unwrap().1))
    );
    let annotated = input
        .replace("00X00000", "99Z99999")
        .replace("ダミー", "別のダミー")
        .replace("15-1=====\n", "%%%% 指示\n15-1=====\n  %%% コメント\n");
    for changed in [
        annotated.clone(),
        annotated.replace('\n', "\r\n"),
        annotated.replace('\n', "\r"),
    ] {
        let r = check("15", format!("\u{feff}{changed}").as_bytes());
        assert_eq!(r.errors(), 0, "{:?}", r.issues);
        assert_eq!(r.hash, baseline.hash);
        assert!(!r.issues.iter().any(|i| i.code == "comment"));
        assert!(!r.normalized.as_ref().unwrap().contains("コメント"));
        let again = check("15", r.normalized.as_ref().unwrap().as_bytes());
        assert_eq!(again.normalized, r.normalized);
        assert_eq!(again.hash, r.hash);
    }
    let malformed = sheet(
        "bad-definition",
        &std::iter::repeat_n("q0,a,Z,q1,Z\n(q0,a,Z,q1,Z)", 40)
            .collect::<Vec<_>>()
            .join("\n"),
        TRACE,
    );
    let r = check("15", malformed.as_bytes());
    let again = check("15", r.normalized.as_ref().unwrap().as_bytes());
    assert_eq!(again.hash, r.hash);
    assert_eq!(again.normalized, r.normalized);
}

#[test]
fn notation_spaces_empty_sections_and_ambiguous_markers_are_checked() {
    for (rule, code) in [
        ("(q0,ε,Z,q1,Z)", "pda-epsilon"),
        ("(q0,$,⊥,q1,Z)", "pda-bottom"),
        ("(q0,$,Z, q1,Z)", "forbidden-space"),
        ("(q0,$,Z,q1,\tZ)", "forbidden-space"),
    ] {
        let input = sheet(DEF, rule, TRACE);
        let r = check("15", input.as_bytes());
        assert!(has(&r, code, Severity::Error));
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
    let r = check("15", sheet("", "", "").as_bytes());
    assert_eq!(
        r.issues.iter().filter(|i| i.code == "unanswered").count(),
        2
    );
    for bad in [
        base().replace("15-1=====", "15-1===="),
        base().replace("15-2=====", "15-1====="),
        base().replace("15-2=====", "15-2===== "),
    ] {
        let r = check("15", bad.as_bytes());
        assert!(r.hash.is_none());
        assert!(
            !r.issues
                .iter()
                .any(|i| i.code == "pda-construction-mismatch")
        );
    }
    assert!(check("14", base().as_bytes()).hash.is_none());
    let r = check(
        "15",
        sheet(
            &format!("\n{DEF}"),
            &RULES.replace('\n', "\n\n"),
            &TRACE.replace('\n', "\n\n"),
        )
        .as_bytes(),
    );
    assert_eq!(r.normalized.as_deref(), Some(base().as_str()));
}
