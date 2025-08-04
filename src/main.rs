mod commit;
mod consts;
mod walk_dir;

use crate::commit::{Change, Commit};
use crate::walk_dir::{WalkMethod, walk_dir};
use consts::*;
use hex::encode;
use std::{
    collections::HashSet,
    env::var,
    fs::{copy, create_dir, create_dir_all, exists, read_dir, write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

enum Mode {
    Init,
    Commit,
}

fn create_pod() {
    let mut current_files = HashSet::new();
    walk_dir(CURRENT_DIR.into(), &mut current_files, WalkMethod::Files);

    for file in &current_files {
        let dest = PathBuf::from(POD_DIR).join(file.strip_prefix(CURRENT_DIR).unwrap());
        create_dir_all(dest.parent().unwrap()).unwrap();
        copy(file, dest).unwrap();
    }
}

fn main() {
    let initialized = exists(POD_DIR).unwrap();
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
            let commits_dir_path = Path::new(POD_DIR).join(Path::new(COMMITS_DIR));
            if !exists(&commits_dir_path).unwrap() {
                create_dir(&commits_dir_path).unwrap();
            }

            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string();

            let commit_dir_path = commits_dir_path.join(time);
            create_dir(&commit_dir_path).unwrap();

            let commit = Commit::new();

            // Handle directories
            if !commit.removed_dirs.is_empty() || !commit.new_dirs.is_empty() {
                let dirs_list = commit
                    .removed_dirs
                    .iter()
                    .map(|dir| format!("- {}\n", dir.to_string_lossy()))
                    .chain(
                        commit
                            .new_dirs
                            .iter()
                            .map(|dir| format!("+ {}\n", dir.to_string_lossy())),
                    )
                    .collect::<String>();

                write(commit_dir_path.join("dirs"), dirs_list).unwrap();
            }

            // Handle files
            if !commit.removed_files.is_empty() || !commit.new_files.is_empty() {
                let files_list = commit
                    .removed_files
                    .iter()
                    .map(|file| format!("- {}\n", file.to_string_lossy()))
                    .chain(
                        commit
                            .new_files
                            .iter()
                            .map(|file| format!("+ {}\n", file.to_string_lossy())),
                    )
                    .collect::<String>();

                write(commit_dir_path.join(FILES_DIR), files_list).unwrap();
            }

            // Handle changes in files
            if !commit.changed_files.is_empty() {
                let changes_dir_path = Path::new(&commit_dir_path).join(Path::new(CHANGES_DIR));
                if !exists(&changes_dir_path).unwrap() {
                    create_dir(&changes_dir_path).unwrap();
                }
                //
                // dbg!(&commit.changed_files);
                // dbg!(commit.changed_files.keys().collect::<Vec<_>>());

                for (name, changes) in &commit.changed_files {
                    let changes_list = changes
                        .iter()
                        .map(|(index, line)| match line {
                            Change::Update(line) => {
                                format!("{} {}\n", index, line)
                            }
                            Change::Delete => {
                                format!("- {}\n", index)
                            }
                        })
                        .collect::<String>();
                    write(
                        changes_dir_path.join(encode(name.to_string_lossy().to_string())),
                        changes_list,
                    )
                    .unwrap();
                }
            }
        }
    }
}
