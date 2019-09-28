extern crate num_bigint;

mod r#const;
mod unification;

use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

use num_bigint::BigInt;

use unification::Unification;
use r#const::Const;

#[derive(PartialEq, Clone, Debug)]
pub enum StackItem {
    Variable(String),
    ConstItem(Const)
}

#[derive(Debug)]
pub struct Err {
    msg: String
}

#[derive(Clone, Debug)]
struct Environment {
    data: VecDeque<StackItem>,
    unified: HashMap<String, Unification>,
    choicepoint: Option<(usize, Box<Environment>)>,
    fresh_counter: usize
}

impl Environment {
    fn new() -> Environment {
        Environment {
            data: VecDeque::new(),
            unified: HashMap::new(),
            choicepoint: None,
            fresh_counter: 0
        }
    }

    fn push(&mut self, new_item: StackItem) -> Result<(), Err> {
        self.data.push_front(new_item);
        return Ok(());
    }

    fn pop(&mut self) -> Result<StackItem, Err> {
        return self.data.pop_front().ok_or(Err {
            msg: "No items on stack to pop".to_string()
        });
    }

    fn access_unified(&mut self, v: &String) -> &mut Unification {
        if !self.unified.contains_key(v) {
            self.unified.insert(v.to_string(), Unification::new());
        }

        return self.unified.get_mut(v).unwrap(); // We can safely unwrap here, since we know we put it in above
    }

    fn get_unified(&self, v: &String) -> Result<&Unification, Err> {
        return self.unified.get(v).ok_or(Err {msg: format!("Unification doesn't exist for {}", v) });
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
            return Err(Err { msg: format!("Could not unify '{}' and '{}'", v1, v2) });
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
            return Err(Err { msg: format!("Could not unify '{}' and '{}'", v, c) });
        }

        let unified = self.access_unified(&v);

        unified.const_unify = Some(c);

        return Ok(());
    }

    fn unify(&mut self) -> Result<(), Err> {
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
                    Err(Err { msg: format!("Cannot unify constants '{}' and '{}'", c1, c2) })
                }
            }
        };
    }

    fn disunify_with(&mut self, v1: &String, v2: &String) -> Result<(), Err> {
        if self.is_unified(v1, v2)? {
            return Err(Err { msg: format!("Could not disunify '{}' and '{}'", v1, v2) });
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
            return Err(Err { msg: format!("Could not unify '{}' and '{}'", v, c) });
        }

        let unified = self.access_unified(&v);
        unified.const_disunify.push(c);

        return Ok(());
    }

    fn disunify(&mut self) -> Result<(), Err> {
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
                    Err(Err { msg: format!("Cannot disunify constants '{}' and '{}'", c1, c2) })
                }
            }
        };
    }
}

#[derive(Clone, Debug)]
pub enum Instr {
    Push(StackItem),
    Goto(usize),
    GotoChoice(usize),
    Unify,
    Dup,
    Disunify,
    Pop
}

fn execute(instrs: Vec<Instr>) -> Result<(), Err> {
    let mut env = Environment::new();

    let mut i = 0;

    loop {
        let instr = instrs[i].clone();

        i += 1;

        let result = match instr {
            Instr::Push(v) => env.push(v),
            Instr::Unify   => env.unify(),
            Instr::Disunify => env.disunify(),
            Instr::Pop     => env.pop().map(|_x| ()), // Drop the returned item because we don't need it here
            Instr::Dup     => {
                match env.pop() {
                    Ok(item) => {
                        env.push(item.clone()).unwrap();
                        env.push(item).unwrap();
                        Ok(())
                    },
                    Err(err) => Err(err)
                }
            },
            Instr::Goto(idx) => {
                i = idx;
                Ok(()) // TODO: Should it be an error if i >= instrs.len()?
            },
            Instr::GotoChoice(idx) => { // This adds a choicepoint. If we fail, we'll jump to the location indicated by this idx
                env.choicepoint = Some((idx, Box::new(env.clone())));
                Ok(())
            }
        };

        if result.is_err() {
            match env.choicepoint {
                Some((idx, new_env)) => {
                    env.data = new_env.data;
                    env.unified = new_env.unified;
                    env.choicepoint = new_env.choicepoint;
                    i = idx;
                },
                None => return result
            }
        }

        if i >= instrs.len() {
            break;
        }
    }

    println!("Stack at end of program:");
    println!("{:?}", env.data);
    println!();

    println!("Unification state at end of program:");
    println!("{:?}", env.unified);
    println!();

    return Ok(());
}

fn is_numeric(s: &str) -> bool {
    for i in s.chars() {
        if '0' > i || i > '9' {
            return false;
        }
    }

    return true;
}

fn parse_stackitem(item: &str) -> StackItem {
    if is_numeric(item) {
        return StackItem::ConstItem(Const::IntConst(BigInt::parse_bytes(item.as_bytes(), 10).unwrap()));
    } else {
        return StackItem::Variable(item.to_string());
    }
}

fn load_instrs(filename: String) -> Vec<Instr> {
    let file = File::open(filename).unwrap(); // TODO: Handle this better
    let reader = BufReader::new(file);

    let mut instrs = Vec::new();

    for line in reader.lines() {
        let line_str = line.unwrap();
        let split: Vec<&str> = line_str.split(" ").collect();
        let opcode = split[0].to_string();

        if opcode == "push" {
            instrs.push(Instr::Push(parse_stackitem(split[1])));
        } else if opcode == "goto" {
            instrs.push(Instr::Goto(split[1].parse::<usize>().unwrap()));
        } else if opcode == "gotochoice" {
            instrs.push(Instr::GotoChoice(split[1].parse::<usize>().unwrap()));
        } else if opcode == "unify" {
            instrs.push(Instr::Unify);
        } else if opcode == "pop" {
            instrs.push(Instr::Pop);
        } else if opcode == "dup" {
            instrs.push(Instr::Dup);
        } else if opcode == "disunify" {
            instrs.push(Instr::Disunify);
        }
    }

    return instrs;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let instrs = load_instrs(args[1].clone());

    println!("Parsed program:");
    println!("{:?}", instrs);
    println!();

    match execute(instrs) {
        Ok(_) => {},
        Err(err) => {
            println!("{}", err.msg);
        }
    }
}
