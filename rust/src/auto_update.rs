use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn ensure_prices_fresh_for_api(db_path: &Path) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|err| err.to_string())?;
    let project_root = project_root();
    if is_inside_project(&cwd, &project_root) {
        return Ok(());
    }

    let command_db_path = resolve_command_db_path(db_path, &cwd);
    let status = Command::new("uv")
        .args(["run", "refresh-prices", "--if-needed", "--db"])
        .arg(&command_db_path)
        .current_dir(&project_root)
        .status()
        .map_err(|err| format!("stock price refresh command failed: {err}"))?;

    if !status.success() {
        return Err(format!(
            "stock price refresh command failed (exit={status})"
        ));
    }

    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust crate must live under stock_db/rust")
        .to_path_buf()
}

fn resolve_command_db_path(db_path: &Path, cwd: &Path) -> PathBuf {
    let path = if db_path.is_absolute() {
        db_path.to_path_buf()
    } else {
        cwd.join(db_path)
    };
    path.canonicalize().unwrap_or(path)
}

fn normalized(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn is_inside_project(cwd: &Path, project_root: &Path) -> bool {
    let cwd = normalized(cwd);
    let project_root = normalized(project_root);
    cwd == project_root || cwd.starts_with(project_root)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::is_inside_project;

    #[test]
    fn inside_project_matches_root_and_children_only() {
        let root = Path::new("/tmp/stock_db");

        assert!(is_inside_project(root, root));
        assert!(is_inside_project(Path::new("/tmp/stock_db/rust"), root));
        assert!(!is_inside_project(Path::new("/tmp/stock_db_other"), root));
    }
}
