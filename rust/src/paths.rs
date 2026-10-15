//! Project path resolution without compile-time CARGO_MANIFEST_DIR.
//!
//! Project root resolution order:
//!   1. STOCK_DB_ROOT environment variable (absolute path)
//!   2. current working directory, walking upward to the first directory that
//!      looks like the stock_db repo (has both config/ and either Cargo.toml
//!      or rust/Cargo.toml)
//!   3. the current working directory itself

use std::env;
use std::path::{Path, PathBuf};

pub fn project_root() -> PathBuf {
    if let Ok(root) = env::var("STOCK_DB_ROOT") {
        let root = PathBuf::from(root);
        if root.is_absolute() {
            return root;
        }
    }
    if let Ok(cwd) = env::current_dir() {
        if let Some(root) = find_repo_root(&cwd) {
            return root;
        }
        return cwd;
    }
    PathBuf::from(".")
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current: Option<&Path> = Some(start);
    while let Some(dir) = current {
        if looks_like_repo_root(dir) {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn looks_like_repo_root(dir: &Path) -> bool {
    dir.join("config").is_dir()
        && (dir.join("Cargo.toml").is_file() || dir.join("rust").join("Cargo.toml").is_file())
}

pub fn var_dir() -> PathBuf {
    if let Ok(var_dir) = env::var("STOCK_DB_VAR_DIR") {
        return PathBuf::from(var_dir);
    }
    project_root().join("var")
}

pub fn stocks_db_path() -> PathBuf {
    var_dir().join("db").join("stocks.db")
}

pub fn config_dir() -> PathBuf {
    project_root().join("config")
}

pub fn edinet_raw_dir() -> PathBuf {
    var_dir().join("raw").join("edinet")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_repo_root_requires_config_and_cargo() {
        let tmp = std::env::temp_dir();
        assert!(!looks_like_repo_root(&tmp));
    }

    #[test]
    fn find_repo_root_returns_none_for_unrelated_path() {
        let deep = std::env::temp_dir().join("a").join("b").join("c");
        assert_eq!(find_repo_root(&deep), None);
    }
}
