use crate::{Report, Severity};
use std::collections::BTreeSet;

// Problem data, not a stored solution. All construction/linear-form rules are
// generic over expressions; tests use different, small synthetic expressions.
pub const GIVEN: &str = "(a+b+c)*(bc+ac+baa)";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expr {
    Zero,
    One,
    Letter(u8),
    Sum(Vec<Expr>),
    Cat(Vec<Expr>),
    Star(Box<Expr>),
}

pub type Form = BTreeSet<(u8, Expr)>;

#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    Syntax,
    Limit,
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    nodes: usize,
}

impl Parser<'_> {
    fn expression(&mut self, depth: usize) -> Result<Expr, ParseError> {
        if depth > 64 {
            return Err(ParseError::Limit);
        }
        let mut sum = vec![self.product(depth)?];
        while self.bytes.get(self.at) == Some(&b'+') {
            self.at += 1;
            sum.push(self.product(depth)?);
        }
        Ok(if sum.len() == 1 {
            sum.pop().unwrap()
        } else {
            Expr::Sum(sum)
        })
    }

    fn product(&mut self, depth: usize) -> Result<Expr, ParseError> {
        let mut factors = Vec::new();
        while let Some(&c) = self.bytes.get(self.at) {
            if matches!(c, b'+' | b')') {
                break;
            }
            self.nodes += 1;
            if self.nodes > 512 {
                return Err(ParseError::Limit);
            }
            self.at += 1;
            let mut atom = match c {
                b'0' => Expr::Zero,
                b'1' => Expr::One,
                b'(' => {
                    let inner = self.expression(depth + 1)?;
                    if self.bytes.get(self.at) != Some(&b')') {
                        return Err(ParseError::Syntax);
                    }
                    self.at += 1;
                    inner
                }
                c if c.is_ascii_alphanumeric() => Expr::Letter(c),
                _ => return Err(ParseError::Syntax),
            };
            while self.bytes.get(self.at) == Some(&b'*') {
                self.at += 1;
                self.nodes += 1;
                if self.nodes > 512 {
                    return Err(ParseError::Limit);
                }
                atom = Expr::Star(Box::new(atom));
            }
            factors.push(atom);
        }
        match factors.len() {
            0 => Err(ParseError::Syntax),
            1 => Ok(factors.pop().unwrap()),
            _ => Ok(Expr::Cat(factors)),
        }
    }
}

impl Expr {
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        if text.len() > 4096 {
            return Err(ParseError::Limit);
        }
        let mut parser = Parser {
            bytes: text.as_bytes(),
            at: 0,
            nodes: 0,
        };
        let result = parser.expression(0)?;
        if parser.at != text.len() {
            return Err(ParseError::Syntax);
        }
        if result.depth() > 64 {
            return Err(ParseError::Limit);
        }
        Ok(result)
    }

    fn depth(&self) -> usize {
        match self {
            Self::Star(e) => 1 + e.depth(),
            Self::Cat(v) | Self::Sum(v) => 1 + v.iter().map(Self::depth).max().unwrap_or(0),
            _ => 1,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Star(e) => 1 + e.size(),
            Self::Cat(v) | Self::Sum(v) => 1 + v.iter().map(Self::size).sum::<usize>(),
            _ => 1,
        }
    }

    fn alphabet(&self) -> bool {
        match self {
            Self::Letter(c) => matches!(c, b'a' | b'b' | b'c'),
            Self::Cat(v) | Self::Sum(v) => v.iter().all(Self::alphabet),
            Self::Star(e) => e.alphabet(),
            _ => true,
        }
    }

    pub fn cat(factors: impl IntoIterator<Item = Self>) -> Self {
        let mut flat = Vec::new();
        for factor in factors {
            match factor {
                Self::Zero => return Self::Zero,
                Self::One => {}
                Self::Cat(v) => flat.extend(v),
                e => flat.push(e),
            }
        }
        match flat.len() {
            0 => Self::One,
            1 => flat.pop().unwrap(),
            _ => Self::Cat(flat),
        }
    }

    pub fn normal(self) -> Self {
        match self {
            Self::Cat(v) => Self::cat(v.into_iter().map(Self::normal)),
            Self::Sum(v) => {
                let mut flat = BTreeSet::new();
                for e in v {
                    match e.normal() {
                        Self::Zero => {}
                        Self::Sum(v) => flat.extend(v),
                        e => {
                            flat.insert(e);
                        }
                    }
                }
                match flat.len() {
                    0 => Self::Zero,
                    1 => flat.pop_first().unwrap(),
                    _ => Self::Sum(flat.into_iter().collect()),
                }
            }
            Self::Star(e) => match e.normal() {
                Self::Zero | Self::One => Self::One,
                e => Self::Star(Box::new(e)),
            },
            e => e,
        }
    }

    pub fn nullable(&self) -> bool {
        match self {
            Self::Zero | Self::Letter(_) => false,
            Self::One | Self::Star(_) => true,
            Self::Sum(v) => v.iter().any(Self::nullable),
            Self::Cat(v) => v.iter().all(Self::nullable),
        }
    }

    pub fn form(&self, budget: &mut usize) -> Result<Form, ()> {
        charge(budget, 1)?;
        let mut result = Form::new();
        match self {
            Self::Zero | Self::One => {}
            Self::Letter(c) => {
                result.insert((*c, Self::One));
            }
            Self::Sum(v) => {
                for e in v {
                    result.extend(e.form(budget)?);
                }
            }
            Self::Cat(v) => {
                for (i, e) in v.iter().enumerate() {
                    let tail = Self::cat(v[i + 1..].iter().cloned());
                    for (c, q) in e.form(budget)? {
                        charge(budget, q.size() + tail.size())?;
                        let q = Self::cat([q, tail.clone()]);
                        if q != Self::Zero {
                            result.insert((c, q));
                        }
                    }
                    if !e.nullable() {
                        break;
                    }
                }
            }
            Self::Star(e) => {
                for (c, q) in e.form(budget)? {
                    charge(budget, q.size() + self.size())?;
                    result.insert((c, Self::cat([q, self.clone()])));
                }
            }
        }
        Ok(result)
    }
}

pub fn charge(budget: &mut usize, cost: usize) -> Result<(), ()> {
    *budget = budget.checked_sub(cost).ok_or(())?;
    Ok(())
}

pub fn given() -> Expr {
    Expr::parse(GIVEN).expect("fixed problem expression")
}

pub fn limit(r: &mut Report, line: Option<usize>) {
    if !r.issues.iter().any(|i| i.code == "mechanical-limit") {
        r.issue(
            Severity::Warning,
            "mechanical-limit",
            line,
            "構成・計算規則の検査上限に達しました。この検査の未完了部分は未検査です。",
        );
    }
}

pub fn read(r: &mut Report, text: &str, line: usize) -> Option<Expr> {
    if text == "e" {
        return Some(given().normal());
    }
    let expression = match Expr::parse(text) {
        Ok(e) => e,
        Err(ParseError::Limit) => {
            limit(r, Some(line));
            return None;
        }
        Err(ParseError::Syntax) => {
            r.issue(
                Severity::Error,
                "mechanical-expression",
                Some(line),
                "正規表現を講義の演算規則として解釈できません。",
            );
            return None;
        }
    };
    if !expression.alphabet() {
        r.issue(
            Severity::Error,
            "mechanical-alphabet",
            Some(line),
            "正規表現の記号は a・b・c、定数は 0・1 です。全体の別名 e は単独で使用してください。",
        );
        return None;
    }
    let expression = expression.normal();
    if expression == given().normal() {
        r.issue(
            Severity::Error,
            "mechanical-alias",
            Some(line),
            "課題の正規表現全体は展開せず e と記載してください。",
        );
    }
    Some(expression)
}
