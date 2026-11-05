use answer_checker::{HASH_VERSION, Report, Severity, assignments, check, sha256};

// A toy accepting ALL binary words, deliberately not the supplied problem.
// Its first stage mismatches, while the subsequent submitted transforms agree.
const TOY: &str = "s2\n\n(s2,a,s2)\n(s2,b,s2)";
const MINIMAL_TOY: &str = "p1\n\n(p1,a,p1)\n(p1,b,p1)";
fn sheet(sections: [&str; 4]) -> String {
    let blocks: Vec<_> = sections
        .iter()
        .enumerate()
        .map(|(i, body)| format!("ex-{}=====\n{body}\n========", i + 1))
        .collect();
    format!(
        "学籍番号: 00X00000\n氏名: ダミー\n\n{}\n",
        blocks.join("\n\n")
    )
}
fn base() -> String {
    sheet([TOY, TOY, TOY, MINIMAL_TOY])
}
fn has(r: &Report, code: &str, severity: Severity) -> bool {
    r.issues
        .iter()
        .any(|i| i.code == code && i.severity == severity)
}
fn stage_has(r: &Report, stage: usize, code: &str) -> bool {
    r.issues
        .iter()
        .any(|i| i.code == code && i.message.starts_with(&format!("ex-{stage}")))
}

#[test]
fn registers_and_compares_each_stage_against_the_previous_submission() {
    let spec = assignments::get("ex").unwrap();
    assert_eq!(spec.comment_prefix, "%%%");
    assert_eq!(
        spec.sections.iter().map(|s| s.marker).collect::<Vec<_>>(),
        ["ex-1=====", "ex-2=====", "ex-3=====", "ex-4====="]
    );
    let input = base();
    let r = check("ex", input.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.warnings(), 1, "{:?}", r.issues);
    assert!(stage_has(&r, 1, "ex-procedure-mismatch"));
    assert!(
        r.issues[0]
            .message
            .contains("非等価・不正解とは判定しません")
    );
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    // A semantic mismatch introduced only at step 4 does not affect 2 or 3.
    let r = check("ex", sheet([TOY, TOY, TOY, "p1\n\n(p1,a,p1)"]).as_bytes());
    assert!(stage_has(&r, 4, "ex-procedure-mismatch"));
    assert!(
        !stage_has(&r, 2, "ex-procedure-mismatch") && !stage_has(&r, 3, "ex-procedure-mismatch")
    );
}

#[test]
fn sorts_final_lists_and_edges_independently_by_ascii_and_preserves_duplicates() {
    let unsorted = "s2,s1\n\n(s2,b,s2)\n(s2,a,s2)\n(s2,a,s2)";
    let input = sheet([unsorted, TOY, TOY, MINIMAL_TOY]);
    let r = check("ex", input.as_bytes());
    assert!(has(&r, "final-state-sort", Severity::Error));
    assert!(has(&r, "sort", Severity::Error));
    assert!(has(&r, "ex-duplicate", Severity::Warning));
    let expected = sheet([
        "s1,s2\n\n(s2,a,s2)\n(s2,a,s2)\n(s2,b,s2)",
        TOY,
        TOY,
        MINIMAL_TOY,
    ]);
    assert_eq!(r.normalized.as_deref(), Some(expected.as_str()));
    assert_eq!(check("ex", expected.as_bytes()).hash, r.hash);
    let input = sheet([
        TOY,
        TOY,
        TOY,
        "p2,p11,p1\n\n(p2,a,p1)\n(p11,a,p1)\n(p1,a,p1)",
    ]);
    let r = check("ex", input.as_bytes());
    assert!(
        r.normalized
            .unwrap()
            .contains("p1,p11,p2\n\n(p1,a,p1)\n(p11,a,p1)\n(p2,a,p1)")
    );
}

#[test]
fn subset_names_use_original_two_byte_tokens_including_s1_s2_and_are_never_renamed() {
    for label in ["q0r1s1s2", "s1s2", "q1q2r0r4s1"] {
        let section = format!("{label}\n\n({label},a,{label})\n({label},b,{label})");
        let r = check("ex", sheet([TOY, TOY, &section, MINIMAL_TOY]).as_bytes());
        assert!(!has(&r, "ex-state-label", Severity::Error));
        assert!(!has(&r, "subset-label-sort", Severity::Error));
    }
    for label in ["s2q0", "r1q0s1", "s2s1"] {
        let section = format!("{label}\n\n({label},a,{label})");
        let input = sheet([TOY, TOY, &section, MINIMAL_TOY]);
        let r = check("ex", input.as_bytes());
        assert!(has(&r, "subset-label-sort", Severity::Error));
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
        assert!(stage_has(&r, 4, "ex-procedure-unchecked"));
    }
    for label in ["q4", "s3", "q0r5", "q0s10", "{q0}"] {
        let r = check("ex", sheet([TOY, TOY, label, MINIMAL_TOY]).as_bytes());
        assert!(has(&r, "ex-state-label", Severity::Error), "{label}");
    }
    let r = check("ex", sheet([TOY, TOY, "s2s2", MINIMAL_TOY]).as_bytes());
    assert!(has(&r, "ex-subset-duplicate", Severity::Warning));
    assert!(stage_has(&r, 4, "ex-procedure-unchecked"));
}

#[test]
fn p_numbers_start_at_one_have_no_gaps_and_do_not_require_numeric_row_order() {
    for label in ["q1", "p0", "p01", "P1", "p-1", "p1x"] {
        assert!(has(
            &check("ex", sheet([TOY, TOY, TOY, label]).as_bytes()),
            "numbered-state",
            Severity::Error
        ));
    }
    let r = check("ex", sheet([TOY, TOY, TOY, "p1,p3"]).as_bytes());
    assert!(has(&r, "numbered-state-sequence", Severity::Error));
    let mut names: Vec<_> = (1..=12).map(|i| format!("p{i}")).collect();
    names.sort_unstable();
    let numbered = names.join(",");
    let r = check("ex", sheet([TOY, TOY, TOY, &numbered]).as_bytes());
    assert!(!has(&r, "numbered-state-sequence", Severity::Error));
    assert!(!has(&r, "final-state-sort", Severity::Error));
}

#[test]
fn notation_and_stage_specific_symbols_and_spaces_are_strict() {
    for (row, code) in [
        ("(s2,ε,s2)", "epsilon-notation"),
        ("(s2,ϵ,s2)", "epsilon-notation"),
        ("(s2,c,s2)", "ex-input-symbol"),
        ("(s2,aa,s2)", "ex-input-symbol"),
        ("(s2, $ ,s2)", "forbidden-space"),
        (" (s2,a,s2)", "forbidden-space"),
        ("(s2,\ta,s2)", "forbidden-space"),
    ] {
        let section = format!("s2\n\n{row}");
        assert!(
            has(
                &check("ex", sheet([&section, TOY, TOY, MINIMAL_TOY]).as_bytes()),
                code,
                Severity::Error
            ),
            "{row}"
        );
    }
    let epsilon = "s2\n\n(s2,$,s2)\n(s2,a,s2)\n(s2,b,s2)";
    assert!(!has(
        &check("ex", sheet([epsilon, TOY, TOY, MINIMAL_TOY]).as_bytes()),
        "ex-input-symbol",
        Severity::Error
    ));
    let r = check("ex", sheet([TOY, epsilon, TOY, MINIMAL_TOY]).as_bytes());
    assert!(has(&r, "ex-input-symbol", Severity::Error));
    assert!(stage_has(&r, 3, "ex-procedure-unchecked"));
    for malformed in ["s2,a,s2", "(s2,a)", "(s2,a,s2,extra)", "(s2,,s2)"] {
        let section = format!("s2\n\n{malformed}");
        let r = check("ex", sheet([&section, TOY, TOY, MINIMAL_TOY]).as_bytes());
        assert!(has(&r, "transition", Severity::Error));
        assert!(stage_has(&r, 2, "ex-procedure-unchecked"));
    }
}

#[test]
fn semantic_duplicates_do_not_change_procedure_comparison_but_do_change_hash() {
    let duplicate = "s2,s2\n\n(s2,a,s2)\n(s2,a,s2)\n(s2,b,s2)";
    let input = sheet([duplicate, duplicate, duplicate, MINIMAL_TOY]);
    let r = check("ex", input.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(
        r.issues
            .iter()
            .filter(|i| i.code == "ex-procedure-mismatch")
            .count(),
        1
    );
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    assert_ne!(r.hash, check("ex", base().as_bytes()).hash);
}

#[test]
fn state_preservation_reachability_and_dfa_dependencies_are_checked() {
    let extra = "r0,s2\n\n(r0,a,r0)\n(r0,b,r0)\n(s2,a,s2)\n(s2,b,s2)";
    let r = check("ex", sheet([extra, TOY, TOY, MINIMAL_TOY]).as_bytes());
    assert!(stage_has(&r, 2, "ex-procedure-mismatch")); // removed original state
    let r = check("ex", sheet([TOY, TOY, extra, MINIMAL_TOY]).as_bytes());
    assert!(stage_has(&r, 3, "ex-procedure-mismatch")); // unreachable state
    let nondeterministic = "s2\n\n(s2,a,r0)\n(s2,a,s2)\n(s2,b,s2)";
    let r = check(
        "ex",
        sheet([TOY, TOY, nondeterministic, MINIMAL_TOY]).as_bytes(),
    );
    assert!(stage_has(&r, 4, "ex-procedure-unchecked"));
    assert!(r.issues.iter().any(|i| i.message.contains("非決定的")));
    let partial = "s2\n\n(s2,a,s2)";
    let r = check("ex", sheet([TOY, partial, TOY, MINIMAL_TOY]).as_bytes());
    assert!(stage_has(&r, 3, "ex-procedure-unchecked"));
    assert!(r.issues.iter().any(|i| i.message.contains("空の部分集合")));
}

#[test]
fn bounds_public_checker_graph_sizes_without_hiding_normalization() {
    // 129 distinct well-formed subset labels from the explicitly allowed names.
    let tokens = ["q0", "q1", "q2", "q3", "r0", "r1", "r2", "r3"];
    let mut labels: Vec<String> = (1usize..=129)
        .map(|mask| {
            tokens
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, t)| *t)
                .collect()
        })
        .collect();
    labels.sort_unstable();
    let input = sheet([TOY, TOY, &labels.join(","), MINIMAL_TOY]);
    let r = check("ex", input.as_bytes());
    assert!(stage_has(&r, 3, "ex-procedure-unchecked"));
    assert!(stage_has(&r, 4, "ex-procedure-unchecked"));
    assert!(r.issues.iter().any(|i| i.message.contains("上限")));
    assert!(r.normalized.is_some() && r.hash.is_some());
}

#[test]
fn comments_identity_encoding_and_repeat_normalization_keep_answer_text_v1() {
    let input = base();
    let baseline = check("ex", input.as_bytes());
    assert_eq!(HASH_VERSION, "answer-text-v1");
    assert_eq!(
        baseline.hash,
        Some(sha256(input.split_once("\n\n").unwrap().1))
    );
    let annotated = input
        .replace("00X00000", "99Z99999")
        .replace("ダミー", "別のダミー")
        .replace("=====\n", "=====\n  %%% コメント\n");
    for changed in [
        annotated.clone(),
        annotated.replace('\n', "\r\n"),
        annotated.replace('\n', "\r"),
    ] {
        let r = check("ex", format!("\u{feff}{changed}").as_bytes());
        assert_eq!(r.hash, baseline.hash);
        assert!(!r.issues.iter().any(|i| i.code == "comment"));
        let again = check("ex", r.normalized.as_ref().unwrap().as_bytes());
        assert_eq!(again.normalized, r.normalized);
        assert_eq!(again.hash, r.hash);
    }
    let input = sheet(["s2\n\ns2,a,s2\n(s2,a,s2)", TOY, TOY, MINIMAL_TOY]);
    let r = check("ex", input.as_bytes());
    let again = check("ex", r.normalized.as_ref().unwrap().as_bytes());
    assert_eq!(again.normalized, r.normalized);
    assert_eq!(again.hash, r.hash);
}

#[test]
fn named_markers_are_exact_and_only_opt_in_for_the_new_assignment() {
    for bad in [
        base().replace("ex-1=====", "ex-1===="),
        base().replace("ex-3=====", "ex-3===== "),
        base().replace("ex-4=====", "ex-3====="),
        base().replace("ex-2=====", "ex-5====="),
        base().replace("(s2,a,s2)", "ex-5=====\n(s2,a,s2)"),
    ] {
        let r = check("ex", bad.as_bytes());
        assert!(r.normalized.is_none() && r.hash.is_none());
    }
    let r = check("ex", sheet(["", "", "", ""]).as_bytes());
    assert_eq!(
        r.issues.iter().filter(|i| i.code == "unanswered").count(),
        4
    );
    assert!(check("15", base().as_bytes()).hash.is_none());
    let old = "学籍番号: 00X00000\n氏名: ダミー\n\n1-1=====\nx\n========\n\n1-2=====\nex-5=====\n========\n\n1-3=====\nx\n========\n";
    // The new marker recognition must not reclassify earlier prose/hash input.
    let r = check("01", old.as_bytes());
    assert!(r.hash_input.as_ref().unwrap().contains("ex-5====="));
}
