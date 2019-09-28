extern crate num_bigint;

mod stackitem;
mod unification;
mod err;
mod enkienv;

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

use num_bigint::BigInt;

use enkienv::Environment;
use err::Err;
use stackitem::{StackItem, Value};

#[derive(Clone, Debug)]
pub enum Instr {
    Int(BigInt),
    Var(String),
    Str(String),
    Goto,
    GotoChoice,
    Unify,
    Dup,
    Disunify,
    Pop,
    NameOf,
    Project,
    Functor,
    Swap
}

fn execute(instrs: Vec<Instr>) -> Result<(), Err> {
    let mut env = Environment::new();

    let mut i = 0;

    loop {
        let instr = instrs[i].clone();

        i += 1;

        let result = match instr {
            Instr::Var(var_name) => env.push(StackItem::Variable(var_name)),
            Instr::Int(i) => env.push(StackItem::Value(Value::IntValue(i))),
            Instr::Str(s) => env.push(StackItem::Value(Value::StringValue(s))),
            Instr::Unify   => env.unify(),
            Instr::Disunify => env.disunify(),
            Instr::Pop     => env.pop().map(|_x| ()), // Drop the returned item because we don't need it here
            Instr::Dup     => env.dup(),
            Instr::Project => env.project(),
            Instr::NameOf  => env.nameof(),
            Instr::Functor => env.functor(),
            Instr::Swap    => env.swap(),
            Instr::Goto => {
                match env.popidx() {
                    Ok(idx) => {
                        i = idx;
                        Ok(()) // TODO: Should it be an error if i >= instrs.len()?
                    }
                    Err(err) => Err(err)
                }
            },
            Instr::GotoChoice => { // This adds a choicepoint. If we fail, we'll jump to the location indicated by idx at the top of the stack
                match env.popidx() {
                    Ok(idx) => {
                        env.choicepoint = Some((idx, Box::new(env.clone())));
                        Ok(())
                    }
                    Err(err) => Err(err)
                }
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

fn load_instrs(filename: String) -> Vec<Instr> {
    let file = File::open(filename).unwrap(); // TODO: Handle this better
    let reader = BufReader::new(file);

    let mut instrs = Vec::new();

    let mut locations = HashMap::new();

    let mut deferred_jumps = Vec::new();

    for line in reader.lines() {
        let line_str = line.unwrap();
        let split: Vec<&str> = line_str.split(" ").collect();
        let opcode = split[0].to_string();

        if opcode == "var" {
            instrs.push(Instr::Var(split[1].to_string()));
        } else if opcode == "int" {
            instrs.push(Instr::Int(BigInt::parse_bytes(split[1].as_bytes(), 10).unwrap()));
        } else if opcode == "str" {
            instrs.push(Instr::Str(split[1].to_string()));
        } else if opcode == "goto" {
            instrs.push(Instr::Goto);
        } else if opcode == "gotochoice" {
            instrs.push(Instr::GotoChoice);
        } else if opcode == "functor" {
            instrs.push(Instr::Functor);
        } else if opcode == "unify" {
            instrs.push(Instr::Unify);
        } else if opcode == "pop" {
            instrs.push(Instr::Pop);
        } else if opcode == "dup" {
            instrs.push(Instr::Dup);
        } else if opcode == "disunify" {
            instrs.push(Instr::Disunify);
        } else if opcode == "project" {
            instrs.push(Instr::Project);
        } else if opcode == "nameof" {
            instrs.push(Instr::NameOf);
        } else if opcode == "label" {
            locations.insert(split[1].to_string(), instrs.len() + deferred_jumps.len() * 2);
        } else if opcode == "jump" {
            match locations.get(&split[1].to_string()) {
                Some(idx) => {
                    instrs.push(Instr::Int(BigInt::from(*idx)));
                    instrs.push(Instr::Goto);
                },
                None => {
                    deferred_jumps.push((instrs.len() + deferred_jumps.len() * 2, split[1].to_string()));
                }
            }
        }
    }

    for (insert_pos, label_name) in deferred_jumps {
        match locations.get(&label_name) {
            Some(idx) => {
                instrs.insert(insert_pos, Instr::Goto);
                instrs.insert(insert_pos, Instr::Int(BigInt::from(*idx)));
            }
            None => println!("Unknown label: {}", label_name)
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
