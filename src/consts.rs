use std::{collections::HashSet, ffi::OsString, fs::read_to_string, path::PathBuf, sync::LazyLock};

pub static CURRENT_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("./"));

pub static POD_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from(".pod"));
pub static POD_IGNORE_FILE: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from(".podignore"));

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
            OsString::from(&*POD_DIR),
            OsString::from(&*COMMITS_DIR),
        ]))
        .cloned()
        .collect::<HashSet<_>>()
});

pub static COMMITS_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from(".commits"));

pub static CHANGES_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("changes"));
pub static FILES_FILE: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("files"));
pub static DIRS_FILE: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("dirs"));
pub static TMP_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("tmp"));
