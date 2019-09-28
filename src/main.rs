extern crate num_bigint;

mod stackitem;
mod unification;
mod err;
mod enkienv;

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

use num_bigint::BigInt;

use enkienv::Environment;
use err::Err;
use stackitem::{StackItem, Const};

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
            Instr::Dup     => env.dup(),
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
            println!("{}", err.msg_clone());
        }
    }
}
