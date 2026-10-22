use answer_checker::{HASH_VERSION, Severity, assignments, check, sha256};

// Synthetic syntax only; intentionally unrelated to the assignment's NFA.
const WORK: &str =
    "X = bY + 1\nY = aZ\nZ = b\n  = (b)\n\nX = a (Y を代入したという検査用の説明)\na";

fn sheet(work: &str) -> String {
    format!("学籍番号: 00X00000\n氏名: ダミー\n\n9-1=====\n{work}\n========\n")
}

fn has(input: &str, code: &str, severity: Severity) -> bool {
    check("09", input.as_bytes())
        .issues
        .iter()
        .any(|i| i.code == code && i.severity == severity)
}

#[test]
fn assignment_is_registered_and_accepts_format_without_grading_equations() {
    let spec = assignments::get("09").unwrap();
    assert_eq!(spec.sections.len(), 1);
    assert_eq!(spec.sections[0].marker, "9-1=====");
    assert_eq!(spec.comment_prefix, "%%%");
    let input = sheet(WORK);
    let report = check("09", input.as_bytes());
    assert_eq!(report.errors(), 0, "{:?}", report.issues);
    assert_eq!(report.warnings(), 0, "{:?}", report.issues);
    assert_eq!(report.normalized.as_deref(), Some(input.as_str()));
    assert!(
        !report
            .issues
            .iter()
            .any(|i| i.code.starts_with("mechanical-"))
    );
}

#[test]
fn final_line_checks_alphabet_and_syntax_without_changing_answer() {
    for valid in ["a", "b", "1", "(a+b)*1", "a(b+1)", "a**", "((a))"] {
        let input = sheet(&format!("{}{valid}", WORK.strip_suffix('a').unwrap()));
        assert!(!has(&input, "final-regex", Severity::Error), "{valid}");
    }
    for invalid in [
        "0",
        "c",
        "X",
        "e",
        "a|b",
        "$",
        "ε",
        "a∪b",
        "a b",
        "a\tb",
        "X=a",
        "a (answer)",
        "a（答え）",
        "a+",
        "*a",
        "()",
        "(a",
        "a)",
        "a(+b)",
        "a^*",
        "a,b",
        "a => b",
        " a",
        "a ",
    ] {
        let input = sheet(&format!("{}{invalid}", WORK.strip_suffix('a').unwrap()));
        let report = check("09", input.as_bytes());
        assert!(has(&input, "final-regex", Severity::Error), "{invalid}");
        assert_eq!(report.normalized.as_deref(), Some(input.as_str()));
        assert!(report.hash.is_some());
    }
}

#[test]
fn explanations_in_japanese_or_english_and_derivation_order_survive() {
    for line in [
        "X = a (Y を代入)",
        "X = a (substitute Y)",
        "X = a (Y の式に X=eX+e' => X=e*e' を使う)",
        "X = a（Y を代入）",
        "X = a 日本語の説明",
        "操作の説明。ε などの記号についても文章では自由に記載する。",
        "Substitute Y into X.",
    ] {
        let input = sheet(&WORK.replace("X = a (Y を代入したという検査用の説明)", line));
        let r = check("09", input.as_bytes());
        assert_eq!(r.errors(), 0, "{line}: {:?}", r.issues);
        assert_eq!(r.warnings(), 0, "{line}: {:?}", r.issues);
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
    let input = sheet(&WORK.replace("X = bY + 1\nY = aZ\nZ = b", "Z = b\nY = aZ\nX = bY + 1"));
    let r = check("09", input.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    assert_ne!(r.hash, check("09", sheet(WORK).as_bytes()).hash);
}

#[test]
fn identifiable_equations_check_operators_parentheses_and_ascii_notation() {
    for invalid in [
        "a+",
        "*a",
        "()",
        "(a",
        "a)",
        "a(+b)",
        "a|b",
        "aY;Y=b",
        "cY",
        "$",
        "eps",
        "a>>b",
        "ε",
        "ϵ",
        "aY → bZ",
        "aY ⇒ bZ",
        "a∪b",
        "ａY",
        "（a+b）Y",
        "a (ε)",
        "a (c)",
        "a | b",
        "a $",
        "a ^ 2",
    ] {
        let input = sheet(&WORK.replace("X = bY + 1", &format!("X = {invalid}")));
        assert!(has(&input, "equation-syntax", Severity::Error), "{invalid}");
    }
    for valid in ["aY = bZ", "aY => bZ", "aY=>bZ=>1", "0", "(a+1)*X"] {
        let input = sheet(&WORK.replace("X = bY + 1", &format!("X = {valid}")));
        let r = check("09", input.as_bytes());
        assert_eq!(r.errors(), 0, "{valid}: {:?}", r.issues);
    }
    let unicode_equals = sheet(&WORK.replace("X = bY + 1", "X ＝ bY + 1"));
    assert!(has(&unicode_equals, "equation-syntax", Severity::Error));
}

#[test]
fn ambiguous_boundaries_warn_and_do_not_rewrite_or_reject_prose() {
    for ambiguous in [
        "X=aYを代入",
        "X=aY substitute Z",
        "X の式に Y を代入",
        "Q=aY",
    ] {
        let input = sheet(&WORK.replace("X = a (Y を代入したという検査用の説明)", ambiguous));
        let r = check("09", input.as_bytes());
        assert_eq!(r.errors(), 0, "{ambiguous}: {:?}", r.issues);
        assert!(
            has(&input, "derivation-ambiguous", Severity::Warning),
            "{ambiguous}"
        );
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
}

#[test]
fn requires_work_and_each_named_equation_but_only_warns_about_missing_explanations() {
    assert!(has(&sheet("a"), "derivation-missing", Severity::Error));
    let missing = sheet(&WORK.replace("Y = aZ\n", ""));
    assert!(has(&missing, "equation-variables", Severity::Error));
    let ambiguous = missing.replace("X = a (Y を代入したという検査用の説明)", "Y=aZを代入");
    assert!(!has(&ambiguous, "equation-variables", Severity::Error));
    let only_equations = sheet("X=a\nY=b\nZ=1\na");
    let r = check("09", only_equations.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert!(has(
        &only_equations,
        "derivation-explanation",
        Severity::Warning
    ));
    assert!(has(&sheet(""), "unanswered", Severity::Error));
}

#[test]
fn catches_orphan_continuations_and_multiple_leading_equals() {
    let orphan = sheet(&format!("  = a\n{WORK}"));
    assert!(has(&orphan, "equation-continuation", Severity::Error));
    for invalid in ["==a", "=== a", "  == a"] {
        let input = sheet(&WORK.replace("  = (b)", invalid));
        assert!(has(&input, "leading-equals", Severity::Error), "{invalid}");
        assert!(check("09", input.as_bytes()).hash.is_some());
    }
    // Bare delimiter-like rows are ambiguous to the common section parser.
    let damaged = sheet(&WORK.replace("  = (b)", "=="));
    assert!(check("09", damaged.as_bytes()).hash.is_none());
}

#[test]
fn keeps_indentation_paragraphs_trailing_spaces_and_duplicates_in_hash_v1() {
    let input = sheet(&WORK.replace("  = (b)", "    = (b)  \n\nX = a\nX = a"));
    let r = check("09", input.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert!(has(&input, "edge-space", Severity::Warning));
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    assert_eq!(HASH_VERSION, "answer-text-v1");
    let body = input.split_once("\n\n").unwrap().1;
    assert_eq!(r.hash_input.as_deref(), Some(body));
    assert_eq!(r.hash, Some(sha256(body)));
    let again = check("09", r.normalized.as_ref().unwrap().as_bytes());
    assert_eq!(again.normalized, r.normalized);
    assert_eq!(again.hash, r.hash);
    let tabbed = sheet(&WORK.replace("  = (b)", "\t= (b)"));
    assert!(has(&tabbed, "tab", Severity::Warning));
    assert_eq!(check("09", tabbed.as_bytes()).normalized, Some(tabbed));
}

#[test]
fn comments_identity_and_encoding_normalization_do_not_change_hash() {
    let base = sheet(WORK);
    let baseline = check("09", base.as_bytes());
    let annotated = base
        .replace("00X00000", "99Z99999")
        .replace("ダミー", "別のダミー")
        .replace("9-1=====\n", "%%%% 指示\n9-1=====\n%%% 指示\n")
        .replace("\na\n========", "\na\n  %%% 指示\n========");
    for changed in [
        annotated.clone(),
        annotated.replace('\n', "\r\n"),
        annotated.replace('\n', "\r"),
    ] {
        let r = check("09", format!("\u{feff}{changed}").as_bytes());
        assert_eq!(r.errors(), 0, "{:?}", r.issues);
        assert_eq!(r.hash, baseline.hash);
        assert_eq!(r.hash_input, baseline.hash_input);
        assert!(!r.issues.iter().any(|i| i.code == "comment"));
        assert!(!r.normalized.as_ref().unwrap().contains("指示"));
        assert_eq!(
            check("09", r.normalized.unwrap().as_bytes()).hash,
            baseline.hash
        );
    }
    let hash_row = base.replace("\nX = a (", "\n# 保持する文章\nX = a (");
    assert!(
        check("09", hash_row.as_bytes())
            .normalized
            .unwrap()
            .contains("# 保持する文章")
    );
    assert_ne!(check("09", hash_row.as_bytes()).hash, baseline.hash);
}

#[test]
fn requires_final_regex_after_explanations_and_ignores_trailing_comments() {
    let final_explanation = sheet(&format!("{WORK}\n最後の説明"));
    assert!(has(&final_explanation, "final-regex", Severity::Error));
    let comments = sheet(&format!("{WORK}\n%%% 最後の注意書き"));
    assert_eq!(
        check("09", comments.as_bytes()).hash,
        check("09", sheet(WORK).as_bytes()).hash
    );
}

#[test]
fn broken_markers_wrong_assignment_and_invisible_characters_are_reported() {
    let base = sheet(WORK);
    for input in [
        base.replace("9-1=====", "9-1===="),
        base.replace("9-1=====", "9-1===== "),
    ] {
        let r = check("09", input.as_bytes());
        assert!(r.errors() > 0);
        assert!(r.hash.is_none());
        assert!(r.normalized.is_none());
    }
    assert!(check("08", base.as_bytes()).hash.is_none());
    assert!(has(
        &base.replace("Z = b", "Z = b\u{200b}"),
        "invisible",
        Severity::Error
    ));
}

#[test]
fn deeply_nested_formulas_are_checked_without_recursion() {
    let nested = format!("{}a{}", "(".repeat(20000), ")".repeat(20000));
    let input = sheet(&format!("X={nested}\nY=b\nZ=1\n検査用の説明\n{nested}"));
    let r = check("09", input.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.warnings(), 0, "{:?}", r.issues);
    let broken = input.replacen(&format!("X={nested}"), &format!("X=({nested}"), 1);
    assert!(has(&broken, "equation-syntax", Severity::Error));
}
