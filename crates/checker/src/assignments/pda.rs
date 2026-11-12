use crate::{AnswerLine, Report, Severity, edge_trim};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub(super) enum Syntax {
    Alphanumeric,
    Ascii,
}

pub(super) type Transition<'a> = [&'a str; 5];

impl Syntax {
    fn token(self, value: &str) -> bool {
        !value.is_empty()
            && value.bytes().all(|byte| match self {
                Self::Alphanumeric => byte.is_ascii_alphanumeric() || byte == b'$',
                Self::Ascii => byte.is_ascii_graphic() && !b",(){}".contains(&byte),
            })
    }

    pub(super) fn tuple<const N: usize>(self, value: &str) -> Option<[&str; N]> {
        let inner = value.strip_prefix('(')?.strip_suffix(')')?;
        let mut fields = inner.split(',');
        let mut result = [""; N];

        for field in &mut result {
            *field = fields.next()?;
        }

        (fields.next().is_none() && result.iter().all(|s| self.token(s))).then_some(result)
    }

    pub(super) fn transition(self, value: &str) -> Option<Transition<'_>> {
        let fields = self.tuple::<5>(value)?;
        (fields[1].len() == 1 && fields[2].len() == 1).then_some(fields)
    }

    fn set(self, value: &str) -> Option<Vec<&str>> {
        let inner = value.strip_prefix('{')?.strip_suffix('}')?;
        if inner.is_empty() {
            return Some(Vec::new());
        }

        inner
            .split(',')
            .map(|s| self.token(s).then_some(s))
            .collect()
    }

    pub(super) fn definition(self, value: &str) -> Option<Definition<'_>> {
        let inner = value.strip_prefix("M=(")?.strip_suffix(')')?;
        let mut fields = Vec::new();
        let mut in_set = false;
        let mut start = 0;

        for (i, c) in inner.bytes().enumerate() {
            match c {
                b'{' if !in_set => in_set = true,
                b'}' if in_set => in_set = false,
                b'{' | b'}' => return None,
                b',' if !in_set => {
                    fields.push(&inner[start..i]);
                    if fields.len() >= 7 {
                        return None;
                    }

                    start = i + 1;
                }
                _ => {}
            }
        }

        if in_set {
            return None;
        }

        fields.push(&inner[start..]);
        let [states, input, stack, function, start, bottom, finals] = fields.as_slice() else {
            return None;
        };
        if !self.token(function) || !self.token(start) || !self.token(bottom) {
            return None;
        }

        Some(Definition {
            states: self.set(states)?,
            input: self.set(input)?,
            stack: self.set(stack)?,
            finals: self.set(finals)?,
            function,
            start,
            bottom,
        })
    }
}

pub(super) struct Definition<'a> {
    pub states: Vec<&'a str>,
    pub input: Vec<&'a str>,
    pub stack: Vec<&'a str>,
    pub finals: Vec<&'a str>,
    pub function: &'a str,
    pub start: &'a str,
    pub bottom: &'a str,
}

pub(super) struct Declared<'a> {
    pub states: BTreeSet<&'a str>,
    pub input: BTreeSet<&'a str>,
    pub finals: BTreeSet<&'a str>,
    stack: [bool; 256],
}

impl<'a> Declared<'a> {
    pub(super) fn from(d: &Definition<'a>) -> Self {
        let mut stack = [false; 256];

        for symbol in &d.stack {
            if let [c] = symbol.as_bytes() {
                stack[usize::from(*c)] = true;
            }
        }

        Self {
            states: d.states.iter().copied().collect(),
            input: d.input.iter().copied().collect(),
            finals: d.finals.iter().copied().collect(),
            stack,
        }
    }

    pub(super) fn stack_contains(&self, value: &str) -> bool {
        value.bytes().all(|c| self.stack[usize::from(c)])
    }

    pub(super) fn contains_transition(&self, [from, input, pop, to, push]: Transition<'_>) -> bool {
        self.states.contains(from)
            && self.states.contains(to)
            && (input == "$" || self.input.contains(input))
            && self.stack_contains(pop)
            && self.stack_contains(decode(push))
    }
}

pub(super) fn decode(value: &str) -> &str {
    if value == "$" { "" } else { value }
}

pub(crate) fn normalize_definition(value: &str) -> String {
    let Some(mut d) = Syntax::Alphanumeric.definition(value) else {
        return value.to_owned();
    };

    for set in [&mut d.states, &mut d.input, &mut d.stack, &mut d.finals] {
        set.sort_unstable();
    }

    format!(
        "M=({{{}}},{{{}}},{{{}}},{},{},{},{{{}}})",
        d.states.join(","),
        d.input.join(","),
        d.stack.join(","),
        d.function,
        d.start,
        d.bottom,
        d.finals.join(",")
    )
}

pub(super) fn check_notation(r: &mut Report, row: &AnswerLine<'_>) {
    if row.text.contains(['ε', 'ϵ']) {
        r.issue(Severity::Error, "pda-epsilon", Some(row.number), "この課題の ε は $ で記載してください。空の入力・積まない記号列・空の残り入力・空スタックは欄全体を $ とします。");
    }

    if row.text.contains('⊥') {
        r.issue(
            Severity::Error,
            "pda-bottom",
            Some(row.number),
            "スタックの底記号は ASCII の Z で記載してください。⊥ は使いません。",
        );
    }
}

pub(super) fn check_definition<'a>(
    r: &mut Report,
    row: &AnswerLine<'a>,
    syntax: Syntax,
    sort_sets: bool,
) -> Option<Definition<'a>> {
    check_notation(r, row);
    let Some(d) = syntax.definition(edge_trim(row.text)) else {
        r.issue(Severity::Error, "pda-definition", Some(row.number), "M=({状態の集合},{入力記号の集合},{スタック記号の集合},p,出発状態,底記号,{最終状態の集合}) の7要素を空白なしで記載してください。集合には半角 {} を使います。");
        return None;
    };

    for (name, set) in [
        ("K", &d.states),
        ("Σ", &d.input),
        ("Γ", &d.stack),
        ("F", &d.finals),
    ] {
        if sort_sets && set.windows(2).any(|pair| pair[0] > pair[1]) {
            r.issue(Severity::Error, "pda-set-sort", Some(row.number), format!("集合 {name} の要素が ASCII 辞書順ではありません。修正案ではこの集合の中だけを並べ替えます。"));
        }

        let unique: BTreeSet<_> = set.iter().copied().collect();
        if unique.len() != set.len() {
            r.issue(
                Severity::Warning,
                "pda-set-duplicate",
                Some(row.number),
                format!("集合 {name} の要素が重複しています。修正案では重複を保持します。"),
            );
        }
    }

    Some(d)
}

pub(super) fn check_transitions<'a>(
    r: &mut Report,
    rows: &[&AnswerLine<'a>],
    syntax: Syntax,
    mut validate: impl FnMut(&mut Report, &AnswerLine<'a>, Transition<'a>, bool),
) {
    let mut previous = None;
    let mut seen = BTreeSet::new();
    let mut duplicate = false;

    for row in rows {
        check_notation(r, row);
        let Some(rule) = syntax.transition(edge_trim(row.text)) else {
            r.issue(Severity::Error, "pda-transition", Some(row.number), "遷移は (状態,入力1記号,取り除く1記号,状態,積む記号列) の5つ組を空白なしで1行に1つ記載してください。空要素や集合表記は使わず、ε・空の記号列は $ とします。");
            continue;
        };

        if previous.is_some_and(|old| old > rule) {
            r.issue(Severity::Error, "pda-transition-sort", Some(row.number), "5つ組を左の要素から ASCII 辞書順で比べて並べてください。$・数字は英字より前、大文字は小文字より前です。修正案は行順だけを整えます。");
        }

        previous = Some(rule);
        let repeated = !seen.insert(rule);
        duplicate |= repeated;
        validate(r, row, rule, repeated);
    }

    if duplicate {
        r.issue(Severity::Warning, "pda-transition-duplicate", None, "同じ5つ組の遷移が重複しています。照合では同じ1遷移として扱い、修正案には重複を保持します。");
    }
}
