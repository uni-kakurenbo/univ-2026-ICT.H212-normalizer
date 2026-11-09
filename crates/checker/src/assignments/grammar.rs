use super::a02::check_production_format;
use crate::{AnswerLine, Report, Severity, edge_trim};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Rule<S = String> {
    pub left: S,
    pub right: Vec<S>,
}

#[derive(Clone, Copy)]
pub(super) enum Notation {
    Indexed {
        stack: &'static str,
        states: &'static [u8],
        alphabet: &'static str,
    },
    Letters,
}

#[derive(Clone, Copy)]
pub(super) struct Form {
    pub name: &'static str,
    pub production_set: &'static str,
    pub notation: Notation,
}

impl Notation {
    fn variable_prefix(self, value: &str) -> Option<(&str, &str)> {
        let (&letter, tail) = value.as_bytes().split_first()?;
        if !letter.is_ascii_uppercase() {
            return None;
        }

        match self {
            Self::Letters => Some(value.split_at(1)),
            Self::Indexed { .. } if letter == b'S' => Some(value.split_at(1)),
            Self::Indexed { stack, states, .. } => {
                let [b'^', b'{', from, b',', to, b'}', ..] = tail else {
                    return None;
                };
                let state = |digit: u8| digit.is_ascii_digit() && states.contains(&(digit - b'0'));

                (stack.as_bytes().contains(&letter) && state(*from) && state(*to))
                    .then(|| value.split_at(7))
            }
        }
    }

    fn variable(self, value: &str) -> bool {
        self.variable_prefix(value)
            .is_some_and(|(_, rest)| rest.is_empty())
    }

    fn terminal(self, value: &str) -> bool {
        let [symbol] = value.as_bytes() else {
            return false;
        };

        match self {
            Self::Indexed { alphabet, .. } => alphabet.as_bytes().contains(symbol),
            Self::Letters => symbol.is_ascii_graphic() && !b",{}()|$".contains(symbol),
        }
    }

    fn rule(self, value: &str) -> Option<Rule<&str>> {
        let (left, mut rest) = value.split_once("->")?;
        if !self.variable(left) || rest.is_empty() || rest.contains("->") {
            return None;
        }

        let mut right = Vec::new();
        if rest != "$" {
            while !rest.is_empty() {
                let (symbol, tail) = self.variable_prefix(rest).or_else(|| {
                    let symbol = rest.get(..1)?;
                    self.terminal(symbol).then(|| rest.split_at(1))
                })?;

                right.push(symbol);
                rest = tail;
            }
        }

        Some(Rule { left, right })
    }
}

struct Declaration<'a> {
    name: &'a str,
    variables: Vec<&'a str>,
    terminals: Vec<&'a str>,
    production_set: &'a str,
    start: &'a str,
}

// An index pair such as A^{1,2} contains a comma within a set member.
fn take_set(value: &str) -> Option<(Vec<&str>, &str)> {
    let value = value.strip_prefix('{')?;
    let mut members = Vec::new();
    let mut nested = false;
    let mut start = 0;

    for (i, byte) in value.bytes().enumerate() {
        match byte {
            b'{' if !nested => nested = true,
            b'{' => return None,
            b'}' if nested => nested = false,
            b'}' if i == 0 => return Some((members, &value[1..])),
            b'}' | b',' if !nested => {
                if start == i {
                    return None;
                }

                members.push(&value[start..i]);

                if byte == b'}' {
                    return Some((members, &value[i + 1..]));
                }

                start = i + 1;
            }
            _ => {}
        }
    }

    None
}

fn declaration(value: &str) -> Option<Declaration<'_>> {
    if !value.bytes().all(|c| c.is_ascii_graphic()) {
        return None;
    }

    let (name, body) = value.strip_suffix(')')?.split_once("=(")?;
    let (variables, rest) = take_set(body)?;
    let (terminals, rest) = take_set(rest.strip_prefix(',')?)?;
    let (production_set, start) = rest.strip_prefix(',')?.split_once(',')?;

    if [name, production_set, start]
        .iter()
        .any(|token| token.is_empty() || !token.bytes().all(|c| c.is_ascii_alphanumeric()))
    {
        return None;
    }

    Some(Declaration {
        name,
        variables,
        terminals,
        production_set,
        start,
    })
}

pub(crate) fn normalize_declaration(value: &str) -> String {
    let Some(mut d) = declaration(value) else {
        return value.to_owned();
    };

    d.variables.sort_unstable();
    d.terminals.sort_unstable();

    format!(
        "{}=({{{}}},{{{}}},{},{})",
        d.name,
        d.variables.join(","),
        d.terminals.join(","),
        d.production_set,
        d.start,
    )
}

pub(super) fn check<'a>(
    r: &mut Report,
    rows: &[&AnswerLine<'a>],
    form: Form,
) -> Option<BTreeSet<Rule<&'a str>>> {
    let (first, rows) = rows.split_first()?;
    let indexed = matches!(form.notation, Notation::Indexed { .. });
    check_production_format(r, rows, indexed);

    if rows.is_empty() {
        r.issue(
            Severity::Error,
            "cfg-missing-rules",
            Some(first.number),
            "文法の宣言の次の行から生成規則を1行に1規則ずつ記載してください。",
        );
    }

    let Some(d) = declaration(edge_trim(first.text)) else {
        r.issue(Severity::Error, "cfg-declaration", Some(first.number), format!("最初の解答行に {}=(N,T,{},S) の形で宣言してください。N と T は半角の {{}} で囲み、集合の要素をカンマで区切ります。", form.name, form.production_set));
        return None;
    };

    let variables: BTreeSet<_> = d.variables.iter().copied().collect();
    let terminals: BTreeSet<_> = d.terminals.iter().copied().collect();
    let mut valid = !rows.is_empty();

    for (name, members, count) in [
        ("N", &d.variables, variables.len()),
        ("T", &d.terminals, terminals.len()),
    ] {
        if indexed && members.windows(2).any(|pair| pair[0] > pair[1]) {
            r.issue(Severity::Error, "cfg-set-sort", Some(first.number), format!("集合 {name} の要素を ASCII 辞書順に並べてください。修正案ではこの集合内を整列します。"));
        }

        if members.len() != count {
            r.issue(
                Severity::Warning,
                "cfg-set-duplicate",
                Some(first.number),
                format!("集合 {name} の要素が重複しています。修正案には重複を保持します。"),
            );
        }
    }

    if d.name != form.name
        || d.production_set != form.production_set
        || d.start != "S"
        || !variables.contains("S")
    {
        valid = false;
        r.issue(
            Severity::Error,
            "cfg-declaration-symbols",
            Some(first.number),
            format!(
                "文法名は {}、規則集合名は {}、開始記号は S です。N に S を含めてください。",
                form.name, form.production_set
            ),
        );
    }

    if variables.iter().any(|s| !form.notation.variable(s)) {
        valid = false;
        r.issue(
            Severity::Error,
            "cfg-nonterminal",
            Some(first.number),
            variable_message(form.notation),
        );
    }

    let exact_alphabet = match form.notation {
        Notation::Indexed { alphabet, .. } => terminals.len() == alphabet.len(),
        Notation::Letters => true,
    };
    if !exact_alphabet
        || terminals.iter().any(|s| !form.notation.terminal(s))
        || !variables.is_disjoint(&terminals)
    {
        valid = false;
        let message = match form.notation {
            Notation::Indexed { alphabet, .. } => format!("終端記号の集合 T は、問題のNPDAの入力記号 {alphabet} を1文字ずつカンマで区切って宣言してください。"),
            Notation::Letters => "T には終端記号を1文字ずつ宣言してください。空列の $ や区切り記号は集合要素にせず、N と T は重ならないようにします。".to_owned(),
        };
        r.issue(Severity::Error, "cfg-terminal", Some(first.number), message);
    }

    let mut rules = BTreeSet::new();
    let mut used = BTreeSet::new();

    for row in rows {
        let Some(rule) = form.notation.rule(edge_trim(row.text)) else {
            valid = false;
            r.issue(
                Severity::Error,
                "cfg-production",
                Some(row.number),
                format!(
                    "{} 左辺は非終端記号1つ、右辺は記号の列または $ です。",
                    variable_message(form.notation)
                ),
            );
            continue;
        };

        if !variables.contains(rule.left)
            || rule
                .right
                .iter()
                .any(|s| !variables.contains(s) && !terminals.contains(s))
        {
            valid = false;
            r.issue(
                Severity::Error,
                "cfg-undeclared",
                Some(row.number),
                "左辺の非終端記号は N、右辺の各記号は N または T に宣言してください。",
            );
        }

        if indexed {
            used.insert(rule.left);
            used.extend(
                rule.right
                    .iter()
                    .copied()
                    .filter(|s| form.notation.variable(s)),
            );
        }

        rules.insert(rule);
    }

    if indexed && valid && used != variables {
        valid = false;
        r.issue(
            Severity::Error,
            "cfg-variable-set",
            Some(first.number),
            "N は、削除後の生成規則の左辺または右辺に現れる非終端記号の集合と一致させてください。",
        );
    }

    valid.then_some(rules)
}

fn variable_message(notation: Notation) -> &'static str {
    match notation {
        Notation::Indexed { .. } => {
            "非終端記号は S、またはスタック記号に ^{i,j} を付けた形です。i,j には問題にある状態番号を使い、底記号は Z と書きます。"
        }
        Notation::Letters => "非終端記号は英大文字1文字で記載してください。",
    }
}
