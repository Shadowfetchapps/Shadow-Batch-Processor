use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::detect::MediaKind;
use crate::error::{Error, Result};
use crate::paths;
use crate::pipeline::{Pipeline, Step};
use crate::probe;
use crate::temps::TempJob;
use crate::validate;

#[derive(Debug, Clone)]
pub struct FileReport {
    pub source: PathBuf,
    pub dest: Option<PathBuf>,
    pub ok: bool,
    pub message: String,
    pub technical: Option<String>,
}

pub fn process_file(
    source: &Path,
    dest: &Path,
    pipeline: &Pipeline,
    copies: bool,
    cancel: &AtomicBool,
) -> FileReport {
    if cancel.load(Ordering::SeqCst) {
        return FileReport {
            source: source.to_path_buf(),
            dest: None,
            ok: false,
            message: "Cancelled".into(),
            technical: None,
        };
    }
    match process_file_inner(source, dest, pipeline, copies, cancel) {
        Ok(path) => FileReport {
            source: source.to_path_buf(),
            dest: Some(path),
            ok: true,
            message: "Done".into(),
            technical: None,
        },
        Err(err) => FileReport {
            source: source.to_path_buf(),
            dest: None,
            ok: false,
            message: err.human_message(),
            technical: err.technical_details(),
        },
    }
}

fn process_file_inner(
    source: &Path,
    dest: &Path,
    pipeline: &Pipeline,
    copies: bool,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    if !copies {
        return Err(Error::user(
            "Destructive processing was refused. Shadow Batch Processor writes copies unless you confirm overwrite mode.",
        ));
    }
    if let Some(parent) = dest.parent() {
        paths::ensure_dir(parent)?;
    }
    let tmp = TempJob::create()?;
    let mut current = source.to_path_buf();
    let mut kind = probe::probe(&current).ok().map(|p| p.kind);

    for (i, step) in pipeline.steps.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            return Err(Error::user("Cancelled"));
        }
        let next = tmp.child(&format!("step-{i}"));
        match apply_step(&current, &next, step, kind, cancel)? {
            StepOut::Unchanged => {}
            StepOut::Wrote(path, new_kind) => {
                current = path;
                kind = new_kind.or(kind);
            }
        }
    }

    let final_dest = if dest.exists() {
        let stem = paths::file_stem(dest);
        let ext = paths::file_ext(dest);
        paths::unique_path(dest.parent().unwrap_or(Path::new(".")), &stem, &ext)
    } else {
        dest.to_path_buf()
    };
    if current == *source {
        std::fs::copy(source, &final_dest)?;
    } else if std::fs::rename(&current, &final_dest).is_err() {
        std::fs::copy(&current, &final_dest)?;
    }
    if let Some(k) = kind {
        validate::validate_media(&final_dest, Some(k))?;
    } else {
        validate::validate_media(&final_dest, None)?;
    }
    Ok(final_dest)
}

enum StepOut {
    Unchanged,
    Wrote(PathBuf, Option<MediaKind>),
}

fn apply_step(
    current: &Path,
    dest_hint: &Path,
    step: &Step,
    kind: Option<MediaKind>,
    cancel: &AtomicBool,
) -> Result<StepOut> {
    match step {
        Step::Rename { .. } => Ok(StepOut::Unchanged),
        Step::ConvertImage {
            format,
            max_width,
            quality,
        } => {
            if kind == Some(MediaKind::Video) || kind == Some(MediaKind::Audio) {
                return Ok(StepOut::Unchanged);
            }
            let ext = normalize_ext(format);
            let dest = dest_hint.with_extension(&ext);
            convert_image(current, &dest, *max_width, *quality, &ext, cancel)?;
            Ok(StepOut::Wrote(dest, Some(MediaKind::Image)))
        }
        Step::ConvertVideo {
            container,
            height,
            compress,
        } => {
            if kind == Some(MediaKind::Image) || kind == Some(MediaKind::Audio) {
                return Ok(StepOut::Unchanged);
            }
            let ext = normalize_ext(container);
            let dest = dest_hint.with_extension(&ext);
            convert_video(current, &dest, *height, *compress, cancel)?;
            Ok(StepOut::Wrote(dest, Some(MediaKind::Video)))
        }
        Step::ConvertAudio { format, normalize } => {
            if kind == Some(MediaKind::Image) {
                return Ok(StepOut::Unchanged);
            }
            let ext = normalize_ext(format);
            let dest = dest_hint.with_extension(&ext);
            convert_audio(current, &dest, &ext, *normalize, false, cancel)?;
            Ok(StepOut::Wrote(dest, Some(MediaKind::Audio)))
        }
        Step::ExtractAudio { format } => {
            if kind == Some(MediaKind::Image) {
                return Ok(StepOut::Unchanged);
            }
            let ext = normalize_ext(format);
            let dest = dest_hint.with_extension(&ext);
            convert_audio(current, &dest, &ext, false, true, cancel)?;
            Ok(StepOut::Wrote(dest, Some(MediaKind::Audio)))
        }
        Step::RemoveMetadata => {
            let ext = paths::file_ext(current);
            let dest = dest_hint.with_extension(&ext);
            strip_metadata(current, &dest, kind, cancel)?;
            Ok(StepOut::Wrote(dest, kind))
        }
    }
}

fn normalize_ext(format: &str) -> String {
    match format.to_ascii_lowercase().as_str() {
        "jpeg" => "jpg".into(),
        "aac" => "m4a".into(),
        other => other.to_string(),
    }
}

fn convert_image(
    src: &Path,
    dest: &Path,
    max_width: Option<u32>,
    quality: u8,
    ext: &str,
    cancel: &AtomicBool,
) -> Result<()> {
    if which::which("convert").is_ok() {
        let mut args = vec![
            OsString::from(src.as_os_str()),
            OsString::from("-auto-orient"),
        ];
        if let Some(w) = max_width {
            args.push("-resize".into());
            args.push(format!("{w}x{w}>").into());
        }
        args.push("-quality".into());
        args.push(quality.to_string().into());
        args.push(dest.as_os_str().to_owned());
        return run_cmd("convert", &args, cancel);
    }
    let mut args = ffmpeg_base();
    args.push("-i".into());
    args.push(src.as_os_str().to_owned());
    args.push("-frames:v".into());
    args.push("1".into());
    if let Some(w) = max_width {
        args.push("-vf".into());
        args.push(format!("scale='min({w},iw)':-1:flags=lanczos").into());
    }
    match ext {
        "jpg" | "jpeg" => {
            args.push("-c:v".into());
            args.push("mjpeg".into());
        }
        "webp" => {
            args.push("-c:v".into());
            args.push("libwebp".into());
            args.push("-quality".into());
            args.push(quality.to_string().into());
        }
        "avif" => {
            args.push("-c:v".into());
            args.push("libaom-av1".into());
            args.push("-still-picture".into());
            args.push("1".into());
        }
        _ => {
            args.push("-c:v".into());
            args.push("png".into());
        }
    }
    args.push(dest.as_os_str().to_owned());
    run_cmd("ffmpeg", &args, cancel)
}

fn convert_video(
    src: &Path,
    dest: &Path,
    height: Option<u32>,
    compress: bool,
    cancel: &AtomicBool,
) -> Result<()> {
    let probe = probe::probe(src)?;
    let mut args = ffmpeg_base();
    args.push("-i".into());
    args.push(src.as_os_str().to_owned());
    let can_copy = height.is_none()
        && !compress
        && probe.video_codec.as_deref() == Some("h264")
        && matches!(probe.audio_codec.as_deref(), Some("aac") | None);
    if can_copy {
        args.push("-c".into());
        args.push("copy".into());
    } else {
        if let Some(h) = height {
            if probe.height.unwrap_or(h) > h {
                args.push("-vf".into());
                args.push(format!("scale=-2:{h}:flags=lanczos").into());
            }
        }
        args.push("-c:v".into());
        args.push("libx264".into());
        args.push("-preset".into());
        args.push("medium".into());
        args.push("-crf".into());
        args.push(if compress { "28" } else { "23" }.into());
        args.push("-pix_fmt".into());
        args.push("yuv420p".into());
        if probe.has_audio {
            args.push("-c:a".into());
            args.push("aac".into());
            args.push("-b:a".into());
            args.push("192k".into());
        } else {
            args.push("-an".into());
        }
    }
    args.push("-movflags".into());
    args.push("+faststart".into());
    args.push(dest.as_os_str().to_owned());
    run_cmd("ffmpeg", &args, cancel)
}

fn convert_audio(
    src: &Path,
    dest: &Path,
    ext: &str,
    normalize: bool,
    extract: bool,
    cancel: &AtomicBool,
) -> Result<()> {
    let mut args = ffmpeg_base();
    args.push("-i".into());
    args.push(src.as_os_str().to_owned());
    if extract {
        args.push("-vn".into());
    }
    if normalize {
        args.push("-af".into());
        args.push("loudnorm=I=-16:TP=-1.5:LRA=11".into());
    }
    match ext {
        "mp3" => {
            args.push("-c:a".into());
            args.push("libmp3lame".into());
            args.push("-b:a".into());
            args.push("192k".into());
        }
        "wav" => {
            args.push("-c:a".into());
            args.push("pcm_s16le".into());
        }
        "flac" => {
            args.push("-c:a".into());
            args.push("flac".into());
        }
        "m4a" | "aac" => {
            args.push("-c:a".into());
            args.push("aac".into());
            args.push("-b:a".into());
            args.push("192k".into());
        }
        "opus" => {
            args.push("-c:a".into());
            args.push("libopus".into());
            args.push("-b:a".into());
            args.push("128k".into());
        }
        other => {
            return Err(Error::user(format!("Unsupported audio format {other}.")));
        }
    }
    args.push(dest.as_os_str().to_owned());
    run_cmd("ffmpeg", &args, cancel)
}

fn strip_metadata(
    src: &Path,
    dest: &Path,
    kind: Option<MediaKind>,
    cancel: &AtomicBool,
) -> Result<()> {
    if kind == Some(MediaKind::Image) && which::which("convert").is_ok() {
        return run_cmd(
            "convert",
            &[
                src.as_os_str().to_owned(),
                "-strip".into(),
                dest.as_os_str().to_owned(),
            ],
            cancel,
        );
    }
    let mut args = ffmpeg_base();
    args.push("-i".into());
    args.push(src.as_os_str().to_owned());
    args.push("-map_metadata".into());
    args.push("-1".into());
    args.push("-c".into());
    args.push("copy".into());
    args.push(dest.as_os_str().to_owned());
    run_cmd("ffmpeg", &args, cancel)
}

fn ffmpeg_base() -> Vec<OsString> {
    vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-y".into(),
        "-loglevel".into(),
        "error".into(),
    ]
}

fn run_cmd(program: &str, args: &[OsString], cancel: &AtomicBool) -> Result<()> {
    let bin = which::which(program).map_err(|_| {
        Error::user(format!(
            "{program} is not installed. Install it to run this pipeline step."
        ))
    })?;
    let mut cmd = Command::new(bin);
    cmd.args(args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = cmd
        .spawn()
        .map_err(|err| Error::detailed(format!("Could not start {program}."), err.to_string()))?;
    loop {
        if cancel.load(Ordering::SeqCst) {
            kill_group(&mut child);
            let _ = child.wait();
            return Err(Error::user("Cancelled"));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                }
                let mut err = String::new();
                if let Some(mut pipe) = child.stderr.take() {
                    use std::io::Read;
                    let _ = pipe.read_to_string(&mut err);
                }
                return Err(Error::detailed(
                    format!("{program} could not finish this file."),
                    err,
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(40)),
            Err(err) => {
                return Err(Error::detailed(
                    format!("{program} stopped unexpectedly."),
                    err.to_string(),
                ));
            }
        }
    }
}

fn kill_group(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        unsafe {
            libc::kill(-pid, libc::SIGTERM);
        }
        thread::sleep(Duration::from_millis(60));
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }
}
