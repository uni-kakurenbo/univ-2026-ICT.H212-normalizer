use answer_checker::{HASH_VERSION, Report, Severity, assignments, check, sha256};

// Older synthetic fixtures isolate notation, normalization and naming. They
// intentionally aren't full solutions, so inspect mechanical checks separately.
trait FormattingReport {
    fn format_errors(&self) -> usize;
    fn format_warnings(&self) -> usize;
}
impl FormattingReport for Report {
    fn format_errors(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error && !i.code.starts_with("mechanical-"))
            .count()
    }
    fn format_warnings(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning && !i.code.starts_with("mechanical-"))
            .count()
    }
}

// Synthetic notation examples, not answers to either supplied grammar.
const TREES: &str = "S(bA(a))\nS(A(b)B(a))";
const CFG: &str = "A->aB\nB->$\nB->b\nS->AB";
const CNF: &str = "A->CG\nB->DH\nC->a\nD->b\nE->AF\nF->AB\nG->BA\nH->AB\nS->AE";
const SMALL_CNF: &str = "A->a\nB->b\nC->a\nD->b\nE->AB\nS->CE";

fn sheet11(trees: &str, rules: &str) -> String {
    format!(
        "学籍番号: 00X00000\n氏名: ダミー\n\n11-1=====\n{trees}\n========\n\n11-2=====\n{rules}\n========\n"
    )
}

fn sheet12(one: &str, two: &str) -> String {
    format!(
        "学籍番号: 00X00000\n氏名: ダミー\n\n12-1=====\n{one}\n========\n\n12-2=====\n{two}\n========\n"
    )
}

fn has(r: &Report, code: &str, severity: Severity) -> bool {
    r.issues
        .iter()
        .any(|i| i.code == code && i.severity == severity)
}

#[test]
fn assignments_register_and_keep_synthetic_format_with_independent_mechanical_diagnostics() {
    for (id, input, markers) in [
        ("11", sheet11(TREES, CFG), ["11-1=====", "11-2====="]),
        ("12", sheet12(CNF, SMALL_CNF), ["12-1=====", "12-2====="]),
    ] {
        let spec = assignments::get(id).unwrap();
        assert_eq!(spec.comment_prefix, "%%%");
        assert_eq!(
            spec.sections.iter().map(|s| s.marker).collect::<Vec<_>>(),
            markers
        );
        let r = check(id, input.as_bytes());
        assert_eq!(r.format_errors(), 0, "{id}: {:?}", r.issues);
        assert_eq!(r.format_warnings(), 0, "{id}: {:?}", r.issues);
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
        assert!(r.hash.is_some());
        assert!(r.issues.iter().any(|i| i.code
            == if id == "11" {
                "mechanical-tree-yield"
            } else {
                "mechanical-cnf-procedure"
            }));
    }
    // These deliberately unrelated fixtures isolate formatting. Mechanical tree
    // errors and procedure mismatch warnings are tested separately.
    assert_eq!(
        check("11", sheet11("S(a)\nS(b)", CFG).as_bytes()).format_errors(),
        0
    );
    assert_eq!(
        check("12", sheet12("S->AB", "A->AA\nS->AB").as_bytes()).format_errors(),
        0
    );
}

#[test]
fn exactly_two_trees_are_required_one_per_nonempty_line() {
    for trees in ["S(a)", "S(a)\nS(b)\nS(ab)", "S(a) S(b)"] {
        let r = check("11", sheet11(trees, CFG).as_bytes());
        assert!(has(&r, "tree-count", Severity::Error), "{trees}");
    }
    let r = check("11", sheet11("", CFG).as_bytes());
    assert!(has(&r, "unanswered", Severity::Error));
    assert!(!has(&r, "tree-count", Severity::Error));
}

#[test]
fn tree_syntax_checks_root_expanded_nonterminals_and_brackets() {
    for invalid in [
        "A(a)",
        "S()",
        "S(A)",
        "S(a(b))",
        "S(a,b)",
        "S(a->b)",
        "S(C(a))",
        "S(a)S(b)",
        "S((a))",
        "S(a",
        "S(a))",
        "S(a B(b))",
        "S(ε)",
        "S($)",
        "S（a）",
        "S(a\tb)",
    ] {
        let input = sheet11(&format!("{invalid}\nS(b)"), CFG);
        let r = check("11", input.as_bytes());
        assert!(has(&r, "tree-syntax", Severity::Error), "{invalid}");
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
        assert!(r.hash.is_some());
    }
}

#[test]
fn tree_order_and_duplicates_are_preserved_while_blank_lines_are_removed() {
    let input = sheet11("  S(A(b)B(a))  \n\nS(bA(a))", CFG);
    let r = check("11", input.as_bytes());
    assert_eq!(r.format_errors(), 0, "{:?}", r.issues);
    assert!(has(&r, "inner-blank", Severity::Warning));
    let expected = sheet11("S(A(b)B(a))\nS(bA(a))", CFG);
    assert_eq!(r.normalized.as_deref(), Some(expected.as_str()));
    assert_ne!(r.hash, check("11", sheet11(TREES, CFG).as_bytes()).hash);
    let repeated = sheet11("S(a)\nS(a)", CFG);
    let r = check("11", repeated.as_bytes());
    assert_eq!(r.format_errors(), 0, "{:?}", r.issues);
    assert!(has(&r, "mechanical-tree-distinct", Severity::Error));
    assert_eq!(r.normalized.as_deref(), Some(repeated.as_str()));
}

#[test]
fn cfg_notation_allows_mixed_rhs_unit_rules_epsilon_and_primed_nonterminals() {
    let rules = "A->ABa\nA->B\nB->$\nQ'->bQ''\nS->Q'";
    let input = sheet11(TREES, rules);
    let r = check("11", input.as_bytes());
    assert_eq!(r.format_errors(), 0, "{:?}", r.issues);
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    for invalid in [
        "AB->a", "a->AB", "A1->a", "A->a'", "A->a+b", "A->c", "A->$B",
    ] {
        let r = check("11", sheet11(TREES, invalid).as_bytes());
        assert!(has(&r, "cfg-production", Severity::Error), "{invalid}");
    }
    for (invalid, code) in [
        ("A→a", "production"),
        ("A->a|b", "production"),
        ("A->ε", "epsilon-notation"),
    ] {
        let r = check("11", sheet11(TREES, invalid).as_bytes());
        assert!(has(&r, code, Severity::Error), "{invalid}");
    }
}

#[test]
fn cnf_requires_one_uppercase_lhs_and_two_nonterminals_or_one_terminal() {
    for invalid in [
        "A->B", "A->ABC", "A->aB", "A->$", "A->a|b", "A->ab", "AB->a", "A'->BC", "A1->a", "a->BC",
        "A->c", "A->ε", "A->B C",
    ] {
        let r = check("12", sheet12(invalid, SMALL_CNF).as_bytes());
        assert!(has(&r, "cnf-production", Severity::Error), "{invalid}");
    }
    for valid in ["A->AB", "A->a", "B->b"] {
        let r = check("12", sheet12(valid, SMALL_CNF).as_bytes());
        assert_eq!(r.format_errors(), 0, "{valid}: {:?}", r.issues);
    }
}

#[test]
fn production_lines_sort_by_ascii_including_uppercase_before_lowercase() {
    for (id, input, expected) in [
        (
            "11",
            sheet11(TREES, "A->a\nA->BB\nS->AB"),
            sheet11(TREES, "A->BB\nA->a\nS->AB"),
        ),
        (
            "12",
            sheet12(&CNF.replace("A->CG", "A->a\nA->CG"), SMALL_CNF),
            sheet12(&CNF.replace("A->CG", "A->CG\nA->a"), SMALL_CNF),
        ),
    ] {
        let r = check(id, input.as_bytes());
        assert!(has(&r, "sort", Severity::Error));
        assert_eq!(r.normalized.as_deref(), Some(expected.as_str()));
        assert_eq!(r.hash, check(id, expected.as_bytes()).hash);
    }
}

#[test]
fn terminal_proxy_order_and_consecutive_added_names_are_checked_without_renaming() {
    let swapped = CNF.replace("C->a\nD->b", "C->b\nD->a");
    let input = sheet12(&swapped, SMALL_CNF);
    let r = check("12", input.as_bytes());
    assert!(has(&r, "cnf-terminal-label", Severity::Error));
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    let missing_proxy = "A->CE\nD->b\nE->AB\nS->DE";
    assert!(has(
        &check("12", sheet12(missing_proxy, SMALL_CNF).as_bytes()),
        "cnf-terminal-label",
        Severity::Error
    ));
    let gap = "C->a\nD->b\nF->AB\nS->CF";
    assert!(has(
        &check("12", sheet12(gap, SMALL_CNF).as_bytes()),
        "cnf-label-sequence",
        Severity::Error
    ));
    let extra_terminal = SMALL_CNF.replace("E->AB", "E->a");
    assert!(has(
        &check("12", sheet12(&extra_terminal, SMALL_CNF).as_bytes()),
        "cnf-helper-role",
        Severity::Error
    ));
}

#[test]
fn auxiliary_origins_follow_s_a_b_through_helper_chains_but_not_original_symbols() {
    let r = check("12", sheet12(CNF, SMALL_CNF).as_bytes());
    // S->AE, E->AF, F->AB: E/F originate in S. Do not traverse the original
    // A/B to misclassify G/H as shared; A->CG and B->DH give their own origins.
    assert_eq!(r.format_errors(), 0, "{:?}", r.issues);
    assert_eq!(r.format_warnings(), 0, "{:?}", r.issues);
    let reversed = "A->CH\nB->DG\nC->a\nD->b\nE->AF\nF->AB\nG->AB\nH->BA\nS->AE";
    let input = sheet12(reversed, SMALL_CNF);
    let r = check("12", input.as_bytes());
    assert!(has(&r, "cnf-label-order", Severity::Error));
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    let issue = r
        .issues
        .iter()
        .find(|i| i.code == "cnf-label-order")
        .unwrap();
    assert_eq!(
        issue.line,
        input
            .lines()
            .position(|line| line == "H->BA")
            .map(|i| i + 1)
    );
    // Within one originating LHS, the template gives no allocation order.
    let same_origin = "C->a\nD->b\nE->AB\nF->AE\nS->CF";
    assert_eq!(
        check("12", sheet12(same_origin, SMALL_CNF).as_bytes()).format_errors(),
        0
    );
}

#[test]
fn shared_untraceable_and_cyclic_origins_are_warnings_with_explicit_unchecked_status() {
    for one in [
        "A->CE\nC->a\nD->b\nE->AB\nS->DE", // shared by A and S
        "C->a\nD->b\nE->AB\nS->AB",        // no originating S/A/B reference
        "C->a\nD->b\nS->CE",               // missing helper definition
        "C->a\nD->b\nE->FE\nF->AB\nS->CE", // cycle, including a downstream helper
    ] {
        let input = sheet12(one, SMALL_CNF);
        let r = check("12", input.as_bytes());
        assert_eq!(r.format_errors(), 0, "{one}: {:?}", r.issues);
        let warnings: Vec<_> = r
            .issues
            .iter()
            .filter(|i| i.code == "cnf-label-origin")
            .collect();
        assert!(!warnings.is_empty(), "{one}");
        assert!(
            warnings
                .iter()
                .all(|i| i.severity == Severity::Warning && i.message.contains("未検査"))
        );
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
}

#[test]
fn duplicate_productions_are_not_removed_and_do_not_create_false_shared_origins() {
    for (id, input, expected) in [
        (
            "11",
            sheet11(TREES, &CFG.replace("B->b", "B->b\nB->b")),
            "B->b\nB->b",
        ),
        (
            "12",
            sheet12(&CNF.replace("E->AF", "E->AF\nE->AF"), SMALL_CNF),
            "E->AF\nE->AF",
        ),
    ] {
        let r = check(id, input.as_bytes());
        assert_eq!(r.format_errors(), 0, "{id}: {:?}", r.issues);
        assert!(has(&r, "duplicate-rule", Severity::Warning));
        assert!(!has(&r, "cnf-label-origin", Severity::Warning));
        assert!(r.normalized.as_ref().unwrap().contains(expected));
    }
}

#[test]
fn comment_identity_bom_and_line_ending_changes_keep_hash_v1_and_idempotence() {
    for (id, base) in [("11", sheet11(TREES, CFG)), ("12", sheet12(CNF, SMALL_CNF))] {
        let baseline = check(id, base.as_bytes());
        assert_eq!(HASH_VERSION, "answer-text-v1");
        assert_eq!(
            baseline.hash_input.as_deref(),
            Some(base.split_once("\n\n").unwrap().1)
        );
        assert_eq!(
            baseline.hash,
            Some(sha256(baseline.hash_input.as_ref().unwrap()))
        );
        let mut annotated = base
            .replace("00X00000", "99Z99999")
            .replace("ダミー", "別のダミー");
        for section in assignments::get(id).unwrap().sections {
            annotated = annotated.replace(
                &format!("{}\n", section.marker),
                &format!("%%%% 見出し\n{}\n  %%% 指示\n", section.marker),
            );
        }
        for value in [
            annotated.clone(),
            annotated.replace('\n', "\r\n"),
            annotated.replace('\n', "\r"),
        ] {
            let r = check(id, format!("\u{feff}{value}").as_bytes());
            assert_eq!(r.format_errors(), 0, "{id}: {:?}", r.issues);
            assert_eq!(r.hash, baseline.hash);
            assert_eq!(r.hash_input, baseline.hash_input);
            assert!(!r.issues.iter().any(|i| i.code == "comment"));
            assert!(!r.normalized.as_ref().unwrap().contains("指示"));
            let again = check(id, r.normalized.as_ref().unwrap().as_bytes());
            assert_eq!(again.normalized, r.normalized);
            assert_eq!(again.hash, r.hash);
        }
    }
}

#[test]
fn markers_missing_sections_and_wrong_assignments_block_ambiguous_output() {
    for (id, input) in [("11", sheet11(TREES, CFG)), ("12", sheet12(CNF, SMALL_CNF))] {
        for damaged in [
            input.replace(&format!("{id}-1====="), &format!("{id}-1====")),
            input.replace(&format!("{id}-2====="), &format!("{id}-2===== ")),
        ] {
            let r = check(id, damaged.as_bytes());
            assert!(r.format_errors() > 0);
            assert!(r.hash.is_none());
            assert!(r.normalized.is_none());
        }
        assert!(
            check(if id == "11" { "12" } else { "11" }, input.as_bytes())
                .hash
                .is_none()
        );
    }
    assert!(has(
        &check("11", sheet11(TREES, "").as_bytes()),
        "unanswered",
        Severity::Error
    ));
    assert!(has(
        &check("12", sheet12("", SMALL_CNF).as_bytes()),
        "unanswered",
        Severity::Error
    ));
}

#[test]
fn deeply_nested_trees_use_an_iterative_parser() {
    let deep = format!("S({}a{})", "A(".repeat(20000), ")".repeat(20000));
    let input = sheet11(&format!("{deep}\nS(b)"), CFG);
    let r = check("11", input.as_bytes());
    assert_eq!(r.format_errors(), 0, "{:?}", r.issues);
    assert_eq!(r.format_warnings(), 0, "{:?}", r.issues);
    let broken = sheet11(&format!("{}\nS(b)", deep.trim_end_matches(')')), CFG);
    assert!(has(
        &check("11", broken.as_bytes()),
        "tree-syntax",
        Severity::Error
    ));
}

#[test]
fn mechanical_tree_checks_are_independent_report_locations_and_preserve_hash_input() {
    // Matching leaves alone do not make this flat, invalid tree a solution.
    let input = sheet11("  S(aababb)\nS(bA(a))", CFG);
    let r = check("11", input.as_bytes());
    assert_eq!(r.format_errors(), 0, "{:?}", r.issues);
    let branches: Vec<_> = r
        .issues
        .iter()
        .filter(|i| i.code == "mechanical-tree-production")
        .collect();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].line, Some(5));
    assert!(branches[0].message.contains("3文字目"));
    let yields: Vec<_> = r
        .issues
        .iter()
        .filter(|i| i.code == "mechanical-tree-yield")
        .collect();
    assert_eq!(yields.len(), 1);
    assert_eq!(yields[0].line, Some(6));
    assert!(
        branches
            .iter()
            .chain(yields.iter())
            .all(|i| i.severity == Severity::Error)
    );
    let expected = sheet11("S(aababb)\nS(bA(a))", CFG);
    assert_eq!(r.normalized.as_deref(), Some(expected.as_str()));
    assert_eq!(r.hash, Some(sha256(expected.split_once("\n\n").unwrap().1)));
    let again = check("11", r.normalized.as_ref().unwrap().as_bytes());
    assert_eq!(again.hash, r.hash);
    assert_eq!(again.normalized, r.normalized);
}

#[test]
fn b_only_proxy_starts_at_c_and_helpers_start_immediately_after_needed_proxies() {
    let one = "A->a\nB->b\nC->b\nD->AB\nS->CD";
    let r = check("12", sheet12(one, one).as_bytes());
    assert_eq!(r.format_errors(), 0, "{:?}", r.issues);
    assert_eq!(r.format_warnings(), 0, "{:?}", r.issues);
    let no_proxies = "C->AB\nS->AC";
    let r = check("12", sheet12(no_proxies, no_proxies).as_bytes());
    assert_eq!(r.format_errors(), 0, "{:?}", r.issues);
    assert_eq!(r.format_warnings(), 0, "{:?}", r.issues);
}

#[test]
fn procedure_comparison_mismatches_only_warn_and_never_publish_or_replace_rules() {
    for (id, input, code) in [
        ("11", sheet11(TREES, CFG), "mechanical-epsilon-procedure"),
        ("12", sheet12(CNF, SMALL_CNF), "mechanical-cnf-procedure"),
    ] {
        let r = check(id, input.as_bytes());
        assert!(has(&r, code, Severity::Warning));
        assert!(!has(&r, code, Severity::Error));
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
        assert_eq!(r.hash, Some(sha256(input.split_once("\n\n").unwrap().1)));
        assert!(
            r.issues
                .iter()
                .filter(|i| i.code == code)
                .all(|i| i.message.contains("非等価・不正解とは判定しません"))
        );
    }
    let bad = check("12", sheet12("A->ABC", "S->AB").as_bytes());
    let first_line = bad
        .issues
        .iter()
        .find(|i| i.code == "cnf-production")
        .unwrap()
        .line;
    assert!(
        !bad.issues
            .iter()
            .any(|i| i.code == "mechanical-cnf-procedure" && i.line == first_line)
    );
}
