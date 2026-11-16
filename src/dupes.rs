use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use rayon::prelude::*;
use xxhash_rust::xxh3::Xxh3;

use eframe::egui;

use crate::model::NodeId;

const PARTIAL_BYTES: u64 = 64 * 1024;
const BUFFER_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug)]
pub struct DupCandidate {
    pub node: NodeId,
    pub path: PathBuf,
    pub size: u64,
    pub modified: i64,
}

#[derive(Clone, Debug)]
pub struct DupFile {
    pub node: NodeId,
    pub path: PathBuf,
    pub size: u64,
    pub modified: i64,
}

#[derive(Clone, Debug)]
pub struct DupGroup {
    pub size: u64,
    pub hash: u64,
    pub files: Vec<DupFile>,
}

impl DupGroup {
    pub fn wasted(&self) -> u64 {
        self.size.saturating_mul(self.files.len().saturating_sub(1) as u64)
    }
}

#[derive(Clone, Debug, Default)]
pub struct DupResults {
    pub groups: Vec<DupGroup>,
    pub candidates: u64,
    pub files_hashed: u64,
    pub bytes_read: u64,
    pub elapsed: Duration,
}

impl DupResults {
    pub fn wasted(&self) -> u64 {
        self.groups.iter().map(DupGroup::wasted).sum()
    }

    pub fn duplicate_files(&self) -> u64 {
        self.groups
            .iter()
            .map(|group| group.files.len().saturating_sub(1) as u64)
            .sum()
    }

    pub fn remove_file(&mut self, node: NodeId) {
        for group in &mut self.groups {
            group.files.retain(|file| file.node != node);
        }
        self.groups.retain(|group| group.files.len() > 1);
    }
}

#[derive(Debug, Default)]
pub struct DupProgress {
    pub phase: Mutex<String>,
    pub current: Mutex<String>,
    pub files_done: AtomicU64,
    pub files_total: AtomicU64,
    pub bytes_read: AtomicU64,
}

#[derive(Clone, Debug, Default)]
pub struct DupProgressSnapshot {
    pub phase: String,
    pub current: String,
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_read: u64,
}

impl DupProgress {
    pub fn snapshot(&self) -> DupProgressSnapshot {
        DupProgressSnapshot {
            phase: self.phase.lock().clone(),
            current: self.current.lock().clone(),
            files_done: self.files_done.load(Ordering::Relaxed),
            files_total: self.files_total.load(Ordering::Relaxed),
            bytes_read: self.bytes_read.load(Ordering::Relaxed),
        }
    }
}

pub enum DupMessage {
    Done(Box<DupResults>),
    Failed(String),
    Cancelled,
}

impl DupResults {
    /// Drops files that were removed from disk, and any group left with a
    /// single copy.
    pub fn remove_paths(&mut self, paths: &[PathBuf]) {
        for group in &mut self.groups {
            group
                .files
                .retain(|file| !paths.iter().any(|path| path == &file.path));
        }
        self.groups.retain(|group| group.files.len() > 1);
    }
}

pub struct DupJob {
    pub rx: Receiver<DupMessage>,
    pub cancel: Arc<AtomicBool>,
    pub progress: Arc<DupProgress>,
    pub started: Instant,
    pub generation: u64,
}

pub fn spawn(
    candidates: Vec<DupCandidate>,
    generation: u64,
    ctx: egui::Context,
) -> DupJob {
    let (tx, rx) = std::sync::mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(DupProgress::default());
    progress
        .phase
        .lock()
        .clone_from(&"Grouping files by size".to_string());
    progress.files_total.store(candidates.len() as u64, Ordering::Relaxed);
    let worker_cancel = cancel.clone();
    let worker_progress = progress.clone();
    let started = Instant::now();
    std::thread::spawn(move || {
        let outcome = find_duplicates(candidates, &worker_progress, &worker_cancel);
        let message = match outcome {
            Ok(Some(results)) => DupMessage::Done(Box::new(results)),
            Ok(None) => DupMessage::Cancelled,
            Err(err) => DupMessage::Failed(err),
        };
        let _ = tx.send(message);
        ctx.request_repaint();
    });
    DupJob {
        rx,
        cancel,
        progress,
        started,
        generation,
    }
}

fn find_duplicates(
    candidates: Vec<DupCandidate>,
    progress: &Arc<DupProgress>,
    cancel: &Arc<AtomicBool>,
) -> Result<Option<DupResults>, String> {
    let started = Instant::now();
    let total_candidates = candidates.len() as u64;
    if candidates.is_empty() {
        return Ok(Some(DupResults::default()));
    }

    // Group by exact size first.
    let mut by_size: HashMap<u64, Vec<DupCandidate>> = HashMap::new();
    for candidate in candidates {
        by_size.entry(candidate.size).or_default().push(candidate);
    }
    let size_groups: Vec<(u64, Vec<DupCandidate>)> = by_size
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .collect();
    if size_groups.is_empty() {
        return Ok(Some(DupResults {
            candidates: total_candidates,
            elapsed: started.elapsed(),
            ..Default::default()
        }));
    }

    *progress.phase.lock() = "Comparing file headers".to_string();
    // Hash the first 64 KiB of each candidate in parallel.
    let partials: Vec<(u64, Vec<(u64, DupCandidate)>)> = size_groups
        .into_par_iter()
        .map(|(size, files)| {
            let mut hashed = Vec::with_capacity(files.len());
            for file in files {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Some(hash) = hash_file(&file.path, Some(PARTIAL_BYTES), progress, cancel) {
                    hashed.push((hash, file));
                } else {
                    progress.files_done.fetch_add(1, Ordering::Relaxed);
                }
            }
            (size, hashed)
        })
        .collect();
    if cancel.load(Ordering::Relaxed) {
        return Ok(None);
    }

    let mut full_jobs: Vec<(u64, u64, Vec<DupCandidate>)> = Vec::new();
    for (size, files) in partials {
        let mut by_partial: HashMap<u64, Vec<DupCandidate>> = HashMap::new();
        for (hash, file) in files {
            by_partial.entry(hash).or_default().push(file);
        }
        for (hash, matching) in by_partial {
            if matching.len() > 1 {
                full_jobs.push((size, hash, matching));
            }
        }
    }

    *progress.phase.lock() = "Verifying matching files".to_string();
    let full_results: Vec<Option<DupGroup>> = full_jobs
        .into_par_iter()
        .map(|(size, partial_hash, files)| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let mut by_full: HashMap<u64, Vec<DupCandidate>> = HashMap::new();
            for file in files {
                if let Some(hash) = hash_file(&file.path, None, progress, cancel) {
                    by_full.entry(hash).or_default().push(file);
                }
            }
            let mut best: Option<(u64, Vec<DupFile>)> = None;
            for (hash, matching) in by_full {
                if matching.len() < 2 {
                    continue;
                }
                let dup_files: Vec<DupFile> = matching
                    .into_iter()
                    .map(|candidate| DupFile {
                        node: candidate.node,
                        path: candidate.path,
                        size: candidate.size,
                        modified: candidate.modified,
                    })
                    .collect();
                let group = (hash, dup_files);
                match &best {
                    Some((_, existing)) if existing.len() >= group.1.len() => {}
                    _ => best = Some(group),
                }
            }
            let _ = partial_hash;
            best.map(|(hash, files)| DupGroup { size, hash, files })
        })
        .collect();

    if cancel.load(Ordering::Relaxed) {
        return Ok(None);
    }
    let mut groups: Vec<DupGroup> = full_results.into_iter().flatten().collect();
    groups.sort_unstable_by(|a, b| {
        b.wasted()
            .cmp(&a.wasted())
            .then_with(|| b.size.cmp(&a.size))
    });
    Ok(Some(DupResults {
        groups,
        candidates: total_candidates,
        files_hashed: progress.files_done.load(Ordering::Relaxed),
        bytes_read: progress.bytes_read.load(Ordering::Relaxed),
        elapsed: started.elapsed(),
    }))
}

fn hash_file(
    path: &PathBuf,
    max_bytes: Option<u64>,
    progress: &Arc<DupProgress>,
    cancel: &Arc<AtomicBool>,
) -> Option<u64> {
    let mut file = File::open(path).ok()?;
    let mut hasher = Xxh3::new();
    let mut buffer = vec![0u8; BUFFER_BYTES];
    let mut remaining = max_bytes.unwrap_or(u64::MAX);
    loop {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        if remaining == 0 {
            break;
        }
        let want = buffer.len().min(remaining as usize);
        let read = match file.read(&mut buffer[..want]) {
            Ok(0) => break,
            Ok(read) => read,
            Err(_) => return None,
        };
        hasher.update(&buffer[..read]);
        progress
            .bytes_read
            .fetch_add(read as u64, Ordering::Relaxed);
        remaining = remaining.saturating_sub(read as u64);
    }
    progress.files_done.fetch_add(1, Ordering::Relaxed);
    Some(hasher.digest())
}
