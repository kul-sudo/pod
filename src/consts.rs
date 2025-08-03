use std::{
    collections::HashSet,
    env::{current_dir, var},
    fs::{copy, create_dir, exists, read_dir},
    path::{Path, PathBuf},
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

pub const CURRENT_DIR: &str = "./";

pub const POD_DIR: &str = ".pod";
pub const COMMITS_DIR: &str = ".commits";
