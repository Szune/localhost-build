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

#[macro_export]
macro_rules! tuple (
    ($name:ident($($var:ident: $type:ty),* $(,)?)) => {
        #[derive(Debug,Clone)]
        pub struct $name {
            $(pub $var: $type),*
        }
        impl $name {
            #[allow(unused_parens)]
            pub fn destructure(self) -> ( $($type),* ) {
                ($(self.$var),*)
            }
        }
        #[allow(unused_parens)]
        impl From<( $($type),* )> for $name {
            fn from(($($var),*): ($($type),*)) -> $name {
                $name {
                    $($var),*
                }
            }
        }
        #[allow(unused_parens)]
        impl From<$name> for ( $($type),* ) {
            fn from(it: $name) -> ($($type),*) {
                ($(it.$var),*)
            }
        }
        #[allow(non_snake_case)]
        pub fn $name($($var: $type),*) -> $name {
            $name {
                $($var),*
            }
        }
    }
);
