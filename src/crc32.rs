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

pub struct Crc32Table(Vec<u32>);
impl Default for Crc32Table {
    fn default() -> Self {
        Crc32Table(
            (0u32..256)
                .map(|i| {
                    (0..8).fold(i, |acc, _| {
                        if acc & 1 != 0 {
                            0xEDB88320 ^ (acc >> 1)
                        } else {
                            acc >> 1
                        }
                    })
                })
                .collect::<Vec<u32>>(),
        )
    }
}

impl Crc32Table {
    #[inline(always)]
    pub fn calculate(&self, bytes: &[u8]) -> u32 {
        bytes.iter().fold(!0u32, |acc, it| {
            self.0[(((acc & 0xFF) as u8) ^ *it) as usize] ^ (acc >> 8)
        })
    }
    pub fn compare(&self, bytes: &[u8], crc32: u32) -> bool {
        self.calculate(bytes) == crc32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn one_byte_change_changes_the_result() {
        let table = Crc32Table::default();
        let value1 = (0..200).map(|i| i + 9).collect::<Vec<u8>>();
        let mut value2 = (0..200).map(|i| i + 9).collect::<Vec<u8>>();
        *value2.get_mut(199).unwrap() = 13;

        assert_ne!(table.calculate(&value1), table.calculate(&value2));
    }

    #[test]
    pub fn no_change_returns_same_result() {
        let table = Crc32Table::default();
        let value1 = (0..200).map(|i| i + 5).collect::<Vec<u8>>();
        let value2 = (0..200).map(|i| i + 5).collect::<Vec<u8>>();

        assert_eq!(table.calculate(&value1), table.calculate(&value2));
    }
}
