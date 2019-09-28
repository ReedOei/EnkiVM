use std::collections::HashSet;
use std::collections::HashMap;
use std::collections::VecDeque;

use num_bigint::Sign;
use num_bigint::BigInt;
use num_traits::pow::Pow;

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
    for i in 0..le_bytes.len() {
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

    pub fn swap(&mut self) -> Result<(), Err> {
        let item1 = self.pop()?;
        let item2 = self.pop()?;

        self.push(item1)?;
        self.push(item2)?;

        return Ok(());
    }

    pub fn print(&mut self) -> Result<(), Err> {
        match self.pop()? {
            StackItem::Variable(var_name) => {
                match self.var_value_opt(&var_name)? {
                    Some(val) => print!("{}", val),
                    None => print!("{}", var_name)
                }
            },

            StackItem::Value(val) => print!("{}", val)
        }

        return Ok(());
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

    fn var_value_opt(&self, var_name: &String) -> Result<Option<Value>, Err> {
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
                        Some(c) => return Ok(Some(c.clone())),
                        None => {}
                    }

                    to_check.extend(&unification.var_unify);
                },

                None => break
            }
        }

        return Ok(None);
    }

    fn var_value(&self, var_name: &String) -> Result<Value, Err> {
        return self.var_value_opt(var_name)?.ok_or(Err::new(format!("No value found for: {}", var_name)));
    }

    pub fn functor(&mut self) -> Result<(), Err> {
        let mut items = Vec::new();

        let name = match self.pop()? {
            StackItem::Value(Value::StringValue(s)) => s,
            item => return Err::err_res(format!("Functor name must be a string. Got: {:?}", item))
        };

        let num = self.popidx()?;

        for _i in 0..num {
            items.push(self.pop()?);
        }

        self.push(StackItem::Value(Value::Functor(name, items)))?;

        return Ok(());
    }

    pub fn over(&mut self) -> Result<(), Err> {
        let b = self.pop()?;
        let a = self.pop()?;

        self.push(a.clone())?;
        self.push(b)?;
        self.push(a)?;

        return Ok(());
    }

    pub fn rot(&mut self) -> Result<(), Err> {
        let c = self.pop()?;
        let b = self.pop()?;
        let a = self.pop()?;

        self.push(c)?;
        self.push(a)?;
        self.push(b)?;

        return Ok(());
    }

    pub fn nameof(&mut self) -> Result<(), Err> {
        return match self.pop()? {
            StackItem::Value(Value::Functor(name, _)) => self.push(StackItem::Value(Value::StringValue(name))),
            item => Err::err_res(format!("Cannot take the name of a non-functor: {:?}", item))
        }
    }

    pub fn popint(&mut self) -> Result<BigInt, Err> {
        let i = match self.pop()? {
            StackItem::Value(Value::IntValue(idx)) => idx,
            StackItem::Variable(var_name) => {
                match self.var_value(&var_name)? {
                    Value::IntValue(idx) => idx,
                    _ => return Err::err_res("Top stack item was not an integer".to_string())
                }
            },
            _ => return Err::err_res("Top stack item was not an integer".to_string())
        };

        return Ok(i);
    }

    pub fn popidx(&mut self) -> Result<usize, Err> {
        let index = self.popint()?;

        let (sign, le_bytes) = index.to_bytes_le();

        if sign == Sign::Minus {
            return Err::err_res("Functor indices must be nonnegative integers!".to_string());
        }

        return Ok(le_bytes_to_usize(le_bytes)?);
    }

    pub fn add(&mut self) -> Result<(), Err> {
        let a = self.popint()?;
        let b = self.popint()?;

        self.push(StackItem::Value(Value::IntValue(a + b)))?;

        return Ok(());
    }

    pub fn sub(&mut self) -> Result<(), Err> {
        let a = self.popint()?;
        let b = self.popint()?;

        self.push(StackItem::Value(Value::IntValue(a - b)))?;

        return Ok(());
    }

    pub fn div(&mut self) -> Result<(), Err> {
        let a = self.popint()?;
        let b = self.popint()?;

        self.push(StackItem::Value(Value::IntValue(a / b)))?;

        return Ok(());
    }

    pub fn mul(&mut self) -> Result<(), Err> {
        let a = self.popint()?;
        let b = self.popint()?;

        self.push(StackItem::Value(Value::IntValue(a * b)))?;

        return Ok(());
    }

    pub fn lt(&mut self) -> Result<(), Err> {
        let a = self.popint()?;
        let b = self.popint()?;

        if a < b {
            return Ok(());
        } else {
            return Err::err_res(format!("{} not less than {}", a, b));
        }
    }

    pub fn gt(&mut self) -> Result<(), Err> {
        let a = self.popint()?;
        let b = self.popint()?;

        if a > b {
            return Ok(());
        } else {
            return Err::err_res(format!("{} not less than {}", a, b));
        }
    }

    pub fn lte(&mut self) -> Result<(), Err> {
        let a = self.popint()?;
        let b = self.popint()?;

        if a <= b {
            return Ok(());
        } else {
            return Err::err_res(format!("{} not less than {}", a, b));
        }
    }

    pub fn gte(&mut self) -> Result<(), Err> {
        let a = self.popint()?;
        let b = self.popint()?;

        if a >= b {
            return Ok(());
        } else {
            return Err::err_res(format!("{} not less than {}", a, b));
        }
    }

    pub fn pow(&mut self) -> Result<(), Err> {
        let a = self.popint()?;
        let bint = self.popint()?;

        return match bint.to_biguint() {
            Some(b) => {
                self.push(StackItem::Value(Value::IntValue(a.pow(b))))?;

                Ok(())
            }
            None => Err::err_res(format!("Cannot raise {} to the power of {} because {} is negative", a, bint, bint))
        };
    }

    pub fn project(&mut self) -> Result<(), Err> {
        let idx: usize = self.popidx()?;

        match self.pop()? {
            StackItem::Value(Value::Functor(_, args)) => {
                if idx < args.len() {
                    self.push(args[idx].clone())?;
                } else {
                    return Err::err_res(format!("Functor has {} arguments, but tried to access index {}", args.len(), idx));
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
        to_check.push_front(v);
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
        to_check.push_front(v);
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

            match (self.var_value_opt(&v1)?, self.var_value_opt(&v2)?) {
                (Some(Value::Functor(name1, args1)), Some(Value::Functor(name2, args2))) => {
                    if name1 != name2 {
                        return Err::err_res(format!("Cannot unify {} and {}: functor names {} and {} don't match", v1, v2, name1, name2));
                    }

                    for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                        self.unify_items(arg1.clone(), arg2.clone())?;
                    }

                    return Ok(());
                }

                _ => {}
            }

            self.unify_with(&v1, &v2)?;
            self.unify_with(&v2, &v1)?;
        }

        return Ok(());
    }

    fn unify_var_value(&mut self, v: String, c: Value) -> Result<(), Err> {
        self.ensure_unification_exists(&v)?;

        match (self.var_value_opt(&v)?, c.clone()) {
            (Some(Value::Functor(name1, args1)), Value::Functor(name2, args2)) => {
                if name1 != name2 {
                    return Err::err_res(format!("Cannot unify {} and {}: functor names {} and {} don't match", v, c, name1, name2));
                }

                for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                    self.unify_items(arg1.clone(), arg2.clone())?;
                }

                return Ok(());
            }

            _ => {}
        }

        if self.is_disunified_value(&v, &c)? {
            return Err::err_res(format!("Could not unify '{}' and '{}'", v, c));
        }

        let unified = self.access_unified(&v);
        unified.value_unify = Some(c);

        return Ok(());
    }

    fn unify_items(&mut self, item1: StackItem, item2: StackItem) -> Result<(), Err> {
        return match (item1, item2) {
            (StackItem::Variable(v1), StackItem::Variable(v2)) => self.unify_vars(v1, v2),
            (StackItem::Variable(v1), StackItem::Value(c2)) => self.unify_var_value(v1, c2),
            (StackItem::Value(c1), StackItem::Variable(v2)) => self.unify_var_value(v2, c1),
            (StackItem::Value(c1), StackItem::Value(c2)) => {
                match (c1.clone(), c2.clone()) {
                    (Value::Functor(name1, args1), Value::Functor(name2, args2)) => {
                        if name1 != name2 {
                            return Err::err_res(format!("Cannot unify {} and {}: functor names {} and {} don't match", c1, c2, name1, name2));
                        }

                        for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                            self.unify_items(arg1.clone(), arg2.clone())?;
                        }

                        return Ok(());
                    }

                    _ => {
                        if c1 == c2 {
                            Ok(())
                        } else {
                            Err::err_res(format!("Cannot unify values '{}' and '{}'", c1, c2))
                        }
                    }
                }
            }
        };
    }

    pub fn unify(&mut self) -> Result<(), Err> {
        let item1 = self.pop()?;
        let item2 = self.pop()?;

        return self.unify_items(item1, item2);
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

            match (self.var_value_opt(&v1)?, self.var_value_opt(&v2)?) {
                (Some(Value::Functor(name1, args1)), Some(Value::Functor(name2, args2))) => {
                    if name1 != name2 {
                        return Ok(());
                    }

                    for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                        self.disunify_items(arg1.clone(), arg2.clone())?;
                    }

                    return Ok(());
                }

                _ => {}
            }

            self.disunify_with(&v1, &v2)?;
            self.disunify_with(&v2, &v1)?;
        }

        return Ok(());
    }

    fn disunify_var_value(&mut self, v: String, c: Value) -> Result<(), Err> {
        self.ensure_unification_exists(&v)?;

        match (self.var_value_opt(&v)?, c.clone()) {
            (Some(Value::Functor(name1, args1)), Value::Functor(name2, args2)) => {
                if name1 != name2 {
                    return Ok(());
                }

                for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                    self.disunify_items(arg1.clone(), arg2.clone())?;
                }

                return Ok(());
            }

            _ => {}
        }

        if self.is_unified_value(&v, &c)? {
            return Err::err_res(format!("Could not unify '{}' and '{}'", v, c));
        }

        let unified = self.access_unified(&v);
        unified.value_disunify.push(c);

        return Ok(());
    }

    pub fn disunify_items(&mut self, item1: StackItem, item2: StackItem) -> Result<(), Err> {
        return match (item1, item2) {
            (StackItem::Variable(v1), StackItem::Variable(v2)) => self.disunify_vars(v1, v2),
            (StackItem::Variable(v1), StackItem::Value(c2)) => self.disunify_var_value(v1, c2),
            (StackItem::Value(c1), StackItem::Variable(v2)) => self.disunify_var_value(v2, c1),
            (StackItem::Value(c1), StackItem::Value(c2)) => {
                match (c1.clone(), c2.clone()) {
                    (Value::Functor(name1, args1), Value::Functor(name2, args2)) => {
                        if name1 != name2 {
                            return Ok(());
                        }

                        for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                            self.disunify_items(arg1.clone(), arg2.clone())?;
                        }

                        return Ok(());
                    }

                    _ => {
                        if c1 != c2 {
                            Ok(())
                        } else {
                            Err::err_res(format!("Cannot disunify values '{}' and '{}'", c1, c2))
                        }
                    }
                }
            }
        };
    }

    pub fn disunify(&mut self) -> Result<(), Err> {
        let item1 = self.pop()?;
        let item2 = self.pop()?;

        return self.disunify_items(item1, item2);
    }
}
