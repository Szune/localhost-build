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
use std::collections::HashMap;
use std::iter::FromIterator;

pub fn get_line_strings(input: String) -> Vec<String> {
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

pub fn get_cache_line(kvp: (&String, &u32)) -> String {
    let file: String = kvp.0.trim_matches('"').to_string();
    let crc: &u32 = kvp.1;
    format!(r#""{}" "{}""#, file, crc)
}

pub fn parse_cache(cache: String) -> HashMap<String, u32> {
    HashMap::from_iter(
        cache
            .lines()
            .map(String::from)
            .map(|l| {
                let mut strings: Vec<String> = get_line_strings(l)
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect();
                assert_eq!(strings.len(), 2);
                (strings.remove(0), strings.remove(0).parse::<u32>().unwrap())
            })
            .collect::<Vec<(String, u32)>>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn space_between_two_strings_should_not_be_its_own_string() {
        let strings = get_line_strings(r#""string1" "string2""#.to_string() + "\n");
        assert_eq!(strings.len(), 2);
    }

    #[test]
    pub fn two_strings() {
        let strings = get_line_strings(r#""string1" "string2""#.to_string() + "\n");
        assert_eq!(strings[0], "string1");
        assert_eq!(strings[1], "string2");
    }

    #[test]
    pub fn get_cache_line_as_expected() {
        let cache = get_cache_line((&"bin/test.txt".to_string(), &!0u32));
        assert_eq!(cache, r#""bin/test.txt" "4294967295""#.to_string())
    }

    #[test]
    pub fn get_cache_line_into_get_line_strings_as_expected() {
        let cache = get_cache_line((&"bin/test.txt".to_string(), &!0u32));
        let strings = get_line_strings(cache);
        assert_eq!(strings[0], "bin/test.txt");
        assert_eq!(strings[1], "4294967295");
    }
}
