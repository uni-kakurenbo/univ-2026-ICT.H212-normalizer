use super::{Assignment, a07::check_proof_notation};
use crate::{AnswerLine, Format, Report, Section};

// Sources: 13_answer-sheet.txt; autolang第13回.pdf pp.5, 8–9.
// The two proof sections allow Japanese prose; math uses ^{...}, |u|,
// <=/>=/</>/=/!=, in/not in and ASCII. Reuse the user's assignment-07 policy:
// optimistic automatic detection, explicit notation errors and uncertain-math
// warnings. Preserve prose, whitespace, paragraphs, order and hash v1.
// Unlike 07, no ASCII empty-word spelling is specified: never require eps.
// Do not judge pumping-lemma arguments, chosen words or quantified cases, and
// never add a model proof or apply the 08/11/12 mechanical extensions here.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "13",
    title: "演習13：文脈自由言語の反復補題",
    note: "13-1・13-2 の証明文は日本語で記載できます。指数は ^{k}、長さは |u|、比較は <=/>=/</>/=/!=、所属は in / not in。数式候補を自動検出し、曖昧な箇所は warning にします。文章の空白・段落・順序を保持し、証明の正しさは判定しません。空語の特定の表記は要求しません。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "13-1",
            marker: "13-1=====",
            ascii: false,
            format: Format::Prose,
            validate: Some(check_notation),
        },
        Section {
            id: "13-2",
            marker: "13-2=====",
            ascii: false,
            format: Format::Prose,
            validate: Some(check_notation),
        },
    ],
};

fn check_notation(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_proof_notation(r, rows, None);
}
