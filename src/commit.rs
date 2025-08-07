use crate::{consts::*, copy_all};
use hex::decode;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    fs::{create_dir, exists, read, read_dir, read_to_string, remove_dir_all, remove_file, write},
    iter::{repeat_n, zip},
    path::PathBuf,
};
use walkdir::WalkDir;

#[derive(Clone)]
pub enum Change {
    Update(u8),
    Delete,
}

pub struct Commit {
    pub removed_files: HashSet<PathBuf>,
    pub new_dirs: HashSet<PathBuf>,
    pub removed_dirs: HashSet<PathBuf>,
    pub changed_files: HashMap<PathBuf, Vec<(usize, Change)>>,
}

impl Commit {
    pub fn new() -> Commit {
        let current_dirs = WalkDir::new(&*CURRENT_DIR)
            .into_iter()
            .filter_entry(|entry| {
                let path = entry.path();
                path == *CURRENT_DIR
                    || path.is_dir() && !IGNORE_ALL.contains(path.file_name().unwrap())
            })
            .map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                path.strip_prefix(&*CURRENT_DIR).unwrap().to_path_buf()
            })
            .collect::<HashSet<_>>();

        create_dir(&*TMP_DIR).unwrap();
        copy_all(&POD_DIR, &TMP_DIR);

        if exists(&*COMMITS_DIR).unwrap() {
            let mut commits_sorted = read_dir(&*COMMITS_DIR)
                .unwrap()
                .map(|x| x.unwrap())
                .collect::<Vec<_>>();
            commits_sorted
                .sort_by_key(|dir| dir.file_name().to_str().unwrap().parse::<u128>().unwrap());

            for commit in &commits_sorted {
                let commit_path = commit.path();

                let commit_dirs_path = commit_path.join(&*DIRS_FILE);
                if exists(&commit_dirs_path).unwrap() {
                    for line in read_to_string(commit_dirs_path).unwrap().lines() {
                        let (operation, path) = line.split_once(' ').unwrap();
                        let relative = TMP_DIR.join(path);
                        (match operation {
                            "+" => create_dir,
                            "-" => remove_dir_all,
                            _ => unreachable!(),
                        })(relative)
                        .unwrap();
                    }
                }

                let commit_files_path = commit_path.join(&*REMOVED_FILES_FILE);
                if exists(&commit_files_path).unwrap() {
                    for path in read_to_string(commit_files_path).unwrap().lines() {
                        let relative = TMP_DIR.join(path);
                        remove_file(&relative).unwrap();
                    }
                }

                let commit_changes_path = commit_path.join(&*CHANGES_DIR);
                if exists(&commit_changes_path).unwrap() {
                    for entry in read_dir(&commit_changes_path).unwrap() {
                        let entry = entry.unwrap();
                        let path = entry.path();
                        let changes = read_to_string(&path).unwrap();

                        let hex = path.file_name().unwrap();
                        let file =
                            String::from_utf8(decode(hex.to_str().unwrap()).unwrap()).unwrap();

                        let relative = TMP_DIR.join(file);

                        if exists(&relative).unwrap() {
                            let b = read(&relative).unwrap();
                            let mut bytes = b.iter().cloned().map(Some).collect::<Vec<_>>();
                            for line in changes.lines() {
                                let (operation, remaining) = line.split_once(' ').unwrap();
                                if operation == "-" {
                                    bytes[remaining.parse::<usize>().unwrap()] = None;
                                } else if let Ok(index) = operation.parse::<usize>() {
                                    if index > bytes.len() {
                                        bytes.resize(index, None);
                                        bytes[index - 1] = Some(remaining.parse::<u8>().unwrap())
                                    }
                                }
                            }

                            write(
                                relative,
                                bytes.iter().filter_map(|byte| *byte).collect::<Vec<_>>(),
                            )
                            .unwrap();
                        } else {
                            let bytes = changes
                                .lines()
                                .map(|line| line.split_once(' ').unwrap().1.parse::<u8>().unwrap())
                                .collect::<Vec<_>>();
                            write(relative, bytes).unwrap();
                        }
                    }
                }
            }
        }

        let initial_dirs = WalkDir::new(&*TMP_DIR)
            .into_iter()
            .filter_entry(|entry| {
                let path = entry.path();
                path == *TMP_DIR || path.is_dir() && !IGNORE_ALL.contains(path.file_name().unwrap())
            })
            .map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                path.strip_prefix(&*TMP_DIR).unwrap().to_path_buf()
            })
            .collect::<HashSet<_>>();

        let mut current_files = HashSet::new();
        for dir in &current_dirs {
            for entry in read_dir(CURRENT_DIR.join(dir)).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if !path.is_dir() && !IGNORE_ALL.contains(path.file_name().unwrap()) {
                    current_files.insert(path.strip_prefix(&*CURRENT_DIR).unwrap().to_path_buf());
                }
            }
        }

        let mut initial_files = HashSet::new();
        for dir in &initial_dirs {
            for entry in read_dir(TMP_DIR.join(dir)).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if !path.is_dir() && !IGNORE_ALL.contains(path.file_name().unwrap()) {
                    initial_files.insert(path.strip_prefix(&*TMP_DIR).unwrap().to_path_buf());
                }
            }
        }

        let new_dirs = current_dirs
            .difference(&initial_dirs)
            .cloned()
            .collect::<HashSet<_>>();

        let removed_dirs = initial_dirs
            .difference(&current_dirs)
            .cloned()
            .collect::<HashSet<_>>();

        let removed_files = initial_files
            .difference(&current_files)
            .cloned()
            .collect::<HashSet<_>>();

        let mut changed_files = HashMap::new();

        for file in &current_files {
            if initial_files.contains(file) {
                let current_content = read(CURRENT_DIR.join(file)).unwrap();
                let initial_content = read(TMP_DIR.join(file)).unwrap();

                for (index, (current_line, initial_line)) in zip(
                    current_content.iter().map(Some).chain(repeat_n(
                        None,
                        initial_content.len().saturating_sub(current_content.len()),
                    )),
                    initial_content.iter().map(Some).chain(repeat_n(
                        None,
                        current_content.len().saturating_sub(initial_content.len()),
                    )),
                )
                .enumerate()
                {
                    let action = match current_line {
                        Some(lhs) => {
                            if match initial_line {
                                Some(rhs) => lhs != rhs,
                                None => true,
                            } {
                                (index, Change::Update(*lhs))
                            } else {
                                continue;
                            }
                        }
                        None => (index, Change::Delete),
                    };

                    changed_files
                        .entry(file.clone())
                        .and_modify(|changes: &mut Vec<(usize, Change)>| {
                            changes.push(action.clone())
                        })
                        .or_insert(vec![action]);
                }
            } else {
                let content = read(CURRENT_DIR.join(file)).unwrap();

                let changes = content
                    .iter()
                    .enumerate()
                    .map(|(index, line)| (index, Change::Update(*line)))
                    .collect::<Vec<_>>();

                changed_files.insert(file.clone(), changes);
            }
        }

        remove_dir_all(&*TMP_DIR).unwrap();

        Commit {
            removed_files,
            new_dirs,
            removed_dirs,
            changed_files,
        }
    }
}
