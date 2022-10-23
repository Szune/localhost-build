/*
 * localhost-build is an experimental build scripting language
 * Copyright (C) 2021  Carl Erik Patrik Iwarson
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published
 * by the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
use crate::crc32::Crc32Table;
use crate::lexer::Lexer;
use crate::token::*;
use crate::{fs, preprocessor, str, table};
use std::collections::HashMap;
use std::iter::FromIterator;
use std::thread::sleep;

pub enum Evaluation {
    And,
    Or,
}

pub struct Executor {
    lexer: Lexer,
    last_proc_out: String,
    last_proc_err: String,
    last_proc_code: i32,
    last_if_result: Option<bool>,
    awaiting_evaluation: Option<Evaluation>,
    last_if_test_value: String,
    goto_phase: Option<String>,
    variables: HashMap<String, String>,
    groups: HashMap<String, GroupDefinition>,
    executing_group_args: HashMap<String, String>,
    announcing_phases: bool,
    cache: HashMap<String, u32>,
    crc_table: Crc32Table,
    table: table::Table,
}

const ARGTO: &str = ":argto";
const AND: &str = ":and";
const CONTAINS: &str = ":contains";
const CPDC: &str = ":cpdc";
const CPC: &str = ":cpc";
const CPD: &str = ":cpd";
const CD: &str = ":cd";
const CP: &str = ":cp";
const EMPTY: &str = ":empty";
const ENW: &str = ":enw";
const EP: &str = ":ep";
const EQ: &str = ":eq";
const E: &str = ":e";
const GOTOF: &str = ":gotof";
const GOTOT: &str = ":gotot";
const GOTO: &str = ":goto";
const HASARG: &str = ":hasarg";
const HASVAR: &str = ":hasvar";
const HV: &str = ":hv";
const H: &str = ":h";
const ISE: &str = ":ise";
const ISS: &str = ":iss";
const IF: &str = ":if";
const LEO: &str = ":leo";
const LOE: &str = ":loe";
const LOS: &str = ":los";
const LF: &str = ":lf";
const LT: &str = ":lt";
const L: &str = ":l";
const MVD: &str = ":mvd";
const MV: &str = ":mv";
const NEQ: &str = ":neq";
const NOT: &str = ":not";
const OR: &str = ":or";
const QEF: &str = ":qef";
const QET: &str = ":qet";
const QOEE: &str = ":qoee";
const QOE: &str = ":qoe";
const QE: &str = ":qe";
const QF: &str = ":qf";
const QT: &str = ":qt";
const Q: &str = ":q";
const SILENT: &str = ":silent";
const SETF: &str = ":setf";
const SETT: &str = ":sett";
const SET: &str = ":set";
const TB: &str = ":tb";
const TE: &str = ":te";
const TH: &str = ":th";
const TR: &str = ":tr";
const WC: &str = ":wc";
const WS: &str = ":ws";

impl Executor {
    pub fn new(script: String) -> Executor {
        let script = preprocessor::perform_imports(script);
        let mut executor = Executor {
            lexer: Lexer::new(script.clone().into(), false),
            last_proc_out: String::new(),
            last_proc_err: String::new(),
            last_proc_code: 0,
            last_if_result: None,
            awaiting_evaluation: None,
            last_if_test_value: "".into(),
            goto_phase: None,
            variables: HashMap::new(),
            groups: HashMap::new(),
            executing_group_args: HashMap::new(),
            announcing_phases: true,
            cache: HashMap::new(),
            crc_table: Crc32Table::default(),
            table: Default::default(),
        };

        let preprocessor_lexer = Lexer::new(script.into(), true);
        executor.groups = preprocessor::run(preprocessor_lexer);
        executor
    }

    pub fn with_cache(script: String, cache: HashMap<String, u32>) -> Executor {
        let script = preprocessor::perform_imports(script);
        let mut executor = Executor {
            lexer: Lexer::new(script.clone().into(), false),
            last_proc_out: String::new(),
            last_proc_err: String::new(),
            last_proc_code: 0,
            last_if_result: None,
            awaiting_evaluation: None,
            last_if_test_value: "".into(),
            goto_phase: None,
            variables: HashMap::new(),
            groups: HashMap::new(),
            executing_group_args: HashMap::new(),
            announcing_phases: true,
            cache,
            crc_table: Crc32Table::default(),
            table: Default::default(),
        };

        let preprocessor_lexer = Lexer::new(script.into(), true);
        executor.groups = preprocessor::run(preprocessor_lexer);
        executor
    }

    fn replace_variable(&self, var: String) -> String {
        match var.as_str() {
            "stderr" => self.last_proc_err.clone(),
            "stdout" => self.last_proc_out.clone(),
            "exit-code" => self.last_proc_code.to_string(),
            "pwd" => std::env::current_dir().unwrap().to_string_lossy().to_string(),
            "args" => Self::get_args(),
            c if self.executing_group_args.contains_key(c) => {
                self.executing_group_args.get(c).unwrap().clone()
            }
            c if self.variables.contains_key(c) => self.variables.get(c).unwrap().clone(),
            _ => "".into(),
        }
    }

    fn get_args() -> String {
        //let SKIP_AMOUNT = if self.file_name_from_args { 2 } else { 1 };
        const SKIP_AMOUNT: usize = 1;
        std::env::args_os()
            .skip(SKIP_AMOUNT)
            .map(|s| {
                s.into_string()
                    .expect("failed to convert OsString to String")
            })
            .collect::<Vec<String>>()
            .join(" ")
    }

    fn get_arg(arg_name: String) -> String {
        if !Self::get_args()
            .split(' ')
            .any(|ar| arg_name.split(' ').any(|inp| ar == inp))
        {
            return String::new();
        }
        let args = Self::get_args();
        let strings = str::get_line_strings(args);
        let strings_iter = &mut strings.iter();
        let idx = strings_iter.position(|arg| arg_name.split(' ').any(|a| a == arg));
        if let Some(idx) = idx {
            strings
                .into_iter()
                .skip(idx + 1)
                .take_while(|a| !a.starts_with('-'))
                .collect::<Vec<String>>()
                .join(" ")
        } else {
            String::new()
        }
    }

    fn interpret_string(&self, s: String) -> String {
        let mut sb = Vec::new();
        let mut it = s.chars();
        let mut current = it.next();
        while let Some(cur) = current {
            match cur {
                '\\' => {
                    current = it.next();
                    if let Some(next) = current {
                        if next == '$' {
                            sb.push('$');
                        } else {
                            sb.push(cur);
                            sb.push(next);
                        }
                    } else {
                        sb.push(cur);
                    }
                }
                '$' => {
                    current = it.next(); // skip $
                    let mut var = Vec::new();
                    while current.is_some()
                        && matches!(current.unwrap(), 'A' ..= 'Z' | 'a' ..= 'z' | '0' ..= '9' | '-' | '_')
                    {
                        var.push(current.unwrap());
                        current = it.next();
                    }
                    let full = String::from_iter(var);
                    let replaced = self.replace_variable(full);
                    for c in replaced.chars() {
                        sb.push(c);
                    }
                    continue;
                }
                c => sb.push(c),
            }
            current = it.next();
        }

        String::from_iter(sb)
    }

    pub fn execute(&mut self) {
        let mut token = self.lexer.next_token();
        'execute_loop: while token.kind != TokenKind::EndOfText {
            //println!("Token: {:?}", token);
            match token.kind {
                TokenKind::Phase(ref s) => {
                    if let Some(goto) = &self.goto_phase {
                        if goto == s {
                            self.goto_phase = None;
                        } else {
                            token = self.lexer.next_token();
                            continue 'execute_loop;
                        }
                    }
                    if self.announcing_phases {
                        println!("Starting phase {}", s);
                    }
                }
                TokenKind::ExecuteGroup(ref s, ref args) => {
                    if self.goto_phase.is_some() {
                        token = self.lexer.next_token();
                        continue 'execute_loop;
                    }
                    let group = self
                        .groups
                        .get(s)
                        .unwrap_or_else(|| panic!("Group {} has not been defined anywhere", s))
                        .clone();
                    if self.execute_group(&group, args) {
                        break 'execute_loop;
                    }
                }
                TokenKind::Command(ref s) => {
                    if self.goto_phase.is_some() {
                        token = self.lexer.next_token();
                        continue 'execute_loop;
                    }
                    let interp_str = self.interpret_string(s.clone());
                    let mut parts = interp_str.split(' ');
                    let command = parts.next().expect("command needs to be specified");

                    let mut sb = Vec::new();
                    let mut current = parts.next();
                    if current.is_some() {
                        sb.push(current.unwrap());
                        current = parts.next();
                    }
                    while current.is_some() {
                        sb.push(" ");
                        sb.push(current.unwrap());
                        current = parts.next();
                    }
                    let input = String::from_iter(sb);
                    if self.execute_command(command, input) {
                        break 'execute_loop;
                    }
                }
                TokenKind::Variable(var_name, value) => {
                    self.variables
                        .entry(var_name)
                        .and_modify(|v| *v = value.clone())
                        .or_insert_with(|| value);
                }
                TokenKind::VariableIfNotSet(var_name, value) => {
                    self.variables.entry(var_name).or_insert_with(|| value);
                }
                _ => unimplemented!(),
            }
            token = self.lexer.next_token();
        }

        if let Some(goto) = &self.goto_phase {
            println!("goto could not find phase '{}'", goto);
        }

        self.write_cache();
    }

    fn write_cache(&mut self) {
        if self.cache.is_empty() {
            return;
        }

        let cache = self
            .cache
            .iter()
            .map(str::get_cache_line)
            .collect::<Vec<String>>()
            .join("\n");

        std::fs::write("build.lb.cache", cache)
            .unwrap_or_else(|_| panic!("Failed to save build.lb.cache"));
    }

    fn execute_group(&mut self, group: &GroupDefinition, args: &[String]) -> bool {
        let args: Vec<String> = args
            .iter()
            .cloned()
            .map(|a| self.interpret_string(a))
            .collect();
        if args.len() < group.args.len() {
            panic!(
                "Tried to execute group {} with too few arguments ({}, expected {})",
                group.name,
                args.len(),
                group.args.len()
            );
        }
        self.executing_group_args.clear();
        for (i, arg) in group.args.iter().enumerate() {
            self.executing_group_args
                .insert(arg.clone(), args[i].clone());
        }

        for c in &group.commands {
            match c.kind {
                TokenKind::Command(ref s) => {
                    let interp_str = self.interpret_string(s.clone());
                    let mut parts = interp_str.split(' ');
                    let command = parts.next().expect("command needs to be specified");

                    let mut sb = Vec::new();
                    let mut current = parts.next();
                    if current.is_some() {
                        sb.push(current.unwrap());
                        current = parts.next();
                    }

                    while current.is_some() {
                        sb.push(" ");
                        sb.push(current.unwrap());
                        current = parts.next();
                    }

                    let input = String::from_iter(sb);
                    if self.execute_command(command, input) {
                        return true;
                    }
                }
                _ => unimplemented!(),
            }
        }
        self.executing_group_args.clear();
        false
    }

    fn get_execution_args(input: String) -> (String, Vec<String>) {
        //let (process, args) = str::separate_first_value_from_rest(input, EP).destructure();
        let args = str::get_line_strings(input);
        let mut iter = args.into_iter();
        let process = iter.next().expect("Requires a process to start");
        (process.to_owned(), iter.collect())
    }

    /// return value is "should_quit"
    fn execute_command(&mut self, command: &str, input: String) -> bool {
        match command {
            ARGTO => {
                let strings = str::get_line_strings(input);
                let mut strings = strings.into_iter();
                let arg = strings
                    .next()
                    .unwrap_or_else(|| panic!("'{}' requires an argument to get (arg 1)", ARGTO));
                let variable = strings
                    .next()
                    .unwrap_or_else(|| panic!("'{}' requires a variable to set (arg 2)", ARGTO));
                let arg_value = Self::get_arg(arg);
                self.variables
                    .entry(variable)
                    .and_modify(|v| *v = arg_value.clone())
                    .or_insert_with(|| arg_value);
            }
            AND => {
                self.awaiting_evaluation = Some(Evaluation::And);
            }
            CONTAINS => {
                self.add_if_result(self.last_if_test_value.contains(&input));
            }
            CD => {
                std::env::set_current_dir(&input)
                    .unwrap_or_else(|_| panic!("failed to set current dir to '{}'", input));
            }
            CPDC => {
                let fs_op = fs::get_source_and_target(input, CPDC);
                if let Ok(is_dir) = std::fs::metadata(&fs_op.source).map(|m| m.is_dir()) {
                    if !is_dir {
                        panic!(
                            "'{}' is not a directory, use {} to copy single files",
                            fs_op.source, CPC
                        );
                    }
                }
                fs::cached_copy_dir(&fs_op, &mut self.cache, &self.crc_table);
            }
            CPC => {
                let fs_op = fs::get_source_and_target(input, CPC);
                if let Ok(is_dir) = std::fs::metadata(&fs_op.source).map(|m| m.is_dir()) {
                    if is_dir {
                        panic!(
                            "'{}' is a directory, use {} to copy directories",
                            fs_op.source, CPD
                        );
                    }
                }

                fs::cached_copy(fs_op, &mut self.cache, &self.crc_table);
            }
            CPD => {
                let fs_op = fs::get_source_and_target(input, CPD);
                if let Ok(is_dir) = std::fs::metadata(&fs_op.source).map(|m| m.is_dir()) {
                    if !is_dir {
                        panic!(
                            "'{}' is not a directory, use {} to copy single files",
                            fs_op.source, CP
                        );
                    }
                }
                fs::copy_dir(&fs_op);
            }
            CP => {
                let fs_op = fs::get_source_and_target(input, CP);
                if let Ok(is_dir) = std::fs::metadata(&fs_op.source).map(|m| m.is_dir()) {
                    if is_dir {
                        panic!(
                            "'{}' is a directory, use {} to copy directories",
                            &fs_op.source, CPD
                        );
                    }
                }
                fs::copy(&fs_op);
            }
            EMPTY => {
                self.add_if_result(self.last_if_test_value.is_empty());
            }
            ENW => {
                // execute, no waiting
                let input_clone = input.clone();
                let (process, args) = Self::get_execution_args(input);

                std::process::Command::new(process)
                    .args(args)
                    .spawn()
                    .unwrap_or_else(|_| panic!("process failed to execute (:enw {})", input_clone));
            }
            EP => {
                let input_clone = input.clone();
                let (process, args) = Self::get_execution_args(input);

                //println!("process: {:?}, args: {:?}", process, args);
                let mut result = std::process::Command::new(process)
                    .args(args)
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .spawn()
                    .unwrap_or_else(|_| panic!("process failed to execute (:ep {})", input_clone));

                self.last_proc_code = result
                    .wait()
                    .unwrap_or_else(|_| {
                        panic!("failed to wait on process exit (:ep {})", input_clone)
                    })
                    .code()
                    .expect("failed to retrieve exit code from process");
            }
            EQ => {
                self.add_if_result(self.last_if_test_value == input);
            }
            E => {
                let input_clone = input.clone();
                let (process, args) = Self::get_execution_args(input);

                //println!("process: {:?}, args: {:?}", process, args);
                let result = std::process::Command::new(&process)
                    .args(&args)
                    .output()
                    .unwrap_or_else(|err| {
                        panic!(
                            "process failed to execute process '{}' with args '{:#?}':\n{}",
                            &process, &args, err
                        )
                    });

                self.last_proc_err =
                    String::from_utf8(result.stderr).expect("stderr was not UTF-8");
                self.last_proc_out =
                    String::from_utf8(result.stdout).expect("stdout was not UTF-8");
                self.last_proc_code = result.status.code().unwrap_or_else(|| {
                    panic!(
                        "failed to retrieve exit code from process when running :e {}",
                        input_clone
                    )
                });
            }
            GOTOF => {
                if !self.get_if_result(GOTOF) {
                    self.goto_phase = Some(input);
                }
            }
            GOTOT => {
                if self.get_if_result(GOTOT) {
                    self.goto_phase = Some(input);
                }
            }
            GOTO => {
                self.goto_phase = Some(input);
            }
            HASARG => {
                if Self::get_args()
                    .split(' ')
                    .any(|ar| input.split(' ').any(|inp| ar == inp))
                {
                    self.add_if_result(true);
                } else {
                    self.add_if_result(false);
                }
            }
            HASVAR => {
                if self.variables.contains_key(&input) {
                    self.add_if_result(true);
                } else {
                    self.add_if_result(false);
                }
            }
            HV | ":help" => {
                Self::print_help(true);
                return true;
            }
            H => {
                Self::print_help(false);
                return true;
            }
            ISE => {
                if self.last_proc_code != 0 {
                    self.add_if_result(true);
                } else {
                    self.add_if_result(false);
                }
            }
            ISS => {
                if self.last_proc_code == 0 {
                    self.add_if_result(true);
                } else {
                    self.add_if_result(false);
                }
            }
            IF => {
                self.last_if_test_value = input;
            }
            LEO => {
                // _l_og std_e_rr std_o_ut
                if !self.last_proc_err.is_empty() {
                    println!("{}", self.last_proc_err);
                }
                if !self.last_proc_out.is_empty() {
                    println!("{}", self.last_proc_out);
                }
            }
            LOE => {
                // log on error
                if self.last_proc_code != 0 {
                    println!("{}", input);
                }
            }
            LOS => {
                if self.last_proc_code == 0 {
                    println!("{}", input);
                }
            }
            LF => {
                if !self.get_if_result(LF) {
                    println!("{}", input);
                }
            }
            LT => {
                if self.get_if_result(LT) {
                    println!("{}", input);
                }
            }
            L => {
                println!("{}", input);
            }
            MVD => {
                todo!();
            }
            MV => {
                let fs_op = fs::get_source_and_target(input, MV);
                if let Ok(is_dir) = std::fs::metadata(&fs_op.source).map(|m| m.is_dir()) {
                    if is_dir {
                        panic!(
                            "'{}' is a directory, use {} to move directories",
                            &fs_op.source, MVD
                        );
                    }
                }

                fs::move_it(&fs_op);
            }
            NEQ => {
                self.add_if_result(self.last_if_test_value != input);
            }
            NOT => {
                let last_res = self.get_if_result(NOT);
                self.add_if_result(!last_res);
            }
            OR => {
                self.awaiting_evaluation = Some(Evaluation::Or);
            }
            QOEE => {
                if self.last_proc_code != 0 {
                    std::process::exit(1);
                }
            }
            QEF => {
                if !self.get_if_result(QEF) {
                    std::process::exit(1);
                }
            }
            QET => {
                if self.get_if_result(QET) {
                    std::process::exit(1);
                }
            }
            QOE => {
                if self.last_proc_code != 0 {
                    return true;
                }
            }
            QE => {
                std::process::exit(1);
            }
            QF => {
                if !self.get_if_result(QF) {
                    return true;
                }
            }
            QT => {
                if self.get_if_result(QT) {
                    return true;
                }
            }
            Q => {
                return true;
            }
            SILENT => {
                self.announcing_phases = false;
            }
            SETF => {
                if !self.get_if_result(SETF) {
                    let (first, rest) =
                        str::separate_first_value_from_rest(input, SETF).destructure();

                    self.variables
                        .entry(first)
                        .and_modify(|v| *v = rest.clone())
                        .or_insert_with(|| rest);
                }
            }
            SETT => {
                if self.get_if_result(SETT) {
                    let (first, rest) =
                        str::separate_first_value_from_rest(input, SETT).destructure();
                    self.variables
                        .entry(first)
                        .and_modify(|v| *v = rest.clone())
                        .or_insert_with(|| rest);
                }
            }
            SET => {
                let (first, rest) = str::separate_first_value_from_rest(input, SET).destructure();
                self.variables
                    .entry(first)
                    .and_modify(|v| *v = rest.clone())
                    .or_insert_with(|| rest);
            }
            TB => {
                // reset table
                let arguments = str::get_line_strings(input);
                let argument = arguments
                    .iter()
                    .filter(|a| a.chars().any(|c| !c.is_ascii_whitespace()))
                    .next();
                if let Some(arg) = argument {
                    let margin = arg.parse::<usize>();
                    match margin {
                        Ok(margin) => self.table = table::Table::new(margin),
                        Err(err) => panic!("Invalid argument to '{}':\n{}", TB, err),
                    }
                } else {
                    self.table = table::Table::new(5);
                }
            } // table begin
            TE => {
                self.table.print();
            } // table end
            TH => {
                self.table.set_headers(str::get_line_strings(input));
            } // table headers (headers separated by spaces or strings)
            TR => {
                self.table.add_row(str::get_line_strings(input));
            } // table row (cells separated by spaces or strings)
            WC => {
                self.write_cache();
            }
            WS => {
                let seconds = input
                    .parse::<u64>()
                    .unwrap_or_else(|_| panic!("expect seconds as argument in {}", WS));
                sleep(std::time::Duration::new(seconds, 0));
            }
            c => unimplemented!("Command not found '{}'", c),
        };
        false
    }

    fn add_if_result(&mut self, value: bool) {
        if let Some(eval) = &self.awaiting_evaluation {
            match eval {
                Evaluation::And => {
                    let last_value = self.last_if_result.unwrap_or_else(|| {
                        panic!("{} requires a previous result to compare with", AND)
                    });
                    self.last_if_result = Some(last_value && value);
                }
                Evaluation::Or => {
                    let last_value = self.last_if_result.unwrap_or_else(|| {
                        panic!("{} requires a previous result to compare with", OR)
                    });
                    self.last_if_result = Some(last_value || value);
                }
            }
            self.awaiting_evaluation = None;
        } else {
            self.last_if_result = Some(value);
        }
    }

    fn get_if_result(&self, command: &str) -> bool {
        self.last_if_result
            .unwrap_or_else(|| panic!("{} requires a bool result to compare with", command))
    }

    fn print_help(verbose: bool) {
        println!("localhost-build {}", env!("CARGO_PKG_VERSION"));
        println!();

        if verbose {
            println!("{:<20}Example", "Command");
        }

        Self::help(
            verbose,
            ARGTO,
            "gets the value of an argument and assigns a variable (sets to empty string if no value)",
            "\"-n\" variablename",
        );
        Self::help(
            verbose,
            AND,
            "returns true if the last result and the following result are true",
            "",
        );
        Self::help(
            verbose,
            CONTAINS,
            "returns true if the value in :if contains the specified string",
            "build-only",
        );
        Self::help(
            verbose,
            CPDC,
            "copies directory's files that have changed",
            "\"C:/1\" \"C:/2\"",
        );
        Self::help(
            verbose,
            CPC,
            "copy the specified file if it has changed since last copy",
            "\"C:/1.txt\" \"C:/2.txt\"",
        );
        Self::help(verbose, CPD, "copies directory", "\"C:/1\" \"C:/2\"");
        Self::help(
            verbose,
            CP,
            "copies the specified file",
            "\"C:/1.txt\" \"C:/2.txt\"",
        );
        Self::help(
            verbose,
            CD,
            "sets the current working directory",
            "test_dir",
        );
        Self::help(
            verbose,
            EMPTY,
            "sets last result to true if the last :if was empty",
            "",
        );
        Self::help(
            verbose,
            ENW,
            "executes process without waiting for the process to exit",
            "cargo build",
        );
        Self::help(
            verbose,
            EP,
            "executes process while inheriting stdout and stderr from lb.exe",
            "cargo build",
        );
        Self::help(
            verbose,
            EQ,
            "compares the value in :if to the value specified in :eq (is equal)",
            "hello",
        );
        Self::help(verbose, E, "executes process", "cargo build");
        Self::help(
            verbose,
            GOTOF,
            "goes to specified phase if last evaluation command returned false",
            "@build-only",
        );
        Self::help(
            verbose,
            GOTOT,
            "goes to specified phase if last evaluation command returned true",
            "@build-only",
        );
        Self::help(verbose, GOTO, "goes to specified phase", "@end");
        Self::help(
            verbose,
            HASARG,
            "sets last result to true if the specified argument(s) were passed to lb",
            "test-only -t",
        );
        Self::help(
            verbose,
            HASVAR,
            "sets last result to true if the specified variable has been set (do not use $ unless you need to)",
            "test",
        );
        Self::help(verbose, HV, "shows verbose help", "");
        Self::help(verbose, H, "shows minimal help", "");
        Self::help(
            verbose,
            ISE,
            "sets last result to true if the last process exited with an error exit code",
            "",
        );
        Self::help(
            verbose,
            ISS,
            "sets last result to true if the last process exited with a success exit code",
            "",
        );
        Self::help(
            verbose,
            IF,
            "sets the value to be evaluated by the following command",
            "$args",
        );
        Self::help(verbose, LEO, "log stdout and stderr", "");
        Self::help(
            verbose,
            LOE,
            "logs specified message if last :e returned error code",
            "process returned error: $stderr",
        );
        Self::help(
            verbose,
            LOS,
            "logs specified message if last :e returned success code",
            "process was successful: $stdout",
        );
        Self::help(
            verbose,
            LF,
            "logs specified message if :if returned false",
            "was false!",
        );
        Self::help(
            verbose,
            LT,
            "logs specified message if :if returned true",
            "was true!",
        );
        Self::help(verbose, L, "logs specified message", "hello world");
        Self::help(
            verbose,
            MVD,
            "(TODO) moves the specified directory",
            "(TODO)",
        );
        Self::help(
            verbose,
            MV,
            "moves the specified file",
            "\"C:/1.txt\" \"C:/2.txt\"",
        );
        Self::help(
            verbose,
            NEQ,
            "compares the value in :if to the value specified in :neq (is not equal)",
            "hello",
        );
        Self::help(
            verbose,
            NOT,
            "negates the result of the last comparison",
            "",
        );
        Self::help(
            verbose,
            OR,
            "returns true if the last result or the following result are true",
            "",
        );
        Self::help(
            verbose,
            QOEE,
            "quits script with exit code 1 (error) if last :e returned error code",
            "",
        );
        Self::help(
            verbose,
            QOE,
            "quits script if last :e returned error code",
            "",
        );
        Self::help(
            verbose,
            QEF,
            "quits script with exit code 1 (error) if last :if returned false",
            "",
        );
        Self::help(
            verbose,
            QET,
            "quits script with exit code 1 (error) if last :if returned true",
            "",
        );
        Self::help(verbose, QF, "quits script if last :if returned false", "");
        Self::help(verbose, QT, "quits script if last :if returned true", "");
        Self::help(verbose, QE, "quits script with exit code 1 (error)", "");
        Self::help(verbose, Q, "quits script", "");
        Self::help(
            verbose,
            SETF,
            "sets a variable if last :if returned false (do not prefix with $ unless you're setting the variable in the variable)",
            "profile \"\"",
        );
        Self::help(
            verbose,
            SETT,
            "sets a variable if last :if returned true (do not prefix with $ unless you're setting the variable in the variable)",
            "profile \"--release\"",
        );
        Self::help(
            verbose,
            SET,
            "sets a variable unconditionally (do not prefix with $ unless you're setting the variable in the variable)",
            "savewd $pwd",
        );
        Self::help(
            verbose,
            SILENT,
            "stops printing \"Starting phase[...]\"",
            "",
        );
        Self::help(verbose, TB, "start a new table", "");
        Self::help(verbose, TE, "ends and prints the table", "");
        Self::help(
            verbose,
            TH,
            "sets the headers of the table",
            "Header-1 Header-2",
        );
        Self::help(verbose, TR, "adds a row to the table", "Value-1 Value-2");
        Self::help(
            verbose,
            WC,
            "writes the cache before continuing, otherwise cache is written at normal script exit",
            "",
        );
        Self::help(verbose, WS, "waits seconds", "1");
    }

    /// `example` should not start with the command name, as it is added by this fn
    fn help(verbose: bool, command: &str, help_text: &str, example: &str) {
        if verbose {
            println!("---------------------------");
            println!("{:<20}{} {}", command, command, example);
            println!("| {}", help_text);
        } else {
            println!("{:<20}{} {}", command, command, example);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn groups_are_not_added_as_commands_to_script() {
        let script = r#"
        :l no such bug
        [test $var1
            :l found a bug!
        ]
        "#;

        let mut executor = Executor::new(script.into());
        assert!(matches!(
            executor.lexer.next_token().kind,
            TokenKind::Command(s) if s == ":l no such bug"
        ));
        assert!(matches!(
            executor.lexer.next_token().kind,
            TokenKind::EndOfText,
        ));
    }

    #[test]
    pub fn groups_are_added_as_groups() {
        let script = r#"
        [test $var1
            :l testing $var1
        ]
        "#;

        let executor = Executor::new(script.into());
        assert!(executor.groups.contains_key("test"));
        assert_eq!(executor.groups.get("test").unwrap().commands.len(), 1);
        assert!(
            matches!(executor.groups.get("test").unwrap().commands.first().unwrap().kind, TokenKind::Command(ref s) if s == ":l testing $var1")
        );
    }

    #[test]
    #[should_panic]
    pub fn calling_undefined_group() {
        let script = r#"
        !undefined 1
        "#;

        let mut executor = Executor::new(script.into());
        executor.execute();
    }

    #[test]
    pub fn calling_defined_group() {
        let script = r#"
        !test-1 world
        !test-1 carl
        !test-1 erik
        !test-1 patrik
        
        [test-1 $var-1
            :l hello $var-1
        ]
        "#;

        let mut executor = Executor::new(script.into());
        executor.execute();
    }

    #[test]
    pub fn logging_to_stdout() {
        let script = r#"
        :l hello world
        "#;

        let mut executor = Executor::new(script.into());
        executor.execute();
    }

    #[test]
    pub fn get_execute_strings() {
        let strings = str::get_line_strings("/c echo \"hello \\\"world\"".into());
        assert_eq!(
            strings,
            vec!["/c", "echo", "hello \"world"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );
    }
}
