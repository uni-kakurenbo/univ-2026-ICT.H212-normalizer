use crate::{AnswerLine, Report, Severity, edge_trim};
use std::collections::{BTreeMap, BTreeSet};

// Generic lecture procedures, applied to ORIGINAL problem grammars below.
// Never retain/display solved sheets, transform submitted input, auto-correct
// an answer, or claim arbitrary CFG equivalence. A mismatch is warning-only.
const MAX_RULES: usize = 4096;
const MAX_RHS: usize = 64;
const STEPS: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Rule {
    left: String,
    right: Vec<String>,
}

#[derive(Clone)]
struct Grammar {
    start: String,
    rules: BTreeSet<Rule>,
}

#[derive(Clone, Copy)]
pub(super) enum Problem {
    Epsilon11,
    Cnf12First,
    Cnf12Second,
}

struct Limit;
type Result<T> = std::result::Result<T, Limit>;

struct Budget(usize);
impl Budget {
    fn step(&mut self) -> Result<()> {
        self.0 = self.0.checked_sub(1).ok_or(Limit)?;
        Ok(())
    }
    fn insert(&mut self, set: &mut BTreeSet<Rule>, rule: Rule) -> Result<()> {
        self.step()?;
        if rule.right.len() > MAX_RHS || (set.len() >= MAX_RULES && !set.contains(&rule)) {
            return Err(Limit);
        }
        set.insert(rule);
        Ok(())
    }
}

fn nonterminal(value: &str) -> bool {
    value.starts_with(|c: char| c.is_ascii_uppercase())
        && value.as_bytes()[1..].iter().all(|&c| c == b'\'')
}

fn parse(value: &str) -> Option<Rule> {
    let (left, right) = value.split_once("->")?;
    if !nonterminal(left) || right.is_empty() {
        return None;
    }
    let mut tokens = Vec::new();
    if right != "$" {
        let mut i = 0;
        let bytes = right.as_bytes();
        while i < bytes.len() {
            let start = i;
            match bytes[i] {
                b'A'..=b'Z' => {
                    i += 1;
                    while bytes.get(i) == Some(&b'\'') {
                        i += 1;
                    }
                }
                b'a' | b'b' => i += 1,
                _ => return None,
            }
            tokens.push(right[start..i].to_owned());
        }
    }
    Some(Rule {
        left: left.to_owned(),
        right: tokens,
    })
}

fn original(problem: Problem) -> Grammar {
    // Source problem data only: 11_answer-sheet.txt and 12_answer-sheet.txt.
    let lines: &[&str] = match problem {
        Problem::Epsilon11 => &[
            "S->aAa", "S->bBb", "A->BC", "A->aBb", "A->bCa", "B->$", "B->b", "C->B", "C->BC",
        ],
        Problem::Cnf12First => &[
            "S->bA", "S->aB", "A->a", "A->aS", "A->bAA", "B->b", "B->bS", "B->aBB",
        ],
        Problem::Cnf12Second => &[
            "S->ASB", "S->AB", "A->AS", "A->A", "A->a", "B->SbS", "B->Sb", "B->bS", "B->b", "B->A",
            "B->bb",
        ],
    };
    Grammar {
        start: "S".into(),
        rules: lines
            .iter()
            .map(|s| parse(s).expect("problem grammar"))
            .collect(),
    }
}

fn epsilon_free(grammar: &Grammar, budget: &mut Budget) -> Result<Grammar> {
    let mut nullable = BTreeSet::new();
    loop {
        let mut changed = false;
        for rule in &grammar.rules {
            budget.step()?;
            if rule.right.len() > MAX_RHS {
                return Err(Limit);
            }
            if rule.right.iter().all(|symbol| nullable.contains(symbol)) {
                changed |= nullable.insert(rule.left.clone());
            }
        }
        if !changed {
            break;
        }
    }
    let mut rules = BTreeSet::new();
    for rule in &grammar.rules {
        // Enumerate nullable omissions iteratively, deduplicating repeated
        // symbols early. The budget bounds exponential combinations.
        let mut choices = BTreeSet::from([Vec::new()]);
        for symbol in &rule.right {
            let mut next = BTreeSet::new();
            for prefix in choices {
                budget.step()?;
                let mut kept = prefix.clone();
                kept.push(symbol.clone());
                next.insert(kept);
                if nullable.contains(symbol) {
                    next.insert(prefix);
                }
                if next.len() > MAX_RULES {
                    return Err(Limit);
                }
            }
            choices = next;
        }
        for right in choices.into_iter().filter(|right| !right.is_empty()) {
            budget.insert(
                &mut rules,
                Rule {
                    left: rule.left.clone(),
                    right,
                },
            )?;
        }
    }
    let mut start = grammar.start.clone();
    if nullable.contains(&start) {
        let names = names(grammar);
        while names.contains(&start) {
            budget.step()?;
            start.push('\'');
        }
        budget.insert(
            &mut rules,
            Rule {
                left: start.clone(),
                right: vec![],
            },
        )?;
        budget.insert(
            &mut rules,
            Rule {
                left: start.clone(),
                right: vec![grammar.start.clone()],
            },
        )?;
    }
    Ok(Grammar { start, rules })
}

fn names(grammar: &Grammar) -> BTreeSet<String> {
    grammar
        .rules
        .iter()
        .flat_map(|r| std::iter::once(&r.left).chain(r.right.iter().filter(|s| nonterminal(s))))
        .cloned()
        .chain(std::iter::once(grammar.start.clone()))
        .collect()
}

fn unit_free(grammar: &Grammar, budget: &mut Budget) -> Result<Grammar> {
    let mut units: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut ordinary: BTreeMap<&str, Vec<&Rule>> = BTreeMap::new();
    for rule in &grammar.rules {
        budget.step()?;
        if rule.right.len() == 1 && nonterminal(&rule.right[0]) {
            units.entry(&rule.left).or_default().insert(&rule.right[0]);
        } else {
            ordinary.entry(&rule.left).or_default().push(rule);
        }
    }
    let mut rules = BTreeSet::new();
    for left in names(grammar) {
        let mut visited = BTreeSet::new();
        let mut pending = vec![left.as_str()];
        while let Some(at) = pending.pop() {
            budget.step()?;
            if !visited.insert(at) {
                continue;
            }
            if let Some(targets) = units.get(at) {
                pending.extend(targets);
            }
            if let Some(rows) = ordinary.get(at) {
                for rule in rows {
                    budget.insert(
                        &mut rules,
                        Rule {
                            left: left.clone(),
                            right: rule.right.clone(),
                        },
                    )?;
                }
            }
        }
    }
    Ok(Grammar {
        start: grammar.start.clone(),
        rules,
    })
}

fn fresh(names: &mut BTreeSet<String>, budget: &mut Budget) -> Result<String> {
    for c in b'C'..=b'Z' {
        budget.step()?;
        let name = (c as char).to_string();
        if names.insert(name.clone()) {
            return Ok(name);
        }
    }
    Err(Limit)
}

fn cnf(grammar: &Grammar, budget: &mut Budget) -> Result<Grammar> {
    let grammar = unit_free(&epsilon_free(grammar, budget)?, budget)?;
    let mut used = names(&grammar);
    let terminals: BTreeSet<_> = grammar
        .rules
        .iter()
        .filter(|r| r.right.len() >= 2)
        .flat_map(|r| r.right.iter())
        .filter(|s| !nonterminal(s))
        .cloned()
        .collect();
    let mut proxies = BTreeMap::new();
    let mut replaced = BTreeSet::new();
    for terminal in terminals {
        let label = fresh(&mut used, budget)?;
        budget.insert(
            &mut replaced,
            Rule {
                left: label.clone(),
                right: vec![terminal.clone()],
            },
        )?;
        proxies.insert(terminal, label);
    }
    for rule in grammar.rules {
        let right = if rule.right.len() >= 2 {
            rule.right
                .into_iter()
                .map(|s| proxies.get(&s).cloned().unwrap_or(s))
                .collect()
        } else {
            rule.right
        };
        budget.insert(
            &mut replaced,
            Rule {
                left: rule.left,
                right,
            },
        )?;
    }
    let mut ordered: Vec<_> = replaced.into_iter().collect();
    ordered.sort_by_key(|rule| {
        (
            match rule.left.as_str() {
                "S" => 0,
                "A" => 1,
                "B" => 2,
                _ => 3,
            },
            rule.clone(),
        )
    });
    let mut rules = BTreeSet::new();
    for rule in ordered {
        let mut left = rule.left;
        let mut tail = rule.right.as_slice();
        while tail.len() > 2 {
            let next = fresh(&mut used, budget)?;
            budget.insert(
                &mut rules,
                Rule {
                    left,
                    right: vec![tail[0].clone(), next.clone()],
                },
            )?;
            left = next;
            tail = &tail[1..];
        }
        budget.insert(
            &mut rules,
            Rule {
                left,
                right: tail.to_vec(),
            },
        )?;
    }
    Ok(Grammar {
        start: grammar.start,
        rules,
    })
}

fn reachable(grammar: &Grammar, budget: &mut Budget) -> Result<BTreeSet<Rule>> {
    let mut seen = BTreeSet::from([grammar.start.clone()]);
    loop {
        let mut changed = false;
        for rule in &grammar.rules {
            budget.step()?;
            if seen.contains(&rule.left) {
                for symbol in rule.right.iter().filter(|s| nonterminal(s)) {
                    changed |= seen.insert(symbol.clone());
                }
            }
        }
        if !changed {
            break;
        }
    }
    Ok(grammar
        .rules
        .iter()
        .filter(|rule| seen.contains(&rule.left))
        .cloned()
        .collect())
}

fn matches(submitted: &BTreeSet<Rule>, expected: &Grammar, budget: &mut Budget) -> Result<bool> {
    Ok(*submitted == expected.rules || *submitted == reachable(expected, budget)?)
}

pub(super) fn check(r: &mut Report, rows: &[&AnswerLine<'_>], problem: Problem) {
    if rows.is_empty() {
        return;
    }
    let Some(submitted) = rows
        .iter()
        .map(|row| parse(edge_trim(row.text)))
        .collect::<Option<BTreeSet<_>>>()
    else {
        return;
    };
    let mut budget = Budget(STEPS);
    let computed = match problem {
        Problem::Epsilon11 => epsilon_free(&original(problem), &mut budget),
        _ => cnf(&original(problem), &mut budget),
    }
    .and_then(|expected| matches(&submitted, &expected, &mut budget));
    match computed {
        Ok(true) => {}
        Ok(false) => {
            let (code, step) = match problem {
                Problem::Epsilon11 => ("mechanical-epsilon-procedure", "ε生成規則を除去する講義の手順"),
                _ => ("mechanical-cnf-procedure", "単位規則除去・終端専用記号の導入・右辺の分割という講義の手順"),
            };
            r.issue(Severity::Warning, code, rows.first().map(|row| row.number), format!("{step}で計算した規則集合との一致を確認できません。規則の不足・追加・異なる変換方法や補助記号の構成を確認してください。不一致だけで非等価・不正解とは判定しません。入力・修正案は変更しません。"));
        }
        Err(_) => r.issue(Severity::Warning, "mechanical-grammar-limit", rows.first().map(|row| row.number), "変換手順の計算上限に達したため、規則集合の照合は未検査です。等価性や正解を確認した状態ではありません。入力は保持します。"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grammar(lines: &[&str]) -> Grammar {
        Grammar {
            start: "S".into(),
            rules: lines.iter().map(|s| parse(s).unwrap()).collect(),
        }
    }
    #[test]
    fn nullable_omissions_and_fresh_start_handle_cycles_and_repeated_symbols() {
        let source = grammar(&["S->AA", "A->B", "B->$", "B->b"]);
        let output = epsilon_free(&source, &mut Budget(STEPS)).unwrap_or_else(|_| panic!("budget"));
        assert_eq!(output.start, "S'");
        assert!(output.rules.contains(&parse("S'->$").unwrap()));
        assert!(output.rules.contains(&parse("S'->S").unwrap()));
        assert!(output.rules.contains(&parse("S->A").unwrap()));
        assert!(!output.rules.contains(&parse("B->$").unwrap()));
        assert!(output.rules.contains(&parse("B->b").unwrap()));
    }
    #[test]
    fn unit_cycles_disappear_and_cnf_allocates_only_needed_proxies_before_s_a_b_helpers() {
        let source = grammar(&["S->ASB", "A->A", "A->a", "B->SbS", "B->A", "B->b"]);
        let output = cnf(&source, &mut Budget(STEPS)).unwrap_or_else(|_| panic!("budget"));
        assert!(output.rules.contains(&parse("C->b").unwrap()));
        assert!(!output.rules.contains(&parse("C->a").unwrap()));
        assert!(output.rules.contains(&parse("B->a").unwrap()));
        assert!(output.rules.contains(&parse("S->AD").unwrap()));
        assert!(output.rules.contains(&parse("D->SB").unwrap()));
        assert!(output.rules.contains(&parse("B->SE").unwrap()));
        assert!(output.rules.contains(&parse("E->CS").unwrap()));
        assert!(
            output
                .rules
                .iter()
                .all(|r| !(r.right.len() == 1 && nonterminal(&r.right[0])))
        );
    }
    #[test]
    fn comparisons_ignore_duplicates_and_allow_unreachable_rule_removal_but_not_other_differences()
    {
        let expected = grammar(&["S->AB", "A->a", "B->b", "C->a"]);
        let pruned = grammar(&["S->AB", "A->a", "B->b", "B->b"]);
        assert!(matches(&pruned.rules, &expected, &mut Budget(STEPS)).is_ok_and(|m| m));
        let wrong = grammar(&["S->AB", "A->b", "B->b"]);
        assert!(matches(&wrong.rules, &expected, &mut Budget(STEPS)).is_ok_and(|m| !m));
        assert!(epsilon_free(&expected, &mut Budget(1)).is_err());
    }
    #[test]
    fn all_problem_data_procedures_finish_within_budget_without_storing_solutions() {
        for problem in [
            Problem::Epsilon11,
            Problem::Cnf12First,
            Problem::Cnf12Second,
        ] {
            let mut budget = Budget(STEPS);
            let expected = match problem {
                Problem::Epsilon11 => epsilon_free(&original(problem), &mut budget),
                _ => cnf(&original(problem), &mut budget),
            }
            .unwrap_or_else(|_| panic!("budget"));
            assert!(matches(&expected.rules, &expected, &mut budget).is_ok_and(|m| m));
            if !matches!(problem, Problem::Epsilon11) {
                assert!(expected.rules.iter().all(
                    |r| r.right.len() == 2 || (r.right.len() == 1 && !nonterminal(&r.right[0]))
                ));
            }
        }
    }
}
