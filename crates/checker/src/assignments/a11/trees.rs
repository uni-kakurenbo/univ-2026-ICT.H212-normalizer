use crate::{AnswerLine, Report, Severity, edge_trim};

// Problem data from 11_answer-sheet.txt, not solved derivation trees.
const RULES: &[(u8, &[u8])] = &[
    (b'S', b"bA"),
    (b'S', b"aB"),
    (b'A', b"a"),
    (b'A', b"aS"),
    (b'A', b"bAA"),
    (b'B', b"b"),
    (b'B', b"bS"),
    (b'B', b"aBB"),
];
const TARGET: &[u8] = b"aababb";

struct Frame {
    label: u8,
    position: usize,
    children: [u8; 3],
    count: usize,
}

impl Frame {
    fn new(label: u8, position: usize) -> Self {
        Self {
            label,
            position,
            children: [0; 3],
            count: 0,
        }
    }
    fn child(&mut self, label: u8) {
        if let Some(slot) = self.children.get_mut(self.count) {
            *slot = label;
        }
        self.count += 1;
    }
}

struct Tree {
    wrong_node: Option<(u8, usize)>,
    yield_matches: bool,
    leaves: usize,
}

// All problem productions have at most three immediate children. Retain only
// that bounded prefix for local matching, not whole nested substrings. Scan
// leaves in order without retaining their text. No recursion or tree search.
fn inspect(value: &str, rules: &[(u8, &[u8])], target: &[u8]) -> Option<Tree> {
    let bytes = value.as_bytes();
    if !bytes.starts_with(b"S(") {
        return None;
    }
    let mut stack = vec![Frame::new(b'S', 0)];
    let mut result = Tree {
        wrong_node: None,
        yield_matches: true,
        leaves: 0,
    };
    let mut i = 2;
    while i < bytes.len() {
        match bytes[i] {
            label @ (b'S' | b'A' | b'B') if bytes.get(i + 1) == Some(&b'(') => {
                stack.last_mut()?.child(label);
                stack.push(Frame::new(label, i));
                i += 2;
            }
            label @ (b'a' | b'b') => {
                stack.last_mut()?.child(label);
                result.yield_matches &= target.get(result.leaves) == Some(&label);
                result.leaves += 1;
                i += 1;
            }
            b')' => {
                let frame = stack.pop()?;
                if frame.count == 0 {
                    return None;
                }
                if !rules.iter().any(|&(left, right)| {
                    left == frame.label && frame.children.get(..frame.count) == Some(right)
                }) {
                    result
                        .wrong_node
                        .get_or_insert((frame.label, frame.position));
                }
                i += 1;
                if stack.is_empty() {
                    result.yield_matches &= result.leaves == target.len();
                    return (i == bytes.len()).then_some(result);
                }
            }
            _ => return None,
        }
    }
    None
}

pub(super) fn check_trees(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check(r, rows, RULES, TARGET);
}

fn check(r: &mut Report, rows: &[&AnswerLine<'_>], rules: &[(u8, &[u8])], target: &[u8]) {
    if rows.is_empty() {
        return;
    }
    if rows.len() != 2 {
        r.issue(Severity::Error, "tree-count", rows.first().map(|row| row.number), "11-1 の導出木は1行に1つ、計2行で記載してください。木を並べ替えたり不足分を補ったりはしません。");
    }
    let mut valid = 0;
    for row in rows {
        let value = edge_trim(row.text);
        let Some(tree) = inspect(value, rules, target) else {
            r.issue(Severity::Error, "tree-syntax", Some(row.number), "導出木は S(...) を根とし、非終端 S/A/B の直後に子を左から (...) で囲んでください。葉は a/b、括弧内は空にせず、木の途中に空白・カンマ・矢印は入れません。構文が壊れた木の葉・生成規則は検査しません。");
            continue;
        };
        valid += 1;
        if let Some((label, position)) = tree.wrong_node {
            let indentation = row.text.len() - row.text.trim_start_matches([' ', '\t']).len();
            r.issue(Severity::Error, "mechanical-tree-production", Some(row.number), format!("行の{}文字目の {} の直下の子が、問題で与えられた生成規則に一致しません。子の順序・記号を確認してください。最初に見つかった違反節点を表示しています。木は自動変更しません。", indentation + position + 1, label as char));
        }
        if !tree.yield_matches {
            r.issue(Severity::Error, "mechanical-tree-yield", Some(row.number), format!("葉を左から読んだ語が問題の指定語と一致しません。葉は{}個です。記号の順序と個数を確認してください。木は自動変更しません。", tree.leaves));
        }
    }
    // The syntax has a unique compact spelling for each ordered labelled tree.
    if rows.len() == 2 && valid == 2 && edge_trim(rows[0].text) == edge_trim(rows[1].text) {
        r.issue(
            Severity::Error,
            "mechanical-tree-distinct",
            Some(rows[1].number),
            "2つの導出木が同一です。異なる木を2つ記載してください。重複は削除せず保持します。",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A small, unrelated ambiguous grammar; no solved assignment trees.
    const SMALL: &[(u8, &[u8])] = &[(b'S', b"AB"), (b'S', b"aB"), (b'A', b"a"), (b'B', b"b")];

    #[test]
    fn matching_checks_immediate_children_separately_from_the_leaf_word() {
        let correct = inspect("S(A(a)B(b))", SMALL, b"ab").unwrap();
        assert!(correct.wrong_node.is_none());
        assert!(correct.yield_matches);
        let different_word = inspect("S(A(a)B(b))", SMALL, b"ba").unwrap();
        assert!(different_word.wrong_node.is_none());
        assert!(!different_word.yield_matches);
        let wrong_branch = inspect("S(ab)", SMALL, b"ab").unwrap();
        assert_eq!(wrong_branch.wrong_node, Some((b'S', 0)));
        assert!(wrong_branch.yield_matches);
        let wrong_child = inspect("S(A(b)B(a))", SMALL, b"ba").unwrap();
        assert_eq!(wrong_child.wrong_node, Some((b'A', 2)));
        assert!(wrong_child.yield_matches);
        assert!(!inspect("S(aB(b))", SMALL, b"abb").unwrap().yield_matches);
    }

    #[test]
    fn two_valid_distinct_trees_pass_and_duplicates_are_errors_without_correction() {
        let one = AnswerLine {
            text: "S(A(a)B(b))",
            number: 5,
        };
        let two = AnswerLine {
            text: "S(aB(b))",
            number: 6,
        };
        let mut r = Report::default();
        check(&mut r, &[&one, &two], SMALL, b"ab");
        assert_eq!(r.errors(), 0, "{:?}", r.issues);
        check(&mut r, &[&one, &one], SMALL, b"ab");
        assert_eq!(r.errors(), 1, "{:?}", r.issues);
        assert_eq!(r.issues[0].code, "mechanical-tree-distinct");
    }
}
