use super::Assignment;
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};

// Sources: 09_answer-sheet.txt; autolang第9回.pdf pp.7–8.
// One section contains equations, free-form descriptions, and a regex-only last
// nonempty, noncomment line. Variables are X/Y/Z; empty word is 1. Only the final
// regex has the explicit alphabet a,b,+,*,(,),1 (not 0, variables, or X=).
// Formula/description delimiters are unspecified. Check identifiable equations,
// including space-indented = continuations and ASCII => chains. Recognize
// parenthetical/spaced explanations conservatively; warn for ambiguous rows.
// Never validate the NFA equations, substitutions, algebra, or language of the
// answer. Preserve work order, descriptions, spaces, and paragraphs in hash v1.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "09",
    title: "演習09：オートマトンから正規表現へ",
    note: "9-1 は X/Y/Z の途中式・操作の説明を記載し、最終行に正規表現だけを書きます。最終行は a,b,+,*,(,),1 のみ。途中式の空語も 1、矢印は =>。式・説明・字下げ・記載順は保持し、判別が曖昧な行は warning にします。方程式の内容・代入・式変形や答えの正しさは判定しません。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[Section {
        id: "9-1",
        marker: "9-1=====",
        ascii: false,
        format: Format::AlgebraicDerivation,
        validate: Some(check_derivation),
    }],
};

// Syntax only, with an iterative scan so deeply nested input is safe in WASM.
// Equations may contain variables and the usual empty-language constant 0;
// neither is permitted in the final answer's explicitly restricted alphabet.
fn expression(value: &str, intermediate: bool) -> bool {
    let mut depth = 0usize;
    let mut need_atom = true;
    for c in value.bytes() {
        match c {
            b' ' | b'\t' if intermediate => {}
            b'(' => {
                depth += 1;
                need_atom = true;
            }
            b')' if depth > 0 && !need_atom => {
                depth -= 1;
                need_atom = false;
            }
            b'+' if !need_atom => need_atom = true,
            b'*' if !need_atom => {}
            b'a' | b'b' | b'1' => need_atom = false,
            b'0' | b'X' | b'Y' | b'Z' if intermediate => need_atom = false,
            _ => return false,
        }
    }
    depth == 0 && !need_atom
}

fn equation_chain(value: &str) -> bool {
    // Each side of = or => is an expression, not a second assignment. This
    // permits X=aY=bZ and X=aY=>bZ without silently accepting X=aY,Y=bZ.
    value.split('=').enumerate().all(|(i, part)| {
        let part = if i > 0 {
            part.strip_prefix('>').unwrap_or(part)
        } else {
            part
        };
        expression(part, true)
    })
}

fn math_char(c: char) -> bool {
    matches!(
        c,
        'a' | 'b' | '0' | '1' | 'X' | 'Y' | 'Z' | '+' | '*' | '(' | ')' | '=' | '>' | ' ' | '\t'
    )
}

fn explicit_notation(c: char) -> bool {
    matches!(c,
        '|' | '^' | '$' | '/' | '\\' | '[' | ']' | '{' | '}' | ',' | ';' |
        'ε' | 'ϵ' | '∅' | '∪' | '×' | '·' | '⇒' | '→' | '＝' | '＋' | '＊' | '（' | '）' |
        '０'..='９' | 'Ａ'..='Ｚ' | 'ａ'..='ｚ')
}

enum Formula<'a> {
    Identified { text: &'a str, explanation: bool },
    Ambiguous,
}

fn formula_part(value: &str) -> Formula<'_> {
    let mut opens = Vec::new();
    let mut previous = None;
    for (index, c) in value.char_indices() {
        if !math_char(c) {
            // A spaced '(' followed by nonformula text starts an explanation,
            // as in the template's (Y の式に ...). Choose the outermost such
            // unmatched '(' to retain nested parentheses in the explanation.
            if let Some(&(start, _)) = opens.iter().find(|(_, spaced)| *spaced)
                && prose_text(&value[index..])
            {
                return Formula::Identified {
                    text: &value[..start],
                    explanation: true,
                };
            }
            // Japanese full-width parentheses are useful for free prose, but
            // never accept them as formula parentheses.
            if c == '（' && value[index..].chars().any(prose_character) {
                return Formula::Identified {
                    text: &value[..index],
                    explanation: true,
                };
            }
            if explicit_notation(c) {
                return Formula::Identified {
                    text: value,
                    explanation: false,
                };
            }
            if !c.is_ascii() && previous.is_some_and(char::is_whitespace) && index > 0 {
                return Formula::Identified {
                    text: &value[..index],
                    explanation: true,
                };
            }
            // Adjacent ASCII tokens such as cY, $, or a|b are clear syntax
            // violations. A spaced English suffix could be an explanation;
            // an unseparated Japanese suffix has no specified boundary.
            if value.is_ascii()
                && !(previous.is_some_and(char::is_whitespace) && equation_chain(&value[..index]))
            {
                return Formula::Identified {
                    text: value,
                    explanation: false,
                };
            }
            return Formula::Ambiguous;
        }
        if c == '(' {
            opens.push((index, previous.is_some_and(char::is_whitespace)));
        } else if c == ')' {
            opens.pop();
        }
        previous = Some(c);
    }
    Formula::Identified {
        text: value,
        explanation: false,
    }
}

fn prose_character(c: char) -> bool {
    matches!(c, 'ぁ'..='ヿ' | '\u{3400}'..='\u{9fff}')
}

fn prose_text(value: &str) -> bool {
    value.chars().any(prose_character)
        || value.split_whitespace().any(|word| {
            let word = word.trim_matches(|c: char| c.is_ascii_punctuation());
            word.chars().all(|c| c.is_ascii_alphabetic())
                && word.chars().filter(|c| c.is_ascii_lowercase()).count() >= 2
                && word.chars().any(|c| !math_char(c))
        })
}

fn check_derivation(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    let Some((last, work)) = rows.split_last() else {
        return;
    };
    let answer = last.text;
    if !expression(answer, false) {
        r.issue(Severity::Error, "final-regex", Some(last.number), "最終行には a,b,+,*,(,),1 だけを使った正規表現を記載してください。X=、変数、説明、空白、0、$、ε は付けません。和は +、空語は 1 です。括弧や演算子の対応も確認してください。");
    }
    if work.is_empty() {
        r.issue(
            Severity::Error,
            "derivation-missing",
            Some(last.number),
            "最終行の前に X/Y/Z の方程式と式変形の過程・操作の説明を記載してください。",
        );
        return;
    }
    let mut variables = [false; 3];
    let mut active_variable = false;
    let mut explanation = false;
    let mut ambiguous = false;
    for row in work {
        let value = edge_trim(row.text);
        if value.starts_with("==") {
            r.issue(
                Severity::Error,
                "leading-equals",
                Some(row.number),
                "途中式の行頭に = を2つ以上続けて書かないでください。区切り行と誤認されます。",
            );
            continue;
        }
        if value.starts_with(['X', 'Y', 'Z']) && edge_trim(&value[1..]).starts_with('＝') {
            r.issue(Severity::Error, "equation-syntax", Some(row.number), "式の等号は ASCII の = を使ってください。全角の ＝ は使いません。文字は自動変更しません。");
            continue;
        }
        let rhs = if let Some(rest) = value.strip_prefix('=') {
            if !active_variable {
                r.issue(
                    Severity::Error,
                    "equation-continuation",
                    Some(row.number),
                    "= から始める続きの行には、先に X/Y/Z の左辺を記載した式が必要です。",
                );
            }
            rest
        } else if let Some((left, right)) = value.split_once('=')
            && let Some(index) = ["X", "Y", "Z"].iter().position(|&v| v == edge_trim(left))
        {
            variables[index] = true;
            active_variable = true;
            right
        } else {
            // No invented prose delimiter: keep recognizable free text, and
            // mark mathematical-looking unidentified lines explicitly unchecked.
            if prose_text(value) && !value.starts_with(['X', 'Y', 'Z', '=']) {
                explanation = true;
            } else {
                warn_ambiguous(r, row.number);
                ambiguous = true;
            }
            continue;
        };
        match formula_part(rhs) {
            Formula::Identified {
                text,
                explanation: has_explanation,
            } => {
                explanation |= has_explanation;
                if !equation_chain(text) {
                    r.issue(Severity::Error, "equation-syntax", Some(row.number), "途中式の構文・記号を確認してください。変数は X/Y/Z、記号は a/b、空語は 1、和は +、積は連結、繰り返しは *、矢印は => です。1行に1つの式を記載します。説明文との区切りが曖昧なら、空白と括弧で区切ると識別できます。");
                }
            }
            Formula::Ambiguous => {
                warn_ambiguous(r, row.number);
                ambiguous = true;
                explanation = true;
            }
        }
    }
    if variables.contains(&false) {
        r.issue(if ambiguous { Severity::Warning } else { Severity::Error }, "equation-variables", Some(work[0].number), "X/Y/Z それぞれを左辺にした式を確認できませんでした。曖昧な行がある場合は、式と説明を分けて確認してください。方程式の内容は判定していません。");
    }
    if !explanation {
        r.issue(Severity::Warning, "derivation-explanation", Some(work[0].number), "どの変数を代入したか、自己参照を除去する式を使ったかなど、操作の説明を確認してください。式だけの行として認識しています。");
    }
}

fn warn_ambiguous(r: &mut Report, line: usize) {
    r.issue(Severity::Warning, "derivation-ambiguous", Some(line), "途中式と説明文を確実に識別できないため、この行の式の書式は未検査です。式の文字は ASCII、変数は X/Y/Z です。記載は変更しません。");
}
