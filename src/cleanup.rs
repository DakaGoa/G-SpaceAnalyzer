// Known space hogs ("cleanup targets") and the background job that sizes them.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use eframe::egui;

#[derive(Clone, Debug)]
pub struct Target {
    pub title: &'static str,
    pub hint: &'static str,
    pub path: PathBuf,
    /// Caches and temp folders may be emptied from the UI; user data may not.
    pub clearable: bool,
}

fn env_path(var: &str, suffix: &str) -> Option<PathBuf> {
    let base = std::env::var_os(var)?;
    let mut path = PathBuf::from(base);
    if !suffix.is_empty() {
        path.push(suffix);
    }
    path.is_dir().then_some(path)
}

pub fn targets() -> Vec<Target> {
    let mut out = Vec::new();
    let mut push = |title: &'static str, hint: &'static str, path: Option<PathBuf>, clearable: bool| {
        if let Some(path) = path {
            out.push(Target {
                title,
                hint,
                path,
                clearable,
            });
        }
    };

    push(
        "User temp files",
        "Scratch files left behind by applications",
        env_path("TEMP", ""),
        true,
    );
    push(
        "Local app temp",
        "Per-user temporary data under LOCALAPPDATA",
        env_path("LOCALAPPDATA", "Temp"),
        true,
    );
    push(
        "Windows temp",
        "System-wide temporary files",
        env_path("SystemRoot", "Temp"),
        true,
    );
    push(
        "Delivery Optimization",
        "Windows Update download cache",
        env_path("SystemRoot", "SoftwareDistribution\\Download"),
        true,
    );
    push(
        "Chrome cache",
        "Cached web content for Google Chrome",
        env_path(
            "LOCALAPPDATA",
            "Google\\Chrome\\User Data\\Default\\Cache\\Cache_Data",
        ),
        true,
    );
    push(
        "Edge cache",
        "Cached web content for Microsoft Edge",
        env_path(
            "LOCALAPPDATA",
            "Microsoft\\Edge\\User Data\\Default\\Cache\\Cache_Data",
        ),
        true,
    );
    push(
        "Firefox cache",
        "Cached web content for Mozilla Firefox",
        env_path("LOCALAPPDATA", "Mozilla\\Firefox\\Profiles"),
        false,
    );
    push(
        "Thumbnail cache",
        "Explorer thumbnail and icon cache",
        env_path("LOCALAPPDATA", "Microsoft\\Windows\\Explorer"),
        false,
    );
    push(
        "Downloads",
        "Files downloaded from the web",
        env_path("USERPROFILE", "Downloads"),
        false,
    );
    push(
        "Previous Windows installation",
        "Left behind by a Windows upgrade",
        env_path("SystemDrive", "Windows.old"),
        true,
    );

    let system = std::env::var_os("SystemDrive")
        .map(|drive| format!("{}\\", drive.to_string_lossy()))
        .unwrap_or_else(|| "C:\\".to_string());
    for (title, hint, file) in [
        ("Hibernation file", "Reserved for hibernation and fast startup", "hiberfil.sys"),
        ("Paging file", "Virtual memory backing store", "pagefile.sys"),
        ("Swap file", "Additional virtual memory file", "swapfile.sys"),
    ] {
        let path = PathBuf::from(&system).join(file);
        if path.is_file() {
            out.push(Target {
                title,
                hint,
                path,
                clearable: false,
            });
        }
    }

    out
}

pub struct Measurement {
    pub index: usize,
    pub bytes: u64,
    pub files: u64,
}

pub enum CleanupMessage {
    Progress(String),
    Measured(Measurement),
    Done,
}

pub struct CleanupJob {
    pub rx: mpsc::Receiver<CleanupMessage>,
    pub cancel: Arc<AtomicBool>,
}

/// Walks every target on a worker thread and reports one size per target.
pub fn spawn(targets: Vec<Target>, ctx: egui::Context) -> CleanupJob {
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();

    std::thread::spawn(move || {
        for (index, target) in targets.iter().enumerate() {
            if worker_cancel.load(Ordering::Relaxed) {
                break;
            }
            let _ = tx.send(CleanupMessage::Progress(format!(
                "Measuring {}",
                target.title
            )));
            ctx.request_repaint();
            let (bytes, files) = measure(&target.path, &worker_cancel);
            if tx
                .send(CleanupMessage::Measured(Measurement {
                    index,
                    bytes,
                    files,
                }))
                .is_err()
            {
                break;
            }
            ctx.request_repaint();
        }
        let _ = tx.send(CleanupMessage::Done);
        ctx.request_repaint();
    });

    CleanupJob { rx, cancel }
}

fn measure(root: &Path, cancel: &AtomicBool) -> (u64, u64) {
    let mut bytes = 0u64;
    let mut files = 0u64;
    let mut stack = vec![root.to_path_buf()];
    let mut guard = 0u32;

    while let Some(dir) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        guard += 1;
        if guard % 256 == 0 && cancel.load(Ordering::Relaxed) {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                if !is_reparse_point(&entry) {
                    stack.push(entry.path());
                }
            } else if file_type.is_file() {
                if let Ok(meta) = entry.metadata() {
                    bytes = bytes.saturating_add(meta.len());
                    files += 1;
                }
            }
        }
    }
    (bytes, files)
}

#[cfg(windows)]
fn is_reparse_point(entry: &std::fs::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    entry
        .metadata()
        .map(|meta| meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_reparse_point(_entry: &std::fs::DirEntry) -> bool {
    false
}

/// Empties a cache folder, either into the Recycle Bin or permanently.
pub fn clear(path: &Path, permanent: bool) -> Result<(u64, u64), String> {
    let entries = std::fs::read_dir(path).map_err(|err| err.to_string())?;
    let mut bytes = 0u64;
    let mut count = 0u64;
    let mut failures = 0u64;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        match crate::platform::delete_item(&entry_path, permanent) {
            Ok(()) => {
                bytes = bytes.saturating_add(size);
                count += 1;
            }
            Err(_) => failures += 1,
        }
    }
    if count == 0 && failures > 0 {
        return Err("Nothing could be removed — the files may be in use.".to_string());
    }
    Ok((bytes, count))
}
