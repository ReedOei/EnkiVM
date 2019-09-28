use std::collections::HashSet;
use std::collections::HashMap;
use std::collections::VecDeque;

use num_bigint::Sign;

use crate::err::Err;
use crate::stackitem::{StackItem, Value};
use crate::unification::Unification;

#[derive(Clone, Debug)]
pub struct Environment {
    pub data: VecDeque<StackItem>,
    pub unified: HashMap<String, Unification>,
    pub choicepoint: Option<(usize, Box<Environment>)>,
    pub fresh_counter: usize
}

fn le_bytes_to_usize(le_bytes: Vec<u8>) -> Result<usize, Err> {
    if le_bytes.len() > 8 {
        return Err::err_res(format!("Index {:?} is too large!", le_bytes));
    }

    let mut new_arr = [0; 8];
    for i in 0..le_bytes.len() - 1 {
        new_arr[i] = le_bytes[i];
    }

    return Ok(usize::from_le_bytes(new_arr));
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

    fn var_value(&self, var_name: &String) -> Result<Value, Err> {
        let mut to_check = VecDeque::new();
        to_check.push_front(var_name);
        let mut checked = HashSet::new();

        loop {
            match to_check.pop_front() {
                Some(var_name) => {
                    if checked.contains(var_name) {
                        continue;
                    }
                    checked.insert(var_name);

                    let unification = self.get_unified(var_name)?;

                    match &unification.value_unify {
                        Some(c) => return Ok(c.clone()),
                        None => {}
                    }

                    to_check.extend(&unification.var_unify);
                },

                None => break
            }
        }

        return Err::err_res(format!("No value found for: {}", var_name));
    }

    pub fn functor(&mut self, num: usize) -> Result<(), Err> {
        let mut items = Vec::new();

        for _i in 0..num {
            items.push(self.pop()?);
        }

        let name = match self.pop()? {
            StackItem::Value(Value::StringValue(s)) => s,
            item => return Err::err_res(format!("Functor name must be a string. Got: {:?}", item))
        };

        self.push(StackItem::Value(Value::Functor(name, items)))?;

        return Ok(());
    }

    pub fn nameof(&mut self) -> Result<(), Err> {
        return match self.pop()? {
            StackItem::Value(Value::Functor(name, _)) => self.push(StackItem::Value(Value::StringValue(name))),
            item => Err::err_res(format!("Cannot take the name of a non-functor: {:?}", item))
        }
    }

    pub fn project(&mut self) -> Result<(), Err> {
        let index = match self.pop()? {
            StackItem::Value(Value::IntValue(idx)) => idx,
            StackItem::Variable(var_name) => {
                match self.var_value(&var_name)? {
                    Value::IntValue(idx) => idx,
                    _ => return Err::err_res("Top stack item was not an integer".to_string())
                }
            },
            _ => return Err::err_res("Top stack item was not an integer".to_string())
        };

        let (sign, le_bytes) = index.to_bytes_le();

        if sign == Sign::Minus {
            return Err::err_res("Functor indices must be nonnegative integers!".to_string());
        }

        let idx: usize = le_bytes_to_usize(le_bytes)?;

        match self.pop()? {
            StackItem::Value(Value::Functor(_, args)) => {
                if idx < args.len() {
                    self.push(args[idx].clone())?;
                } else {
                    return Err::err_res(format!("Functor has {} arguments, but tried to access index {}", args.len(), index));
                }
            }
            item => return Err::err_res(format!("Cannot index into a non-functor: {:?}", item))
        }

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


    fn is_disunified_value(&self, v: &String, c: &Value) -> Result<bool, Err> {
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

                    for check_value in &unification.value_disunify {
                        if check_value == c {
                            return Ok(true);
                        }
                    }

                    match &unification.value_unify {
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

    fn is_unified_value(&self, v: &String, c: &Value) -> Result<bool, Err> {
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

                    for check_value in &unification.value_disunify {
                        if check_value == c {
                            return Ok(false);
                        }
                    }

                    match &unification.value_unify {
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

    fn unify_var_value(&mut self, v: String, c: Value) -> Result<(), Err> {
        self.ensure_unification_exists(&v)?;

        if self.is_disunified_value(&v, &c)? {
            return Err::err_res(format!("Could not unify '{}' and '{}'", v, c));
        }

        let unified = self.access_unified(&v);

        unified.value_unify = Some(c);

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
            (StackItem::Variable(v1), StackItem::Value(c2)) => self.unify_var_value(v1, c2),
            (StackItem::Value(c1), StackItem::Variable(v2)) => self.unify_var_value(v2, c1),
            (StackItem::Value(c1), StackItem::Value(c2)) => {
                if c1 == c2 {
                    Ok(())
                } else {
                    Err::err_res(format!("Cannot unify values '{}' and '{}'", c1, c2))
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

    fn disunify_var_value(&mut self, v: String, c: Value) -> Result<(), Err> {
        self.ensure_unification_exists(&v)?;

        if self.is_unified_value(&v, &c)? {
            return Err::err_res(format!("Could not unify '{}' and '{}'", v, c));
        }

        let unified = self.access_unified(&v);
        unified.value_disunify.push(c);

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
            (StackItem::Variable(v1), StackItem::Value(c2)) => self.disunify_var_value(v1, c2),
            (StackItem::Value(c1), StackItem::Variable(v2)) => self.disunify_var_value(v2, c1),
            (StackItem::Value(c1), StackItem::Value(c2)) => {
                if c1 != c2 {
                    Ok(())
                } else {
                    Err::err_res(format!("Cannot disunify values '{}' and '{}'", c1, c2))
                }
            }
        };
    }
}
