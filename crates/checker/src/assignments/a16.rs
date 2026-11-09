use super::{
    Assignment,
    cfg_procedure::npda,
    grammar::{self, Form},
};
use crate::{AnswerLine, Format, Report, Section, Severity};

// Sources: 16_answer-sheet.txt and autolang第16回.pdf pp.6–11.
pub const ASSIGNMENT: Assignment = Assignment {
    id: "16",
    title: "演習16：NPDAから文脈自由文法への構成",
    note: "各欄の先頭に G=(N,T,P,S)、その後に1行1規則を記載します。集合と規則はASCII辞書順、非終端記号の添字は ^{i,j}、底記号は Z、空列は $ です。N は規則に現れる非終端記号の集合に合わせます。講義の変換手順との不一致は warning です。",
    comment_prefix: "%%%",
    hash_prefixed_finals: false,
    sections: &[
        Section {
            id: "16-1",
            marker: "16-1=====",
            ascii: true,
            format: Format::DeclaredGrammar { sorted: true },
            validate: Some(check_first),
        },
        Section {
            id: "16-2",
            marker: "16-2=====",
            ascii: true,
            format: Format::DeclaredGrammar { sorted: true },
            validate: Some(check_second),
        },
    ],
};

fn check_first(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check(r, rows, npda::Problem::First);
}

fn check_second(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check(r, rows, npda::Problem::Second);
}

fn check(r: &mut Report, rows: &[&AnswerLine<'_>], problem: npda::Problem) {
    let Some(first) = rows.first() else {
        return;
    };
    let form = Form {
        name: "G",
        production_set: "P",
        notation: problem.notation(),
    };

    if let Some(rules) = grammar::check(r, rows, form) {
        npda::check(r, first.number, &rules, problem);
    } else {
        r.issue(
            Severity::Warning,
            "npda-cfg-unchecked",
            Some(first.number),
            "宣言や生成規則を読み取れないため、NPDA→CFGの手順照合は未検査です。",
        );
    }
}
