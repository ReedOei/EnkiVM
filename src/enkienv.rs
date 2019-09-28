use std::collections::HashSet;
use std::collections::HashMap;
use std::collections::VecDeque;

use crate::err::Err;
use crate::stackitem::{StackItem, Const};
use crate::unification::Unification;

#[derive(Clone, Debug)]
pub struct Environment {
    pub data: VecDeque<StackItem>,
    pub unified: HashMap<String, Unification>,
    pub choicepoint: Option<(usize, Box<Environment>)>,
    pub fresh_counter: usize
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            data: VecDeque::new(),
            unified: HashMap::new(),
            choicepoint: None,
            fresh_counter: 0
        }
    }

    pub fn push(&mut self, new_item: StackItem) -> Result<(), Err> {
        self.data.push_front(new_item);
        return Ok(());
    }

    pub fn pop(&mut self) -> Result<StackItem, Err> {
        return self.data.pop_front().ok_or(Err::new("No items on stack to pop".to_string()));
    }

    pub fn dup(&mut self) -> Result<(), Err> {
        let item = self.pop()?;
        self.push(item.clone())?;
        self.push(item)?;

        return Ok(());
    }

    fn access_unified(&mut self, v: &String) -> &mut Unification {
        if !self.unified.contains_key(v) {
            self.unified.insert(v.to_string(), Unification::new());
        }

        return self.unified.get_mut(v).unwrap(); // We can safely unwrap here, since we know we put it in above
    }

    fn get_unified(&self, v: &String) -> Result<&Unification, Err> {
        return self.unified.get(v).ok_or(Err::new(format!("Unification doesn't exist for {}", v)));
    }

    fn is_disunified(&self, v1: &String, v2: &String) -> Result<bool, Err> {
        let mut to_check = VecDeque::new();
        to_check.push_front(v2);
        let mut checked = HashSet::new();

        loop {
            match to_check.pop_front() {
                Some(var_name) => {
                    if checked.contains(var_name) {
                        continue;
                    }
                    checked.insert(var_name);

                    let unification = self.get_unified(var_name)?;

                    if unification.var_disunify.contains(v1) {
                        return Ok(true);
                    }

                    to_check.extend(&unification.var_unify);
                },

                None => break
            }
        }

        return Ok(false);
    }


    fn is_disunified_const(&self, v: &String, c: &Const) -> Result<bool, Err> {
        let mut to_check = VecDeque::new();
        to_check.extend(&self.get_unified(v)?.var_unify);
        let mut checked = HashSet::new();

        loop {
            match to_check.pop_front() {
                Some(var_name) => {
                    if checked.contains(var_name) {
                        continue;
                    }
                    checked.insert(var_name);

                    let unification = self.get_unified(var_name)?;

                    for check_const in &unification.const_disunify {
                        if check_const == c {
                            return Ok(true);
                        }
                    }

                    match &unification.const_unify {
                        Some(cur_c) => {
                            return Ok(cur_c != c);
                        },
                        None => {}
                    }

                    to_check.extend(&unification.var_unify);
                },

                None => break
            }
        }

        return Ok(false);
    }

    fn is_unified(&self, v1: &String, v2: &String) -> Result<bool, Err> {
        let mut to_check = VecDeque::new();
        to_check.push_front(v2);
        let mut checked = HashSet::new();

        loop {
            match to_check.pop_front() {
                Some(var_name) => {
                    if checked.contains(var_name) {
                        continue;
                    }
                    checked.insert(var_name);

                    let unification = self.get_unified(var_name)?;

                    if unification.var_unify.contains(v1) {
                        return Ok(true);
                    }

                    to_check.extend(&unification.var_unify);
                },

                None => break
            }
        }

        return Ok(false);
    }

    fn is_unified_const(&self, v: &String, c: &Const) -> Result<bool, Err> {
        let mut to_check = VecDeque::new();
        to_check.extend(&self.get_unified(v)?.var_unify);
        let mut checked = HashSet::new();

        loop {
            match to_check.pop_front() {
                Some(var_name) => {
                    if checked.contains(var_name) {
                        continue;
                    }
                    checked.insert(var_name);

                    let unification = self.get_unified(var_name)?;

                    for check_const in &unification.const_disunify {
                        if check_const == c {
                            return Ok(false);
                        }
                    }

                    match &unification.const_unify {
                        Some(cur_c) => {
                            return Ok(cur_c == c);
                        },
                        None => {}
                    }

                    to_check.extend(&unification.var_unify);
                },

                None => break
            }
        }

        return Ok(false);
    }

    fn unify_with(&mut self, v1: &String, v2: &String) -> Result<(), Err> {
        if self.is_disunified(v1, v2)? {
            return Err::err_res(format!("Could not unify '{}' and '{}'", v1, v2));
        }

        let unified = self.access_unified(v1);
        unified.var_unify.insert(v2.clone());

        return Ok(());
    }

    fn ensure_unification_exists(&mut self, v: &String) -> Result<(), Err> {
        if !self.unified.contains_key(v) {
            self.unified.insert(v.clone(), Unification::new());
        }

        return Ok(());
    }

    fn unify_vars(&mut self, v1: String, v2: String) -> Result<(), Err> {
        if v1 != v2 {
            self.ensure_unification_exists(&v1)?;
            self.ensure_unification_exists(&v2)?;

            self.unify_with(&v1, &v2)?;
            self.unify_with(&v2, &v1)?;
        }

        return Ok(());
    }

    fn unify_var_const(&mut self, v: String, c: Const) -> Result<(), Err> {
        self.ensure_unification_exists(&v)?;

        if self.is_disunified_const(&v, &c)? {
            return Err::err_res(format!("Could not unify '{}' and '{}'", v, c));
        }

        let unified = self.access_unified(&v);

        unified.const_unify = Some(c);

        return Ok(());
    }

    pub fn unify(&mut self) -> Result<(), Err> {
        let item1 = self.pop();
        let item2 = self.pop();

        if item1.is_err() {
            return item1.map(|_| ());
        } else if item2.is_err() {
            return item2.map(|_| ());
        }

        return match (item1.unwrap(), item2.unwrap()) {
            (StackItem::Variable(v1), StackItem::Variable(v2)) => self.unify_vars(v1, v2),
            (StackItem::Variable(v1), StackItem::ConstItem(c2)) => self.unify_var_const(v1, c2),
            (StackItem::ConstItem(c1), StackItem::Variable(v2)) => self.unify_var_const(v2, c1),
            (StackItem::ConstItem(c1), StackItem::ConstItem(c2)) => {
                if c1 == c2 {
                    Ok(())
                } else {
                    Err::err_res(format!("Cannot unify constants '{}' and '{}'", c1, c2))
                }
            }
        };
    }

    fn disunify_with(&mut self, v1: &String, v2: &String) -> Result<(), Err> {
        if self.is_unified(v1, v2)? {
            return Err::err_res(format!("Could not disunify '{}' and '{}'", v1, v2));
        }

        let unified = self.access_unified(v1);
        unified.var_disunify.insert(v2.clone());

        return Ok(());
    }

    fn disunify_vars(&mut self, v1: String, v2: String) -> Result<(), Err> {
        if v1 != v2 {
            self.ensure_unification_exists(&v1)?;
            self.ensure_unification_exists(&v2)?;

            self.disunify_with(&v1, &v2)?;
            self.disunify_with(&v2, &v1)?;
        }

        return Ok(());
    }

    fn disunify_var_const(&mut self, v: String, c: Const) -> Result<(), Err> {
        self.ensure_unification_exists(&v)?;

        if self.is_unified_const(&v, &c)? {
            return Err::err_res(format!("Could not unify '{}' and '{}'", v, c));
        }

        let unified = self.access_unified(&v);
        unified.const_disunify.push(c);

        return Ok(());
    }

    pub fn disunify(&mut self) -> Result<(), Err> {
        let item1 = self.pop();
        let item2 = self.pop();

        if item1.is_err() {
            return item1.map(|_| ());
        } else if item2.is_err() {
            return item2.map(|_| ());
        }

        return match (item1.unwrap(), item2.unwrap()) {
            (StackItem::Variable(v1), StackItem::Variable(v2)) => self.disunify_vars(v1, v2),
            (StackItem::Variable(v1), StackItem::ConstItem(c2)) => self.disunify_var_const(v1, c2),
            (StackItem::ConstItem(c1), StackItem::Variable(v2)) => self.disunify_var_const(v2, c1),
            (StackItem::ConstItem(c1), StackItem::ConstItem(c2)) => {
                if c1 != c2 {
                    Ok(())
                } else {
                    Err::err_res(format!("Cannot disunify constants '{}' and '{}'", c1, c2))
                }
            }
        };
    }
}
