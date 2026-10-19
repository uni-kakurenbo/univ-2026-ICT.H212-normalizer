use answer_checker::{Severity, check};

// These older fixtures isolate notation, normalization and numbering. They are
// intentionally not full solutions; construction/calculation diagnostics are
// tested separately against small, hand-written examples in the rule modules.
trait FormattingReport {
    fn format_errors(&self) -> usize;
}
impl FormattingReport for answer_checker::Report {
    fn format_errors(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| {
                i.severity == answer_checker::Severity::Error && !i.code.starts_with("mechanical-")
            })
            .count()
    }
}

// Synthetic notation examples, not proofs or automata for the assignment.
const A07: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n7-1=====\n検査用の文章。u=x^{n+1}、|u|>=1 と記す。\n\nu in X と u not in Y は記法の例。\n========\n\n7-2=====\n別の文章。v=y^{|u|}、eps、0<=k<n と書く。\n========\n";
const FORMS: &str = "lf(e)={<y,(x+y)*>,<x,1>}\nlf(1)={}\ni=0,K={},Delta={e}\ni=1,K={e},Delta={1,(x+y)*}\ni=2,K={1,(x+y)*,e},Delta={}";
const A08: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n8-1.1=====\n2\n3\n\n(1,x,3)\n(2,$,1)\n(2,y,3)\n========\n\n8-1.2.1=====\nlf(e)={<y,(x+y)*>,<x,1>}\nlf(1)={}\ni=0,K={},Delta={e}\ni=1,K={e},Delta={1,(x+y)*}\ni=2,K={1,(x+y)*,e},Delta={}\n========\n\n8-1.2.2=====\n(x+y)*x\n(x+y)*,1\n\n((x+y)*,x,1)\n(1,y,e)\n(e,y,(x+y)*)\n========\n";

fn has(id: &str, text: &str, code: &str) -> bool {
    check(id, text.as_bytes())
        .issues
        .iter()
        .any(|issue| issue.code == code)
}

#[test]
fn synthetic_fixtures_keep_format_and_hash_with_independent_mechanical_diagnostics() {
    for (id, input) in [("07", A07), ("08", A08)] {
        let r = check(id, input.as_bytes());
        assert_eq!(r.format_errors(), 0, "{id}: {:?}", r.issues);
        if id == "08" {
            assert!(r.issues.iter().any(|i| i.code == "mechanical-alphabet"));
        }
        // The old x/y fixture is rejected by the new alphabet check, while
        // its original formatting and canonical output remain unchanged.
        assert_eq!(r.warnings(), 0, "{id}: {:?}", r.issues);
        assert_eq!(r.normalized.as_deref(), Some(input));
        assert!(r.hash.is_some());
    }
}

#[test]
fn proof_preserves_japanese_indentation_spaces_paragraphs_and_order() {
    let input = A07.replace(
        "検査用の文章。",
        "  日本語の文章と  空白を保持。  \n\n次の段落。\n",
    );
    let r = check("07", input.as_bytes());
    assert_eq!(r.format_errors(), 0);
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    assert!(!r.issues.iter().any(|issue| issue.code == "ascii"));
    assert_ne!(r.hash, check("07", A07.as_bytes()).hash);
}

#[test]
fn explicit_proof_notation_violations_are_errors_without_rewriting() {
    for (value, code) in [
        ("u≤n", "math-comparison"),
        ("u≠n", "math-comparison"),
        ("u∈L", "math-membership"),
        ("u∉L", "math-membership"),
        ("u=ε", "proof-empty-word"),
        ("u=ϵ", "proof-empty-word"),
        ("x^k", "exponent-braces"),
        ("x^{k", "exponent-braces"),
        ("x^{}", "exponent-braces"),
        ("u^{ }", "exponent-braces"),
    ] {
        let input = A07.replace("u=x^{n+1}", value);
        let r = check("07", input.as_bytes());
        assert!(
            r.issues
                .iter()
                .any(|issue| issue.code == code && issue.severity == Severity::Error),
            "{value}"
        );
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
}

#[test]
fn uncertain_math_candidates_only_warn_and_never_force_ascii_on_prose() {
    for value in [
        "α とおく",
        "a²",
        "aₙ",
        "𝑥ₙ",
        "x^{文字数}",
        "|文字列|",
        "空語候補 $",
        "記号 ^ の説明",
        "\\forall の説明",
        "１つの例",
        "文章 → 次の文章",
    ] {
        let input = A07.replace("検査用の文章。", value);
        let r = check("07", input.as_bytes());
        assert_eq!(r.format_errors(), 0, "{value}: {:?}", r.issues);
        assert!(
            r.issues
                .iter()
                .any(|issue| issue.severity == Severity::Warning
                    && issue.message.contains("数式の誤表記である可能性があります")),
            "{value}"
        );
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
}

#[test]
fn exponent_notation_handles_nesting_without_semantic_checks() {
    let input = A07.replace("x^{n+1}", "x^{k^{m}+|v|}");
    assert_eq!(check("07", input.as_bytes()).format_errors(), 0);
    let nested = format!("{}1{}", "x^{".repeat(5000), "}".repeat(5000));
    assert_eq!(
        check("07", A07.replace("x^{n+1}", &nested).as_bytes()).format_errors(),
        0
    );
    let unfinished = A07.replace("x^{n+1}", &"x^{".repeat(5000));
    let r = check("07", unfinished.as_bytes());
    assert_eq!(
        r.issues
            .iter()
            .filter(|issue| issue.code == "exponent-braces")
            .count(),
        1
    );
}

#[test]
fn start_states_stay_separate_from_final_states_and_sorted_transitions() {
    let input = A08
        .replace(
            "2\n3\n\n(1,x,3)\n(2,$,1)\n(2,y,3)",
            "2\n\n3\n\n(2,y,3)\n(2,$,1)\n(1,x,3)",
        )
        .replace(
            "(x+y)*,1\n\n((x+y)*,x,1)\n(1,y,e)\n(e,y,(x+y)*)",
            "1,(x+y)*\n\n(e,y,(x+y)*)\n(1,y,e)\n((x+y)*,x,1)",
        );
    let r = check("08", input.as_bytes());
    assert!(r.issues.iter().any(|issue| issue.code == "sort"));
    assert!(
        r.issues
            .iter()
            .any(|issue| issue.code == "final-state-sort")
    );
    assert_eq!(r.normalized.as_deref(), Some(A08));
    assert_eq!(r.hash, check("08", A08.as_bytes()).hash);
    let again = check("08", r.normalized.unwrap().as_bytes());
    assert_eq!(again.format_errors(), 0);
    assert_eq!(again.warnings(), check("08", A08.as_bytes()).warnings());
    assert_eq!(again.hash, r.hash);
}

#[test]
fn numbered_states_are_consecutive_with_one_maximum_final_and_no_renaming() {
    for value in ["0", "01", "-1", "q1", "184467440737095516160000"] {
        let input = A08.replace("(1,x,3)", &format!("({value},x,3)"));
        assert!(
            has(
                "08",
                &input,
                if value.starts_with('1') {
                    "state-number-sequence"
                } else {
                    "state-notation"
                }
            ),
            "{value}"
        );
        assert!(
            check("08", input.as_bytes())
                .normalized
                .unwrap()
                .contains(value)
        );
    }
    assert!(has(
        "08",
        &A08.replace("(1,x,3)", "(1,x,4)"),
        "maximum-final-state"
    ));
    assert!(has(
        "08",
        &A08.replace("2\n3\n", "2\n2\n"),
        "maximum-final-state"
    ));
    assert!(has(
        "08",
        &A08.replace("2\n3\n", "2\n2,3\n"),
        "maximum-final-state"
    ));
    assert!(has(
        "08",
        &A08.replace("(1,x,3)\n(2,$,1)", "(2,$,3)"),
        "state-number-sequence"
    ));
}

#[test]
fn numeric_state_names_sort_as_strings_including_eleven_before_two() {
    let mut transitions: Vec<_> = (1..=11).map(|i| format!("({i},x,{i})")).collect();
    let unsorted = transitions.join("\n");
    transitions.sort_unstable();
    let original = "8-1.1=====\n2\n3\n\n(1,x,3)\n(2,$,1)\n(2,y,3)";
    let input = A08.replace(
        original,
        &format!("8-1.1=====\n1\n11\n\n{}", transitions.join("\n")),
    );
    assert_eq!(check("08", input.as_bytes()).format_errors(), 0);
    assert!(input.find("(11,x,11)").unwrap() < input.find("(2,x,2)").unwrap());
    let numeric_order = input.replace(&transitions.join("\n"), &unsorted);
    assert!(has("08", &numeric_order, "sort"));
    assert_eq!(
        check("08", numeric_order.as_bytes()).normalized.as_deref(),
        Some(input.as_str())
    );
}

#[test]
fn top_down_regex_labels_allow_nested_parentheses_but_reject_bad_syntax() {
    for value in ["0", "1", "e", "xy", "(x+y)*x", "x(y+z)*", "((x))**"] {
        let input = A08.replace("8-1.2.2=====\n(x+y)*x", &format!("8-1.2.2=====\n{value}"));
        assert_eq!(check("08", input.as_bytes()).format_errors(), 0, "{value}");
    }
    for value in ["()", "x+", "+x", "x(+y)", "((x)", "$", "x|y", "x{2}", "*x"] {
        let input = A08.replace("8-1.2.2=====\n(x+y)*x", &format!("8-1.2.2=====\n{value}"));
        assert!(has("08", &input, "state-notation"), "{value}");
        assert!(
            check("08", input.as_bytes())
                .normalized
                .unwrap()
                .contains(value)
        );
    }
    let nested = format!("{}x{}", "(".repeat(10000), ")".repeat(10000));
    let input = A08.replace("8-1.2.2=====\n(x+y)*x", &format!("8-1.2.2=====\n{nested}"));
    assert_eq!(check("08", input.as_bytes()).format_errors(), 0);
}

#[test]
fn linear_forms_and_stage_members_keep_their_input_order_and_duplicates() {
    let r = check("08", A08.as_bytes());
    assert!(r.normalized.unwrap().contains(FORMS));
    let swapped = A08.replace(
        "lf(e)={<y,(x+y)*>,<x,1>}\nlf(1)={}",
        "lf(1)={}\nlf(e)={<x,1>,<y,(x+y)*>}",
    );
    let r = check("08", swapped.as_bytes());
    assert_eq!(r.format_errors(), 0);
    assert_eq!(r.normalized.as_deref(), Some(swapped.as_str()));
    assert_ne!(r.hash, check("08", A08.as_bytes()).hash);
    let duplicate = A08.replace("lf(1)={}", "lf(1)={}\nlf(1)={}");
    assert_eq!(
        check("08", duplicate.as_bytes())
            .normalized
            .unwrap()
            .matches("lf(1)={}")
            .count(),
        2
    );
}

#[test]
fn linear_form_and_iteration_syntax_are_checked_independently_of_values() {
    for value in [
        "lf()={}",
        "lf(e)={(x,1)}",
        "lf(e)={⟨x,1⟩}",
        "lf(e)={<xy,1>}",
        "lf(e)={<x,>}",
        "lf(e)={<x,1,z>}",
        "lf(e)={<x,1>,}",
        "lf(e)={<x,$>}",
    ] {
        let input = A08.replace("lf(e)={<y,(x+y)*>,<x,1>}", value);
        assert!(has("08", &input, "linear-form"), "{value}");
    }
    for value in [
        "i=0,K={},Δ={e}",
        "i=0,k={},Delta={e}",
        "i=0,K={,},Delta={e}",
        "i=0,K={(x+)},Delta={e}",
        "i=0,K=e,Delta={}",
    ] {
        assert!(
            has(
                "08",
                &A08.replace("i=0,K={},Delta={e}", value),
                "derivation-stage"
            ),
            "{value}"
        );
    }
    for value in ["i=1,K={},Delta={e}", "i=00,K={},Delta={e}"] {
        assert!(has(
            "08",
            &A08.replace("i=0,K={},Delta={e}", value),
            "stage-index"
        ));
    }
    let wrong_but_typed = A08.replace(
        FORMS,
        "lf(z)={<x,0>}\ni=0,K={z},Delta={}\ni=1,K={},Delta={z}",
    );
    assert_eq!(check("08", wrong_but_typed.as_bytes()).format_errors(), 0);
}

#[test]
fn derivation_stage_order_is_checked_and_never_sorted_to_hide_mistakes() {
    let input = A08.replace(
        "lf(1)={}\ni=0,K={},Delta={e}",
        "i=0,K={},Delta={e}\nlf(1)={}",
    );
    assert!(has("08", &input, "derivation-order"));
    assert_eq!(
        check("08", input.as_bytes()).normalized.as_deref(),
        Some(input.as_str())
    );
    let input = A08.replace(
        "i=0,K={},Delta={e}\ni=1,K={e},Delta={1,(x+y)*}",
        "i=1,K={e},Delta={1,(x+y)*}\ni=0,K={},Delta={e}",
    );
    assert!(has("08", &input, "stage-index"));
    assert_eq!(
        check("08", input.as_bytes()).normalized.as_deref(),
        Some(input.as_str())
    );
    assert!(has(
        "08",
        &A08.replace(FORMS, "lf(1)={}"),
        "derivation-parts"
    ));
    assert!(has(
        "08",
        &A08.replace(FORMS, "i=0,K={},Delta={e}"),
        "derivation-parts"
    ));
}

#[test]
fn assignment_08_spaces_tabs_epsilon_and_missing_headers_are_diagnosed() {
    for (before, after) in [
        ("2\n3\n", " 2\n3\n"),
        ("(1,x,3)", "(1, x,3)"),
        ("lf(1)={}", "lf(1) ={}"),
        ("lf(1)={}", "\t\nlf(1)={}"),
    ] {
        assert!(has("08", &A08.replace(before, after), "forbidden-space"));
    }
    for value in ["ε", "ϵ", "$x"] {
        assert!(has(
            "08",
            &A08.replace("(2,$,1)", &format!("(2,{value},1)")),
            "epsilon-notation"
        ));
    }
    assert!(has(
        "08",
        &A08.replace("2\n3\n\n(1,x,3)\n(2,$,1)\n(2,y,3)", "2"),
        "automaton-headers"
    ));
    assert!(has("08", &A08.replace("(1,y,e)", "(1,,e)"), "transition"));
}

#[test]
fn comments_identity_bom_and_line_endings_do_not_change_new_hashes() {
    for (id, input) in [("07", A07), ("08", A08)] {
        let renamed = input
            .replace("00X00000", "99Z99999")
            .replace("ダミー", "別のダミー");
        let mut edited = renamed.clone();
        for spec in answer_checker::assignments::get(id).unwrap().sections {
            edited = edited.replace(
                spec.marker,
                &format!("%%%% 指示\n{}\n%%% 注意書き", spec.marker),
            );
        }
        let with_comments = check(id, edited.as_bytes());
        assert_eq!(
            with_comments.warnings(),
            check(id, renamed.as_bytes()).warnings(),
            "{id}: {:?}",
            with_comments.issues
        );
        assert_eq!(with_comments.normalized.as_deref(), Some(renamed.as_str()));
        for text in [
            edited.clone(),
            edited.replace('\n', "\r\n"),
            edited.replace('\n', "\r"),
        ] {
            let r = check(id, format!("\u{feff}{text}").as_bytes());
            assert_eq!(r.format_errors(), 0, "{id}: {:?}", r.issues);
            assert_eq!(r.hash, check(id, input.as_bytes()).hash);
            let again = check(id, r.normalized.as_ref().unwrap().as_bytes());
            assert_eq!(again.hash, r.hash);
            assert_eq!(again.normalized, r.normalized);
        }
    }
}

#[test]
fn exact_markers_and_nonempty_proofs_and_derivation_are_required() {
    for (id, input, marker) in [("07", A07, "7-1====="), ("08", A08, "8-1.2.1=====")] {
        for wrong in [format!("{marker} "), "99-1=====".to_string()] {
            assert!(
                check(id, input.replace(marker, &wrong).as_bytes())
                    .hash
                    .is_none()
            );
        }
    }
    assert!(has(
        "07",
        &A07.replace("別の文章。v=y^{|u|}、eps、0<=k<n と書く。\n", ""),
        "unanswered"
    ));
    assert!(has("08", &A08.replace(FORMS, ""), "unanswered"));
    assert!(check("07", A08.as_bytes()).hash.is_none());
    assert!(check("08", A07.as_bytes()).hash.is_none());
}
