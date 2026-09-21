use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::paths;
use crate::pipeline::Pipeline;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn from_index(index: u32) -> Self {
        match index {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn index(self) -> u32 {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub output_dir: PathBuf,
    pub process_copies: bool,
    pub prefer_hardware: bool,
    pub preserve_metadata: bool,
    pub theme: Theme,
    pub last_pipeline: Pipeline,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_dir: paths::default_output_dir(),
            process_copies: true,
            prefer_hardware: true,
            preserve_metadata: true,
            theme: Theme::System,
            last_pipeline: Pipeline::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let Ok(path) = paths::settings_path() else {
            return Self::default();
        };
        let Ok(bytes) = std::fs::read(path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = paths::settings_path()?;
        if let Some(parent) = path.parent() {
            paths::ensure_dir(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|err| {
            crate::error::Error::detailed("Could not save settings.", err.to_string())
        })?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
