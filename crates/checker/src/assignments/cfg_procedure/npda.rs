use super::{Budget, Grammar, Limit, MAX_RHS, Result, Rule, STEPS, reachable};
use crate::assignments::grammar::Notation;
use crate::{Report, Severity};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub(in crate::assignments) enum Problem {
    First,
    Second,
}

// Template tuple order: source, input, popped symbol, target, replacement.
struct Transition(u8, Option<char>, char, u8, &'static str);

struct Machine {
    states: &'static [u8],
    stack: &'static str,
    alphabet: &'static str,
    start: u8,
    bottom: char,
    transitions: &'static [Transition],
}

fn original(problem: Problem) -> Machine {
    match problem {
        Problem::First => Machine {
            states: &[1, 2, 3],
            stack: "AZ",
            alphabet: "ab",
            start: 1,
            bottom: 'Z',
            transitions: &[
                Transition(1, Some('a'), 'Z', 1, "AZ"),
                Transition(1, Some('a'), 'A', 1, "AA"),
                Transition(1, Some('b'), 'A', 2, ""),
                Transition(2, Some('b'), 'A', 2, ""),
                Transition(2, None, 'Z', 3, ""),
            ],
        },
        Problem::Second => Machine {
            states: &[1, 2],
            stack: "XZ",
            alphabet: "01",
            start: 2,
            bottom: 'Z',
            transitions: &[
                Transition(2, Some('1'), 'Z', 2, "XZ"),
                Transition(2, Some('1'), 'X', 2, "XX"),
                Transition(2, Some('0'), 'X', 1, "X"),
                Transition(2, None, 'X', 2, ""),
                Transition(1, Some('1'), 'X', 1, ""),
                Transition(1, Some('0'), 'Z', 2, ""),
            ],
        },
    }
}

impl Problem {
    pub(in crate::assignments) fn notation(self) -> Notation {
        let machine = original(self);

        Notation::Indexed {
            stack: machine.stack,
            states: machine.states,
            alphabet: machine.alphabet,
        }
    }
}

fn variable(symbol: char, from: u8, to: u8) -> String {
    format!("{symbol}^{{{from},{to}}}")
}

fn is_variable(symbol: &str) -> bool {
    symbol.starts_with(|c: char| c.is_ascii_uppercase())
}

fn construct(machine: &Machine, budget: &mut Budget) -> Result<Grammar> {
    if machine.states.is_empty() {
        return Err(Limit);
    }

    let start = "S".to_owned();
    let mut rules = BTreeSet::new();

    for &end in machine.states {
        budget.insert(
            &mut rules,
            Rule {
                left: start.clone(),
                right: vec![variable(machine.bottom, machine.start, end)],
            },
        )?;
    }

    for &Transition(from, input, pop, target, push) in machine.transitions {
        if push.len() + usize::from(input.is_some()) > MAX_RHS {
            return Err(Limit);
        }

        // The endpoint of each pushed symbol is the start of the next one.
        let mut endpoints = vec![0; push.len()];
        loop {
            let mut state = target;
            let mut right: Vec<_> = input.iter().map(char::to_string).collect();

            for (symbol, &index) in push.chars().zip(&endpoints) {
                let next = machine.states[index];
                right.push(variable(symbol, state, next));
                state = next;
            }

            budget.insert(
                &mut rules,
                Rule {
                    left: variable(pop, from, state),
                    right,
                },
            )?;

            let mut advanced = false;
            for index in endpoints.iter_mut().rev() {
                *index += 1;
                if *index < machine.states.len() {
                    advanced = true;
                    break;
                }

                *index = 0;
            }

            if !advanced {
                break;
            }
        }
    }

    Ok(Grammar { start, rules })
}

fn remove_nonproductive(grammar: &mut Grammar, budget: &mut Budget) -> Result<()> {
    let mut productive = BTreeSet::new();

    loop {
        let mut changed = false;

        for rule in &grammar.rules {
            budget.step()?;
            if rule
                .right
                .iter()
                .all(|s| !is_variable(s) || productive.contains(s))
            {
                changed |= productive.insert(rule.left.clone());
            }
        }

        if !changed {
            break;
        }
    }

    grammar.rules.retain(|rule| {
        rule.right
            .iter()
            .all(|s| !is_variable(s) || productive.contains(s))
    });

    Ok(())
}

fn same_rules(submitted: &BTreeSet<Rule<&str>>, expected: &BTreeSet<Rule>) -> bool {
    submitted.len() == expected.len()
        && submitted.iter().zip(expected).all(|(actual, expected)| {
            actual.left == expected.left
                && actual
                    .right
                    .iter()
                    .copied()
                    .eq(expected.right.iter().map(String::as_str))
        })
}

fn compare(
    submitted: &BTreeSet<Rule<&str>>,
    machine: &Machine,
    budget: &mut Budget,
) -> Result<bool> {
    let mut expected = construct(machine, budget)?;
    remove_nonproductive(&mut expected, budget)?;

    if same_rules(submitted, &expected.rules) {
        return Ok(true);
    }

    Ok(same_rules(
        submitted,
        &reachable(&expected, budget, is_variable)?,
    ))
}

pub(in crate::assignments) fn check(
    r: &mut Report,
    line: usize,
    submitted: &BTreeSet<Rule<&str>>,
    problem: Problem,
) {
    match compare(submitted, &original(problem), &mut Budget(STEPS)) {
        Ok(true) => {}
        Ok(false) => r.issue(Severity::Warning, "mechanical-npda-cfg-procedure", Some(line), "講義のNPDA→CFG変換で計算した規則集合と一致しません。添字の対応、終端列を生成できない規則の除去、単位規則の保持を確認してください。この照合は講義手順との一致を確認するものです。"),
        Err(_) => r.issue(Severity::Warning, "mechanical-grammar-limit", Some(line), "計算上限に達したため、NPDA→CFGの手順照合は未検査です。"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn borrowed(rules: &BTreeSet<Rule>) -> BTreeSet<Rule<&str>> {
        rules
            .iter()
            .map(|rule| Rule {
                left: rule.left.as_str(),
                right: rule.right.iter().map(String::as_str).collect(),
            })
            .collect()
    }

    #[test]
    fn construction_composes_state_indices_and_keeps_productive_unit_rules() {
        let machine = Machine {
            states: &[1, 2],
            stack: "ABCXYZ",
            alphabet: "cde",
            start: 1,
            bottom: 'Z',
            transitions: &[
                Transition(1, Some('c'), 'Z', 2, "AB"),
                Transition(2, None, 'A', 1, ""),
                Transition(1, Some('d'), 'B', 2, ""),
                Transition(1, Some('e'), 'X', 1, ""),
                Transition(1, None, 'C', 1, "C"),
                Transition(2, None, 'Y', 2, "A"),
            ],
        };
        let pruned = BTreeSet::from([
            Rule {
                left: "S",
                right: vec!["Z^{1,2}"],
            },
            Rule {
                left: "Z^{1,2}",
                right: vec!["c", "A^{2,1}", "B^{1,2}"],
            },
            Rule {
                left: "A^{2,1}",
                right: vec![],
            },
            Rule {
                left: "B^{1,2}",
                right: vec!["d"],
            },
        ]);
        let mut productive = pruned.clone();
        productive.extend([
            Rule {
                left: "X^{1,1}",
                right: vec!["e"],
            },
            Rule {
                left: "Y^{2,1}",
                right: vec!["A^{2,1}"],
            },
        ]);

        assert!(compare(&pruned, &machine, &mut Budget(STEPS)).is_ok_and(|matches| matches));
        assert!(compare(&productive, &machine, &mut Budget(STEPS)).is_ok_and(|matches| matches));

        let mut rewritten = pruned;
        rewritten.retain(|rule| rule.left != "S");
        assert!(compare(&rewritten, &machine, &mut Budget(STEPS)).is_ok_and(|matches| !matches));
        assert!(construct(&machine, &mut Budget(1)).is_err());
    }

    #[test]
    fn both_problem_machines_finish_within_budget() {
        for problem in [Problem::First, Problem::Second] {
            let machine = original(problem);
            let mut budget = Budget(STEPS);
            let mut grammar =
                construct(&machine, &mut budget).unwrap_or_else(|_| panic!("construction budget"));
            remove_nonproductive(&mut grammar, &mut budget)
                .unwrap_or_else(|_| panic!("pruning budget"));
            let useful = reachable(&grammar, &mut budget, is_variable)
                .unwrap_or_else(|_| panic!("reachability budget"));

            assert!(
                useful
                    .iter()
                    .any(|rule| rule.left == "S" && rule.right.len() == 1)
            );
            assert!(
                compare(&borrowed(&useful), &machine, &mut Budget(STEPS))
                    .is_ok_and(|matches| matches)
            );
        }
    }
}
