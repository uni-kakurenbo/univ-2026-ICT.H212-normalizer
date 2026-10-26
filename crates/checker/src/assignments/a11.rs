use super::{Assignment, a02::check_productions, cfg_procedure};
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};

mod trees;

// Sources: 11_answer-sheet.txt; autolang第11回.pdf pp.3, 5–8.
// 11-1: exactly two S-rooted trees, nonterminals S/A/B immediately followed
// by nonempty (...), leaves a/b. The user additionally authorized checking
// every local production, the target yield, and structural distinctness.
// The order of the two trees is unspecified: preserve it and all duplicates.
// 11-2 also compares the generic epsilon-elimination procedure, warning-only.
// The lecture permits a fresh primed start symbol for nullable grammars; keep
// uppercase letters with prime suffixes rather than inventing a naming rule.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "11",
    title: "演習11：文脈自由文法・導出木",
    note: "11-1 は導出木2行の記法に加え、各節点の生成規則、葉の語 aababb、2つの木の相違も検査します。11-2 は -> と $、1行1規則・ASCII辞書順を検査し、講義のε除去手順と照合します。手順の不一致は warning で、非等価・不正解とは断定しません。解答は自動修正しません。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "11-1",
            marker: "11-1=====",
            ascii: true,
            format: Format::Compact,
            validate: Some(trees::check_trees),
        },
        Section {
            id: "11-2",
            marker: "11-2=====",
            ascii: true,
            format: Format::Productions,
            validate: Some(check_cfg_productions),
        },
    ],
};

fn nonterminal(value: &str) -> bool {
    value.starts_with(|c: char| c.is_ascii_uppercase())
        && value.as_bytes()[1..].iter().all(|&c| c == b'\'')
}

fn cfg_rhs(value: &str) -> bool {
    if value == "$" {
        return true;
    }
    let mut after_nonterminal = false;
    !value.is_empty()
        && value.bytes().all(|c| match c {
            b'A'..=b'Z' => {
                after_nonterminal = true;
                true
            }
            b'a' | b'b' => {
                after_nonterminal = false;
                true
            }
            b'\'' => after_nonterminal,
            _ => false,
        })
}

fn check_cfg_productions(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_productions(r, rows);
    let mut valid = true;
    for row in rows {
        let Some((left, right)) = edge_trim(row.text).split_once("->") else {
            continue;
        };
        if !nonterminal(left) || !cfg_rhs(right) {
            valid = false;
            r.issue(Severity::Error, "cfg-production", Some(row.number), "文脈自由文法の左辺は大文字の非終端記号1つ（S' などのプライム付きも可）、右辺は非終端記号と a/b の列、または空列 $ 全体で記載してください。構文が壊れた欄の変換手順照合は行いません。");
        }
    }
    if valid {
        cfg_procedure::check(r, rows, cfg_procedure::Problem::Epsilon11);
    }
}
