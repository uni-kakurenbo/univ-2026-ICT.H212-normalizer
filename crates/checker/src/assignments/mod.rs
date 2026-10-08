mod a01;
mod a02;
mod a03;
mod a04;

use crate::Section;

pub struct Assignment {
    pub id: &'static str,
    pub title: &'static str,
    pub note: &'static str,
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
];

pub fn get(id: &str) -> Option<&'static Assignment> {
    ASSIGNMENTS.iter().find(|a| a.id == id)
}
