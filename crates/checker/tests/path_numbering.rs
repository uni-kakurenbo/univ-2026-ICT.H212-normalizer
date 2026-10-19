use answer_checker::check;

// Finite, synthetic three-branch graph. This is deliberately not a construction
// for the assignment's regex; only the requested numbering rules are exercised.
const EDGES: &[&str] = &[
    "(1,a,2)", "(1,b,4)", "(1,c,6)", "(2,a,3)", "(3,c,9)", "(4,b,5)", "(5,c,9)", "(6,b,7)",
    "(7,a,8)", "(8,a,9)",
];

fn answer(edges: &[&str]) -> String {
    answer_with_final(edges, 9)
}

fn answer_with_final(edges: &[&str], final_state: usize) -> String {
    let mut edges = edges.to_vec();
    edges.sort_unstable();
    let rest = "8-1.2.1=====\nlf(e)={}\ni=0,K={},Delta={}\n========\n\n8-1.2.2=====\ne\n1\n\n(e,x,1)\n========\n";
    format!(
        "学籍番号: 00X00000\n氏名: ダミー\n\n8-1.1=====\n1\n{final_state}\n\n{}\n========\n\n{rest}",
        edges.join("\n")
    )
}

fn has(text: &str, code: &str) -> bool {
    check("08", text.as_bytes())
        .issues
        .iter()
        .any(|issue| issue.code == code)
}

fn relabeled_answer(permutation: &[(u8, u8)]) -> String {
    let edges: Vec<String> = EDGES
        .iter()
        .map(|edge| {
            edge.chars()
                .map(|c| {
                    permutation
                        .iter()
                        .find(|(from, _)| c == char::from(*from))
                        .map_or(c, |(_, to)| char::from(*to))
                })
                .collect()
        })
        .collect();
    answer(&edges.iter().map(String::as_str).collect::<Vec<_>>())
}

#[test]
fn finite_toy_paths_pass_numbering_without_grading_the_language() {
    let text = answer(EDGES);
    let r = check("08", text.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.warnings(), 0, "{:?}", r.issues);
    assert_eq!(r.normalized.as_deref(), Some(text.as_str()));
    // bbc ends at 9 while cbaa's fourth state is 8. The i < n restriction
    // excludes this comparison; comparing all zipped positions would be wrong.
    let repeated = check("08", r.normalized.as_ref().unwrap().as_bytes());
    assert_eq!(repeated.hash, r.hash);
    assert_eq!(repeated.normalized, r.normalized);
}

#[test]
fn decreasing_number_on_a_selected_word_is_diagnosed_without_renaming() {
    let text = relabeled_answer(&[(b'2', b'3'), (b'3', b'2')]);
    let r = check("08", text.as_bytes());
    assert!(has(&text, "path-number-increase"));
    assert_eq!(r.normalized.as_deref(), Some(text.as_str()));
}

#[test]
fn epsilon_edges_are_included_in_the_numbered_state_sequence() {
    let mut edges: Vec<_> = EDGES
        .iter()
        .copied()
        .filter(|edge| !["(1,a,2)", "(2,a,3)", "(3,c,9)"].contains(edge))
        .collect();
    edges.extend(["(1,a,3)", "(3,$,2)", "(2,a,4)", "(4,c,9)"]);
    let text = answer(&edges);
    let r = check("08", text.as_bytes());
    let issue = r
        .issues
        .iter()
        .find(|issue| issue.code == "path-number-increase")
        .expect("epsilon descent must be detected");
    assert_eq!(
        issue.line,
        text.lines()
            .position(|line| line == "(3,$,2)")
            .map(|i| i + 1)
    );
    assert_eq!(r.normalized.as_deref(), Some(text.as_str()));
}

#[test]
fn each_of_the_two_cross_word_comparisons_is_checked() {
    let first = relabeled_answer(&[(b'2', b'4'), (b'4', b'2'), (b'3', b'5'), (b'5', b'3')]);
    assert!(has(&first, "path-order-aac-bbc"));
    let second = relabeled_answer(&[(b'4', b'6'), (b'6', b'4'), (b'5', b'7'), (b'7', b'5')]);
    assert!(has(&second, "path-order-bbc-cbaa"));
    for text in [first, second] {
        assert!(!has(&text, "path-number-increase"));
        assert_eq!(
            check("08", text.as_bytes()).normalized.as_deref(),
            Some(text.as_str())
        );
    }
}

#[test]
fn path_lengths_must_match_the_two_stated_comparison_shapes() {
    let mut edges: Vec<_> = EDGES
        .iter()
        .copied()
        .filter(|edge| *edge != "(1,a,2)")
        .collect();
    edges.extend(["(1,$,2)", "(3,a,4)", "(4,c,9)"]);
    assert!(has(&answer(&edges), "path-length-aac-bbc"));

    let mut edges = EDGES.to_vec();
    edges.retain(|edge| *edge != "(1,b,4)");
    edges.extend(["(1,$,2)", "(2,b,4)"]);
    assert!(has(&answer(&edges), "path-length-bbc-cbaa"));
}

#[test]
fn returning_paths_and_duplicate_edges_do_not_create_ambiguous_sequences() {
    let mut edges = EDGES.to_vec();
    edges.extend(["(2,$,3)", "(3,$,2)", "(9,x,1)", "(1,a,2)"]);
    let text = answer(&edges);
    let r = check("08", text.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.warnings(), 0, "{:?}", r.issues);
    assert_eq!(r.normalized.as_deref(), Some(text.as_str()));
    assert_eq!(text.matches("(1,a,2)").count(), 2);
}

#[test]
fn different_epsilon_placements_with_the_same_state_sequence_are_not_ambiguous() {
    let text = answer_with_final(
        &[
            "(1,$,2)", "(1,a,2)", "(2,$,3)", "(2,a,3)", "(3,a,4)", "(4,c,5)",
        ],
        5,
    );
    assert!(!has(&text, "path-numbering-ambiguous"));
    assert_eq!(check("08", text.as_bytes()).warnings(), 2); // bbc, cbaa only.
    assert_eq!(check("08", text.as_bytes()).errors(), 0);
}

#[test]
fn path_number_comparisons_use_integer_order_for_two_digit_states() {
    let mut edges: Vec<String> = EDGES
        .iter()
        .map(|edge| {
            let parts: Vec<_> = edge.trim_matches(['(', ')']).split(',').collect();
            format!(
                "({},{},{})",
                parts[0].parse::<usize>().unwrap() + 2,
                parts[1],
                parts[2].parse::<usize>().unwrap() + 2
            )
        })
        .collect();
    edges.extend(["(1,$,2)".into(), "(2,$,3)".into()]);
    let text = answer_with_final(&edges.iter().map(String::as_str).collect::<Vec<_>>(), 11);
    let r = check("08", text.as_bytes());
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.warnings(), 0, "{:?}", r.issues);
    assert_eq!(r.normalized.as_deref(), Some(text.as_str()));
}

#[test]
fn missing_and_ambiguous_simple_paths_warn_instead_of_selecting_or_grading() {
    let mut missing = EDGES.to_vec();
    missing.retain(|edge| *edge != "(3,c,9)");
    let missing = answer(&missing);
    let mut ambiguous = EDGES.to_vec();
    ambiguous.extend(["(2,$,4)", "(4,a,3)"]);
    let ambiguous = answer(&ambiguous);
    for (text, code) in [
        (missing, "path-numbering-missing"),
        (ambiguous, "path-numbering-ambiguous"),
    ] {
        let r = check("08", text.as_bytes());
        assert_eq!(r.errors(), 0, "{:?}", r.issues);
        assert!(has(&text, code));
        assert!(!has(&text, "path-order-aac-bbc"));
        assert_eq!(r.normalized.as_deref(), Some(text.as_str()));
        assert!(r.hash.is_some());
    }
}

#[test]
fn exponential_simple_path_search_stops_with_an_explicit_unchecked_warning() {
    let mut edges = vec![
        "(1,$,4)".to_string(),
        "(1,a,2)".to_string(),
        "(2,a,3)".to_string(),
    ];
    let mut current = 4;
    for _ in 0..17 {
        let (left, right, next) = (current + 1, current + 2, current + 3);
        edges.extend([
            format!("({current},$,{left})"),
            format!("({current},$,{right})"),
            format!("({left},$,{next})"),
            format!("({right},$,{next})"),
        ]);
        current = next;
    }
    edges.extend([format!("({current},$,1)"), format!("(3,c,{})", current + 1)]);
    let text = answer_with_final(
        &edges.iter().map(String::as_str).collect::<Vec<_>>(),
        current + 1,
    );
    let r = check("08", text.as_bytes());
    assert!(has(&text, "path-numbering-limit"));
    assert_eq!(r.errors(), 0, "{:?}", r.issues);
    assert_eq!(r.normalized.as_deref(), Some(text.as_str()));
}
