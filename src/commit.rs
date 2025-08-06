use crate::consts::*;
use crate::copy_all;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    fs::{create_dir, exists, read, read_dir, read_to_string, remove_dir_all, remove_file},
    iter::{repeat_n, zip},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Clone)]
pub enum Change {
    Update(u8),
    Delete,
}

pub struct Commit {
    pub new_files: HashSet<PathBuf>,
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

        let initial_dir = if exists(&*COMMITS_DIR).unwrap() {
            let mut commits_sorted = read_dir(&*COMMITS_DIR)
                .unwrap()
                .map(|x| x.unwrap())
                .collect::<Vec<_>>();
            commits_sorted
                .sort_by_key(|dir| dir.file_name().to_string_lossy().parse::<u128>().unwrap());

            create_dir(&*TMP_DIR).unwrap();
            copy_all(&POD_DIR, &TMP_DIR);

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

                let commit_files_path = commit_path.join(&*FILES_FILE);
                if exists(&commit_files_path).unwrap() {
                    for line in read_to_string(commit_files_path).unwrap().lines() {
                        let (operation, path) = line.split_once(' ').unwrap();
                        let relative = TMP_DIR.join(path);
                        match operation {
                            "+" => {
                                File::create(relative).unwrap();
                            }
                            "-" => {
                                remove_file(relative).unwrap();
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }

            TMP_DIR.clone()
        } else {
            POD_DIR.clone()
        };

        let initial_dirs = WalkDir::new(&initial_dir)
            .into_iter()
            .filter_entry(|entry| {
                let path = entry.path();
                path == initial_dir
                    || path.is_dir() && !IGNORE_ALL.contains(path.file_name().unwrap())
            })
            .map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                path.strip_prefix(initial_dir.clone())
                    .unwrap()
                    .to_path_buf()
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
            for entry in read_dir(initial_dir.join(dir)).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if !path.is_dir() && !IGNORE_ALL.contains(path.file_name().unwrap()) {
                    initial_files.insert(path.strip_prefix(&initial_dir).unwrap().to_path_buf());
                }
            }
        }

        dbg!(&initial_files, &current_files);

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

        let mut new_files = HashSet::new();
        let mut changed_files = HashMap::new();

        for file in &current_files {
            if initial_files.contains(file) {
                let current_content = read(CURRENT_DIR.join(file)).unwrap();
                let initial_content = read(initial_dir.join(file)).unwrap();

                for (index, (current_line, initial_line)) in zip(
                    current_content.iter().map(Some).chain(repeat_n(
                        None,
                        (initial_content.len() as isize - current_content.len() as isize).max(0)
                            as usize,
                    )),
                    initial_content.iter().map(Some).chain(repeat_n(
                        None,
                        (current_content.len() as isize - initial_content.len() as isize).max(0)
                            as usize,
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
                new_files.insert(file.clone());
            }
        }

        Commit {
            new_files,
            removed_files,
            new_dirs,
            removed_dirs,
            changed_files,
        }
    }
}
