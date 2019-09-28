extern crate clap;
extern crate num_bigint;
extern crate num_traits;

mod stackitem;
mod unification;
mod err;
mod enkienv;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use clap::{Arg, App};

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
    Fail,
    Print,
    Fresh,
    GotoChoice,
    Unify,
    Dup,
    Disunify,
    Pop,
    NameOf,
    Project,
    Functor,
    Swap,
    Add,
    Sub,
    Div,
    Mul,
    Pow,
    Lt,
    Lte,
    Gt,
    Gte
}

fn execute(instrs: Vec<Instr>, debug: bool) -> Result<(), Err> {
    let mut env = Environment::new();

    let mut i = 0;

    loop {
        let instr = instrs[i].clone();

        i += 1;

        let result = match instr {
            Instr::Var(var_name) => env.push(StackItem::Variable(var_name)),
            Instr::Fresh => {
                let fresh_var_name = format!("T_{}", env.fresh_counter);
                env.fresh_counter += 1;
                env.push(StackItem::Variable(fresh_var_name))
            },
            Instr::Fail => Err::err_res("fail".to_string()),
            Instr::Print => env.print(),
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
            Instr::Add  => env.add(),
            Instr::Sub => env.sub(),
            Instr::Mul => env.mul(),
            Instr::Div => env.div(),
            Instr::Pow => env.pow(),
            Instr::Lt => env.lt(),
            Instr::Gt => env.gt(),
            Instr::Lte => env.lte(),
            Instr::Gte => env.gte(),
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

    if debug {
        println!("");
        println!("Stack at end of program:");
        println!("{:?}", env.data);
        println!();

        println!("Unification state at end of program:");
        println!("{:?}", env.unified);
        println!();
    }

    return Ok(());
}

fn process_str_const(str: &String) -> String {
    return str.replace("\\n", "\n")
              .replace("\\t", "\t")
              .replace("\\r", "\r");
}

fn load_instrs(filename: String) -> Option<Vec<Instr>> {
    let file = File::open(filename).unwrap(); // TODO: Handle this better
    let reader = BufReader::new(file);

    let mut instrs = Vec::new();

    let mut locations = HashMap::new();

    let mut positions = Vec::new();

    let mut error = false;

    for line in reader.lines() {
        let line_str = line.unwrap();
        let split: Vec<&str> = line_str.split(" ").collect();
        let opcode = split[0].to_string();

        if opcode == "var" {
            instrs.push(Instr::Var(split[1].to_string()));
        } else if opcode == "int" {
            instrs.push(Instr::Int(BigInt::parse_bytes(split[1].as_bytes(), 10).unwrap()));
        } else if opcode == "str" {
            let str_const = process_str_const(&(line_str["str".len() + 1..line_str.len()]).to_string());
            instrs.push(Instr::Str(str_const));
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
        } else if opcode.starts_with(":") {
            let label_name = (&opcode[1..opcode.len()]).to_string();
            locations.insert(label_name, instrs.len() + positions.len());
        } else if opcode == "position" {
            positions.push((instrs.len() + positions.len(), split[1].to_string()));
        } else if opcode == "fresh" {
            instrs.push(Instr::Fresh)
        } else if opcode == "print" {
            instrs.push(Instr::Print)
        } else if opcode == "" {
            // Ignore blank lines
        } else if opcode == "#" {
            // Ignore comments
        } else if opcode == "here" {
            instrs.push(Instr::Int(BigInt::from(instrs.len())));
        } else if opcode == "fail" {
            instrs.push(Instr::Fail);
        } else if opcode == "add" {
            instrs.push(Instr::Add);
        } else if opcode == "sub" {
            instrs.push(Instr::Sub);
        } else if opcode == "mul" {
            instrs.push(Instr::Mul);
        } else if opcode == "div" {
            instrs.push(Instr::Div);
        } else if opcode == "pow" {
            instrs.push(Instr::Pow);
        } else if opcode == "lt" {
            instrs.push(Instr::Lt);
        } else if opcode == "gt" {
            instrs.push(Instr::Gt);
        } else if opcode == "lte" {
            instrs.push(Instr::Lte);
        } else if opcode == "gte" {
            instrs.push(Instr::Gte);
        } else {
            println!("Unknown opcode '{}' in: '{}'", opcode, line_str);
            error = true;
        }
    }

    for (insert_pos, label_name) in positions {
        match locations.get(&label_name) {
            Some(idx) => {
                instrs.insert(insert_pos, Instr::Int(BigInt::from(*idx)));
            }

            None => {
                println!("Unknown label: {}", label_name);
                error = true;
            }
        }
    }

    if error {
        return None;
    } else {
        return Some(instrs);
    }
}

fn main() {
    let matches = App::new("EnkiVM")
        .version("0.1.0")
        .author("Reed Oei <reedoei2@illinois.edu>")
        .about("A VM for logic languages")
        .arg(Arg::with_name("debug")
                .long("debug")
                .help("Whether to print out additional debug information before/after execution"))
        .arg(Arg::with_name("file")
                .index(1)
                .help("The file containing code to execute"))
        .get_matches();

    let debug = matches.is_present("debug");

    match matches.value_of("file") {
        Some(filepath) =>
            match load_instrs(filepath.to_string()) {
                None => {
                    println!("Exited due to parsing errors.");
                }

                Some(instrs) => {
                    if debug {
                        println!("Parsed program:");
                        println!("{:?}", instrs);
                        println!();
                    }

                    match execute(instrs, debug) {
                        Ok(_) => {},
                        Err(err) => {
                            println!("{}", err.msg_clone());
                        }
                    }
                }
            }
        None => {}
    }
}
