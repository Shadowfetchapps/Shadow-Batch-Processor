use std::path::Path;

use crate::detect::MediaKind;
use crate::error::{Error, Result};
use crate::probe;

pub fn validate_media(path: &Path, expected: Option<MediaKind>) -> Result<()> {
    if !path.is_file() {
        return Err(Error::user("The output file was not created."));
    }
    let meta = std::fs::metadata(path)?;
    if meta.len() == 0 {
        return Err(Error::user("The output file is empty."));
    }
    let probed = probe::probe(path)?;
    if let Some(kind) = expected {
        if probed.kind != kind && !(kind == MediaKind::Image && probed.kind == MediaKind::Video) {
            return Err(Error::detailed(
                "The output file is not the expected media type.",
                format!("expected {:?} got {:?}", kind, probed.kind),
            ));
        }
    }
    match probed.kind {
        MediaKind::Video if probed.width.unwrap_or(0) < 2 => {
            Err(Error::user("The output video has an invalid size."))
        }
        MediaKind::Image if probed.width.unwrap_or(0) < 1 => {
            Err(Error::user("The output image has an invalid size."))
        }
        MediaKind::Audio if !probed.has_audio => {
            Err(Error::user("The output audio file has no audio track."))
        }
        _ => Ok(()),
    }
}
