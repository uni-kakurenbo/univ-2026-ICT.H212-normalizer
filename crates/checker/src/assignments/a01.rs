use super::Assignment;
use crate::{Format, Section};

// Source: autolang第1回.pdf pp.15–16; 01_answer-sheet.txt instruction line.
// No restrictions on the language generated or derivation correctness are inferred.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "01",
    hash_prefixed_finals: false,
    title: "演習01：句構造文法",
    note: "1-1・1-3 は ASCII。1-2 の文章は日本語を使用できます。導出の順序は変更しません。",
    sections: &[
        Section {
            id: "1-1",
            marker: "1-1=====",
            ascii: true,
            format: Format::Compact,
            validate: None,
        },
        Section {
            id: "1-2",
            marker: "1-2=====",
            ascii: false,
            format: Format::Prose,
            validate: None,
        },
        Section {
            id: "1-3",
            marker: "1-3=====",
            ascii: true,
            format: Format::Compact,
            validate: None,
        },
    ],
};
