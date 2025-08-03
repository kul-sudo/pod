mod consts;
mod walk_dir;

use consts::*;
use hex::{decode, encode};
use std::{
    collections::{HashMap, HashSet},
    env::var,
    fs::{copy, create_dir, exists, read, read_dir, write},
    iter::{repeat, zip},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use walk_dir::{WalkMethod, walk_dir};

enum Mode {
    Init,
    Commit,
}

#[derive(Debug)]
enum Change {
    Update(u8),
    Delete,
}

#[derive(Debug)]
struct Commit {
    new_files: HashSet<PathBuf>,
    removed_files: HashSet<PathBuf>,
    new_dirs: HashSet<PathBuf>,
    removed_dirs: HashSet<PathBuf>,
    changed_files: HashMap<PathBuf, Vec<(usize, Change)>>,
}

impl Commit {
    fn new() -> Commit {
        let mut current_dirs = HashSet::new();
        walk_dir(CURRENT_DIR.into(), &mut current_dirs, WalkMethod::Dirs);
        let current_dirs = current_dirs
            .iter()
            .map(|x| x.strip_prefix(CURRENT_DIR).unwrap().to_path_buf())
            .collect::<HashSet<_>>();

        let mut initial_dirs = HashSet::new();
        walk_dir(PathBuf::from(POD_DIR), &mut initial_dirs, WalkMethod::Dirs);
        let initial_dirs = initial_dirs
            .iter()
            .map(|x| x.strip_prefix(POD_DIR).unwrap().to_path_buf())
            .collect::<HashSet<_>>();

        let new_dirs = current_dirs
            .difference(&initial_dirs)
            .cloned()
            .collect::<HashSet<_>>();
        let removed_dirs = initial_dirs
            .difference(&current_dirs)
            .cloned()
            .collect::<HashSet<_>>();

        let mut current_files = HashSet::new();
        walk_dir(CURRENT_DIR.into(), &mut current_files, WalkMethod::Files);

        let mut initial_files = HashSet::new();
        walk_dir(
            PathBuf::from(POD_DIR),
            &mut initial_files,
            WalkMethod::Files,
        );

        let current_files = current_files
            .iter()
            .map(|x| x.strip_prefix(CURRENT_DIR).unwrap().to_path_buf())
            .collect::<HashSet<_>>();

        let initial_files = initial_files
            .iter()
            .map(|x| x.strip_prefix(POD_DIR).unwrap().to_path_buf())
            .collect::<HashSet<_>>();

        let removed_files = initial_files
            .difference(&current_files)
            .cloned()
            .collect::<HashSet<_>>();

        let mut new_files = HashSet::new();
        let mut changed_files = HashMap::new();

        for file in &current_files {
            changed_files.insert(file.clone(), Vec::new());
            if initial_files.contains(file) {
                let current_content = read(PathBuf::from(CURRENT_DIR).join(file)).unwrap();
                let initial_content = read(PathBuf::from(POD_DIR).join(file)).unwrap();

                for (index, (current_line, initial_line)) in zip(
                    current_content
                        .iter()
                        .map(|line| Some(line))
                        .chain(repeat(None).take(
                            (initial_content.len() as isize - current_content.len() as isize).max(0)
                                as usize,
                        )),
                    initial_content
                        .iter()
                        .map(|line| Some(line))
                        .chain(repeat(None).take(
                            (current_content.len() as isize - initial_content.len() as isize).max(0)
                                as usize,
                        )),
                )
                .enumerate()
                {
                    changed_files.entry(file.clone()).and_modify(
                        |changes: &mut Vec<(usize, Change)>| match current_line {
                            Some(lhs) => {
                                if match initial_line {
                                    Some(rhs) => lhs != rhs,
                                    None => true,
                                } {
                                    changes.push((index, Change::Update(*lhs)))
                                }
                            }
                            None => changes.push((index, Change::Delete)),
                        },
                    );
                }
            } else {
                let content = read(PathBuf::from(CURRENT_DIR).join(file)).unwrap();

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
