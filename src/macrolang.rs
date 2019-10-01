use std::collections::HashMap;

use crate::err::Err;
use crate::instr::Instr;

#[derive(Clone, Debug)]
pub enum MacroInstr {
    Lit(Instr),
    Label(String),
    Position(String),
    Quote(Vec<String>),
    Noop
}


impl std::fmt::Display for MacroInstr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MacroInstr::Lit(instr) => write!(f, "{}", instr),
            MacroInstr::Label(label_name) => write!(f, ":{}", label_name),
            MacroInstr::Position(label_name) => write!(f, "position {}", label_name),
            MacroInstr::Quote(split) => write!(f, "{}", split.join(" ")),
            MacroInstr::Noop => write!(f, "")
        }
    }
}

fn lookup(v: &mut String, subs_map: &HashMap<String, String>) {
    match subs_map.get(v) {
        Some(new_val) => {
            *v = new_val.clone();
        }

        None => {}
    }
}

impl MacroInstr {
    pub fn substitute(&mut self, subs_map: &HashMap<String, String>) {
        match self {
            MacroInstr::Lit(instr) => instr.substitute(subs_map),
            MacroInstr::Label(ref mut name) => lookup(name, subs_map),
            MacroInstr::Position(ref mut label_name) => lookup(label_name, subs_map),
            MacroInstr::Quote(ref mut split) => {
                for arg in split.iter_mut() {
                    lookup(arg, subs_map);
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
pub enum MacroStmt {
    Simple(MacroInstr),
    Call(String, Vec<MacroInstr>),
    CallMacro(String, Vec<String>),
    Macro(String, Vec<String>, Vec<MacroStmt>)
}

impl MacroStmt {
    pub fn substitute(&mut self, subs_map: &HashMap<String, String>) {
        match self {
            MacroStmt::Simple(instr) => instr.substitute(subs_map),
            MacroStmt::Call(ref mut name, ref mut body) => {
                lookup(name, subs_map);

                for stmt in body.iter_mut() {
                    stmt.substitute(subs_map);
                }
            },

            MacroStmt::CallMacro(ref mut name, ref mut args) => {
                lookup(name, subs_map);

                for arg in args.iter_mut() {
                    lookup(arg, subs_map);
                }
            },

            MacroStmt::Macro(ref mut name, ref mut args, ref mut stmts) => {
                lookup(name, subs_map);

                for arg in args.iter_mut() {
                    lookup(arg, subs_map);
                }

                for stmt in stmts.iter_mut() {
                    stmt.substitute(subs_map);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct MacroProgram {
    statements: Vec<MacroStmt>
}

fn fresh_label(fresh_counter: usize) -> (usize, String) {
    return (fresh_counter + 1, format!("label_{}", fresh_counter));
}

fn make_subs_map(arg_names: Vec<String>, arg_vals: Vec<String>) -> HashMap<String, String> {
    let mut res = HashMap::new();

    for (arg_name, arg_val) in arg_names.iter().zip(arg_vals.iter()) {
        res.insert(arg_name.clone(), arg_val.clone());
    }

    return res;
}

fn from_simple(stmts: &Vec<MacroStmt>) -> Result<Option<Vec<MacroInstr>>, Err> {
    let mut res = Vec::new();

    for stmt in stmts {
        match stmt {
            MacroStmt::Simple(instr) => {
                res.push(instr.clone());
            }

            _ => {
                return Ok(None);
            }
        }
    }

    return Ok(Some(res));
}

impl MacroProgram {
    pub fn new(stmts: Vec<MacroStmt>) -> MacroProgram {
        MacroProgram {
            statements: stmts
        }
    }

    pub fn execute(&self) -> Result<Vec<MacroInstr>, Err> {
        let mut result = self.statements.clone();
        let mut new_result = Vec::new();

        let mut fresh_counter = 0;
        let mut macros = HashMap::new();

        loop {
            for stmt in result {
                match stmt {
                    MacroStmt::Simple(i) => {
                        new_result.push(MacroStmt::Simple(i));
                    }

                    MacroStmt::Macro(name, args, body) => {
                        macros.insert(name, (args, body));
                    }

                    MacroStmt::Call(label_name, body) => {
                        let (new_counter, new_label) = fresh_label(fresh_counter);
                        fresh_counter = new_counter;

                        new_result.push(MacroStmt::Simple(MacroInstr::Position(new_label.clone())));

                        for instr in body {
                            new_result.push(MacroStmt::Simple(instr.clone()));
                        }

                        new_result.push(MacroStmt::Simple(MacroInstr::Position(label_name.clone())));
                        new_result.push(MacroStmt::Simple(MacroInstr::Lit(Instr::Goto)));
                        new_result.push(MacroStmt::Simple(MacroInstr::Label(new_label)));
                    }

                    MacroStmt::CallMacro(macro_name, macro_args) => {
                        match macros.get(&macro_name) {
                            Some((macro_arg_names, macro_body)) => {
                                let subs_map = make_subs_map(macro_arg_names.to_vec(), macro_args.clone());

                                for stmt in macro_body.to_vec() {
                                    let mut new_stmt = stmt;
                                    new_stmt.substitute(&subs_map);
                                    new_result.push(new_stmt);
                                }
                            }

                            None => {
                                return Err::err_res(format!("No such macro defined yet: {}", macro_name));
                            }
                        }
                    }
                }
            }

            match from_simple(&new_result)? {
                Some(all_simple) => return Ok(all_simple),

                None => {
                    result = new_result;
                    new_result = Vec::new();
                }
            }
        }
    }
}
