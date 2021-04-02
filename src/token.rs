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
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub kind: TokenKind,
}

impl Token {
    pub fn new(kind: TokenKind) -> Token {
        Token { kind }
    }

    pub fn none() -> Token {
        Token::new(TokenKind::None)
    }

    pub fn end() -> Token {
        Token::new(TokenKind::EndOfText)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GroupDefinition {
    pub name: String,
    pub args: Vec<String>,
    pub commands: Vec<Token>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    None,
    String(String),
    Variable(String, String),
    VariableIfNotSet(String, String),
    Command(String),
    Phase(String),
    ExecuteGroup(String, Vec<String>),
    EndGroup,
    GroupDefinition(GroupDefinition),
    EndOfText,
}
