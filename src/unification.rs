use std::collections::HashSet;

use crate::r#const::Const;

#[derive(Clone, Debug)]
pub struct Unification {
    var_unify: HashSet<String>,
    var_disunify: HashSet<String>, // The variable that this variable is NOT unifiable with

    // We can only be unified with at most one constant, but we can be disunified with as many as we want
    const_unify: Option<Const>,
    const_disunify: Vec<Const>
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

    pub fn var_unify_clone(&self) -> HashSet<String> {
        return self.var_unify.clone();
    }

    pub fn do_disunify(&mut self, other: &String) -> bool {
        if self.var_unify.contains(other) {
            return false;
        } else {
            self.var_disunify.insert(other.to_string());
            return true;
        }
    }

    pub fn do_disunify_const(&mut self, c: &Const) -> bool {
        return match &self.const_unify {
            Some(cur_c) => {
                if cur_c == c {
                    false
                } else {
                    self.const_disunify.push(c.clone());
                    true
                }
            },

            None => {
                self.const_disunify.push(c.clone()); // TODO: Can probably simplify this by having constant tables and such
                true
            }
        };
    }

    pub fn do_unify(&mut self, other: &String) -> bool {
        if self.var_disunify.contains(other) {
            return false;
        } else {
            self.var_unify.insert(other.to_string());
            return true;
        }
    }

    pub fn do_unify_const(&mut self, c: &Const) -> bool {
        return match &self.const_unify {
            Some(cur_c) => cur_c == c,
            None => {
                self.const_unify = Some(c.clone());
                true
            }
        };
    }
}
