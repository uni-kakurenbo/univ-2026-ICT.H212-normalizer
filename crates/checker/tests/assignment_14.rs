use answer_checker::{HASH_VERSION, Report, Severity, assignments, check, sha256};

// A deliberately unrelated toy NPDA accepting ALL nonempty binary words,
// including nonpalindromes. This is not a solution of the supplied problem.
const RULES: &str = "(q0,a,Z,q1,Z)\n(q0,b,Z,q1,Z)\n(q1,$,Z,qF,$)\n(q1,a,Z,q1,Z)\n(q1,b,Z,q1,Z)";
const DEF: &str = "M=({q0,q1,qF},{a,b},{Z},p,q0,Z,{qF})";
const DEF_ABZ: &str = "M=({q0,q1,qF},{a,b},{A,B,Z},p,q0,Z,{qF})";
const AA: &str = "(q0,aa,Z)\n(q1,a,Z)\n(q1,$,Z)\n(qF,$,$)";
const ABA: &str = "(q0,aba,Z)\n(q1,ba,Z)\n(q1,a,Z)\n(q1,$,Z)\n(qF,$,$)";

fn sheet(rules: &str, definition: &str, aa: &str, aba: &str) -> String {
    format!(
        "学籍番号: 00X00000\n氏名: ダミー\n\n14-1=====\n{rules}\n========\n\n14-2=====\n{definition}\n========\n\n14-3=====\n{aa}\n========\n\n14-4=====\n{aba}\n========\n"
    )
}

fn base() -> String {
    sheet(RULES, DEF, AA, ABA)
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
fn assignment_registers_and_toy_machine_passes_local_and_trace_checks_without_language_grading() {
    let spec = assignments::get("14").unwrap();
    assert_eq!(spec.comment_prefix, "%%%");
    assert_eq!(
        spec.sections.iter().map(|s| s.marker).collect::<Vec<_>>(),
        ["14-1=====", "14-2=====", "14-3=====", "14-4====="]
    );
    let input = base();
    let r = check("14", input.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.warnings(), 0, "{:?}", r.issues);
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    assert!(r.hash.is_some());
}

#[test]
fn tuple_sorting_and_set_sorting_are_independent_and_never_reorder_traces_or_remove_duplicates() {
    let rules = RULES.lines().rev().collect::<Vec<_>>().join("\n");
    let def = "M=({qF,q1,q0},{b,a},{Z,B,A,A},p,q0,Z,{qF})";
    let input = sheet(&rules, def, AA, ABA);
    let r = check("14", input.as_bytes());
    assert!(has(&r, "pda-transition-sort", Severity::Error));
    assert!(has(&r, "pda-set-sort", Severity::Error));
    assert!(has(&r, "pda-set-duplicate", Severity::Warning));
    let expected = sheet(RULES, "M=({q0,q1,qF},{a,b},{A,A,B,Z},p,q0,Z,{qF})", AA, ABA);
    assert_eq!(r.normalized.as_deref(), Some(expected.as_str()));
    assert_eq!(check("14", expected.as_bytes()).hash, r.hash);
    let repeated = sheet(
        &RULES.replace("(q1,a,Z,q1,Z)", "(q1,a,Z,q1,Z)\n(q1,a,Z,q1,Z)"),
        DEF,
        AA,
        ABA,
    );
    let r = check("14", repeated.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert!(has(&r, "pda-transition-duplicate", Severity::Warning));
    assert_eq!(r.normalized.as_deref(), Some(repeated.as_str()));
}

#[test]
fn field_comparison_uses_ascii_strings_rather_than_numbers_or_whole_line_punctuation() {
    // Invalid state names remain errors, but their fields still sort correctly.
    let rules = "(q2,a,Z,q1,Z)\n(q11,a,Z,q1,Z)\n(q$,a,Z,q1,Z)\n(q,a,Z,q1,Z)";
    let expected = "(q,a,Z,q1,Z)\n(q$,a,Z,q1,Z)\n(q11,a,Z,q1,Z)\n(q2,a,Z,q1,Z)";
    let r = check("14", sheet(rules, DEF, AA, ABA).as_bytes());
    assert!(has(&r, "pda-transition-sort", Severity::Error));
    assert!(r.normalized.unwrap().contains(expected));
    // Malformed wrappers can have equal field keys; raw-text tie-breaking
    // keeps normalization deterministic and idempotent even with errors.
    let malformed = std::iter::repeat_n("q0,a,Z,q1,Z\n(q0,a,Z,q1,Z)", 40)
        .collect::<Vec<_>>()
        .join("\n");
    let r = check("14", sheet(&malformed, DEF, AA, ABA).as_bytes());
    let again = check("14", r.normalized.as_ref().unwrap().as_bytes());
    assert_eq!(again.normalized, r.normalized);
    assert_eq!(again.hash, r.hash);
}

#[test]
fn tuple_definition_and_configuration_syntax_are_strict_and_do_not_guess_missing_fields() {
    for value in [
        "(q0,a,Z,q1)",
        "(q0,a,Z,q1,Z,Z)",
        "(q0,,Z,q1,Z)",
        "(q0,aa,Z,q1,Z)",
        "(q0,a,AZ,q1,Z)",
        "p(q0,a,Z)={(q1,Z)}",
        "(q0,a,Z,q1,Z) (q0,b,Z,q1,Z)",
    ] {
        let input = sheet(value, DEF, AA, ABA);
        let r = check("14", input.as_bytes());
        assert!(has(&r, "pda-transition", Severity::Error), "{value}");
        assert!(has(&r, "pda-trace-unchecked", Severity::Warning));
        assert!(r.hash.is_some());
    }
    for value in [
        "({q0,q1,qF},{a,b},{Z},p,q0,Z,{qF})",
        "M=({q0,q1,qF},{a,b},{Z},p,q0,{qF})",
        "M=({{q0},q1,qF},{a,b},{Z},p,q0,Z,{qF})",
        "M=({q0,q1,qF},{a,b},{Z},p,q0,Z,{qF},Z)",
    ] {
        let input = sheet(RULES, value, AA, ABA);
        let r = check("14", input.as_bytes());
        assert!(has(&r, "pda-definition", Severity::Error), "{value}");
        assert!(has(&r, "pda-trace-unchecked", Severity::Warning));
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
    let r = check(
        "14",
        sheet(RULES, &format!("{DEF}\n{DEF}"), AA, ABA).as_bytes(),
    );
    assert!(has(&r, "pda-definition-lines", Severity::Error));
    for value in [
        "(q0,,Z)",
        "(q0,aa,)",
        "(q0,aa,Z,extra)",
        "(q0,c,Z)",
        "(q0,aa,C)",
        "(q0,aa,Z)⊢(q1,a,Z)",
    ] {
        let r = check("14", sheet(RULES, DEF, value, ABA).as_bytes());
        assert!(has(&r, "pda-configuration", Severity::Error), "{value}");
    }
}

#[test]
fn epsilon_bottom_and_space_errors_are_reported_without_replacing_answer_symbols() {
    for (value, code) in [
        ("(q0,ε,Z,q1,Z)", "pda-epsilon"),
        ("(q0,ϵ,Z,q1,Z)", "pda-epsilon"),
        ("(q0,a,⊥,q1,Z)", "pda-bottom"),
        ("(q0,a,Z, q1,Z)", "forbidden-space"),
        ("(q0,a,Z,q1,\tZ)", "forbidden-space"),
    ] {
        let input = sheet(value, DEF, AA, ABA);
        let r = check("14", input.as_bytes());
        assert!(has(&r, code, Severity::Error), "{value}");
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
    let input = sheet(RULES, DEF, "(q0,aa,Z)⊢(q1,a,Z)", ABA);
    assert!(has(
        &check("14", input.as_bytes()),
        "pda-turnstile",
        Severity::Error
    ));
}

#[test]
fn local_stack_rules_check_preservation_push_pop_epsilon_and_entry_to_final() {
    for (rule, code) in [
        ("(q2,a,Z,q1,Z)", "mechanical-pda-state"),
        ("(q0,c,Z,q1,Z)", "mechanical-pda-input"),
        ("(q0,a,C,q1,C)", "mechanical-pda-stack"),
        ("(q0,a,Z,q1,A$Z)", "mechanical-pda-stack"),
        ("(q0,a,Z,q0,A)", "mechanical-pda-q0-stack"),
        ("(q0,$,Z,q1,AZ)", "mechanical-pda-epsilon-stack"),
        ("(q1,a,A,q1,B)", "mechanical-pda-q1-stack"),
        ("(q1,a,Z,q1,AZ)", "mechanical-pda-q1-stack"),
        ("(q1,$,Z,q0,$)", "mechanical-pda-epsilon-stack"),
        ("(q0,a,Z,qF,$)", "mechanical-pda-final-edge"),
        ("(q1,a,A,qF,$)", "mechanical-pda-final-edge"),
        ("(q1,$,Z,qF,Z)", "mechanical-pda-final-edge"),
        ("(qF,a,A,q0,Z)", "mechanical-pda-bottom-stack"),
    ] {
        let rules = sorted(&format!("{RULES}\n{rule}"));
        let r = check("14", sheet(&rules, DEF_ABZ, AA, ABA).as_bytes());
        assert!(has(&r, code, Severity::Error), "{rule}: {:?}", r.issues);
    }
    // No invented single-push, monotonic-phase, or epsilon-only-final rule.
    for rule in [
        "(q0,a,A,q0,BA)",
        "(q0,a,Z,q0,ABZ)",
        "(q0,$,Z,q1,Z)",
        "(q1,$,A,q0,A)",
        "(q1,a,Z,qF,$)",
    ] {
        let rules = sorted(&format!("{RULES}\n{rule}"));
        let r = check("14", sheet(&rules, DEF_ABZ, AA, ABA).as_bytes());
        assert_eq!(r.errors(), 0, "{rule}: {:?}", r.issues);
    }
}

#[test]
fn declared_components_and_references_are_checked_across_sections() {
    for (value, code) in [
        (DEF.replace("q0,q1,qF", "q0,q1"), "mechanical-pda-states"),
        (DEF.replace("{a,b}", "{a}"), "mechanical-pda-alphabet"),
        (DEF.replace("{Z}", "{A}"), "mechanical-pda-stack-alphabet"),
        (
            DEF.replace(",p,q0,Z,", ",t,q0,Z,"),
            "mechanical-pda-definition",
        ),
        (
            DEF.replace(",p,q0,Z,", ",p,q1,Z,"),
            "mechanical-pda-definition",
        ),
        (DEF.replace("{qF}", "{q1}"), "mechanical-pda-definition"),
    ] {
        let r = check("14", sheet(RULES, &value, AA, ABA).as_bytes());
        assert!(has(&r, code, Severity::Error), "{value}");
    }
    let rules = sorted(&format!("{RULES}\n(q0,a,A,q0,BA)"));
    let r = check("14", sheet(&rules, DEF, AA, ABA).as_bytes());
    assert!(has(&r, "mechanical-pda-declaration", Severity::Error));
    let trace = AA.replace("(q1,a,Z)", "(q1,a,AZ)");
    let r = check("14", sheet(RULES, DEF, &trace, ABA).as_bytes());
    assert!(has(
        &r,
        "mechanical-pda-config-declaration",
        Severity::Error
    ));
}

#[test]
fn trace_initial_word_initial_stack_final_state_and_consumption_are_checked() {
    for bad in [
        AA.replace("(q0,aa,Z)", "(q1,aa,Z)"),
        AA.replace("(q0,aa,Z)", "(q0,ab,Z)"),
        AA.replace("(q0,aa,Z)", "(q0,aa,AZ)"),
    ] {
        let r = check("14", sheet(RULES, DEF_ABZ, &bad, ABA).as_bytes());
        assert!(has(&r, "mechanical-pda-trace-start", Severity::Error));
    }
    for bad in [
        AA.replace("(qF,$,$)", "(q1,$,Z)"),
        AA.replace("(qF,$,$)", "(q1,$,$)"),
        AA.replace("(qF,$,$)", "(qF,a,$)"),
    ] {
        let r = check("14", sheet(RULES, DEF, &bad, ABA).as_bytes());
        assert!(has(&r, "mechanical-pda-trace-end", Severity::Error));
    }
    let r = check("14", sheet(RULES, DEF, AA, AA).as_bytes());
    assert!(has(&r, "mechanical-pda-trace-start", Severity::Error));
    // A nonempty stack alone isn't a T(M) acceptance violation; this fixture
    // still fails because the preceding STEP doesn't perform the listed pop.
    let r = check(
        "14",
        sheet(RULES, DEF, &AA.replace("(qF,$,$)", "(qF,$,Z)"), ABA).as_bytes(),
    );
    assert!(has(&r, "mechanical-pda-trace-step", Severity::Error));
    assert!(!has(&r, "mechanical-pda-trace-end", Severity::Error));
}

#[test]
fn adjacent_trace_rows_must_be_exactly_one_listed_transition_and_keep_their_order() {
    for bad in [
        AA.replace("(q1,a,Z)\n", ""),
        AA.replace("(q1,a,Z)", "(q1,aa,Z)"),
        AA.replace("(q1,$,Z)", "(q1,$,$)"),
    ] {
        let input = sheet(RULES, DEF, &bad, ABA);
        let r = check("14", input.as_bytes());
        assert!(has(&r, "mechanical-pda-trace-step", Severity::Error));
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
    let rules = RULES.replace("(q1,a,Z,q1,Z)\n", "");
    let r = check("14", sheet(&rules, DEF, AA, ABA).as_bytes());
    assert!(has(&r, "mechanical-pda-trace-step", Severity::Error));
    let issue = r
        .issues
        .iter()
        .find(|i| i.code == "mechanical-pda-trace-step")
        .unwrap();
    let input = sheet(&rules, DEF, AA, ABA);
    assert_eq!(
        issue.line,
        input
            .lines()
            .position(|line| line == "(q1,$,Z)")
            .map(|i| i + 1)
    );
}

#[test]
fn a_long_explicit_stack_and_large_duplicate_declarations_are_checked_without_search() {
    let push = format!("{}Z", "A".repeat(12000));
    let rules = sorted(&format!("{RULES}\n(q0,a,Z,q0,{push})"));
    let states = std::iter::repeat_n("q0", 10000)
        .collect::<Vec<_>>()
        .join(",");
    let def = format!("M=({{{states},q1,qF}},{{a,b}},{{A,B,Z}},p,q0,Z,{{qF}})");
    let trace = format!("(q0,aa,Z)\n(q0,a,{push})");
    let r = check("14", sheet(&rules, &def, &trace, ABA).as_bytes());
    assert!(!has(&r, "mechanical-pda-trace-step", Severity::Error));
    assert!(has(&r, "mechanical-pda-trace-end", Severity::Error));
    assert!(has(&r, "pda-set-duplicate", Severity::Warning));
}

#[test]
fn comments_identity_newlines_and_sorting_keep_hash_v1_idempotent() {
    let input = base();
    let baseline = check("14", input.as_bytes());
    assert_eq!(HASH_VERSION, "answer-text-v1");
    assert_eq!(
        baseline.hash,
        Some(sha256(input.split_once("\n\n").unwrap().1))
    );
    let mut annotated = input
        .replace("00X00000", "99Z99999")
        .replace("ダミー", "別のダミー");
    for section in assignments::get("14").unwrap().sections {
        annotated = annotated.replace(
            &format!("{}\n", section.marker),
            &format!("%%%% 指示\n{}\n  %%% コメント\n", section.marker),
        );
    }
    for changed in [
        annotated.clone(),
        annotated.replace('\n', "\r\n"),
        annotated.replace('\n', "\r"),
    ] {
        let r = check("14", format!("\u{feff}{changed}").as_bytes());
        assert_eq!(r.errors(), 0, "{:?}", r.issues);
        assert_eq!(r.hash, baseline.hash);
        assert_eq!(r.hash_input, baseline.hash_input);
        assert!(!r.issues.iter().any(|i| i.code == "comment"));
        assert!(!r.normalized.as_ref().unwrap().contains("コメント"));
        let again = check("14", r.normalized.as_ref().unwrap().as_bytes());
        assert_eq!(again.normalized, r.normalized);
        assert_eq!(again.hash, r.hash);
    }
}

#[test]
fn empty_sections_damaged_markers_and_wrong_assignment_are_reported() {
    let empty = sheet("", "", "", "");
    assert_eq!(
        check("14", empty.as_bytes())
            .issues
            .iter()
            .filter(|i| i.code == "unanswered")
            .count(),
        4
    );
    for bad in [
        base().replace("14-1=====", "14-1===="),
        base().replace("14-3=====", "14-3===== "),
        base().replace("14-4=====", "14-3====="),
    ] {
        let r = check("14", bad.as_bytes());
        assert!(r.errors() > 0);
        assert!(r.hash.is_none());
        assert!(r.normalized.is_none());
    }
    assert!(check("13", base().as_bytes()).hash.is_none());
    let blanks = sheet(&format!("\n{RULES}\n"), DEF, &AA.replace('\n', "\n\n"), ABA);
    let r = check("14", blanks.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert!(has(&r, "inner-blank", Severity::Warning));
    assert_eq!(r.normalized.as_deref(), Some(base().as_str()));
}
