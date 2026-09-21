use crate::pipeline::{NumberStyle, Pipeline, Step};

pub fn builtins() -> Vec<Pipeline> {
    vec![
        Pipeline {
            name: "YouTube Images".into(),
            steps: vec![
                Step::ConvertImage {
                    format: "jpeg".into(),
                    max_width: Some(1920),
                    quality: 85,
                },
                Step::RemoveMetadata,
            ],
        },
        Pipeline {
            name: "Website Images".into(),
            steps: vec![Step::ConvertImage {
                format: "webp".into(),
                max_width: Some(1600),
                quality: 80,
            }],
        },
        Pipeline {
            name: "Archive Photos".into(),
            steps: vec![
                Step::Rename {
                    prefix: String::new(),
                    suffix: String::new(),
                    numbering: NumberStyle::ZeroPad3,
                    extension: None,
                },
                Step::ConvertImage {
                    format: "jpeg".into(),
                    max_width: Some(4096),
                    quality: 92,
                },
            ],
        },
        Pipeline {
            name: "1080p Video".into(),
            steps: vec![Step::ConvertVideo {
                container: "mp4".into(),
                height: Some(1080),
                compress: false,
            }],
        },
        Pipeline {
            name: "MP3 Conversion".into(),
            steps: vec![Step::ConvertAudio {
                format: "mp3".into(),
                normalize: true,
            }],
        },
        Pipeline {
            name: "App Assets".into(),
            steps: vec![
                Step::ConvertImage {
                    format: "png".into(),
                    max_width: Some(1024),
                    quality: 95,
                },
                Step::RemoveMetadata,
            ],
        },
    ]
}

pub fn save_user_preset(pipeline: &Pipeline) -> crate::error::Result<()> {
    let dir = crate::paths::presets_dir()?;
    crate::paths::ensure_dir(&dir)?;
    let slug = pipeline
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .to_ascii_lowercase();
    let path = dir.join(format!("{slug}.json"));
    let json = serde_json::to_string_pretty(pipeline).map_err(|err| {
        crate::error::Error::detailed("Could not save the preset.", err.to_string())
    })?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_user_presets() -> Vec<Pipeline> {
    let Ok(dir) = crate::paths::presets_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(p) = serde_json::from_slice::<Pipeline>(&bytes) {
                out.push(p);
            }
        }
    }
    out
}

pub fn all_presets() -> Vec<Pipeline> {
    let mut v = builtins();
    v.extend(load_user_presets());
    v
}
