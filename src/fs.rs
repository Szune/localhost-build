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
use crate::str;
use crate::tuple;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::fs::{DirEntry, ReadDir};
use std::path::{Component, Path, PathBuf};

tuple!(CanonicalFsOp(source: PathBuf, target: PathBuf));
tuple!(FsOp(source: String, target: String));

impl FsOp {
    pub fn canonicalize(&self) -> CanonicalFsOp {
        let source = canonicalize_any(&PathBuf::from(&self.source));
        let target = canonicalize_any(&PathBuf::from(&self.target));

        (source, target).into()
    }
}

/// `std::fs::canonicalize()` does not work for paths that don't exist
fn canonicalize_any(path: &Path) -> PathBuf {
    let mut components = path.components();
    let mut new_path = PathBuf::new();

    if let Some(c) = components.next() {
        match c {
            Component::RootDir => new_path.push(c.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                new_path.pop();
            }
            Component::Normal(_) | Component::Prefix(_) => new_path.push(c.as_os_str()),
        }
    }

    while let Some(c) = components.next() {
        match c {
            Component::RootDir => unreachable!(),
            Component::CurDir => {}
            Component::ParentDir => {
                new_path.pop();
            }
            Component::Normal(_) | Component::Prefix(_) => new_path.push(c.as_os_str()),
        }
    }

    new_path
}

pub fn get_source_and_target(input: String, op: &str) -> FsOp {
    let parts = str::get_strings(input);
    let mut it = parts.into_iter();
    let source = it
        .next()
        .unwrap_or_else(|| panic!("missing source argument in {}", op));
    let target = it
        .next()
        .unwrap_or_else(|| panic!("missing target argument in {}", op));
    (source, target).into()
}

pub fn move_it(fs_op: &FsOp) {
    fs::rename(&fs_op.source, &fs_op.target).unwrap_or_else(|err| {
        panic!(
            "failed to move from '{}' to '{}':\n{}",
            fs_op.source, fs_op.target, err
        )
    });
}

pub fn copy(fs_op: &FsOp) {
    fs::copy(&fs_op.source, &fs_op.target).unwrap_or_else(|err| {
        panic!(
            "failed to copy file from '{}' to '{}':\n{}",
            fs_op.source, fs_op.target, err
        )
    });
}

pub fn copy_canonical(fs_op: &CanonicalFsOp) {
    fs::copy(&fs_op.source, &fs_op.target).unwrap_or_else(|err| {
        panic!(
            "failed to copy file from '{:?}' to '{:?}':\n{}",
            fs_op.source, fs_op.target, err
        )
    });
}

pub fn cached_copy(fs_op: FsOp, cache: &mut HashMap<String, u32>, crc_table: &Crc32Table) {
    if cache.contains_key(&fs_op.target) {
        // target has been cached from before, check if source has same hash
        let crc = cache.get(&fs_op.target).unwrap();
        let target_exists = PathBuf::from(&fs_op.target).exists();
        if target_exists
            && fs::read(&fs_op.source)
                .map(|b| crc_table.compare(&b, *crc))
                .unwrap_or(false)
        {
            // no change, skip copy
            return;
        }
    }

    copy(&fs_op);

    // update/store crc32 of target
    let target_crc = fs::read(&fs_op.target).map(|b| crc_table.calculate(&b));
    if let Ok(crc) = target_crc {
        cache
            .entry(fs_op.target)
            .and_modify(|c| *c = crc)
            .or_insert(crc);
    }
}

pub fn cached_copy_canonical(
    fs_op: CanonicalFsOp,
    cache: &mut HashMap<String, u32>,
    crc_table: &Crc32Table,
) {
    if fs_op.target.exists() && cache.contains_key(&fs_op.target.to_string_lossy().to_string()) {
        // target has been cached from before, check if source has same hash
        let crc = cache
            .get(&fs_op.target.to_string_lossy().to_string())
            .unwrap();
        if fs::read(&fs_op.source)
            .map(|b| crc_table.compare(&b, *crc))
            .unwrap_or(false)
        {
            // no change, skip copy
            return;
        }
    }

    copy_canonical(&fs_op);

    // update/store crc32 of target
    let target_crc = fs::read(&fs_op.target).map(|b| crc_table.calculate(&b));
    if let Ok(crc) = target_crc {
        cache
            .entry(fs_op.target.to_string_lossy().to_string())
            .and_modify(|c| *c = crc)
            .or_insert(crc);
    }
}

pub fn copy_dir(fs_op: &FsOp) {
    let paths = create_recursive_dir_copy_ops(&fs_op);
    copy_dir_inner(paths, &mut |op| copy_canonical(&op));
}

pub fn cached_copy_dir(fs_op: &FsOp, cache: &mut HashMap<String, u32>, crc_table: &Crc32Table) {
    let paths = create_recursive_dir_copy_ops(&fs_op);
    copy_dir_inner(paths, &mut |op| {
        cached_copy_canonical(op, cache, &crc_table)
    });
}

fn copy_dir_inner<F>(paths: Vec<CanonicalFsOp>, copy_fn: &mut F)
where
    F: FnMut(CanonicalFsOp) -> (),
{
    let mut created_dirs: HashSet<PathBuf> = HashSet::new();
    for op in paths {
        if op.source.is_dir() {
            if created_dirs.insert(op.target.clone()) {
                create_dirs(&op.target);
            }
        } else if op.source.is_file() {
            let path_buf = op
                .target
                .parent()
                .unwrap_or_else(|| panic!("Failed to get parent of '{:?}'", op.target))
                .to_path_buf();
            if created_dirs.insert(path_buf.clone()) {
                create_dirs(&path_buf)
            }
            // if we're lucky we've now got a directory to copy the file into
            copy_fn(op);
        } else {
            println!("Warning: could not copy {:?} (file: {}, dir: {})", &op.source, &op.source.is_file(), &op.source.is_dir());
        }
    }
}

fn create_dirs(path_buf: &PathBuf) {
    if !path_buf.exists() {
        fs::create_dir_all(&path_buf).unwrap_or_else(|err| {
            panic!(
                "Failed to create directories of path '{:?}':\n{}",
                &path_buf, err
            )
        });
    }
}

fn create_recursive_dir_copy_ops(fs_op: &FsOp) -> Vec<CanonicalFsOp> {
    let canonical_fs_op = fs_op.canonicalize();
    let tree = DirectoryTree::new(canonical_fs_op.source.clone());
    tree.into_iter()
        .map(|f| {
            let path = f
                .unwrap_or_else(|err| panic!("File system error: {}", err))
                .path();
            let source_path = canonicalize_any(&path);
            let relative_path = source_path.clone();
            let relative_path = relative_path
                .strip_prefix(&canonical_fs_op.source)
                .unwrap_or_else(|err| {
                    panic!(
                        "Failed to strip prefix '{:?}' from path:\n{}",
                        canonical_fs_op.source, err
                    )
                });
            (source_path, PathBuf::from(relative_path))
        })
        .map(|(source_path, relative_path)| {
            let mut target_path = PathBuf::from(&fs_op.target);
            target_path.push(relative_path);
            let target_path = canonicalize_any(&target_path);

            (source_path, target_path).into()
        })
        .collect::<Vec<CanonicalFsOp>>()
}

/// Does not take symlinks into account at this time.
pub struct DirectoryTree {
    pub root: PathBuf,
    current_dir_iter: Option<ReadDir>,
    remaining_dirs: VecDeque<PathBuf>,
}

impl DirectoryTree {
    pub fn new(root: PathBuf) -> DirectoryTree {
        let mut remaining = VecDeque::new();
        remaining.push_back(root.clone());
        DirectoryTree {
            root,
            current_dir_iter: None,
            remaining_dirs: remaining,
        }
    }
}

impl Iterator for DirectoryTree {
    type Item = Result<DirEntry, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        macro_rules! pop_remaining (
            () => (
                if let Some(dir) = self.remaining_dirs.pop_front() {
                    self.current_dir_iter = Some(dir.read_dir().ok()?);
                } else {
                    return None;
                }
                )
            );

        let next_item = loop {
            if let Some(ref mut curr_dir) = self.current_dir_iter {
                if let Some(current) = curr_dir.next() {
                    if let Ok(ref it) = current {
                        if it.path().is_dir() {
                            self.remaining_dirs.push_back(it.path());
                        }
                    }
                    break current;
                }
            }
            pop_remaining!();
        };

        Some(match next_item {
            Ok(it) => Ok(it),
            Err(e) => Err(e),
        })
    }
}
