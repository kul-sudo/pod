use std::{collections::HashSet, ffi::OsString, fs::read_to_string, sync::LazyLock};

pub const CURRENT_DIR: &str = "./";

pub const POD_DIR: &str = ".pod";
pub const POD_IGNORE_FILE: &str = ".podignore";

pub static IGNORE: LazyLock<HashSet<OsString>> = LazyLock::new(|| {
    let content = read_to_string(POD_IGNORE_FILE).unwrap();

    content
        .lines()
        .map(OsString::from)
        .collect::<HashSet<OsString>>()
});

pub const COMMITS_DIR: &str = ".commits";

pub const CHANGES_DIR: &str = "changes";
pub const FILES_DIR: &str = "files";
