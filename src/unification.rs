use std::collections::HashSet;

use crate::r#const::Const;

#[derive(Clone, Debug)]
pub struct Unification {
    pub var_unify: HashSet<String>,
    pub var_disunify: HashSet<String>, // The variable that this variable is NOT unifiable with

    // We can only be unified with at most one constant, but we can be disunified with as many as we want
    pub const_unify: Option<Const>,
    pub const_disunify: Vec<Const>
}

impl Unification {
    pub fn new() -> Unification {
        Unification {
            var_unify: HashSet::new(),
            var_disunify: HashSet::new(),
            const_unify: None,
            const_disunify: Vec::new()
        }
    }
}
