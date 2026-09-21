use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum NumberStyle {
    #[default]
    None,
    OneBased,
    ZeroPad3,
}

impl NumberStyle {
    pub fn apply(self, stem: &str, index: usize) -> String {
        match self {
            Self::None => stem.to_string(),
            Self::OneBased => format!("{stem}-{}", index + 1),
            Self::ZeroPad3 => format!("{stem}-{:03}", index + 1),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No numbers",
            Self::OneBased => "1, 2, 3…",
            Self::ZeroPad3 => "001, 002, 003…",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Step {
    Rename {
        prefix: String,
        suffix: String,
        numbering: NumberStyle,
        extension: Option<String>,
    },
    ConvertImage {
        format: String,
        max_width: Option<u32>,
        quality: u8,
    },
    ConvertVideo {
        container: String,
        height: Option<u32>,
        compress: bool,
    },
    ConvertAudio {
        format: String,
        normalize: bool,
    },
    ExtractAudio {
        format: String,
    },
    RemoveMetadata,
}

impl Step {
    pub fn label(&self) -> String {
        match self {
            Self::Rename {
                prefix,
                suffix,
                numbering,
                extension,
            } => {
                let mut bits = vec!["Rename".to_string()];
                if !prefix.is_empty() {
                    bits.push(format!("prefix {prefix}"));
                }
                if !suffix.is_empty() {
                    bits.push(format!("suffix {suffix}"));
                }
                if *numbering != NumberStyle::None {
                    bits.push(numbering.label().into());
                }
                if let Some(ext) = extension {
                    bits.push(format!(".{ext}"));
                }
                bits.join(" · ")
            }
            Self::ConvertImage {
                format,
                max_width,
                quality,
            } => match max_width {
                Some(w) => format!("Images → {format} (max {w}px, q{quality})"),
                None => format!("Images → {format} (q{quality})"),
            },
            Self::ConvertVideo {
                container,
                height,
                compress,
            } => {
                let res = height.map(|h| format!("{h}p")).unwrap_or_else(|| "original".into());
                format!(
                    "Video → {container} {res}{}",
                    if *compress { " compressed" } else { "" }
                )
            }
            Self::ConvertAudio { format, normalize } => {
                format!(
                    "Audio → {format}{}",
                    if *normalize { " normalized" } else { "" }
                )
            }
            Self::ExtractAudio { format } => format!("Extract audio → {format}"),
            Self::RemoveMetadata => "Remove metadata".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Pipeline {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct Mapping {
    pub source: PathBuf,
    pub dest: PathBuf,
    pub skipped_reason: Option<String>,
}

pub fn preview(sources: &[PathBuf], pipeline: &Pipeline, output_dir: &Path) -> Vec<Mapping> {
    sources
        .iter()
        .enumerate()
        .map(|(i, source)| map_one(source, i, pipeline, output_dir))
        .collect()
}

fn map_one(source: &Path, index: usize, pipeline: &Pipeline, output_dir: &Path) -> Mapping {
    let mut stem = paths::file_stem(source);
    let mut ext = paths::file_ext(source);
    for step in &pipeline.steps {
        match step {
            Step::Rename {
                prefix,
                suffix,
                numbering,
                extension,
            } => {
                stem = format!("{prefix}{}{suffix}", numbering.apply(&stem, index));
                if let Some(e) = extension {
                    if !e.is_empty() {
                        ext = e.trim_start_matches('.').to_string();
                    }
                }
            }
            Step::ConvertImage { format, .. } => {
                if looks_image(source, &ext) {
                    ext = normalize_ext(format);
                }
            }
            Step::ConvertVideo { container, .. } => {
                if looks_video(&ext) {
                    ext = normalize_ext(container);
                }
            }
            Step::ConvertAudio { format, .. } | Step::ExtractAudio { format } => {
                if looks_audio(&ext) || looks_video(&ext) {
                    ext = normalize_ext(format);
                }
            }
            Step::RemoveMetadata => {}
        }
    }
    let dest = paths::unique_path(output_dir, &stem, &ext);
    Mapping {
        source: source.to_path_buf(),
        dest,
        skipped_reason: None,
    }
}

fn normalize_ext(format: &str) -> String {
    match format.to_ascii_lowercase().as_str() {
        "jpeg" => "jpg".into(),
        "aac" => "m4a".into(),
        other => other.to_string(),
    }
}

fn looks_image(_path: &Path, ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "avif" | "tif" | "tiff" | "gif" | "bmp"
    )
}

fn looks_video(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v"
    )
}

fn looks_audio(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp3" | "wav" | "flac" | "aac" | "m4a" | "opus" | "ogg"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbering_and_prefix() {
        let dir = PathBuf::from("/tmp/out");
        let pipeline = Pipeline {
            name: "t".into(),
            steps: vec![Step::Rename {
                prefix: "img-".into(),
                suffix: String::new(),
                numbering: NumberStyle::ZeroPad3,
                extension: Some("jpg".into()),
            }],
        };
        let maps = preview(&[PathBuf::from("/in/Holiday Photo.png")], &pipeline, &dir);
        assert!(maps[0]
            .dest
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("img-Holiday Photo-001.jpg"));
    }
}
