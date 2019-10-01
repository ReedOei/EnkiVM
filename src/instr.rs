use std::collections::HashMap;

use num_bigint::BigInt;

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
    Gte,
    Rot,
    Over,
    PrintStack,
    PrintUnification,
    Destroy
}

impl Instr {
    pub fn substitute(&mut self, subs_map: &HashMap<String, String>) {
        match self {
            Instr::Var(ref mut name) => {
                match subs_map.get(name) {
                    Some(new_name) => {
                        *name = new_name.to_string();
                    }

                    None => {}
                }
            }

            _ => {}
        }
    }
}

fn escape_str(s: &String) -> String {
    return s.replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
            .replace("\"", "\\\"");
}

impl std::fmt::Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Instr::Int(i) => write!(f, "int {}", i.to_str_radix(10)),
            Instr::Var(name) => write!(f, "var {}", name),
            Instr::Str(s) => write!(f, "str \"{}\"", escape_str(s)),
            Instr::Goto => write!(f, "goto"),
            Instr::Fail => write!(f, "fail"),
            Instr::Print => write!(f, "print"),
            Instr::Fresh => write!(f, "fresh"),
            Instr::GotoChoice => write!(f, "gotochoice"),
            Instr::Unify => write!(f, "unify"),
            Instr::Dup => write!(f, "dup"),
            Instr::Disunify => write!(f, "disunify"),
            Instr::Pop => write!(f, "pop"),
            Instr::NameOf => write!(f, "nameof"),
            Instr::Project => write!(f, "project"),
            Instr::Functor => write!(f, "functor"),
            Instr::Swap => write!(f, "swap"),
            Instr::Add => write!(f, "add"),
            Instr::Sub => write!(f, "sub"),
            Instr::Div => write!(f, "div"),
            Instr::Mul => write!(f, "mul"),
            Instr::Pow => write!(f, "pow"),
            Instr::Lt => write!(f, "lt"),
            Instr::Gt => write!(f, "gt"),
            Instr::Lte => write!(f, "lte"),
            Instr::Gte => write!(f, "gte"),
            Instr::Rot => write!(f, "rot"),
            Instr::Over => write!(f, "over"),
            Instr::PrintStack => write!(f, "printstack"),
            Instr::PrintUnification => write!(f, "printunification"),
            Instr::Destroy => write!(f, "destroy")
        }
    }
}
