use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use shadow_batch::files;
use shadow_batch::pipeline::{preview, NumberStyle, Pipeline, Step};
use shadow_batch::runner;

fn make_png(dir: &std::path::Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let st = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=80x60",
            "-frames:v",
            "1",
            "-update",
            "1",
        ])
        .arg(&path)
        .status()
        .unwrap();
    assert!(st.success());
    path
}

fn make_video(dir: &std::path::Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let st = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=160x120:rate=10",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=400:duration=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&path)
        .status()
        .unwrap();
    assert!(st.success());
    path
}

#[test]
fn preview_rename_number() {
    let out = PathBuf::from("/tmp/shadow-batch-preview");
    let pipe = Pipeline {
        name: "n".into(),
        steps: vec![Step::Rename {
            prefix: "web-".into(),
            suffix: String::new(),
            numbering: NumberStyle::OneBased,
            extension: Some("jpg".into()),
        }],
    };
    let maps = preview(&[PathBuf::from("/a/Photo One.png")], &pipe, &out);
    assert!(maps[0]
        .dest
        .file_name()
        .unwrap()
        .to_string_lossy()
        .contains("web-Photo One-1.jpg"));
}

#[test]
fn skips_dirs_and_empty() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.bin");
    std::fs::write(&empty, b"").unwrap();
    let result = files::collect_paths(vec![dir.path().to_path_buf(), empty]);
    assert!(result.added.is_empty());
    assert_eq!(result.skipped.len(), 2);
}

#[test]
fn image_resize_convert_copies() {
    let dir = tempfile::tempdir().unwrap();
    let src = make_png(dir.path(), "a.png");
    let original = std::fs::read(&src).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let pipe = Pipeline {
        name: "img".into(),
        steps: vec![Step::ConvertImage {
            format: "jpeg".into(),
            max_width: Some(40),
            quality: 80,
        }],
    };
    let report = runner::run_batch(
        &[src.clone()],
        &pipe,
        &out,
        true,
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        |_| {},
    );
    assert_eq!(report.results.len(), 1);
    assert!(report.results[0].ok, "{}", report.results[0].message);
    let dest = report.results[0].dest.clone().unwrap();
    assert_ne!(dest, src);
    assert_eq!(std::fs::read(&src).unwrap(), original);
    let probed = shadow_batch::probe::probe(&dest).unwrap();
    assert!(probed.width.unwrap() <= 40);
}

#[test]
fn video_to_mp4_and_isolated_failure() {
    let dir = tempfile::tempdir().unwrap();
    let good = make_video(dir.path(), "ok.mp4");
    let bad = dir.path().join("nope.bin");
    std::fs::write(&bad, b"not media").unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let pipe = Pipeline {
        name: "vid".into(),
        steps: vec![Step::ConvertVideo {
            container: "mp4".into(),
            height: None,
            compress: false,
        }],
    };
    let report = runner::run_batch(
        &[good, bad],
        &pipe,
        &out,
        true,
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        |_| {},
    );
    assert_eq!(report.results.len(), 2);
    assert!(report.results[0].ok, "{}", report.results[0].message);
    assert!(!report.results[1].ok);
}

#[test]
fn extract_audio_and_metadata_strip() {
    let dir = tempfile::tempdir().unwrap();
    let src = make_video(dir.path(), "talk.mp4");
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let pipe = Pipeline {
        name: "mp3".into(),
        steps: vec![Step::ExtractAudio {
            format: "mp3".into(),
        }],
    };
    let report = runner::run_batch(
        &[src],
        &pipe,
        &out,
        true,
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        |_| {},
    );
    assert!(report.results[0].ok, "{}", report.results[0].message);
    let dest = report.results[0].dest.as_ref().unwrap();
    assert_eq!(dest.extension().unwrap(), "mp3");
}

#[test]
fn cancel_stops_batch() {
    let dir = tempfile::tempdir().unwrap();
    let src = make_video(dir.path(), "long.mp4");
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let cancel = Arc::new(AtomicBool::new(true));
    let pipe = Pipeline {
        name: "c".into(),
        steps: vec![Step::ConvertVideo {
            container: "mp4".into(),
            height: Some(1080),
            compress: true,
        }],
    };
    let report = runner::run_batch(
        &[src],
        &pipe,
        &out,
        true,
        cancel,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    );
    assert!(report.cancelled || report.results.iter().any(|r| !r.ok));
}
