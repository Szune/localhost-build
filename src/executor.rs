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
use crate::lexer::Lexer;
use crate::preprocessor;
use crate::token::*;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::path::PathBuf;
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
}

impl Executor {
    pub fn new(script: String) -> Executor {
        let script = preprocessor::perform_imports(script);
        let mut executor = Executor {
            lexer: Lexer::new(script.clone(), false),
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
        };

        let preprocessor_lexer = Lexer::new(script, true);
        executor.groups = preprocessor::run(preprocessor_lexer);
        executor
    }

    fn replace_variable(&self, var: String) -> String {
        match var.as_str() {
            "stderr" => self.last_proc_err.clone(),
            "stdout" => self.last_proc_out.clone(),
            "exit-code" => self.last_proc_code.to_string(),
            "args" => Self::get_args(),
            c if self.executing_group_args.contains_key(c) => {
                self.executing_group_args.get(c).unwrap().clone()
            }
            c if self.variables.contains_key(c) => self.variables.get(c).unwrap().clone(),
            _ => var,
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
        while token.kind != TokenKind::EndOfText {
            //println!("Token: {:?}", token);
            match token.kind {
                TokenKind::Phase(ref s) => {
                    if let Some(goto) = &self.goto_phase {
                        if goto == s {
                            self.goto_phase = None;
                        } else {
                            token = self.lexer.next_token();
                            continue;
                        }
                    }
                    if self.announcing_phases {
                        println!("Starting phase {}", s);
                    }
                }
                TokenKind::ExecuteGroup(ref s, ref args) => {
                    if self.goto_phase.is_some() {
                        token = self.lexer.next_token();
                        continue;
                    }
                    let group = self
                        .groups
                        .get(s)
                        .unwrap_or_else(|| panic!("Group {} has not been defined anywhere", s))
                        .clone();
                    if self.execute_group(&group, args) {
                        return;
                    }
                }
                TokenKind::Command(ref s) => {
                    if self.goto_phase.is_some() {
                        token = self.lexer.next_token();
                        continue;
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
                        return;
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

    fn get_strings(&self, input: String) -> Vec<String> {
        let mut strings = Vec::new();
        let mut sb = Vec::new();
        let mut it = input.char_indices();
        let mut current = it.next();
        while current.is_some() {
            match current.unwrap().1 {
                '"' => {
                    if sb.iter().any(|c| !matches!(c, ' ' | '\r')) {
                        strings.push(String::from_iter(&sb));
                    }
                    sb.clear();
                }
                c => sb.push(c),
            }
            current = it.next();
        }
        if sb.iter().any(|c| !matches!(c, ' ' | '\r')) {
            strings.push(String::from_iter(&sb));
        }
        strings
    }

    fn get_execute_strings(input: String) -> Vec<String> {
        let mut strings = Vec::new();
        let mut sb = Vec::new();
        let mut it = input.chars();
        let mut buffer = ['\n', '\n'];
        let mut in_string = false;
        buffer[0] = it.next().unwrap_or('\n');
        buffer[1] = it.next().unwrap_or('\n');

        loop {
            macro_rules! eat(
                () => {
                    if let Some(current) = it.next() {
                        buffer[0] = buffer[1];
                        buffer[1] = current;
                    } else {
                        buffer[0] = buffer[1];
                        buffer[1] = '\n';
                        if buffer[0] == '\n' && buffer[1] == '\n' {
                            break;
                        }
                    }
                }
            );

            match (buffer[0], buffer[1], in_string) {
                (' ', _, false) => {
                    if !sb.is_empty() {
                        strings.push(String::from_iter(&sb));
                        sb.clear();
                    }
                }
                ('\\', '"', _) => {
                    eat!();
                    sb.push('"');
                }
                ('\\', c, _) => {
                    panic!("unknown escape char '{}' in string {:?}", c, input);
                }
                ('"', '"', false) => {
                    eat!();
                    strings.push(String::new());
                }
                ('"', '"', true) => {
                    panic!("unescaped quote in string {:?}", input);
                }
                ('"', _, true) => {
                    if !sb.is_empty() {
                        strings.push(String::from_iter(&sb));
                        sb.clear();
                    }
                    in_string = false;
                }
                ('"', _, false) => {
                    in_string = true;
                }
                (c, ..) => {
                    sb.push(c);
                }
            }

            eat!();
        }
        if !sb.is_empty() {
            strings.push(String::from_iter(&sb));
        }
        strings
    }

    /// return value is "should_quit"
    fn execute_command(&mut self, command: &str, input: String) -> bool {
        match command {
            ":l" => {
                println!("{}", input);
            }
            ":lt" => {
                if self.get_if_result(":lt") {
                    println!("{}", input);
                }
            }
            ":lf" => {
                if !self.get_if_result(":lf") {
                    println!("{}", input);
                }
            }
            ":ws" => {
                let seconds = input
                    .parse::<u64>()
                    .expect("expect seconds as argument in :ws");
                sleep(std::time::Duration::new(seconds, 0));
            }
            ":e" => {
                let mut parts = input.split(' ');
                let process = parts
                    .borrow_mut()
                    .take(1)
                    .next()
                    .expect("Command ':e' requires a process to execute");

                let args = parts
                    .borrow_mut()
                    .map(|p| p.to_owned())
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<String>>()
                    .join(" ");
                let args = Self::get_execute_strings(args);

                //println!("process: {:?}, args: {:?}", process, args);
                let result = std::process::Command::new(process)
                    .args(args)
                    .output()
                    .unwrap_or_else(|_| panic!("process failed to execute (:e {})", input));

                self.last_proc_err =
                    String::from_utf8(result.stderr).expect("stderr was not UTF-8");
                self.last_proc_out =
                    String::from_utf8(result.stdout).expect("stdout was not UTF-8");
                self.last_proc_code = result
                    .status
                    .code()
                    .expect("failed to retrieve exit code from process");
            }
            ":leo" => {
                if !self.last_proc_err.is_empty() {
                    println!("{}", self.last_proc_err);
                }
                if !self.last_proc_out.is_empty() {
                    println!("{}", self.last_proc_out);
                }
            }
            ":hasarg" => {
                if Self::get_args()
                    .split(' ')
                    .any(|ar| input.split(' ').any(|inp| ar == inp))
                {
                    self.add_if_result(true);
                } else {
                    self.add_if_result(false);
                }
            }
            ":hasvar" => {
                if self.variables.contains_key(&input) {
                    self.add_if_result(true);
                } else {
                    self.add_if_result(false);
                }
            }
            ":hv" | ":help" => {
                Self::print_help(true);
                return true;
            }
            ":h" => {
                Self::print_help(false);
                return true;
            }
            ":loe" => {
                if self.last_proc_code != 0 {
                    println!("{}", input);
                }
            }
            ":qoe" => {
                if self.last_proc_code != 0 {
                    return true;
                }
            }
            ":gotot" => {
                if self.get_if_result(":gotot") {
                    self.goto_phase = Some(input);
                }
            }
            ":goto" => {
                self.goto_phase = Some(input);
            }
            ":if" => {
                self.last_if_test_value = input;
            }
            ":eq" => {
                self.add_if_result(self.last_if_test_value == input);
            }
            ":neq" => {
                self.add_if_result(self.last_if_test_value != input);
            }
            ":contains" => {
                self.add_if_result(self.last_if_test_value.contains(&input));
            }
            ":not" => {
                let last_res = self.get_if_result(":not");
                self.add_if_result(!last_res);
            }
            ":and" => {
                self.awaiting_evaluation = Some(Evaluation::And);
            }
            ":or" => {
                self.awaiting_evaluation = Some(Evaluation::Or);
            }
            ":silent" => {
                self.announcing_phases = false;
            }
            ":sett" => {
                if self.get_if_result(":sett") {
                    let mut parts = input.split(' ');
                    let var = parts
                        .borrow_mut()
                        .take(1)
                        .next()
                        .expect("Command ':sett' requires a variable to set")
                        .to_string();
                    let value: String = parts
                        .borrow_mut()
                        .map(|p| p.to_owned())
                        .filter(|p| !p.is_empty())
                        .collect::<Vec<String>>()
                        .join(" ");

                    self.variables
                        .entry(var)
                        .and_modify(|v| *v = value.clone())
                        .or_insert_with(|| value);
                }
            }
            ":setf" => {
                if !self.get_if_result(":setf") {
                    let mut parts = input.split(' ');
                    let var = parts
                        .borrow_mut()
                        .take(1)
                        .next()
                        .expect("Command ':setf' requires a variable to set")
                        .to_string();
                    let value: String = parts
                        .borrow_mut()
                        .map(|p| p.to_owned())
                        .filter(|p| !p.is_empty())
                        .collect::<Vec<String>>()
                        .join(" ");

                    self.variables
                        .entry(var)
                        .and_modify(|v| *v = value.clone())
                        .or_insert_with(|| value);
                }
            }
            ":qt" => {
                if self.get_if_result(":qt") {
                    return true;
                }
            }
            ":qf" => {
                if !self.get_if_result(":qf") {
                    return true;
                }
            }
            ":q" => {
                return true;
            }
            ":los" => {
                if self.last_proc_code == 0 {
                    println!("{}", input);
                }
            }
            ":cpd" => {
                todo!();
                /*
                let parts = self.get_strings(input);
                let mut it = parts.iter();
                let mut source_path = PathBuf::from(
                it.next().expect("missing source name argument in :cpd"));
                let mut target_path = PathBuf::from(
                it.next().expect("missing target name argument in :cpd")
                );
                */
            }
            ":mv" => {
                let parts = self.get_strings(input);
                let mut it = parts.iter();
                let source = it.next().expect("missing source name argument in :mv");
                let target = it.next().expect("missing target name argument in :mv");
                std::fs::rename(source, target)
                    .unwrap_or_else(|_| panic!("failed to move from '{}' to '{}'", source, target));
            }
            ":cp" => {
                let parts = self.get_strings(input);
                let mut it = parts.iter();
                let source = it.next().expect("missing source argument in :cp");
                let target = it.next().expect("missing target argument in :cp");
                if let Ok(is_dir) = std::fs::metadata(source).map(|m| m.is_dir()) {
                    if is_dir {
                        panic!("'{}' is a directory, use :cpd to copy directories", source);
                    }
                }
                std::fs::copy(source, target).unwrap_or_else(|_| {
                    panic!("failed to copy file from '{}' to '{}'", source, target)
                });
            }
            c => unimplemented!("Command not found '{}'", c),
        };
        false
    }

    fn add_if_result(&mut self, value: bool) {
        if let Some(eval) = &self.awaiting_evaluation {
            match eval {
                Evaluation::And => {
                    let last_value = self
                        .last_if_result
                        .expect(":and requires a previous result to compare with");
                    self.last_if_result = Some(last_value && value);
                }
                Evaluation::Or => {
                    let last_value = self
                        .last_if_result
                        .expect(":or requires a previous result to compare with");
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
        if verbose {
            println!("{:<20}Example", "Command");
        }

        Self::help(verbose, ":l", "logs specified message", ":l hello world");
        Self::help(
            verbose,
            ":lt",
            "logs specified message if :if returned true",
            ":lt was true!",
        );
        Self::help(
            verbose,
            ":lf",
            "logs specified message if :if returned false",
            ":lf was false!",
        );
        Self::help(verbose, ":ws", "waits seconds", ":ws 1");
        Self::help(verbose, ":e", "executes process", ":e cargo build");
        Self::help(verbose, ":leo", "log stdout and stderr", ":leo");
        Self::help(
            verbose,
            ":hasarg",
            "sets last result to true if the specified argument(s) were passed to lb",
            ":hasarg test-only -t",
        );
        Self::help(
            verbose,
            ":hasvar",
            "sets last result to true if the specified variable has been set (do not use $ unless you need to)",
            ":hasvar test",
        );
        Self::help(
            verbose,
            ":loe",
            "logs specified message if last :e returned error code",
            ":loe process returned error: $stderr",
        );
        Self::help(
            verbose,
            ":los",
            "logs specified message if last :e returned success code",
            ":los process was successful: $stdout",
        );
        Self::help(
            verbose,
            ":qoe",
            "quits script if last :e returned error code",
            ":qoe",
        );
        Self::help(
            verbose,
            ":gotot",
            "goes to specified phase if last evaluation command returned true",
            ":gotot @build-only",
        );
        Self::help(verbose, ":goto", "goes to specified phase", ":goto @end");
        Self::help(
            verbose,
            ":if",
            "sets the value to be evaluated by the following command",
            ":if $args",
        );
        Self::help(
            verbose,
            ":eq",
            "compares the value in :if to the value specified in :eq (is equal)",
            ":eq hello",
        );
        Self::help(
            verbose,
            ":neq",
            "compares the value in :if to the value specified in :neq (is not equal)",
            ":neq hello",
        );
        Self::help(
            verbose,
            ":contains",
            "returns true if the value in :if contains the specified string",
            ":contains build-only",
        );
        Self::help(
            verbose,
            ":not",
            "negates the result of the last comparison",
            ":not",
        );
        Self::help(
            verbose,
            ":and",
            "returns true if the last result and the following result are true",
            ":and",
        );
        Self::help(
            verbose,
            ":or",
            "returns true if the last result or the following result are true",
            ":or",
        );
        Self::help(
            verbose,
            ":qt",
            "quits script if last :if returned true",
            ":qt",
        );
        Self::help(
            verbose,
            ":qf",
            "quits script if last :if returned false",
            ":qf",
        );
        Self::help(
            verbose,
            ":sett",
            "sets a variable if last :if returned true (do not prefix with $ unless you're setting the variable in the variable)",
            ":sett profile \"--release\"",
        );
        Self::help(
            verbose,
            ":setf",
            "sets a variable if last :if returned false (do not prefix with $ unless you're setting the variable in the variable)",
            ":setf profile \"\"",
        );
        Self::help(verbose, ":q", "quits script", ":q");
        Self::help(verbose, ":cpd", "(TODO) copies directory", "(TODO)");
        Self::help(
            verbose,
            ":mv",
            "moves the specified file",
            ":mv \"C:/1.txt\" \"C:/2.txt\"",
        );
        Self::help(
            verbose,
            ":cp",
            "copies the specified file",
            ":cp \"C:/1.txt\" \"C:/2.txt\"",
        );
        Self::help(
            verbose,
            ":silent",
            "stops printing \"Starting phase[...]\"",
            ":silent",
        );
        Self::help(verbose, ":hv", "shows verbose help", ":hv");
        Self::help(verbose, ":h", "shows minimal help", ":h");
    }

    fn help(verbose: bool, c: &str, h: &str, e: &str) {
        if verbose {
            println!("---------------------------");
            println!("{:<20}{}", c, e);
            println!("| {}", h);
        } else {
            println!("{:<20}{}", c, e);
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
        let strings = Executor::get_execute_strings("/c echo \"hello \\\"world\"".into());
        assert_eq!(
            strings,
            vec!["/c", "echo", "hello \"world"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        );
    }
}
