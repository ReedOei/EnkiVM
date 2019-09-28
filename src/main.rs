extern crate num_bigint;

mod r#const;
mod unification;

use std::collections::VecDeque;
use std::collections::HashMap;
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

    fn unify_vars(&mut self, v1: String, v2: String) -> Result<(), Err> {
        if v1 != v2 {
            let unified1 = self.access_unified(&v1);

            if !unified1.do_unify(&v2) {
                return Err(Err { msg: format!("Could not unify '{}' and '{}'", v1, v2) });
            }

            let unified2 = self.access_unified(&v2);
            if !unified2.do_unify(&v1) {
                return Err(Err { msg: format!("Could not unify '{}' and '{}'", v1, v2) });
            }
        }

        return Ok(());
    }

    fn unify_var_const(&mut self, v: String, c: Const) -> Result<(), Err> {
        let unified = self.access_unified(&v);

        if !unified.do_unify_const(&c) {
            return Err(Err { msg: format!("Could not unify '{}' and '{}'", v, c) });
        }

        let vars = unified.var_unify_clone();
        for var in vars {
            let var_unification = self.access_unified(&var);

            if !var_unification.do_unify_const(&c) {
                return Err(Err { msg: format!("Could not unify '{}' and '{}'", v, c) });
            }
        }

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

    fn disunify_vars(&mut self, v1: String, v2: String) -> Result<(), Err> {
        if v1 != v2 {
            let unified1 = self.access_unified(&v1);

            if !unified1.do_disunify(&v2) {
                return Err(Err { msg: format!("Could not disunify '{}' and '{}'", v1, v2) });
            }

            let unified2 = self.access_unified(&v2);
            if !unified2.do_disunify(&v1) {
                return Err(Err { msg: format!("Could not disunify '{}' and '{}'", v1, v2) });
            }
        }

        return Ok(());
    }

    fn disunify_var_const(&mut self, v: String, c: Const) -> Result<(), Err> {
        let unified = self.access_unified(&v);

        if !unified.do_disunify_const(&c) {
            return Err(Err { msg: format!("Could not disunify '{}' and '{}'", v, c) });
        }

        let vars = unified.var_unify_clone();
        for var in vars {
            let var_unification = self.access_unified(&var);

            if !var_unification.do_disunify_const(&c) {
                return Err(Err { msg: format!("Could not disunify '{}' and '{}'", v, c) });
            }
        }

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
