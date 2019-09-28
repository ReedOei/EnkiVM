use std::collections::HashSet;

use crate::stackitem::Value;

#[derive(Clone, Debug)]
pub struct Unification {
    pub var_unify: HashSet<String>,
    pub var_disunify: HashSet<String>, // The variable that this variable is NOT unifiable with

    // We can only be unified with at most one value, but we can be disunified with as many as we want
    pub value_unify: Option<Value>,
    pub value_disunify: Vec<Value>
}

impl Unification {
    pub fn new() -> Unification {
        Unification {
            var_unify: HashSet::new(),
            var_disunify: HashSet::new(),
            value_unify: None,
            value_disunify: Vec::new()
        }
    }
}
