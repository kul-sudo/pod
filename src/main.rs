mod commit;
mod consts;
mod walk_dir;

use crate::commit::{Change, Commit};
use consts::*;
use hex::encode;
use std::{
    env::var,
    fs::{copy, create_dir, exists, read_dir, write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

enum Mode {
    Init,
    Commit,
}

fn create_pod(source: PathBuf, dir: PathBuf) {
    let content = read_dir(source).unwrap().collect::<Vec<_>>();
    create_dir(&dir).unwrap();
    for entry in content {
        let entry = entry.unwrap();
        let file_type = entry.file_type().unwrap();
        let path = entry.path();

        if file_type.is_dir() {
            create_pod(path.clone(), dir.join(entry.file_name()));
        } else {
            copy(path.clone(), dir.join(entry.file_name())).unwrap();
        }
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
            create_pod(CURRENT_DIR.into(), POD_DIR.into());
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
