use answer_checker::{Report, Severity, check, sha256};

const DEFINITION: &str = "M=({start,other},{c,b,a},{Z,#},p,start,Z,{other,start})";
const RULES: &str =
    "(other,$,#,start,$)\n(other,a,Z,other,Z)\n(start,b,#,start,##)\n(start,c,Z,other,#Z)";
const PROOF: &str = "  L1 or L2 と ~L を考える。\n\n  a^{k} in L1 and L2 と書く。";

fn sheet(machines: [(&str, &str); 3], proof: &str) -> String {
    let blocks: Vec<_> = machines
        .iter()
        .enumerate()
        .map(|(index, (definition, rules))| {
            format!("18-1.{}=====\n{definition}\n{rules}\n========", index + 1)
        })
        .collect();
    format!(
        "学籍番号: 00X00000\n氏名: ダミー\n\n{}\n\n18-2=====\n{proof}\n========\n",
        blocks.join("\n\n")
    )
}

fn report(definition: &str, rules: &str) -> Report {
    check(
        "18",
        sheet(
            [
                (definition, rules),
                (DEFINITION, RULES),
                (DEFINITION, RULES),
            ],
            PROOF,
        )
        .as_bytes(),
    )
}

#[test]
fn normalization_sorts_transitions_and_preserves_set_order_duplicates_and_proof_layout() {
    let unsorted = format!("(start,c,Z,other,#Z)\n{RULES}");
    let sorted = format!("{RULES}\n(start,c,Z,other,#Z)");
    let input = sheet(
        [
            (DEFINITION, &unsorted),
            (DEFINITION, RULES),
            (DEFINITION, RULES),
        ],
        PROOF,
    )
    .replace("18-1.1=====\n", "18-1.1=====\n%%% 注意書き\n")
    .replace('\n', "\r\n");
    let expected = sheet(
        [
            (DEFINITION, &sorted),
            (DEFINITION, RULES),
            (DEFINITION, RULES),
        ],
        PROOF,
    );
    let result = check("18", input.as_bytes());

    assert!(
        result
            .issues
            .iter()
            .any(|issue| issue.code == "pda-transition-sort")
    );
    assert!(result.issues.iter().any(
        |issue| issue.code == "pda-transition-duplicate" && issue.severity == Severity::Warning
    ));
    assert_eq!(result.normalized.as_deref(), Some(expected.as_str()));
    assert_eq!(
        result.hash,
        Some(sha256(expected.split_once("\n\n").unwrap().1))
    );

    let again = check("18", expected.as_bytes());
    assert_eq!(again.errors(), 0, "{:?}", again.issues);
    assert_eq!(again.hash, result.hash);
    assert_eq!(again.normalized, result.normalized);
}

#[test]
fn determinism_checks_each_stack_top_and_reports_each_distinct_conflict() {
    assert_eq!(report(DEFINITION, RULES).errors(), 0);

    for alternatives in [
        "(other,a,Z,other,#Z)\n(other,$,Z,other,Z)",
        "(other,$,Z,other,Z)\n(other,a,Z,other,#Z)",
    ] {
        let result = report(
            DEFINITION,
            &format!("{alternatives}\n{RULES}\n(other,a,Z,other,Z)"),
        );
        for code in ["dpda-conflicting-transition", "dpda-epsilon-conflict"] {
            assert_eq!(
                result
                    .issues
                    .iter()
                    .filter(|issue| issue.code == code && issue.severity == Severity::Error)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn limits_count_distinct_declared_states_and_each_transition_uses_declared_symbols() {
    let four = DEFINITION.replace("{start,other}", "{start,other,third,fourth,other}");
    let input = sheet([(&four, RULES); 3], PROOF);
    let result = check("18", input.as_bytes());
    let limits: Vec<_> = result
        .issues
        .iter()
        .filter(|issue| issue.code == "mechanical-pda-states")
        .collect();
    assert_eq!(limits.len(), 1);
    assert!(limits[0].message.contains("上限は 3"));

    let five = four.replace("fourth,other", "fourth,fifth");
    assert!(
        report(&five, RULES).issues.iter().any(
            |issue| issue.code == "mechanical-pda-states" && issue.message.contains("上限は 4")
        )
    );

    for rule in [
        "(unknown,a,Z,other,Z)",
        "(other,x,Z,other,Z)",
        "(other,a,X,other,Z)",
        "(other,a,Z,other,XZ)",
    ] {
        assert!(
            report(DEFINITION, rule)
                .issues
                .iter()
                .any(|issue| issue.code == "mechanical-pda-declaration"
                    && issue.severity == Severity::Error),
            "{rule}"
        );
    }
}

#[test]
fn proof_uses_set_operator_notation_checks_without_rewriting_the_text() {
    let input = sheet([(DEFINITION, RULES); 3], "L1 ∪ L2 と a^k ∈ L を考える。");
    let result = check("18", input.as_bytes());
    for code in ["math-set-operator", "math-membership", "exponent-braces"] {
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.code == code && issue.severity == Severity::Error),
            "{code}"
        );
    }

    assert_eq!(result.normalized.as_deref(), Some(input.as_str()));
}
