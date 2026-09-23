use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub const APP_ID: &str = "com.shadowfetch.BatchProcessor";
pub const APP_NAME: &str = "Shadow Batch Processor";
pub const APP_ICON: &str = "shadow-batch-processor";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_WEBSITE: &str = "https://github.com/Shadowfetchapps/Shadow-Batch-Processor";

pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| {
        Error::user("Could not find the user configuration directory (XDG_CONFIG_HOME).")
    })?;
    Ok(base.join("shadow-batch-processor"))
}

pub fn settings_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("settings.json"))
}

pub fn presets_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("presets"))
}

pub fn cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir().ok_or_else(|| {
        Error::user("Could not find the user cache directory (XDG_CACHE_HOME).")
    })?;
    Ok(base.join("shadow-batch-processor"))
}

pub fn temp_root() -> Result<PathBuf> {
    Ok(cache_dir()?.join("tmp"))
}

pub fn default_output_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Shadow Batch")
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).map_err(|err| {
        Error::detailed(
            format!("Could not create folder {}", path.display()),
            err.to_string(),
        )
    })
}

pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let ext = ext.trim_start_matches('.');
    let name = if ext.is_empty() {
        stem.to_string()
    } else {
        format!("{stem}.{ext}")
    };
    let mut candidate = dir.join(&name);
    if !candidate.exists() {
        return candidate;
    }
    for n in 2..10_000 {
        let name = if ext.is_empty() {
            format!("{stem} ({n})")
        } else {
            format!("{stem} ({n}).{ext}")
        };
        candidate = dir.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem}-{}.{ext}", std::process::id()))
}

pub fn display_home_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

pub fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into())
}

pub fn file_ext(path: &Path) -> String {
    path.extension()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}
