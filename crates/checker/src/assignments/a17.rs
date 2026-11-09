use super::{
    Assignment,
    a07::{ProofNotation, check_proof_notation},
    grammar::{self, Form, Notation},
};
use crate::{AnswerLine, Format, Report, Section};

// Sources: 17_answer-sheet.txt and autolang第17回.pdf pp.5,7.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "17",
    title: "演習17：文脈自由文法の性質",
    note: "文法の欄は G・G1・G2 の宣言を先頭に置き、非終端記号を英大文字1文字、開始記号を S、空列を $ とします。生成規則は記載順を保持します。17-2 は日本語で記載でき、数式には ^{…}、~L、and / or、in / not in などのASCII表記を使います。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "17-1",
            marker: "17-1=====",
            ascii: true,
            format: Format::DeclaredGrammar { sorted: false },
            validate: Some(check_g),
        },
        Section {
            id: "17-2",
            marker: "17-2=====",
            ascii: false,
            format: Format::Prose,
            validate: Some(check_proof),
        },
        Section {
            id: "17-3.1",
            marker: "17-3.1=====",
            ascii: true,
            format: Format::DeclaredGrammar { sorted: false },
            validate: Some(check_g1),
        },
        Section {
            id: "17-3.2",
            marker: "17-3.2=====",
            ascii: true,
            format: Format::DeclaredGrammar { sorted: false },
            validate: Some(check_g2),
        },
    ],
};

fn check_g(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    let _ = grammar::check(
        r,
        rows,
        Form {
            name: "G",
            production_set: "P",
            notation: Notation::Letters,
        },
    );
}

fn check_g1(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    let _ = grammar::check(
        r,
        rows,
        Form {
            name: "G1",
            production_set: "P1",
            notation: Notation::Letters,
        },
    );
}

fn check_g2(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    let _ = grammar::check(
        r,
        rows,
        Form {
            name: "G2",
            production_set: "P2",
            notation: Notation::Letters,
        },
    );
}

fn check_proof(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_proof_notation(
        r,
        rows,
        ProofNotation {
            set_operators: true,
            ..ProofNotation::default()
        },
    );
}
