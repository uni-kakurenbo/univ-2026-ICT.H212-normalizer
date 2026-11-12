use super::configuration;
use crate::assignments::pda::{Declared, Definition, Syntax};
use crate::{AnswerLine, Report, Severity, edge_trim};
use std::collections::BTreeSet;

pub(super) fn check_related(r: &mut Report, bodies: &[Vec<AnswerLine<'_>>]) {
    let [transitions, definitions, aa, aba] = bodies else {
        return;
    };
    let nonempty = |row: &&AnswerLine<'_>| !row.text.trim().is_empty();
    let definitions: Vec<_> = definitions.iter().filter(nonempty).collect();
    let transitions: Vec<_> = transitions.iter().filter(nonempty).collect();
    check_machine(
        r,
        &definitions,
        &transitions,
        &[(aa, "aa"), (aba, "aba")],
        &Spec {
            alphabet: super::STACK_SYMBOLS,
            definition_section: "14-2",
            transition_section: "14-1",
        },
    );
}

pub(in crate::assignments) struct Spec<'a> {
    pub alphabet: &'a str,
    pub definition_section: &'a str,
    pub transition_section: &'a str,
}

pub(in crate::assignments) fn check_machine(
    r: &mut Report,
    definitions: &[&AnswerLine<'_>],
    transitions: &[&AnswerLine<'_>],
    traces: &[(&[AnswerLine<'_>], &str)],
    spec: &Spec<'_>,
) {
    let trace_entered = traces
        .iter()
        .any(|(rows, _)| rows.iter().any(|row| !row.text.trim().is_empty()));
    let definition_unchecked = |r: &mut Report| {
        if trace_entered {
            r.issue(Severity::Warning, "pda-trace-unchecked", None, format!("{} の定義が空欄・複数行・構文エラーのため、時点表示と NPDA の宣言・遷移の照合は未検査です。", spec.definition_section));
        }
    };
    let mut definitions = definitions.iter();
    let Some(row) = definitions.next() else {
        definition_unchecked(r);
        return;
    };
    if definitions.next().is_some() {
        definition_unchecked(r);
        return;
    }
    let Some(d) = Syntax::Alphanumeric.definition(edge_trim(row.text)) else {
        definition_unchecked(r);
        return;
    };
    // Compile membership once; duplicate set members must not make every
    // transition/trace rescan the whole declaration (quadratic input cost).
    let declared = Declared::from(&d);
    let mut rules = BTreeSet::new();
    let mut valid = true;
    for row in transitions {
        let Some(rule) = Syntax::Alphanumeric.transition(edge_trim(row.text)) else {
            valid = false;
            continue;
        };
        if !declared.contains_transition(rule) {
            r.issue(Severity::Error, "mechanical-pda-declaration", Some(row.number), format!("遷移の状態・入力記号・スタック記号が {} の K/Σ/Γ に含まれていません。宣言と遷移を対応させてください。", spec.definition_section));
        }
        rules.insert(rule);
    }
    if rules.is_empty() || !valid {
        if trace_entered {
            r.issue(
                Severity::Warning,
                "pda-trace-unchecked",
                None,
                format!(
                    "{} の遷移が空欄または構文エラーのため、時点表示の遷移照合は未検査です。",
                    spec.transition_section
                ),
            );
        }
        return;
    }
    let machine = Machine {
        definition: &d,
        declared: &declared,
        rules: &rules,
        spec,
    };
    for (rows, word) in traces {
        check_trace(r, rows, word, &machine);
    }
}

struct Machine<'a, 'b> {
    definition: &'a Definition<'b>,
    declared: &'a Declared<'b>,
    rules: &'a BTreeSet<[&'b str; 5]>,
    spec: &'a Spec<'a>,
}

// Match the exact rule inferred from two consecutive configurations. There is
// no NPDA run enumeration: nondeterminism only means at least one listed rule
// can match. A BTreeSet deduplicates for lookup, never for output/hashing.
fn check_trace(r: &mut Report, rows: &[AnswerLine<'_>], word: &str, machine: &Machine<'_, '_>) {
    let Machine {
        definition: d,
        declared,
        rules,
        spec,
    } = machine;
    let nonempty: Vec<_> = rows
        .iter()
        .filter(|row| !row.text.trim().is_empty())
        .collect();
    let mut previous = None;
    for (i, row) in nonempty.iter().enumerate() {
        let Some(config @ [q, remaining, stack]) =
            configuration(edge_trim(row.text), spec.alphabet)
        else {
            previous = None;
            continue;
        };
        if !declared.states.contains(q)
            || !declared.stack_contains(stack)
            || !remaining
                .bytes()
                .all(|c| declared.input.contains(if c == b'a' { "a" } else { "b" }))
        {
            r.issue(
                Severity::Error,
                "mechanical-pda-config-declaration",
                Some(row.number),
                format!(
                    "時点表示の状態・残り入力・スタック記号が {} の K/Σ/Γ に含まれていません。",
                    spec.definition_section
                ),
            );
        }
        if i == 0 && (q != d.start || remaining != word || stack != d.bottom) {
            r.issue(Severity::Error, "mechanical-pda-trace-start", Some(row.number), format!("この欄の最初の時点表示は、{} の出発状態・底記号と入力 {word} を使って記載してください。", spec.definition_section));
        }
        // T(M) uses final states and exhausted input. Do not substitute the
        // different definition N(M) (empty stack acceptance) from lecture p.10.
        if i + 1 == nonempty.len() && (!declared.finals.contains(q) || !remaining.is_empty()) {
            r.issue(Severity::Error, "mechanical-pda-trace-end", Some(row.number), "最後の時点表示は、入力をすべて読み終えた最終状態でなければなりません。空の残り入力は $ で表します。");
        }
        if let Some(old) = previous
            && !one_step(old, config, rules)
        {
            r.issue(Severity::Error, "mechanical-pda-trace-step", Some(row.number), format!("直前の行からこの行への変化が、{} のいずれの遷移1回分にも一致しません。入力は左から1記号（εなら0記号）を読み、スタックは左端1記号を取り除いて積む列に置き換えます。途中の行を省略していないかも確認してください。", spec.transition_section));
        }
        previous = Some(config);
    }
}

fn one_step<'a>(old: [&'a str; 3], new: [&'a str; 3], rules: &BTreeSet<[&str; 5]>) -> bool {
    let [from, before, stack] = old;
    let [to, after, next_stack] = new;
    // Configurations are ASCII; byte slicing stays on character boundaries.
    if stack.is_empty() {
        return false;
    }
    let input = if before == after {
        "$"
    } else if !before.is_empty() && &before[1..] == after {
        &before[..1]
    } else {
        return false;
    };
    let Some(pushed) = next_stack.strip_suffix(&stack[1..]) else {
        return false;
    };
    rules.contains(&[
        from,
        input,
        &stack[..1],
        to,
        if pushed.is_empty() { "$" } else { pushed },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_step_uses_left_top_and_preserves_the_entire_untouched_stack_suffix() {
        let rules = BTreeSet::from([
            ["q0", "b", "Z", "q0", "BAZ"],
            ["q0", "$", "B", "q1", "B"],
            ["q1", "b", "B", "q1", "$"],
            ["q1", "$", "Z", "qF", "$"],
        ]);
        assert!(one_step(["q0", "bb", "Z"], ["q0", "b", "BAZ"], &rules));
        assert!(one_step(["q0", "b", "BAZ"], ["q1", "b", "BAZ"], &rules));
        assert!(one_step(["q1", "b", "BAZ"], ["q1", "", "AZ"], &rules));
        assert!(one_step(["q1", "", "Z"], ["qF", "", ""], &rules));
        assert!(!one_step(["q1", "b", "BAZ"], ["q1", "", "ZA"], &rules));
        assert!(!one_step(["q1", "b", "BAZ"], ["q1", "", "Z"], &rules));
        assert!(!one_step(["q1", "", ""], ["qF", "", ""], &rules));
    }
}
