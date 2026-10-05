use answer_checker::{HASH_VERSION, MAX_BYTES, check, sha256};

const A01: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n1-1=====\nx,y\n========\n\n1-2=====\n日本語の 説明\n========\n\n1-3=====\nZ\nY\nX\n========\n";
const A02: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n2-1.1===\nA->x\nS->$\n========\n\n2-1.2===\nB->y\nS->zB\n========\n\n2-2=====\nC->z\nS->zC\n========\n";
const A03: &str = "学籍番号: 00X00000\n氏名: ダミー\n\n3-1===\nA\n\n(S,x,A)\n========\n\n3-2=====\nB\n\n(S,y,B)\n========\n\n3-3=====\ndiagram.png\n========\n";

fn has(input: &str, id: &str, code: &str) -> bool {
    check(id, input.as_bytes())
        .issues
        .iter()
        .any(|i| i.code == code)
}

#[test]
fn dummy_answers_are_not_graded() {
    for (id, input) in [("01", A01), ("02", A02), ("03", A03)] {
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
