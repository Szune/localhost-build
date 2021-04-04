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
pub mod crc32;
pub mod executor;
pub mod lexer;
pub mod preprocessor;
pub mod str_utils;
pub mod token;

fn main() {
    let script =
        std::fs::read_to_string("build.lb").unwrap_or_else(|_| panic!("couldn't read build.lb"));
    let cache = std::fs::read_to_string("build.lb.cache");

    let mut executor = if let Ok(cache) = cache {
        let cache = str_utils::parse_cache(cache);
        executor::Executor::with_cache(script, cache)
    } else {
        executor::Executor::new(script)
    };

    executor.execute();
}
