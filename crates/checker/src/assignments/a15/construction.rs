use std::collections::BTreeSet;

// Lecture p.7: initialize the start symbol, rewrite a nonterminal by each CFG
// production on epsilon input, consume matching terminals, then pop the bottom
// into the final state. Bounded and parameterized by original grammar data.
pub(super) fn construct(
    start: &str,
    terminals: &str,
    productions: &[(&str, &str)],
) -> Option<BTreeSet<[String; 5]>> {
    let nonterminal = |s: &str| s.len() == 1 && s.as_bytes()[0].is_ascii_uppercase() && s != "Z";
    if !nonterminal(start)
        || terminals.is_empty()
        || !terminals.bytes().all(|c| c.is_ascii_lowercase())
        || productions.len() > 4090
    {
        return None;
    }
    let mut rules = BTreeSet::new();
    rules.insert(["q0", "$", "Z", "q1", &format!("{start}Z")].map(str::to_owned));
    rules.insert(["q1", "$", "Z", "qF", "$"].map(str::to_owned));
    for c in terminals.bytes() {
        let symbol = char::from(c).to_string();
        rules.insert(["q1", &symbol, &symbol, "q1", "$"].map(str::to_owned));
    }
    for &(lhs, rhs) in productions {
        if !nonterminal(lhs)
            || rhs.is_empty()
            || rhs.len() > 64
            || (rhs != "$"
                && !rhs.bytes().all(|c| {
                    (c.is_ascii_uppercase() && c != b'Z') || terminals.as_bytes().contains(&c)
                }))
        {
            return None;
        }
        rules.insert(["q1", "$", lhs, "q1", rhs].map(str::to_owned));
    }
    (rules.len() <= 4096).then_some(rules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_an_unrelated_grammar_with_epsilon_and_two_terminals() {
        let rules = construct("X", "cd", &[("X", "cY"), ("Y", "d"), ("Y", "$")]).unwrap();
        let expected: BTreeSet<_> = [
            ["q0", "$", "Z", "q1", "XZ"],
            ["q1", "$", "X", "q1", "cY"],
            ["q1", "$", "Y", "q1", "d"],
            ["q1", "$", "Y", "q1", "$"],
            ["q1", "c", "c", "q1", "$"],
            ["q1", "d", "d", "q1", "$"],
            ["q1", "$", "Z", "qF", "$"],
        ]
        .into_iter()
        .map(|t| t.map(str::to_owned))
        .collect();
        assert_eq!(rules, expected);
        assert_eq!(
            construct(
                "X",
                "cd",
                &[("X", "cY"), ("Y", "d"), ("Y", "$"), ("Y", "$")]
            ),
            Some(rules)
        );
    }

    #[test]
    fn bounds_grammar_input_and_rejects_mixed_epsilon_or_bottom_symbols() {
        for rhs in ["", "c$", "Z", "e"] {
            assert!(construct("X", "cd", &[("X", rhs)]).is_none());
        }
        assert!(construct("Z", "c", &[("Z", "c")]).is_none());
        assert!(construct("X", "c", &[("X", &"c".repeat(65))]).is_none());
        assert!(construct("X", "c", &vec![("X", "c"); 4091]).is_none());
    }
}
