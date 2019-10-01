extern crate clap;
extern crate num_bigint;
extern crate num_traits;

mod err;
mod enkienv;
mod instr;
mod macrolang;
mod stackitem;
mod unification;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use clap::{Arg, App};

use num_bigint::BigInt;

use enkienv::Environment;
use err::Err;
use stackitem::{StackItem, Value};
use instr::Instr;
use macrolang::{MacroInstr, MacroStmt, MacroProgram};

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
            Instr::Destroy => env.destroy(),
            Instr::Add  => env.add(),
            Instr::Sub => env.sub(),
            Instr::Mul => env.mul(),
            Instr::Div => env.div(),
            Instr::Pow => env.pow(),
            Instr::Lt => env.lt(),
            Instr::Gt => env.gt(),
            Instr::Lte => env.lte(),
            Instr::Gte => env.gte(),
            Instr::Rot => env.rot(),
            Instr::Over => env.over(),
            Instr::PrintStack => env.print_stack(),
            Instr::PrintUnification => env.print_unification(),
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

fn process_str_const(s: &String) -> Option<String> {
    let temp_str =
        s.replace("\\n", "\n")
           .replace("\\t", "\t")
           .replace("\\r", "\r")
           .replace("\\\"", "\"");

    let start_pos = temp_str.find("\"")?;
    let end_pos = temp_str.rfind("\"")?;

    return Some((&temp_str[start_pos + 1..end_pos]).to_string());
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

        match parse_macro_instr(&line_str) {
            Some(MacroInstr::Lit(instr)) => {
                instrs.push(instr);
            }

            Some(MacroInstr::Quote(_split)) => {
                error = true;
                println!("Quote not allowed in .envm files!");
            }

            Some(MacroInstr::Label(label_name)) => {
                locations.insert(label_name, instrs.len() + positions.len());
            }

            Some(MacroInstr::Position(label_name)) => {
                positions.push((instrs.len() + positions.len(), label_name));
            }

            Some(MacroInstr::Noop) => {}

            None => {
                error = true;
            }
        }
    }

    // Insert all position informations. This has to be done after because we can reference labels before
    // we define them
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

fn parse_macro_instr(line_str: &String) -> Option<MacroInstr> {
    let split: Vec<&str> = line_str.split(" ").collect();
    let opcode = split[0].to_string();

    if opcode == "var" {
        return Some(MacroInstr::Lit(Instr::Var(split[1].to_string())));
    } else if opcode == "int" {
        let big_int = BigInt::parse_bytes(split[1].as_bytes(), 10).unwrap();
        return Some(MacroInstr::Lit(Instr::Int(big_int)));
    } else if opcode == "str" {
        let str_const_opt = process_str_const(&(line_str["str".len() + 1..]).to_string());

        match str_const_opt {
            Some(str_const) => {
                return Some(MacroInstr::Lit(Instr::Str(str_const)));
            }

            None => {
                println!("Could not parse string constant in: '{}'", line_str);
                return None;
            }
        }
    } else if opcode == "goto" {
        return Some(MacroInstr::Lit(Instr::Goto));
    } else if opcode == "gotochoice" {
        return Some(MacroInstr::Lit(Instr::GotoChoice));
    } else if opcode == "functor" {
        return Some(MacroInstr::Lit(Instr::Functor));
    } else if opcode == "unify" {
        return Some(MacroInstr::Lit(Instr::Unify));
    } else if opcode == "pop" {
        return Some(MacroInstr::Lit(Instr::Pop));
    } else if opcode == "dup" {
        return Some(MacroInstr::Lit(Instr::Dup));
    } else if opcode == "disunify" {
        return Some(MacroInstr::Lit(Instr::Disunify));
    } else if opcode == "project" {
        return Some(MacroInstr::Lit(Instr::Project));
    } else if opcode == "nameof" {
        return Some(MacroInstr::Lit(Instr::NameOf));
    } else if opcode.starts_with(":") {
        let label_name = (&opcode[1..]).to_string();
        return Some(MacroInstr::Label(label_name));
    } else if opcode == "position" {
        return Some(MacroInstr::Position(split[1].to_string()));
    } else if opcode == "fresh" {
        return Some(MacroInstr::Lit(Instr::Fresh));
    } else if opcode == "print" {
        return Some(MacroInstr::Lit(Instr::Print));
    } else if opcode == "" {
        // Ignore blank lines
        return Some(MacroInstr::Noop);
    } else if opcode == "#" {
        // Ignore comments
        return Some(MacroInstr::Noop);
    } else if opcode == "fail" {
        return Some(MacroInstr::Lit(Instr::Fail));
    } else if opcode == "add" {
        return Some(MacroInstr::Lit(Instr::Add));
    } else if opcode == "sub" {
        return Some(MacroInstr::Lit(Instr::Sub));
    } else if opcode == "mul" {
        return Some(MacroInstr::Lit(Instr::Mul));
    } else if opcode == "div" {
        return Some(MacroInstr::Lit(Instr::Div));
    } else if opcode == "pow" {
        return Some(MacroInstr::Lit(Instr::Pow));
    } else if opcode == "lt" {
        return Some(MacroInstr::Lit(Instr::Lt));
    } else if opcode == "gt" {
        return Some(MacroInstr::Lit(Instr::Gt));
    } else if opcode == "lte" {
        return Some(MacroInstr::Lit(Instr::Lte));
    } else if opcode == "gte" {
        return Some(MacroInstr::Lit(Instr::Gte));
    } else if opcode == "rot" {
        return Some(MacroInstr::Lit(Instr::Rot));
    } else if opcode == "over" {
        return Some(MacroInstr::Lit(Instr::Over));
    } else if opcode == "swap" {
        return Some(MacroInstr::Lit(Instr::Swap));
    } else if opcode == "printstack" {
        return Some(MacroInstr::Lit(Instr::PrintStack));
    } else if opcode == "printunification" {
        return Some(MacroInstr::Lit(Instr::PrintUnification));
    } else if opcode == "destroy" {
        return Some(MacroInstr::Lit(Instr::Destroy));
    } else if opcode == "quote" {
        return Some(MacroInstr::Quote((&split[1..]).iter().map(|x| x.to_string()).collect()));
    } else {
        println!("Unknown opcode '{}' in: '{}'", opcode, line_str);
        return None;
    }
}

fn load_macro_stmts(filepath: String) -> Option<MacroProgram> {
    let mut stmts = Vec::new();

    let mut error = false;

    let file = File::open(filepath).unwrap(); // TODO: Handle this better
    let reader = BufReader::new(file);

    let mut macro_name = "".to_string();
    let mut macro_args = Vec::new();
    let mut macro_stmts = Vec::new();
    let mut in_macro = false;

    let mut in_call = false;
    let mut call_instrs = Vec::new();
    let mut call_name = "".to_string();

    for line in reader.lines() {
        let line_str = line.unwrap();
        let split: Vec<&str> = line_str.split(" ").collect();
        let command = split[0].to_string();

        if command == "macro" {
            macro_name = split[1].to_string();

            macro_args = Vec::new();

            for arg in &split[2..] {
                macro_args.push(arg.to_string());
            }

            in_macro = true;
        } else if command == "endmacro" {
            if in_macro {
                in_macro = false;

                let temp_name = macro_name;
                macro_name = "".to_string();
                let temp_args = macro_args;
                macro_args = Vec::new();
                let temp_stmts = macro_stmts;
                macro_stmts = Vec::new();

                stmts.push(MacroStmt::Macro(temp_name, temp_args, temp_stmts));
            } else {
                error = true;
                println!("Unmatched endmacro!");
            }
        } else if command.starts_with("$") {
            let name = (&command[1..]).to_string();

            let mut args = Vec::new();

            for arg in &split[1..] {
                args.push(arg.to_string());
            }

            if in_macro {
                macro_stmts.push(MacroStmt::CallMacro(name, args));
            } else {
                stmts.push(MacroStmt::CallMacro(name, args));
            }
        } else if command == "call" {
            in_call = true;
            call_name = split[1].to_string();
        } else if command == "endcall" {
            if in_call {
                in_call = false;

                let temp_name = call_name;
                call_name = "".to_string();
                let temp_body = call_instrs;
                call_instrs = Vec::new();

                let call_stmt = MacroStmt::Call(temp_name, temp_body);

                if in_macro {
                    macro_stmts.push(call_stmt);
                } else {
                    stmts.push(call_stmt);
                }
            } else {
                error = true;
                println!("Unmatched endcall!")
            }
        } else {
            match parse_macro_instr(&line_str) {
                Some(MacroInstr::Noop) => {}

                Some(instr) => {
                    if in_call {
                        call_instrs.push(instr);
                    } else if in_macro {
                        macro_stmts.push(MacroStmt::Simple(instr));
                    } else {
                        stmts.push(MacroStmt::Simple(instr));
                    }
                }

                None => {
                    error = true;
                }
            }
        }
    }

    if error {
        return None;
    } else {
        return Some(MacroProgram::new(stmts));
    }
}

fn run_macro_envm_file(debug: bool, filepath: String) {
    match load_macro_stmts(filepath) {
        None => {
            println!("Exited due to parsing errors.");
        }

        Some(macro_prog) => {
            if debug {
                println!("Parsed program: ");
                println!("{:?}", macro_prog);
                println!();
            }

            let expanded = macro_prog.execute();

            if debug {
                println!("Expanded program:");
                println!("{:?}", expanded);
            }

            match expanded {
                Ok(res) => {
                    for instr in res {
                        println!("{}", instr);
                    }
                }

                Err(err) => {
                    println!("An error occurred during expansion: {}", err.msg_clone());
                }
            }
        }
    }
}

fn run_envm_file(debug: bool, filepath: String) {
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
            if filepath.ends_with(".menvm") {
                run_macro_envm_file(debug, filepath.to_string());
            } else {
                run_envm_file(debug, filepath.to_string());
            }
        None => {}
    }
}
