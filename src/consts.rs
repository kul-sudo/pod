use std::{
    collections::HashSet, env::current_dir, ffi::OsString, fs::read_to_string, path::PathBuf,
    sync::LazyLock,
};

pub static CURRENT_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from(current_dir().unwrap()));

const POD_DIR_CORE: &str = ".pod";
pub static POD_DIR: LazyLock<PathBuf> = LazyLock::new(|| CURRENT_DIR.join(POD_DIR_CORE));
pub static POD_IGNORE_FILE: LazyLock<PathBuf> = LazyLock::new(|| CURRENT_DIR.join(".podignore"));

pub static IGNORE: LazyLock<HashSet<OsString>> = LazyLock::new(|| {
    let content = read_to_string(&*POD_IGNORE_FILE).unwrap();

    content
        .lines()
        .map(OsString::from)
        .collect::<HashSet<OsString>>()
});

pub static IGNORE_ALL: LazyLock<HashSet<OsString>> = LazyLock::new(|| {
    IGNORE
        .union(&HashSet::from([
            OsString::from(POD_DIR_CORE),
            OsString::from(COMMITS_DIR_CORE),
        ]))
        .cloned()
        .collect::<HashSet<_>>()
});

const COMMITS_DIR_CORE: &str = ".commits";
pub static COMMITS_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(CURRENT_DIR.join(&*POD_DIR).join(COMMITS_DIR_CORE)));

pub static CHANGES_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("changes"));
pub static FILES_FILE: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("files"));
pub static DIRS_FILE: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("dirs"));
pub static TMP_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(CURRENT_DIR.join(&*POD_DIR).join(".tmp")));
