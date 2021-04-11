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
use crate::token::{GroupDefinition, TokenKind};
use crate::tuple;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

tuple!(PreprocessedScript(script: String));

pub fn run(mut lexer: Lexer) -> HashMap<String, GroupDefinition> {
    if !lexer.preprocessor {
        // this is just an awful way to handle it, this should be handled by having different types
        // to handle it at compile time
        panic!("tried to use the regular lexer for preprocessor lexing");
    }
    let mut groups = HashMap::new();
    let mut token = lexer.next_token();
    while token.kind != TokenKind::EndOfText {
        //println!("preprocessoring token {:?}", &token.kind);
        if let TokenKind::GroupDefinition(group_def) = token.kind {
            groups.insert(group_def.name.clone(), group_def);
        }
        token = lexer.next_token();
    }

    groups
}

fn perform_imports_inner(script: String) -> String {
    let script_lines = script
        .lines()
        .map(|l| {
            if !(l.starts_with("&import(") && l.ends_with(')')) {
                l.to_string()
            } else {
                let l = l.get(
                    "&import(".len()..l.rfind(')')
                        .unwrap_or_else(|| {
                            panic!(
                                "Import line '{}' did not end with a closing parenthesis ')'",
                                l
                            )
                        }),
                )
                    .unwrap_or_else(|| panic!("Malformed import line '{}'", l));
                let dir_to_use = if l.starts_with("lblib/") {
                    let mut executable_path = match std::env::current_exe() {
                        Ok(path) => path,
                        Err(e) => {
                            panic!(
                                "Cannot use lblib import, failed to get path to lb.exe:\n{}",
                                e
                            )
                        }
                    };
                    if !executable_path.pop() {
                        panic!("Failed to get parent directory of directory containing lb.exe, cannot use lblib import");
                    }

                    executable_path.push(l);
                    executable_path.to_str()
                        .unwrap_or_else(|| panic!("Path to import was not valid UTF-8: {:?}", executable_path))
                        .to_string()
                } else {
                    l.to_string()
                };
                let mut buffer = String::new();
                File::open(dir_to_use)
                    .unwrap_or_else(|_| panic!("Failed to open {} for importing", l))
                    .read_to_string(&mut buffer)
                    .unwrap_or_else(|_| panic!("Failed to import file '{}'", l));
                buffer
            }
        })
        .collect::<Vec<String>>();

    let mut script: String = script_lines.join("\n");

    if script
        .lines()
        .any(|l| l.starts_with("&import(") && l.ends_with(')'))
    {
        script = perform_imports_inner(script);
    }

    script
}

pub fn perform_imports(script: String) -> PreprocessedScript {
    let script = perform_imports_inner(script);

    script.into()
}
