mod consts;
mod walk_dir;

use consts::*;
use std::{
    collections::{HashMap, HashSet},
    env::{current_dir, var},
    fs::{copy, create_dir, exists, read_dir, read_to_string, write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use walk_dir::{WalkMethod, walk_dir};

enum Mode {
    Init,
    Commit,
}

#[derive(Debug)]
struct Commit {
    new_files: Vec<PathBuf>,
    removed_files: Vec<PathBuf>,
    new_dirs: Vec<PathBuf>,
    removed_dirs: Vec<PathBuf>,
    changed_files: HashMap<PathBuf, Vec<(usize, String)>>,
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
            .collect::<Vec<_>>();
        let removed_dirs = initial_dirs
            .difference(&current_dirs)
            .cloned()
            .collect::<Vec<_>>();

        let mut current_files = HashSet::new();
        walk_dir(CURRENT_DIR.into(), &mut current_files, WalkMethod::Files);
        let current_files = current_files
            .iter()
            .map(|x| x.strip_prefix(CURRENT_DIR).unwrap().to_path_buf())
            .collect::<HashSet<_>>();

        let mut initial_files = HashSet::new();
        walk_dir(
            PathBuf::from(POD_DIR),
            &mut initial_files,
            WalkMethod::Files,
        );
        let initial_files = initial_files
            .iter()
            .map(|x| x.strip_prefix(POD_DIR).unwrap().to_path_buf())
            .collect::<HashSet<_>>();

        let new_files = current_files
            .difference(&initial_files)
            .cloned()
            .collect::<Vec<_>>();

        let removed_files = initial_files
            .difference(&current_files)
            .cloned()
            .collect::<Vec<_>>();

        let mut changed_files = HashMap::with_capacity(new_files.len());
        for file in &new_files {
            let content = read_to_string(file).unwrap();

            let changes = content
                .lines()
                .enumerate()
                .map(|(index, line)| (index, line.to_string()))
                .collect::<Vec<_>>();
            changed_files.insert(file.clone(), changes);
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

            write(commit_dir_path.join("files"), files_list).unwrap();
        }
    }
    // let pod = Pod {
    //     contents: PodNode::Dir(Pod::create(&current_dir().unwrap())),
    // };

    // let mut slice = Vec::new();
    // let length = bincode::encode_into_slice(&pod, &mut slice, bincode::config::standard()).unwrap();
    //
    // let slice = &slice[..length];
    // println!("Bytes written: {:?}", slice);
}
