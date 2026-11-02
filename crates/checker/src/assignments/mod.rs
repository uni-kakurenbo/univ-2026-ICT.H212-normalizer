mod a01;
mod a02;
mod a03;
mod a04;
mod a05;
mod a06;
mod a07;
mod a08;
mod a09;
mod a11;
mod a12;
mod a13;
mod a14;
mod cfg_procedure;

use crate::{AnswerLine, Report, Section};

pub(crate) use a14::normalize_definition as normalize_pda_definition;

pub struct Assignment {
    pub id: &'static str,
    pub title: &'static str,
    pub note: &'static str,
    pub comment_prefix: &'static str,
    /// In transition sections, the first answer row may start with a '#' state.
    /// Opt-in so extending assignments does not change existing hash inputs.
    pub hash_prefixed_finals: bool,
    pub sections: &'static [Section],
}

pub static ASSIGNMENTS: &[Assignment] = &[
    a01::ASSIGNMENT,
    a02::ASSIGNMENT,
    a03::ASSIGNMENT,
    a04::ASSIGNMENT,
    a05::ASSIGNMENT,
    a06::ASSIGNMENT,
    a07::ASSIGNMENT,
    a08::ASSIGNMENT,
    a09::ASSIGNMENT,
    a11::ASSIGNMENT,
    a12::ASSIGNMENT,
    a13::ASSIGNMENT,
    a14::ASSIGNMENT,
];

pub fn get(id: &str) -> Option<&'static Assignment> {
    ASSIGNMENTS.iter().find(|a| a.id == id)
}

// Cross-section checks are opt-in and run only after section boundaries have
// been identified unambiguously. Earlier assignments keep their hash behavior.
pub(crate) fn check_related_sections(id: &str, r: &mut Report, bodies: &[Vec<AnswerLine<'_>>]) {
    if id == "14" {
        a14::check_related(r, bodies);
    }
}
