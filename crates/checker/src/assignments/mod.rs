mod a01;
mod a02;
mod a03;

use crate::Section;

pub struct Assignment {
    pub id: &'static str,
    pub title: &'static str,
    pub note: &'static str,
    pub sections: &'static [Section],
}

pub static ASSIGNMENTS: &[Assignment] = &[a01::ASSIGNMENT, a02::ASSIGNMENT, a03::ASSIGNMENT];

pub fn get(id: &str) -> Option<&'static Assignment> {
    ASSIGNMENTS.iter().find(|a| a.id == id)
}
