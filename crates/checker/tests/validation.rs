use answer_checker::{HASH_VERSION, MAX_BYTES, check, sha256};

const A01: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n1-1=====\nx,y\n========\n\n1-2=====\n日本語の 説明\n========\n\n1-3=====\nZ\nY\nX\n========\n";
const A02: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n2-1.1===\nA->x\nS->$\n========\n\n2-1.2===\nB->y\nS->zB\n========\n\n2-2=====\nC->z\nS->zC\n========\n";
const A03: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n3-1===\nA\n\n(S,x,A)\n========\n\n3-2=====\nB\n\n(S,y,B)\n========\n\n3-3=====\ndiagram.png\n========\n";
const A04: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n4-1=====\n#,A\n\n(#,x,A)\n(A,x,#)\n(S,x,A)\n========\n\n4-2=====\n#A,#AB\n\n(#A,x,#AB)\n(AB,y,#A)\n(S,x,AB)\n========\n\n4-3=====\nA->$\nA->xA\nA->yB\nB->xA\nB->yB\nS->xA\nS->yB\n========\n";
const A05: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n5-1=====\nq3,r4\n\n(q0,$,r0)\n(q0,x,q1)\n(r0,y,r4)\n========\n\n5-2=====\nq0q2r1,r4\n\n(q0,x,q0q2r1)\n(q0q2r1,y,r4)\n(r4,x,r4)\n========\n";
const A06: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n6-1.1=====\nB,Z\n\n(A,x,B)\n(B,y,Z)\n(Z,x,Z)\n========\n\n6-1.2=====\n(A,B)\n(A,Z)\n(B,Z)\n========\n\n6-1.3=====\nq2\n\n(q1,x,q2)\n(q2,y,q1)\n========\n";

fn has(input: &str, id: &str, code: &str) -> bool {
    check(id, input.as_bytes())
        .issues
        .iter()
        .any(|i| i.code == code)
}

#[test]
fn dummy_answers_are_not_graded() {
    for (id, input) in [
        ("01", A01),
        ("02", A02),
        ("03", A03),
        ("04", A04),
        ("05", A05),
        ("06", A06),
    ] {
        let r = check(id, input.as_bytes());
        assert_eq!(r.errors(), 0, "{id}: {:?}", r.issues);
        assert!(r.hash.is_some());
        assert_eq!(r.normalized.as_deref(), Some(input));
    }
}

#[test]
fn hash_v1_has_exact_representation() {
    let r = check("01", A01.as_bytes());
    let body = "1-1=====\nx,y\n========\n\n1-2=====\n日本語の 説明\n========\n\n1-3=====\nZ\nY\nX\n========\n";
    assert_eq!(HASH_VERSION, "answer-text-v1");
    assert_eq!(r.hash_input.as_deref(), Some(body));
    assert_eq!(
        sha256("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(r.hash, Some(sha256(body)));
}

#[test]
fn private_headers_comments_and_newlines_do_not_affect_hash() {
    let expected = check("01", A01.as_bytes()).hash;
    let edited = A01
        .replace("00X00000", "99Z99999")
        .replace("ダミー", "別の名前")
        .replace(
            "1-1=====\n",
            "# top instruction\n1-1=====\n### inner instruction\n",
        );
    for input in [
        edited.clone(),
        edited.replace('\n', "\r\n"),
        edited.replace('\n', "\r"),
    ] {
        let bom = format!("\u{feff}{input}\n\n");
        assert_eq!(check("01", bom.as_bytes()).hash, expected);
    }
}

#[test]
fn content_changes_hash_and_cannot_migrate_between_questions() {
    let first = check("01", A01.as_bytes()).hash;
    assert_ne!(
        first,
        check("01", A01.replace("x,y", "x,z").as_bytes()).hash
    );
    let moved = A01
        .replace("x,y", "Z\nY\nX")
        .replace("1-3=====\nZ\nY\nX", "1-3=====\nx,y");
    assert_ne!(first, check("01", moved.as_bytes()).hash);
}

#[test]
fn derivation_is_not_sorted_and_dfa_rows_are_kept_separate() {
    let r = check("02", A02.replace("A->x\nS->$", "S->$\nA->x").as_bytes());
    assert!(r.issues.iter().any(|i| i.code == "sort"));
    assert_eq!(r.normalized.as_deref(), Some(A02));
    assert!(
        check("01", A01.as_bytes())
            .normalized
            .unwrap()
            .contains("Z\nY\nX")
    );
    let r = check("03", A03.as_bytes());
    assert!(r.normalized.as_ref().unwrap().contains("A\n\n(S,x,A)"));
    assert_eq!(r.errors(), 0);
}

#[test]
fn prose_spaces_and_paragraphs_survive_normalization() {
    let input = A01.replace("日本語の 説明", "  日本語の 説明  \n\n別の段落");
    assert!(
        check("01", input.as_bytes())
            .normalized
            .unwrap()
            .contains("  日本語の 説明  \n\n別の段落")
    );
    assert!(!has(&input, "01", "ascii"));
    assert!(has(&A01.replace("x,y", "ｘ,y"), "01", "ascii"));
}

#[test]
fn ambiguous_structure_blocks_output_instead_of_discarding_text() {
    for input in [
        A01.replace("1-1=====", "1-1===="),
        A01.replace("1-1=====", "1-1===== "),
        A01.replace("1-3=====", "9-3====="),
        A01.replace("1-2=====", "1-1====="),
        format!("{A01}stray text\n"),
        A01.trim_end().trim_end_matches("========").to_string(),
        A01.replace("氏名: ダミー", "氏名: ダミー\n氏名: 別人"),
    ] {
        let r = check("01", input.as_bytes());
        assert!(r.errors() > 0);
        assert!(r.hash.is_none(), "{input}");
        assert!(r.normalized.is_none());
    }
    assert!(check("02", A01.as_bytes()).hash.is_none());
}

#[test]
fn encoding_and_size_are_strict() {
    for bytes in [&[0xff, 0xfe][..], &[0xe3, 0x81][..]] {
        let r = check("01", bytes);
        assert!(r.hash.is_none());
        assert!(r.issues.iter().any(|i| i.code == "utf8"));
    }
    assert!(check("01", &vec![b'a'; MAX_BYTES + 1]).hash.is_none());
}

#[test]
fn mixed_newlines_and_invisible_characters_are_reported() {
    let r = check("01", A01.replacen('\n', "\r\n", 1).as_bytes());
    assert_eq!(r.info.crlf, 1);
    assert!(r.info.lf > 0);
    assert!(r.issues.iter().any(|i| i.code == "mixed-newline"));
    assert!(has(&A01.replace("x,y", "x,\u{200b}y"), "01", "invisible"));
    assert!(has(&A01.replace("x,y", "x,\0y"), "01", "invisible"));
}

#[test]
fn production_notation_and_png_extension_are_checked_without_semantics() {
    for value in ["S=>x", "S->x|y", "S->", "->x", "S ->x", "S->x->y"] {
        assert!(has(&A02.replace("A->x", value), "02", "production"));
    }
    assert!(has(&A02.replace("A->x", "A->ε"), "02", "epsilon-notation"));
    assert!(has(
        &A03.replace("diagram.png", "diagram.jpg"),
        "03",
        "png-filename"
    ));
    assert!(has(
        &A03.replace("diagram.png", "a.png\nb.png"),
        "03",
        "png-filename"
    ));
    assert!(has(
        &A03.replace("(S,x,A)", "opaque ASCII"),
        "03",
        "transition"
    ));
}

#[test]
fn triples_and_final_states_follow_supplementary_notation() {
    for transition in [
        "S,x,A",
        "(S,,A)",
        "(S,x,A,B)",
        "(S,x, A)",
        "((S,x,A))",
        "(S,x,A)(S,y,B)",
    ] {
        assert!(
            has(&A03.replace("(S,x,A)", transition), "03", "transition"),
            "{transition}"
        );
    }
    for finals in ["A,,B", "(A,B)", "A B", "A, B", ",A"] {
        assert!(has(
            &A03.replace("3-1===\nA", &format!("3-1===\n{finals}")),
            "03",
            "final-states"
        ));
    }
    assert!(has(&A02.replace("A->x", "A->$x"), "02", "epsilon-notation"));
}

#[test]
fn final_states_and_transitions_sort_separately_and_hash_tokens_survive() {
    let input = A03.replace(
        "3-1===\nA\n\n(S,x,A)",
        "3-1===\nB,A\n\n(S,x,A)\n# remove this comment\n(A,x,#)",
    );
    let r = check("03", input.as_bytes());
    assert!(r.issues.iter().any(|i| i.code == "sort"));
    let normalized = r.normalized.unwrap();
    assert!(r.issues.iter().any(|i| i.code == "final-state-sort"));
    assert!(normalized.contains("3-1===\nA,B\n\n(A,x,#)\n(S,x,A)"));
    assert!(!normalized.contains("remove this comment"));
    assert!(r.hash_input.as_ref().unwrap().contains("(A,x,#)"));
    let again = check("03", normalized.as_bytes());
    assert_eq!(again.errors(), 0);
    assert_eq!(again.hash, r.hash);
    assert_eq!(again.normalized.as_deref(), Some(normalized.as_str()));
}

#[test]
fn normalization_is_idempotent_and_does_not_deduplicate_answers() {
    let r = check("02", A02.replace("A->x", " A->x \nA->x\n").as_bytes());
    let normalized = r.normalized.unwrap();
    let again = check("02", normalized.as_bytes());
    assert_eq!(again.normalized.as_deref(), Some(normalized.as_str()));
    assert_eq!(again.hash, r.hash);
    assert_eq!(normalized.matches("A->x").count(), 2);
}

#[test]
fn missing_identity_does_not_prevent_anonymous_hash_but_is_an_error() {
    let input = A01.replace("学籍番号: 00X00000\n氏名: ダミー\n\n", "");
    let r = check("01", input.as_bytes());
    assert!(r.issues.iter().any(|i| i.code == "header-missing"));
    assert_eq!(r.hash, check("01", A01.as_bytes()).hash);
}

#[test]
fn adding_assignment_04_keeps_previous_hashes_fixed() {
    for (id, input, digest) in [
        (
            "01",
            A01,
            "648f1d96fd90d7ab03c905ea83c6e74db99ba7c758b7ec6916c15962f662ea17",
        ),
        (
            "02",
            A02,
            "b1fb3de0b6ed408e2b6326e20aeb99808f2cde587f8739878787dff3ee73f04f",
        ),
        (
            "03",
            A03,
            "bf2790e8942cc22b6f6f5560b93134be3ebba88cefc7161b170cc01045462d3f",
        ),
    ] {
        assert_eq!(check(id, input.as_bytes()).hash.as_deref(), Some(digest));
        let annotated = input.replace("\n\n", "\n#AB\n#\n\n");
        assert_eq!(
            check(id, annotated.as_bytes()).hash.as_deref(),
            Some(digest)
        );
    }
}

#[test]
fn adding_assignments_05_and_06_keeps_assignment_04_and_old_percent_text() {
    for (id, input, digest) in [
        (
            "05",
            A05,
            "0aef71e62209047a617e77d5593af771710478fa921dcaa572f2c0f0f7a32911",
        ),
        (
            "06",
            A06,
            "5c284bdf0e027dd8f040aa1c4612879d60c764a5cc9966a9d82035ecd179003e",
        ),
    ] {
        assert_eq!(check(id, input.as_bytes()).hash.as_deref(), Some(digest));
    }
    assert_eq!(
        check("04", A04.as_bytes()).hash.as_deref(),
        Some("84d4362c7e8c218f67605b8f1cdeca6cd80eb9998340fe94e9e6611f9bf71daf")
    );
    let original_percent = A01.replace("x,y", "%%% retained answer text\nx,y");
    let r = check("01", original_percent.as_bytes());
    assert_eq!(r.errors(), 0);
    assert!(r.hash_input.unwrap().contains("%%% retained answer text"));
    assert_ne!(r.hash, check("01", A01.as_bytes()).hash);
    assert!(
        check("04", format!("%%% outside\n{A04}").as_bytes())
            .hash
            .is_none()
    );
}

#[test]
fn new_percent_instructions_and_private_headers_never_enter_hashes() {
    for (id, input) in [("05", A05), ("06", A06)] {
        let mut edited = input
            .replace("00X00000", "99Z99999")
            .replace("ダミー", "別の名前");
        for spec in answer_checker::assignments::get(id).unwrap().sections {
            edited = edited.replace(
                spec.marker,
                &format!("%%%% 設問の指示\n{}\n\t%%% 注意書き", spec.marker),
            );
        }
        let expected = check(id, input.as_bytes());
        for text in [
            edited.clone(),
            edited.replace('\n', "\r\n"),
            edited.replace('\n', "\r"),
        ] {
            let r = check(id, format!("\u{feff}{text}\n\n").as_bytes());
            assert_eq!(r.errors(), 0, "{id}: {:?}", r.issues);
            assert_eq!(r.hash, expected.hash);
            assert_eq!(r.hash_input, expected.hash_input);
            let normalized = r.normalized.as_ref().unwrap();
            assert!(!normalized.contains("%%%"));
            let again = check(id, normalized.as_bytes());
            assert_eq!(again.normalized, r.normalized);
            assert_eq!(again.hash, r.hash);
            assert_eq!(again.errors(), 0);
        }
    }
    // Only the specified whole-line prefix is removed in the new templates.
    let hash_state = A05.replace("q3,r4", "#");
    assert_eq!(check("05", hash_state.as_bytes()).errors(), 0);
    assert!(
        check("05", hash_state.as_bytes())
            .hash_input
            .unwrap()
            .contains("5-1=====\n#\n")
    );
    let symbol = A05.replace("(q0,x,q1)", "(q0,%%%,q1)");
    assert_eq!(check("05", symbol.as_bytes()).errors(), 0);
    assert!(
        check("05", symbol.as_bytes())
            .hash_input
            .unwrap()
            .contains("(q0,%%%,q1)")
    );
    assert!(
        check("05", format!("%% not a comment\n{A05}").as_bytes())
            .hash
            .is_none()
    );
}

#[test]
fn assignment_05_uses_whole_state_names_for_subset_order() {
    for replacement in ["q2q0r1", "r1q0q2"] {
        let input = A05.replace("q0q2r1", replacement);
        let r = check("05", input.as_bytes());
        assert!(
            r.issues
                .iter()
                .any(|issue| issue.code == "subset-label-sort")
        );
        assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    }
    // Checking is limited to labels whose token boundaries are specified.
    for label in ["S", "addedState", "r1q0S", "q10", "q0q0r4"] {
        let input = A05.replace("q0q2r1", label);
        assert!(!has(&input, "05", "subset-label-sort"), "{label}");
        assert!(
            check("05", input.as_bytes())
                .normalized
                .unwrap()
                .contains(label)
        );
    }
    let input_symbol = A05.replace("(q0,x,q0q2r1)", "(q0,r1q0,q0q2r1)");
    assert_eq!(check("05", input_symbol.as_bytes()).errors(), 0);
    // No subset-label convention is imposed on the epsilon-NFA in 5-1.
    assert!(!has(
        &A05.replace("q3,r4", "r1q0"),
        "05",
        "subset-label-sort"
    ));
}

#[test]
fn assignment_05_checks_epsilon_spelling_and_sorts_independent_lists() {
    for symbol in ["ε", "ϵ", "$x", "x$", "$$"] {
        let input = A05.replace("(q0,$,r0)", &format!("(q0,{symbol},r0)"));
        assert!(has(&input, "05", "epsilon-notation"));
        assert!(
            check("05", input.as_bytes())
                .normalized
                .unwrap()
                .contains(symbol)
        );
    }
    let input = A05
        .replace("q3,r4", "r4,q3")
        .replace("q0q2r1,r4", "r4,q0q2r1")
        .replace(
            "(q0,$,r0)\n(q0,x,q1)\n(r0,y,r4)",
            "(r0,y,r4)\n(q0,x,q1)\n(q0,$,r0)",
        )
        .replace(
            "(q0,x,q0q2r1)\n(q0q2r1,y,r4)\n(r4,x,r4)",
            "(r4,x,r4)\n(q0q2r1,y,r4)\n(q0,x,q0q2r1)",
        );
    let r = check("05", input.as_bytes());
    assert!(
        r.issues
            .iter()
            .any(|issue| issue.code == "final-state-sort")
    );
    assert!(r.issues.iter().any(|issue| issue.code == "sort"));
    assert_eq!(r.normalized.as_deref(), Some(A05));
    assert_eq!(r.hash, check("05", A05.as_bytes()).hash);
}

#[test]
fn assignment_06_pairs_normalize_sides_then_rows_without_deduplication() {
    let input = A06.replace("(A,B)\n(A,Z)\n(B,Z)", " (Z,B) \n\n(B,A)\n(Z,A)");
    let r = check("06", input.as_bytes());
    assert!(r.issues.iter().any(|issue| issue.code == "pair-order"));
    assert!(r.issues.iter().any(|issue| issue.code == "pair-sort"));
    assert_eq!(r.normalized.as_deref(), Some(A06));
    assert_eq!(r.hash, check("06", A06.as_bytes()).hash);
    let again = check("06", r.normalized.unwrap().as_bytes());
    assert_eq!(again.normalized.as_deref(), Some(A06));
    assert_eq!(again.hash, r.hash);
    assert_eq!(again.errors(), 0);

    let duplicate = A06.replace("(A,B)", "(B,A)\n(A,B)");
    let r = check("06", duplicate.as_bytes());
    assert!(r.issues.iter().any(|issue| issue.code == "duplicate-pair"));
    let normalized = r.normalized.unwrap();
    assert_eq!(normalized.matches("(A,B)").count(), 2);
    assert_eq!(check("06", normalized.as_bytes()).hash, r.hash);

    let lexical = A06.replace("(A,B)\n(A,Z)\n(B,Z)", "(q2,q10)");
    assert!(
        check("06", lexical.as_bytes())
            .normalized
            .unwrap()
            .contains("6-1.2=====\n(q10,q2)\n")
    );
}

#[test]
fn assignment_06_pairs_check_shape_without_deciding_membership() {
    for pair in [
        "A,B",
        "(A,)",
        "(,B)",
        "(A,B,C)",
        "((A,B))",
        "{A,B}",
        "(A, B)",
        "(A,B)(C,D)",
    ] {
        let input = A06.replace("(A,B)", pair);
        assert!(has(&input, "06", "state-pair"), "{pair}");
        assert!(
            check("06", input.as_bytes())
                .hash_input
                .unwrap()
                .contains(pair)
        );
    }
    for pair in ["(A,A)", "(notInGraph,otherState)"] {
        let input = A06.replace("(A,B)\n(A,Z)\n(B,Z)", pair);
        assert_eq!(check("06", input.as_bytes()).errors(), 0, "{pair}");
    }
}

#[test]
fn assignment_06_empty_pair_list_is_allowed_and_has_canonical_hash() {
    let input = A06.replace("(A,B)\n(A,Z)\n(B,Z)\n", "");
    let r = check("06", input.as_bytes());
    assert_eq!(r.errors(), 0);
    assert_eq!(r.normalized.as_deref(), Some(input.as_str()));
    assert!(
        r.hash_input
            .as_ref()
            .unwrap()
            .contains("6-1.2=====\n========\n")
    );
    assert_eq!(check("06", r.normalized.unwrap().as_bytes()).hash, r.hash);
    let comments = input.replace("6-1.2=====\n", "6-1.2=====\n\n%%% 0件の組\n\n");
    assert_eq!(check("06", comments.as_bytes()).errors(), 0);
    assert_eq!(check("06", comments.as_bytes()).hash, r.hash);
    // Only this list section permits an empty answer; automaton sections do not.
    let missing_automaton =
        input.replace("6-1.3=====\nq2\n\n(q1,x,q2)\n(q2,y,q1)\n", "6-1.3=====\n");
    assert!(has(&missing_automaton, "06", "unanswered"));
}

#[test]
fn assignment_06_checks_numbered_labels_but_never_reassigns_classes() {
    for label in ["q0", "q02", "Q2", "q-2", "A", "q", "q2x"] {
        let input = A06.replace("q2", label);
        assert!(has(&input, "06", "numbered-state"), "{label}");
        assert!(
            check("06", input.as_bytes())
                .normalized
                .unwrap()
                .contains(label)
        );
    }
    assert!(has(
        &A06.replace("q2", "q3"),
        "06",
        "numbered-state-sequence"
    ));
    assert!(has(
        &A06.replace("q2", "q184467440737095516160000"),
        "06",
        "numbered-state-sequence"
    ));
    let states: Vec<_> = (1..=10).map(|i| format!("q{i}")).collect();
    let mut sorted = states.clone();
    sorted.sort_unstable();
    let transitions = sorted
        .iter()
        .map(|state| format!("({state},x,{state})"))
        .collect::<Vec<_>>()
        .join("\n");
    let replacement = format!("6-1.3=====\n{}\n\n{transitions}\n", sorted.join(","));
    let input = A06.replace("6-1.3=====\nq2\n\n(q1,x,q2)\n(q2,y,q1)\n", &replacement);
    assert_eq!(check("06", input.as_bytes()).errors(), 0);
    let numeric_order = input.replace(&sorted.join(","), &states.join(","));
    assert!(has(&numeric_order, "06", "final-state-sort"));
    assert_eq!(
        check("06", numeric_order.as_bytes()).normalized.as_deref(),
        Some(input.as_str())
    );
}

#[test]
fn new_assignments_reject_malformed_markers_and_triples() {
    for (id, input, marker) in [("05", A05, "5-1====="), ("06", A06, "6-1.1=====")] {
        assert!(
            check(id, input.replace(marker, &format!("{marker} ")).as_bytes())
                .hash
                .is_none()
        );
        assert!(
            check(id, input.replace(marker, "99-1=====").as_bytes())
                .hash
                .is_none()
        );
    }
    assert!(has(
        &A05.replace("(q0,x,q1)", "(q0,,q1)"),
        "05",
        "transition"
    ));
    assert!(has(
        &A06.replace("(A,x,B)", "(A,x,B,C)"),
        "06",
        "transition"
    ));
    assert!(check("05", A06.as_bytes()).hash.is_none());
    assert!(check("06", A05.as_bytes()).hash.is_none());
}

#[test]
fn assignment_04_hash_states_survive_comments_newlines_and_repeat_checking() {
    let edited = A04
        .replace("00X00000", "99Z99999")
        .replace("ダミー", "別のダミー")
        .replace("4-1=====\n", "4-1=====\n### instruction\n# comment\n")
        .replace(
            "4-2=====\n",
            "#### outer instruction\n4-2=====\n### inner instruction\n",
        )
        .replace(
            "(AB,y,#A)\n",
            "(AB,y,#A)\n# remove this comment\n### instruction\n",
        )
        .replace('\n', "\r\n");
    let r = check("04", format!("\u{feff}{edited}").as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    let expected = check("04", A04.as_bytes());
    assert_eq!(r.hash, expected.hash);
    let normalized = r.normalized.as_ref().unwrap();
    assert!(normalized.contains("4-1=====\n#,A\n"));
    assert!(normalized.contains("4-2=====\n#A,#AB\n"));
    assert!(!normalized.contains("instruction"));
    assert!(!normalized.contains("comment"));
    let again = check("04", normalized.as_bytes());
    assert_eq!(again.errors(), 0);
    assert_eq!(again.normalized, r.normalized);
    assert_eq!(again.hash, r.hash);
    let only_hash = A04.replace("4-1=====\n#,A", "4-1=====\n#");
    assert_eq!(check("04", only_hash.as_bytes()).errors(), 0);
    assert!(
        check("04", only_hash.as_bytes())
            .hash_input
            .unwrap()
            .contains("4-1=====\n#\n")
    );
}

#[test]
fn assignment_04_sorts_lists_but_never_renames_states() {
    let input = A04
        .replace("4-1=====\n#,A", "4-1=====\nA,#")
        .replace("4-2=====\n#A,#AB", "4-2=====\n#AB,#A")
        .replace(
            "(#A,x,#AB)\n(AB,y,#A)\n(S,x,AB)",
            "(S,x,AB)\n(#A,x,#AB)\n(AB,y,#A)",
        )
        .replace("A->$\nA->xA", "A->xA\nA->$");
    let r = check("04", input.as_bytes());
    assert!(r.issues.iter().any(|i| i.code == "final-state-sort"));
    assert!(r.issues.iter().any(|i| i.code == "sort"));
    assert_eq!(r.normalized.as_deref(), Some(A04));
    assert_eq!(r.hash, check("04", A04.as_bytes()).hash);
    assert_eq!(check("04", r.normalized.unwrap().as_bytes()).errors(), 0);

    for (before, after) in [
        ("#A,#AB", "#A,AB#"),
        ("#A,#AB", "#BA"),
        ("(S,x,AB)", "(BA,x,AB)"),
        ("(S,x,AB)", "(S,x,BA)"),
    ] {
        let bad = A04.replace(before, after);
        let r = check("04", bad.as_bytes());
        assert!(r.issues.iter().any(|i| i.code == "subset-label-sort"));
        assert_eq!(r.normalized.as_deref(), Some(bad.as_str()));
        assert!(has(
            r.normalized.as_ref().unwrap(),
            "04",
            "subset-label-sort"
        ));
    }
    // The label-order check inspects source/target, never the input symbol.
    assert!(!has(
        &A04.replace("(S,x,AB)", "(S,zx,AB)"),
        "04",
        "subset-label-sort"
    ));
}

#[test]
fn assignment_04_keeps_malformed_hash_final_rows_for_diagnostics() {
    for finals in ["#A,", "#A B", "#A,,B"] {
        let input = A04.replace("#A,#AB", finals);
        let r = check("04", input.as_bytes());
        assert!(
            r.issues.iter().any(|i| i.code == "final-states"),
            "{finals}"
        );
        assert!(r.hash_input.unwrap().contains("#A"));
    }
    let input = A04.replace("#A,#AB", "　#A,#AB");
    assert!(has(&input, "04", "ascii"));
    assert!(
        check("04", input.as_bytes())
            .normalized
            .unwrap()
            .contains("　#A")
    );
}

#[test]
fn assignment_04_checks_renaming_not_graph_correctness() {
    for (before, after, code) in [
        ("S->xA\nS->yB", "C->xA\nC->yB", "start-label"),
        ("B->xA", "D->xA", "state-label-sequence"),
        ("A->xA", "AB->xA", "state-label"),
        ("S->xA", "S->xAB", "state-label"),
        ("S->xA", "S->x#", "state-label"),
        ("S->xA", "S->x#AB", "state-label"),
        ("S->xA", "S->xq", "state-label"),
    ] {
        assert!(has(&A04.replace(before, after), "04", code), "{after}");
    }
    for rhs in ["S->xB\nS->yA", "S->xA\nS->xB\nS->yA"] {
        let input = A04.replace("S->xA\nS->yB", rhs);
        let r = check("04", input.as_bytes());
        assert!(r.issues.iter().any(|i| i.code == "state-order-cross"));
        assert!(r.normalized.unwrap().contains(rhs));
    }
    // Crossings across different left sides, equal terminals, determinism and
    // whether these arbitrary productions describe the supplied DFA are not checked.
    let different_lhs = A04
        .replace("A->xA\nA->yB", "A->xB")
        .replace("B->xA\nB->yB", "B->yA");
    assert_eq!(check("04", different_lhs.as_bytes()).errors(), 0);
    let equal_terminal = A04.replace("S->xA\nS->yB", "S->xA\nS->xB");
    assert_eq!(check("04", equal_terminal.as_bytes()).errors(), 0);
}

#[test]
fn assignment_04_requires_its_own_exact_markers_and_notation() {
    for input in [
        A04.replace("4-1=====", "4-1==="),
        A04.replace("4-2=====", "3-2====="),
        A04.replace("4-3=====", "4-2====="),
    ] {
        assert!(check("04", input.as_bytes()).hash.is_none());
    }
    assert!(has(&A04.replace("(S,x,A)", "S,x,A"), "04", "transition"));
    assert!(has(&A04.replace("A->$", "A->ε"), "04", "epsilon-notation"));
    assert!(has(&A04.replace("A->xA", "A->xA|yB"), "04", "production"));
    assert!(check("04", A03.as_bytes()).hash.is_none());
}
