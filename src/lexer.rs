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
use std::iter::FromIterator;

use crate::token::*;
use std::cmp::Ordering;

/// Lexer/mini-parser
pub struct Lexer {
    script: String,
    buffer: [char; 2],
    pos: usize,
    eof: bool,
    line: usize,
    col: usize,
    in_group: bool,
    pub preprocessor: bool,
}

impl Lexer {
    pub fn new(script: String, preprocessor: bool) -> Lexer {
        Lexer {
            script,
            buffer: [' ', ' '],
            pos: 0,
            eof: false,
            line: 0,
            col: 0,
            in_group: false,
            preprocessor,
        }
    }

    pub fn next_token(&mut self) -> Token {
        if self.eof {
            return Token::end();
        }
        while !self.eof {
            if !self.in_group {
                match self.buffer[0] {
                    '@' => {
                        // phase
                        return Token::new(TokenKind::Phase(self.get_line_as_string()));
                    }
                    '?' => {
                        // conditional
                        todo!();
                        //return Token::new(TokenKind::Command(self.get_line_as_string()));
                    }
                    '!' => {
                        return self.get_execute_group();
                    }
                    '$' => {
                        return self.get_variable();
                    }
                    ':' => {
                        // command
                        return Token::new(TokenKind::Command(self.get_line_as_string()));
                    }
                    '#' => {
                        // comment
                        while self.buffer[0] != '\n' && !self.eof {
                            self.eat();
                        }
                    }
                    '[' => {
                        let group = self.get_group_definition();
                        if self.preprocessor {
                            return group;
                        } // else keep eating
                    }
                    '\r' | '\n' | ' ' | '\t' => (),
                    t => unimplemented!("char {} not a known token", t),
                }
            } else {
                match self.buffer[0] {
                    '@' => {
                        // phase
                        return Token::new(TokenKind::Phase(self.get_line_as_string()));
                    }
                    '?' => {
                        // conditional
                        todo!();
                        //return Token::new(TokenKind::Command(self.get_line_as_string()));
                    }
                    '$' => {
                        return self.get_variable();
                    }
                    ':' => {
                        // command
                        return Token::new(TokenKind::Command(self.get_line_as_string()));
                    }
                    '#' => {
                        // comment
                        while self.buffer[0] != '\n' && !self.eof {
                            self.eat();
                        }
                    }
                    ']' => {
                        return Token::new(TokenKind::EndGroup);
                    }
                    '\r' | '\n' | ' ' | '\t' => (),
                    t => unimplemented!("char {} not a known token", t),
                }
            }
            self.eat();
        }
        Token::end()
    }

    fn get_group_definition(&mut self) -> Token {
        self.eat(); // [
        let name = self.get_ident();
        self.eat_whitespace_except_newlines();
        let mut args = Vec::new();
        if self.buffer[0] != '\n' {
            while self.buffer[0] == '$' {
                self.eat();
                let arg = self.get_ident();
                args.push(arg);
                self.eat_whitespace_except_newlines();
            }
        }
        self.in_group = true;
        let mut commands = Vec::new();
        loop {
            let command = self.next_token();
            if command.kind == TokenKind::EndGroup || self.eof {
                break;
            }
            commands.push(command);
        }
        self.in_group = false;

        if self.eof {
            panic!("Group {} did not have a closing bracket ']'", name);
        }

        self.eat(); // ]
        Token::new(TokenKind::GroupDefinition(GroupDefinition {
            name,
            args,
            commands,
        }))
    }

    fn get_execute_group(&mut self) -> Token {
        self.eat();
        let name = self.get_ident();
        self.eat_whitespace_except_newlines();
        let mut args = Vec::new();
        while self.buffer[0] != '\n' && !self.eof {
            self.eat_whitespace_except_newlines();
            if self.buffer[0] == '"' {
                args.push(self.get_string())
            } else if self.buffer[0] == '$' {
                self.eat();
                let var = self.get_ident();
                args.push(format!("${}", var));
            } else {
                let arg = self.get_until_whitespace();
                args.push(arg);
            }
        }
        Token::new(TokenKind::ExecuteGroup(name, args))
    }

    fn get_ident(&mut self) -> String {
        let mut ident = Vec::new();
        while matches!(self.buffer[0], 'A' ..= 'Z' | 'a' ..= 'z' | '0' ..= '9' | '-' | '_')
            && !self.eof
        {
            ident.push(self.buffer[0]);
            self.eat();
        }
        String::from_iter(ident)
    }

    fn get_variable(&mut self) -> Token {
        self.eat(); // $
        let ident = self.get_ident();
        while self.buffer[0] != '=' && self.buffer[0] != '?' && !self.eof {
            self.eat();
        }
        if self.eof {
            panic!("Variable ${} was never assigned a value", ident);
        }
        let if_not_set = self.buffer[0] == '?';
        if if_not_set {
            self.eat();
        }
        if self.buffer[0] != '=' {
            panic!("Variable ${} was never assigned a value", ident);
        }
        self.eat(); // =
        self.eat_whitespace();
        let value = if self.buffer[0] == '"' {
            self.get_string()
        } else {
            self.get_line_as_string()
        };
        if if_not_set {
            Token::new(TokenKind::VariableIfNotSet(ident, value))
        } else {
            Token::new(TokenKind::Variable(ident, value))
        }
    }

    fn get_string(&mut self) -> String {
        self.eat(); // "
        let mut sb = Vec::new();
        while self.buffer[0] != '"' && !self.eof {
            match (self.buffer[0], self.buffer[1]) {
                ('\\', '"') => {
                    sb.push('"');
                    self.eat();
                }
                (c, _) => {
                    sb.push(c);
                }
            }
            self.eat();
        }
        self.eat(); // "
        String::from_iter(sb)
    }

    fn get_line_as_string(&mut self) -> String {
        let mut sb = Vec::new();
        while self.buffer[0] != '\n' && !self.eof {
            match self.buffer[0] {
                '\\' => match self.buffer[1] {
                    'n' => {
                        sb.push('\n');
                        self.eat();
                        self.eat();
                    }
                    '$' => {
                        sb.push('\\');
                        sb.push('$');
                        self.eat();
                        self.eat();
                    }
                    _ => {
                        sb.push(self.buffer[0]);
                        self.eat();
                        self.eat();
                    }
                },
                '\r' => self.eat(),
                _ => {
                    sb.push(self.buffer[0]);
                    self.eat();
                }
            }
        }
        self.eat();
        String::from_iter(sb)
    }

    fn get_until_whitespace(&mut self) -> String {
        let mut sb = Vec::new();
        while !matches!(&self.buffer[0], '\t' | '\n' | '\r' | ' ') && !self.eof {
            sb.push(self.buffer[0]);
            self.eat();
        }
        String::from_iter(sb)
    }

    #[inline(always)]
    fn eat_whitespace_except_newlines(&mut self) {
        while matches!(&self.buffer[0], '\t' | '\r' | ' ') && !self.eof {
            self.eat();
        }
    }

    #[inline(always)]
    fn eat_whitespace(&mut self) {
        while matches!(&self.buffer[0], '\t' | '\n' | '\r' | ' ') && !self.eof {
            self.eat();
        }
    }

    fn eat(&mut self) {
        if !self.eof && self.buffer[0] == '\n' {
            self.line += 1;
            self.col = 0;
        }
        self.buffer[0] = self.buffer[1];

        match self.pos.cmp(&self.script.len()) {
            Ordering::Equal => {
                self.buffer[1] = '\n';
                self.pos += 1;
            }
            Ordering::Greater => {
                self.eof = true;
            }
            Ordering::Less => {
                self.buffer[1] = self.script.as_bytes()[self.pos] as char;
                self.pos += 1;
                self.col += 1;
            }
        }
    }
}
