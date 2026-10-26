use super::{Assignment, a02::check_productions};
use crate::{AnswerLine, Format, Report, Section, Severity, edge_trim};

// Sources: 11_answer-sheet.txt; autolang第11回.pdf pp.3, 5–8.
// 11-1: exactly two S-rooted trees, nonterminals S/A/B immediately followed
// by nonempty (...), leaves a/b. No production-edge or leaf-word comparison.
// The order of the two trees is unspecified: preserve it and all duplicates.
// 11-2: CFG notation, not a comparison against epsilon-elimination results.
// The lecture permits a fresh primed start symbol for nullable grammars; keep
// uppercase letters with prime suffixes rather than inventing a naming rule.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "11",
    title: "演習11：文脈自由文法・導出木",
    note: "11-1 は S を根とする導出木を1行に1つ、計2行。非終端 S/A/B の直後に子を (...) で囲み、葉は a/b。木の順序は保持します。11-2 は -> と $ を使い、1行1規則を ASCII 辞書順で列挙します。導出木の正しさ・葉の語・ε除去の結果は判定しません。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "11-1",
            marker: "11-1=====",
            ascii: true,
            format: Format::Compact,
            validate: Some(check_trees),
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

// Iterative, linear scan: nested trees never recurse on the WASM call stack.
fn tree(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !bytes.starts_with(b"S(") {
        return false;
    }
    let mut children = vec![false];
    let mut i = 2;
    while i < bytes.len() {
        match bytes[i] {
            b'S' | b'A' | b'B' if bytes.get(i + 1) == Some(&b'(') => {
                *children.last_mut().expect("open tree") = true;
                children.push(false);
                i += 2;
            }
            b'a' | b'b' => {
                *children.last_mut().expect("open tree") = true;
                i += 1;
            }
            b')' => {
                if children.pop() != Some(true) {
                    return false;
                }
                i += 1;
                if children.is_empty() {
                    return i == bytes.len();
                }
            }
            _ => return false,
        }
    }
    false
}

fn check_trees(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    if rows.is_empty() {
        return;
    }
    if rows.len() != 2 {
        r.issue(Severity::Error, "tree-count", rows.first().map(|row| row.number), "11-1 の導出木は1行に1つ、計2行で記載してください。木を並べ替えたり不足分を補ったりはしません。");
    }
    for row in rows {
        if !tree(edge_trim(row.text)) {
            r.issue(Severity::Error, "tree-syntax", Some(row.number), "導出木は S(...) を根とし、非終端 S/A/B の直後に子を左から (...) で囲んでください。葉は a/b、括弧内は空にせず、木の途中に空白・カンマ・矢印は入れません。生成規則の適用や葉の語は判定しません。");
        }
    }
    if rows.len() == 2 && edge_trim(rows[0].text) == edge_trim(rows[1].text) {
        r.issue(Severity::Warning, "duplicate-tree", Some(rows[1].number), "同じ導出木が2行記載されています。2つの異なる木を記載したか確認してください。重複は削除しません。");
    }
}

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
    for row in rows {
        let Some((left, right)) = edge_trim(row.text).split_once("->") else {
            continue;
        };
        if !nonterminal(left) || !cfg_rhs(right) {
            r.issue(Severity::Error, "cfg-production", Some(row.number), "文脈自由文法の左辺は大文字の非終端記号1つ（S' などのプライム付きも可）、右辺は非終端記号と a/b の列、または空列 $ 全体で記載してください。どの規則が必要かは判定しません。");
        }
    }
}
