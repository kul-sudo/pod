use crate::consts::*;
use crate::walk_dir::{WalkMethod, walk_dir};
use std::{
    collections::{HashMap, HashSet},
    fs::read,
    iter::{repeat_n, zip},
    path::PathBuf,
};

#[derive(Debug)]
pub enum Change {
    Update(u8),
    Delete,
}

#[derive(Debug)]
pub struct Commit {
    pub new_files: HashSet<PathBuf>,
    pub removed_files: HashSet<PathBuf>,
    pub new_dirs: HashSet<PathBuf>,
    pub removed_dirs: HashSet<PathBuf>,
    pub changed_files: HashMap<PathBuf, Vec<(usize, Change)>>,
}

impl Commit {
    pub fn new() -> Commit {
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
