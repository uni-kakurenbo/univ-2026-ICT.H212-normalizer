use super::{Definition, configuration, decode, definition, transition};
use crate::{AnswerLine, Report, Severity, edge_trim};
use std::collections::BTreeSet;

struct Declared<'a> {
    states: BTreeSet<&'a str>,
    input: BTreeSet<&'a str>,
    stack: [bool; 256],
    finals: BTreeSet<&'a str>,
}

impl<'a> Declared<'a> {
    fn from(d: &Definition<'a>) -> Self {
        let mut stack = [false; 256];
        for symbol in &d.sets[2] {
            if let [c] = symbol.as_bytes() {
                stack[usize::from(*c)] = true;
            }
        }
        Self {
            states: d.sets[0].iter().copied().collect(),
            input: d.sets[1].iter().copied().collect(),
            stack,
            finals: d.sets[3].iter().copied().collect(),
        }
    }
    fn stack_contains(&self, value: &str) -> bool {
        value.bytes().all(|c| self.stack[usize::from(c)])
    }
}

pub(super) fn check_related(r: &mut Report, bodies: &[Vec<AnswerLine<'_>>]) {
    let [transitions, definitions, aa, aba] = bodies else {
        return;
    };
    let nonempty = |row: &&AnswerLine<'_>| !row.text.trim().is_empty();
    let trace_entered = aa.iter().chain(aba).any(|row| !row.text.trim().is_empty());
    let definition_unchecked = |r: &mut Report| {
        if trace_entered {
            r.issue(Severity::Warning, "pda-trace-unchecked", None, "14-2 が空欄・複数行・構文エラーのため、時点表示と NPDA の宣言・遷移の照合は未検査です。");
        }
    };
    let mut definitions = definitions.iter().filter(nonempty);
    let Some(row) = definitions.next() else {
        definition_unchecked(r);
        return;
    };
    if definitions.next().is_some() {
        definition_unchecked(r);
        return;
    }
    let Some(d) = definition(edge_trim(row.text)) else {
        definition_unchecked(r);
        return;
    };
    // Compile membership once; duplicate set members must not make every
    // transition/trace rescan the whole declaration (quadratic input cost).
    let declared = Declared::from(&d);
    let mut rules = BTreeSet::new();
    let mut valid = true;
    for row in transitions.iter().filter(nonempty) {
        let Some(rule) = transition(edge_trim(row.text)) else {
            valid = false;
            continue;
        };
        let [from, input, pop, to, push] = rule;
        if !declared.states.contains(from)
            || !declared.states.contains(to)
            || (input != "$" && !declared.input.contains(input))
            || !declared.stack_contains(pop)
            || !declared.stack_contains(decode(push))
        {
            r.issue(Severity::Error, "mechanical-pda-declaration", Some(row.number), "遷移の状態・入力記号・スタック記号が 14-2 の K/Σ/Γ に含まれていません。宣言と遷移を対応させてください。");
        }
        rules.insert(rule);
    }
    if rules.is_empty() || !valid {
        if trace_entered {
            r.issue(
                Severity::Warning,
                "pda-trace-unchecked",
                None,
                "14-1 の遷移が空欄または構文エラーのため、時点表示の遷移照合は未検査です。",
            );
        }
        return;
    }
    check_trace(r, aa, "aa", &d, &declared, &rules);
    check_trace(r, aba, "aba", &d, &declared, &rules);
}

// Match the exact rule inferred from two consecutive configurations. There is
// no NPDA run enumeration: nondeterminism only means at least one listed rule
// can match. A BTreeSet deduplicates for lookup, never for output/hashing.
fn check_trace(
    r: &mut Report,
    rows: &[AnswerLine<'_>],
    word: &str,
    d: &Definition<'_>,
    declared: &Declared<'_>,
    rules: &BTreeSet<[&str; 5]>,
) {
    let nonempty: Vec<_> = rows
        .iter()
        .filter(|row| !row.text.trim().is_empty())
        .collect();
    let mut previous = None;
    for (i, row) in nonempty.iter().enumerate() {
        let Some(config @ [q, remaining, stack]) = configuration(edge_trim(row.text)) else {
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
                "時点表示の状態・残り入力・スタック記号が 14-2 の K/Σ/Γ に含まれていません。",
            );
        }
        if i == 0 && (q != d.start || remaining != word || stack != d.bottom) {
            r.issue(Severity::Error, "mechanical-pda-trace-start", Some(row.number), format!("この欄の最初の時点表示は、14-2 の出発状態・底記号と入力 {word} を使って記載してください。"));
        }
        // T(M) uses final states and exhausted input. Do not substitute the
        // different definition N(M) (empty stack acceptance) from lecture p.10.
        if i + 1 == nonempty.len() && (!declared.finals.contains(q) || !remaining.is_empty()) {
            r.issue(Severity::Error, "mechanical-pda-trace-end", Some(row.number), "最後の時点表示は、入力をすべて読み終えた最終状態でなければなりません。空の残り入力は $ で表します。");
        }
        if let Some(old) = previous
            && !one_step(old, config, rules)
        {
            r.issue(Severity::Error, "mechanical-pda-trace-step", Some(row.number), "直前の行からこの行への変化が、14-1 のいずれの遷移1回分にも一致しません。入力は左から1記号（εなら0記号）を読み、スタックは左端1記号を取り除いて積む列に置き換えます。途中の行を省略していないかも確認してください。");
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
