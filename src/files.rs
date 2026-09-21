use std::path::{Path, PathBuf};

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct AddResult {
    pub added: Vec<PathBuf>,
    pub skipped: Vec<(PathBuf, String)>,
}

pub fn collect_paths<I>(paths: I) -> AddResult
where
    I: IntoIterator<Item = PathBuf>,
{
    let mut added = Vec::new();
    let mut skipped = Vec::new();
    for path in paths {
        match classify_add(&path) {
            Ok(p) => {
                if !added.iter().any(|e| e == &p) {
                    added.push(p);
                }
            }
            Err(reason) => skipped.push((path, reason)),
        }
    }
    AddResult { added, skipped }
}

fn classify_add(path: &Path) -> std::result::Result<PathBuf, String> {
    let resolved = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if resolved.is_dir() {
        return Err("Folders are skipped. Add the files inside them.".into());
    }
    let meta = std::fs::metadata(&resolved).map_err(|_| "Could not read this file.".to_string())?;
    if meta.len() == 0 {
        return Err("Empty (0-byte) files are skipped.".into());
    }
    Ok(resolved)
}

pub fn explain_skips(result: &AddResult) -> Option<Error> {
    if result.skipped.is_empty() {
        return None;
    }
    let mut human = String::from("Some items were not added:\n");
    let mut tech = String::new();
    for (path, reason) in result.skipped.iter().take(8) {
        human.push_str(&format!("• {} — {reason}\n", path.display()));
        tech.push_str(&format!("{}: {reason}\n", path.display()));
    }
    if result.skipped.len() > 8 {
        human.push_str(&format!("…and {} more.\n", result.skipped.len() - 8));
    }
    Some(Error::detailed(human, tech))
}
