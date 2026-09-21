use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::ops::{self, FileReport};
use crate::pipeline::{preview, Pipeline};

#[derive(Debug, Clone)]
pub struct BatchProgress {
    pub done: usize,
    pub total: usize,
    pub current: Option<PathBuf>,
    pub fraction: f64,
}

#[derive(Debug, Clone)]
pub struct BatchReport {
    pub results: Vec<FileReport>,
    pub cancelled: bool,
}

pub fn run_batch(
    sources: &[PathBuf],
    pipeline: &Pipeline,
    output_dir: &std::path::Path,
    copies: bool,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    mut on_progress: impl FnMut(BatchProgress),
) -> BatchReport {
    let maps = preview(sources, pipeline, output_dir);
    let total = maps.len();
    let mut results = Vec::new();
    for (i, map) in maps.into_iter().enumerate() {
        while pause.load(Ordering::SeqCst) && !cancel.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
        if cancel.load(Ordering::SeqCst) {
            results.push(FileReport {
                source: map.source,
                dest: None,
                ok: false,
                message: "Cancelled".into(),
                technical: None,
            });
            return BatchReport {
                results,
                cancelled: true,
            };
        }
        on_progress(BatchProgress {
            done: i,
            total,
            current: Some(map.source.clone()),
            fraction: if total == 0 {
                1.0
            } else {
                i as f64 / total as f64
            },
        });
        results.push(ops::process_file(
            &map.source,
            &map.dest,
            pipeline,
            copies,
            &cancel,
        ));
    }
    on_progress(BatchProgress {
        done: total,
        total,
        current: None,
        fraction: 1.0,
    });
    BatchReport {
        results,
        cancelled: false,
    }
}
