mod commit;
mod consts;

use crate::commit::{Change, Commit};
use consts::*;
use hex::encode;
use std::{
    env::var,
    fs::{copy, create_dir, create_dir_all, exists, write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use walkdir::WalkDir;

enum Mode {
    Init,
    Commit,
}

pub fn copy_all(source: &Path, dest: &Path) {
    for entry in WalkDir::new(source).into_iter().filter_entry(|entry| {
        let path = entry.path();

        path == source || !IGNORE_ALL.contains(path.file_name().unwrap())
    }) {
        let file = entry.unwrap();
        let path = file.path();

        let relative = dest.join(path.strip_prefix(source).unwrap());

        if path.is_dir() {
            create_dir_all(relative).unwrap();
        } else {
            copy(path, relative).unwrap();
        }
    }
}

fn create_pod() {
    copy_all(&CURRENT_DIR, &POD_DIR);
}

fn main() {
    let initialized = exists(&*POD_DIR).unwrap();
    let mode = match var("MODE").expect("MODE not provided.").as_str() {
        "INIT" => {
            if initialized {
                panic!("Pod already initialized.")
            }
            Mode::Init
        }
        "COMMIT" => {
            if !initialized {
                panic!("Pod needs to first be initialized.")
            }
            Mode::Commit
        }
        _ => panic!("Unknown MODE provided."),
    };

    match mode {
        Mode::Init => {
            create_pod();
        }
        Mode::Commit => {
            if !exists(&*COMMITS_DIR).unwrap() {
                create_dir(&*COMMITS_DIR).unwrap();
            }

            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string();

            let commit = Commit::new();

            let commit_dir_path = COMMITS_DIR.join(time);
            create_dir(&commit_dir_path).unwrap();

            // Handle directories
            if !commit.removed_dirs.is_empty() || !commit.new_dirs.is_empty() {
                let dirs_list = commit
                    .removed_dirs
                    .iter()
                    .map(|dir| format!("- {}\n", dir.to_str().unwrap()))
                    .chain(
                        commit
                            .new_dirs
                            .iter()
                            .map(|dir| format!("+ {}\n", dir.to_str().unwrap())),
                    )
                    .collect::<String>();

                write(commit_dir_path.join(&*DIRS_FILE), dirs_list).unwrap();
            }

            // Handle files
            if !commit.removed_files.is_empty() || !commit.new_files.is_empty() {
                let files_list = commit
                    .removed_files
                    .iter()
                    .map(|file| format!("- {}\n", file.to_str().unwrap()))
                    .chain(
                        commit
                            .new_files
                            .iter()
                            .map(|file| format!("+ {}\n", file.to_str().unwrap())),
                    )
                    .collect::<String>();

                write(commit_dir_path.join(&*FILES_FILE), files_list).unwrap();
            }

            // Handle changss in files
            if !commit.changed_files.is_empty() {
                let changes_dir_path = commit_dir_path.join(&*CHANGES_DIR);
                if !exists(&changes_dir_path).unwrap() {
                    create_dir(&changes_dir_path).unwrap();
                }

                for (name, changes) in &commit.changed_files {
                    let changes_list = changes
                        .iter()
                        .map(|(index, line)| match line {
                            Change::Update(line) => {
                                format!("{index} {line}\n")
                            }
                            Change::Delete => {
                                format!("- {index}\n")
                            }
                        })
                        .collect::<String>();
                    write(
                        changes_dir_path.join(encode(name.to_str().unwrap())),
                        changes_list,
                    )
                    .unwrap();
                }
            }
        }
    }
}
