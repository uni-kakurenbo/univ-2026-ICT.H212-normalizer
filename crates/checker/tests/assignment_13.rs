use answer_checker::{HASH_VERSION, Severity, assignments, check, sha256};

// Synthetic notation examples only, not proofs for either question.
const A13: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n13-1=====\n検査用の文章。u=x^{n+1}、|u|>=1 と書く。\n\nu in X、u not in Y、0<=k<n は表記の例。\n========\n\n13-2=====\n別の文章。v=y^{|u|+2}、v!=u と書く。\n集合 {x,y}^{+} も表記の例。\n========\n";

fn has(text: &str, code: &str, severity: Severity) -> bool {
    check("13", text.as_bytes())
        .issues
        .iter()
        .any(|i| i.code == code && i.severity == severity)
}

#[test]
fn assignment_is_registered_and_accepts_japanese_and_ascii_math_without_grading_proofs() {
    let spec = assignments::get("13").unwrap();
    assert_eq!(spec.comment_prefix, "%%%");
    assert_eq!(
        spec.sections.iter().map(|s| s.marker).collect::<Vec<_>>(),
        ["13-1=====", "13-2====="]
    );
    let r = check("13", A13.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.warnings(), 0, "{:?}", r.issues);
    assert_eq!(r.normalized.as_deref(), Some(A13));
    assert!(!r.issues.iter().any(|i| i.code.starts_with("mechanical-")));
    let arbitrary = A13.replace(
        "検査用の文章。u=x^{n+1}、|u|>=1 と書く。",
        "証明ではない検査用の文章だけでも内容は判定しない。",
    );
    assert_eq!(check("13", arbitrary.as_bytes()).errors(), 0);
}

#[test]
fn prose_spaces_indentation_paragraphs_and_order_are_preserved_in_hash_v1() {
    let input = A13.replace(
        "検査用の文章。",
        "  日本語の  空白を保持。  \n\n次の段落。\n",
    );
    let r = check("13", input.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    assert!(has(&input, "edge-space", Severity::Warning));
    assert_eq!(HASH_VERSION, "answer-text-v1");
    let body = input.split_once("\n\n").unwrap().1;
    assert_eq!(r.hash_input.as_deref(), Some(body));
    assert_eq!(r.hash, Some(sha256(body)));
    assert_ne!(r.hash, check("13", A13.as_bytes()).hash);
    let again = check("13", r.normalized.as_ref().unwrap().as_bytes());
    assert_eq!(again.normalized, r.normalized);
    assert_eq!(again.hash, r.hash);
}

#[test]
fn explicit_exponent_comparison_and_membership_errors_are_not_autocorrected() {
    for (value, code) in [
        ("u≤n", "math-comparison"),
        ("u≥n", "math-comparison"),
        ("u≠n", "math-comparison"),
        ("u＝n", "math-comparison"),
        ("u∈L", "math-membership"),
        ("u∉L", "math-membership"),
        ("x^k", "exponent-braces"),
        ("u^{k", "exponent-braces"),
        ("{x,y}^+", "exponent-braces"),
        ("x^{}", "exponent-braces"),
        ("x^{ }", "exponent-braces"),
    ] {
        let input = A13.replace("u=x^{n+1}", value);
        let r = check("13", input.as_bytes());
        assert!(has(&input, code, Severity::Error), "{value}");
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
        assert!(r.hash.is_some());
        let issue = r.issues.iter().find(|i| i.code == code).unwrap();
        assert_eq!(issue.line, Some(5));
    }
}

#[test]
fn uncertain_math_candidates_only_warn_including_unspecified_empty_word_notation() {
    for value in [
        "α とおく",
        "x²",
        "xₙ",
        "𝑥ₙ",
        "x^{文字数}",
        "|文字列|",
        "空語候補 ε",
        "空語候補 ϵ",
        "空語候補 $",
        "記号 ^ の説明",
        "\\forall の説明",
        "１つの例",
        "文章 → 次の文章",
    ] {
        let input = A13.replace("検査用の文章。", value);
        let r = check("13", input.as_bytes());
        assert_eq!(r.errors(), 0, "{value}: {:?}", r.issues);
        assert!(has(&input, "math-possible", Severity::Warning), "{value}");
        assert!(!has(&input, "proof-empty-word", Severity::Error));
        assert!(!r.issues.iter().any(|i| i.message.contains("eps")));
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
    for value in ["空語を eps と記す", "空語を lambda と記す"] {
        let input = A13.replace("検査用の文章。", value);
        assert_eq!(check("13", input.as_bytes()).errors(), 0);
    }
}

#[test]
fn assignment_07_keeps_its_explicit_empty_word_requirement_and_existing_hash() {
    let seven = A13
        .replace("13-1=====", "7-1=====")
        .replace("13-2=====", "7-2=====");
    let r = check("07", seven.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.normalized.as_deref(), Some(seven.as_str()));
    assert_eq!(r.hash, Some(sha256(seven.split_once("\n\n").unwrap().1)));
    for value in ["ε", "ϵ"] {
        let input = seven.replace("検査用の文章。", value);
        let report = check("07", input.as_bytes());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "proof-empty-word" && i.severity == Severity::Error)
        );
        assert!(!report.issues.iter().any(|i| i.code == "math-possible"));
        assert!(report.issues.iter().any(|i| i.message.contains("eps")));
    }
}

#[test]
fn comments_identity_and_line_endings_do_not_change_hash_or_leave_comment_warnings() {
    let base = check("13", A13.as_bytes());
    let mut annotated = A13
        .replace("00X00000", "99Z99999")
        .replace("ダミー", "別のダミー");
    for marker in ["13-1=====", "13-2====="] {
        annotated = annotated.replace(
            &format!("{marker}\n"),
            &format!("%%%% 見出し\n{marker}\n  %%% 注意書き\n"),
        );
    }
    for value in [
        annotated.clone(),
        annotated.replace('\n', "\r\n"),
        annotated.replace('\n', "\r"),
    ] {
        let r = check("13", format!("\u{feff}{value}").as_bytes());
        assert_eq!(r.errors(), 0, "{:?}", r.issues);
        assert_eq!(r.hash, base.hash);
        assert_eq!(r.hash_input, base.hash_input);
        assert!(!r.issues.iter().any(|i| i.code == "comment"));
        assert!(!r.normalized.as_ref().unwrap().contains("注意書き"));
        assert_eq!(
            check("13", r.normalized.unwrap().as_bytes()).hash,
            base.hash
        );
    }
    let hash_line = A13.replace("検査用の文章。", "# はコメントとして消さず保持。\n");
    let r = check("13", hash_line.as_bytes());
    assert!(
        r.normalized
            .unwrap()
            .contains("# はコメントとして消さず保持。")
    );
    assert_ne!(r.hash, base.hash);
}

#[test]
fn empty_sections_wrong_markers_and_wrong_assignments_are_rejected() {
    let empty = "学籍番号: 00X00000\n氏名: ダミー\n\n13-1=====\n%%% 指示のみ\n========\n\n13-2=====\n========\n";
    assert_eq!(
        check("13", empty.as_bytes())
            .issues
            .iter()
            .filter(|i| i.code == "unanswered")
            .count(),
        2
    );
    for damaged in [
        A13.replace("13-1=====", "13-1===="),
        A13.replace("13-2=====", "13-2===== "),
        A13.replace("13-2=====", "13-1====="),
    ] {
        let r = check("13", damaged.as_bytes());
        assert!(r.errors() > 0);
        assert!(r.hash.is_none());
        assert!(r.normalized.is_none());
    }
    assert!(check("07", A13.as_bytes()).hash.is_none());
    assert!(
        check("13", A13.replace("13-1=====", "7-1=====").as_bytes())
            .hash
            .is_none()
    );
}

#[test]
fn deeply_nested_and_unfinished_exponents_are_checked_without_recursion_or_repeated_warnings() {
    let nested = format!("{}1{}", "x^{".repeat(5000), "}".repeat(5000));
    let r = check("13", A13.replace("x^{n+1}", &nested).as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    let broken = A13.replace("x^{n+1}", &"x^{".repeat(5000));
    let r = check("13", broken.as_bytes());
    assert_eq!(
        r.issues
            .iter()
            .filter(|i| i.code == "exponent-braces")
            .count(),
        1
    );
    let uncertain = A13.replace("検査用の文章。", "α・β・γ・⇒・a²");
    assert_eq!(
        check("13", uncertain.as_bytes())
            .issues
            .iter()
            .filter(|i| i.code == "math-possible")
            .count(),
        1
    );
}
