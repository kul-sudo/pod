use crate::consts::*;
use std::{collections::HashSet, fs::read_dir, path::PathBuf};

#[derive(Clone, Copy)]
pub enum WalkMethod {
    Dirs,
    Files,
}

pub fn walk_dir(dir: PathBuf, data: &mut HashSet<PathBuf>, method: WalkMethod) {
    walk_dir_recursive(dir, data, method);
}

fn walk_dir_recursive(dir: PathBuf, data: &mut HashSet<PathBuf>, method: WalkMethod) {
    dbg!(&dir);
    if !IGNORE.contains(dir.file_name().unwrap()) {
        for entry in read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let file_type = entry.file_type().unwrap();
            let path = entry.path();

            let file_name = path.file_name().unwrap();

            if file_name == POD_DIR || file_name == COMMITS_DIR || IGNORE.contains(file_name) {
                continue;
            }

            match method {
                WalkMethod::Dirs => {
                    if file_type.is_dir() {
                        data.insert(path.clone());
                        walk_dir_recursive(path.clone(), data, method);
                    }
                }
                WalkMethod::Files => {
                    if file_type.is_dir() {
                        walk_dir_recursive(path.clone(), data, method);
                    } else {
                        data.insert(path.clone());
                    }
                }
            }
        }
    }
}
