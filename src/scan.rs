use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use rayon::prelude::*;

use crate::model::{Tree, TreeNode};

#[derive(Debug, Default)]
pub struct ScanProgress {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub errors: AtomicU64,
    pub skipped: AtomicU64,
    pub current: Mutex<String>,
    pub done: AtomicBool,
}

#[derive(Clone, Debug, Default)]
pub struct ProgressSnapshot {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub errors: u64,
    pub skipped: u64,
    pub current: String,
    pub done: bool,
}

impl ScanProgress {
    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            files: self.files.load(Ordering::Relaxed),
            dirs: self.dirs.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            skipped: self.skipped.load(Ordering::Relaxed),
            current: self.current.lock().clone(),
            done: self.done.load(Ordering::Relaxed),
        }
    }
}

pub fn scan_path(
    root: PathBuf,
    label: String,
    progress: Arc<ScanProgress>,
    cancel: Arc<AtomicBool>,
) -> Result<Tree, String> {
    scan_path_with(root, label, progress, cancel, ScanOptions::default())
}

#[derive(Clone, Copy, Debug)]
pub struct ScanOptions {
    /// Open files to detect NTFS hardlinks so shared bytes count only once.
    pub detect_hardlinks: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            detect_hardlinks: true,
        }
    }
}

pub fn scan_path_with(
    root: PathBuf,
    label: String,
    progress: Arc<ScanProgress>,
    cancel: Arc<AtomicBool>,
    options: ScanOptions,
) -> Result<Tree, String> {
    let started = Instant::now();
    let metadata = fs::metadata(&root).map_err(|err| format!("Cannot open {}: {err}", root.display()))?;
    if !metadata.is_dir() {
        return Err(format!("{} is not a folder", root.display()));
    }

    let context = ScanContext {
        options,
        seen_links: Default::default(),
        hardlinked_bytes: AtomicU64::new(0),
    };
    let mut root_tree = scan_dir(&root, 0, &progress, &cancel, &context);
    if cancel.load(Ordering::Relaxed) {
        return Err("Scan cancelled".to_string());
    }
    root_tree.name = label.clone();
    root_tree.sort_children();
    let errors = progress.errors.load(Ordering::Relaxed);
    progress.done.store(true, Ordering::Relaxed);
    let elapsed = started.elapsed();
    let hardlinked_bytes = context.hardlinked_bytes.load(Ordering::Relaxed);
    let mut tree = Tree::from_tree(root_tree, root, label, elapsed, errors, false);
    tree.hardlinked_bytes = hardlinked_bytes;
    Ok(tree)
}

/// Shared state for one scan: which hardlinked file ids were already counted.
struct ScanContext {
    options: ScanOptions,
    seen_links: Mutex<std::collections::HashSet<u64>>,
    hardlinked_bytes: AtomicU64,
}

/// Below this size a duplicated link is not worth an extra file open.
const HARDLINK_MIN_BYTES: u64 = 64 * 1024;

fn effective_file_size(path: &Path, metadata: &fs::Metadata, ctx: &ScanContext) -> (u64, bool) {
    // Sparse, compressed and cloud-placeholder files occupy less (or nothing)
    // on disk than their logical length claims.
    let size = allocated_size(path, metadata).unwrap_or_else(|| metadata.len());
    if ctx.options.detect_hardlinks && size >= HARDLINK_MIN_BYTES {
        if let Some(key) = hardlink_key(path) {
            if !ctx.seen_links.lock().insert(key) {
                ctx.hardlinked_bytes.fetch_add(size, Ordering::Relaxed);
                return (size, true);
            }
        }
    }
    (size, false)
}

/// Identifies the underlying file (volume + file index) when it has more than
/// one link. `std`'s by-handle metadata accessors are still unstable, so the
/// Win32 call is made directly.
#[cfg(windows)]
fn hardlink_key(path: &Path) -> Option<u64> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};

    let file = fs::File::open(path).ok()?;
    let handle = HANDLE(file.as_raw_handle() as _);
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(handle, &mut info) }.ok()?;
    if info.nNumberOfLinks <= 1 {
        return None;
    }
    let index = ((info.nFileIndexHigh as u64) << 32) | u64::from(info.nFileIndexLow);
    let volume = u64::from(info.dwVolumeSerialNumber);
    Some(index ^ volume.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(17))
}

#[cfg(not(windows))]
fn hardlink_key(_path: &Path) -> Option<u64> {
    None
}

#[cfg(windows)]
fn allocated_size(path: &Path, metadata: &fs::Metadata) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::MetadataExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetCompressedFileSizeW;

    const FILE_ATTRIBUTE_SPARSE_FILE: u32 = 0x0200;
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;

    let attributes = metadata.file_attributes();
    if attributes
        & (FILE_ATTRIBUTE_SPARSE_FILE | FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        == 0
    {
        return None;
    }

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut high = 0u32;
    let low = unsafe { GetCompressedFileSizeW(PCWSTR(wide.as_ptr()), Some(&mut high)) };
    if low == u32::MAX && high == u32::MAX {
        return None;
    }
    Some(((high as u64) << 32) | u64::from(low))
}

#[cfg(not(windows))]
fn allocated_size(_path: &Path, _metadata: &fs::Metadata) -> Option<u64> {
    None
}

struct PendingDir {
    name: String,
    path: PathBuf,
    mtime: i64,
}

fn scan_dir(
    path: &Path,
    depth: u16,
    progress: &Arc<ScanProgress>,
    cancel: &Arc<AtomicBool>,
    ctx: &ScanContext,
) -> TreeNode {
    let visited = progress.dirs.fetch_add(1, Ordering::Relaxed);
    if visited % 64 == 0 {
        *progress.current.lock() = path.display().to_string();
    }
    if cancel.load(Ordering::Relaxed) {
        return TreeNode::dir(path_name(path), 0, Vec::new());
    }

    let mut files: Vec<TreeNode> = Vec::new();
    let mut dirs: Vec<PendingDir> = Vec::new();
    let mut local_bytes = 0u64;
    let mut local_files = 0u64;
    let mut local_errors = 0u64;
    let mut local_skipped = 0u64;

    let read_dir = match fs::read_dir(path) {
        Ok(iter) => iter,
        Err(_) => {
            progress.errors.fetch_add(1, Ordering::Relaxed);
            return TreeNode::dir(path_name(path), 0, Vec::new());
        }
    };

    for (index, entry) in read_dir.enumerate() {
        if index % 512 == 0 && cancel.load(Ordering::Relaxed) {
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                local_errors += 1;
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(kind) => kind,
            Err(_) => {
                local_errors += 1;
                continue;
            }
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => {
                local_errors += 1;
                continue;
            }
        };
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);

        if file_type.is_dir() {
            if is_reparse_point(&metadata) {
                local_skipped += 1;
                continue;
            }
            if depth >= 512 {
                local_skipped += 1;
                continue;
            }
            dirs.push(PendingDir {
                name,
                path: entry.path(),
                mtime,
            });
        } else if file_type.is_file() {
            let (size, shared) = effective_file_size(&entry.path(), &metadata, ctx);
            local_bytes = local_bytes.saturating_add(size);
            local_files += 1;
            files.push(if shared {
                TreeNode::shared_file(name, size, mtime)
            } else {
                TreeNode::file(name, size, mtime)
            });
        } else if file_type.is_symlink() {
            local_skipped += 1;
        } else {
            local_skipped += 1;
        }
    }

    let mut children: Vec<TreeNode> = if dirs.len() > 1 && depth < 64 {
        dirs.into_par_iter()
            .map(|dir| scan_dir(&dir.path, depth + 1, progress, cancel, ctx))
            .collect()
    } else {
        dirs.into_iter()
            .map(|dir| scan_dir(&dir.path, depth + 1, progress, cancel, ctx))
            .collect()
    };
    children.append(&mut files);

    progress.files.fetch_add(local_files, Ordering::Relaxed);
    progress.bytes.fetch_add(local_bytes, Ordering::Relaxed);
    progress.errors.fetch_add(local_errors, Ordering::Relaxed);
    progress.skipped.fetch_add(local_skipped, Ordering::Relaxed);

    TreeNode::dir(path_name(path), 0, children)
}

fn path_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

pub fn scan_time_label(tree: &Tree) -> String {
    crate::util::format_duration(tree.elapsed)
}

// ---------------------------------------------------------------------------
// Demo volume
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 16
    }

    fn range(&mut self, low: u64, high: u64) -> u64 {
        if high <= low {
            low
        } else {
            low + self.next() % (high - low)
        }
    }
}

const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * 1024 * 1024;

fn demo_time(days_ago: u64) -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(1_760_000_000);
    now.saturating_sub(days_ago * 86_400) as i64
}

fn file_mb(name: &str, mb: u64, days_ago: u64) -> TreeNode {
    TreeNode::file(name, mb * MB, demo_time(days_ago))
}

fn file_gb(name: &str, gb: f64, days_ago: u64) -> TreeNode {
    TreeNode::file(name, (gb * GB as f64) as u64, demo_time(days_ago))
}

fn dir(name: &str, days_ago: u64, children: Vec<TreeNode>) -> TreeNode {
    TreeNode::dir(name, demo_time(days_ago), children)
}

fn photos(prefix: &str, count: u64, low_mb: u64, high_mb: u64, days: u64, rng: &mut Rng) -> Vec<TreeNode> {
    (0..count)
        .map(|index| {
            let size = rng.range(low_mb, high_mb);
            file_mb(
                &format!("{prefix}_{:04}.jpg", index + 1),
                size,
                days + index % 120,
            )
        })
        .collect()
}

fn music(count: u64, rng: &mut Rng) -> Vec<TreeNode> {
    let artists = [
        "Aurora Fields",
        "Northbound",
        "The Paper Kites",
        "Low Light",
        "Coastline",
        "Mono No Aware",
    ];
    let titles = [
        "Golden Hour",
        "Static Bloom",
        "Polaroid",
        "Night Drive",
        "Slow Motion",
        "Afterglow",
        "Paper Planes",
        "Blue Hour",
        "Sundial",
        "Drift",
    ];
    (0..count)
        .map(|index| {
            let artist = artists[index as usize % artists.len()];
            let title = titles[(index as usize * 3) % titles.len()];
            let size = rng.range(5, 15);
            file_mb(
                &format!("{artist} - {title} {:02}.flac", index + 1),
                size,
                rng.range(5, 900),
            )
        })
        .collect()
}

fn code_files(count: u64, rng: &mut Rng) -> Vec<TreeNode> {
    let names = [
        "app.rs",
        "renderer.rs",
        "scanner.rs",
        "model.rs",
        "theme.rs",
        "main.rs",
        "lib.rs",
        "utils.rs",
        "tests.rs",
        "build.rs",
    ];
    (0..count)
        .map(|index| {
            let name = names[index as usize % names.len()];
            TreeNode::file(
                format!("{name}.{index}.rs"),
                rng.range(4, 480) * 1024,
                rng.range(10, 500) as i64,
            )
        })
        .collect()
}

fn games() -> Vec<TreeNode> {
    let mut rng = Rng(0x51A7_2D09);
    let titles = [
        ("Cyberpunk 2077", 78.0, 24.8),
        ("Baldur's Gate 3", 122.0, 18.2),
        ("Elden Ring", 49.0, 12.4),
        ("Grand Theft Auto V", 105.0, 16.8),
        ("Red Dead Redemption 2", 116.0, 22.1),
        ("Horizon Zero Dawn", 68.0, 14.6),
        ("Forza Horizon 5", 103.0, 19.8),
        ("Starfield", 118.0, 21.4),
    ];
    let mut steam_games = Vec::new();
    for (title, gb, biggest) in titles {
        let base = title.to_ascii_lowercase().replace([' ', '\''], "_");
        let mut files = Vec::new();
        let archive_count = 6;
        let mut remaining = gb - biggest;
        for index in 0..archive_count {
            let size = if index == 0 {
                biggest
            } else {
                remaining / (archive_count as f64 - 1.0)
            };
            files.push(file_gb(&format!("{base}_data_{:02}.archive", index + 1), size, rng.range(30, 700)));
        }
        files.push(file_mb(&format!("{base}.exe"), rng.range(20, 80), rng.range(30, 700)));
        files.push(file_mb("steam_api64.dll", 9, 900));
        steam_games.push(dir(title, rng.range(30, 700), files));
    }
    let steam = dir("Steam", 4, steam_games);

    let epic = dir(
        "Epic Games",
        12,
        vec![
            dir(
                "Alan Wake 2",
                30,
                vec![
                    file_gb("AlanWake2/Data/pakchunk0.utoc", 31.4, 60),
                    file_gb("AlanWake2/Data/pakchunk0.ucas", 22.8, 60),
                    file_gb("AlanWake2/Data/pakchunk1.ucas", 8.2, 60),
                ],
            ),
            dir(
                "Control Ultimate Edition",
                90,
                vec![
                    file_gb("Control/data/packages/global.rcs", 16.4, 120),
                    file_gb("Control/data/packages/tex.rcs", 11.2, 120),
                ],
            ),
            dir(
                "Satisfactory",
                60,
                vec![file_gb("Satisfactory/FactoryGame/Content/pak0.pak", 14.1, 200)],
            ),
            dir(
                "Rocket League",
                200,
                vec![file_gb("RocketLeague/TAGame/CookedPCConsole/pak0.pak", 9.2, 300)],
            ),
        ],
    );
    vec![steam, epic, dir("Game Saves", 8, vec![file_mb("save_slot_01.sav", 84, 8), file_mb("profile.bin", 12, 8)])]
}

fn media_vault() -> Vec<TreeNode> {
    let mut rng = Rng(0x9B2C_0FF1);
    let movies = [
        ("Dune - Part Two", 8.4, "mkv"),
        ("Blade Runner 2049", 7.2, "mkv"),
        ("Interstellar", 6.8, "mkv"),
        ("The Batman", 5.9, "mp4"),
        ("Oppenheimer", 7.7, "mkv"),
        ("Arrival", 4.6, "mkv"),
        ("Sicario", 4.1, "mp4"),
        ("The Grand Budapest Hotel", 3.2, "mkv"),
        ("Whiplash", 3.0, "mp4"),
        ("La La Land", 3.6, "mkv"),
        ("Mad Max - Fury Road", 4.4, "mkv"),
        ("Everything Everywhere All at Once", 4.8, "mp4"),
    ];
    let mut files = Vec::new();
    for (index, (title, gb, ext)) in movies.iter().enumerate() {
        files.push(file_gb(
            &format!("{title} ({}) [2160p].{ext}", 1990 + index * 3),
            *gb,
            rng.range(50, 900),
        ));
    }
    for index in 0..30 {
        files.push(file_gb(
            &format!("Concert - Live Session {:02}.mkv", index + 1),
            (rng.range(14, 36) as f64) / 10.0,
            rng.range(100, 700),
        ));
    }
    vec![
        dir("Films", 20, files),
        dir(
            "Series",
            25,
            vec![
                dir("Foundation S03", 40, (0..8).map(|i| file_gb(&format!("Foundation.S03E{:02}.2160p.mkv", i + 1), 4.6, 40)).collect()),
                dir("Severance S02", 90, (0..9).map(|i| file_gb(&format!("Severance.S02E{:02}.2160p.mkv", i + 1), 3.8, 90)).collect()),
                dir("The Expanse S06", 140, (0..6).map(|i| file_gb(&format!("The.Expanse.S06E{:02}.1080p.mkv", i + 1), 2.9, 140)).collect()),
            ],
        ),
    ]
}

pub fn demo_tree() -> Tree {
    let mut rng = Rng(0xC0FF_EE42);

    // Videos: large masters plus travel clips.
    let mut video_files = vec![
        file_gb("Iceland - Highlands 8K master.mov", 28.4, 9),
        file_gb("Dolomites - Sunrise 6K master.mov", 21.8, 14),
        file_gb("Japan - Tokyo Nights 4K.mov", 18.2, 22),
        file_gb("Patagonia - Expedition Cut v12.mov", 26.9, 3),
        file_gb("Commercial - Aurora Spot FINAL.mov", 12.6, 5),
        file_gb("Behind the Scenes - Studio.mov", 8.4, 40),
        file_gb("Drone Reel 2026.mp4", 6.2, 1),
        file_gb("Color Grade Reference.mp4", 3.4, 12),
    ];
    for index in 0..22 {
        video_files.push(file_gb(
            &format!("Travel Clip {:02} - {}.mp4", index + 1, ["Lisbon", "Kyoto", "Oslo", "Marrakesh", "Cape Town"][index as usize % 5]),
            (rng.range(8, 42) as f64) / 10.0,
            rng.range(10, 400),
        ));
    }
    let videos = dir("Videos", 2, video_files);

    let creative = dir(
        "Creative",
        6,
        vec![
            dir(
                "DaVinci Resolve",
                4,
                vec![
                    file_gb("CacheClip/ProxyMedia_01.bin", 18.4, 4),
                    file_gb("CacheClip/OptimizedMedia_01.bin", 12.8, 4),
                    file_mb("Resolve Project Library.db", 640, 4),
                    file_gb("Gallery Stills/stills_4k.dng", 4.2, 11),
                ],
            ),
            dir(
                "After Effects",
                9,
                vec![
                    file_gb("Media Cache/ae_cache_01.bin", 9.6, 9),
                    file_mb("Motion Graphics - Titles.aep", 860, 9),
                    file_mb("Lower Thirds Pack.aep", 420, 20),
                ],
            ),
            dir(
                "Blender",
                15,
                vec![
                    file_gb("Projects/Environment Kit.blend", 3.8, 15),
                    file_gb("Renders/Sequence_0101-0420.exr", 7.4, 18),
                    file_gb("Assets/Scanned Rocks.pack", 5.1, 18),
                ],
            ),
        ],
    );

    let mut picture_files = photos("DSC", 220, 8, 38, 30, &mut rng);
    picture_files.extend(photos("RAW", 48, 42, 96, 60, &mut rng));
    picture_files.push(file_gb("Panorama - Alps.tif", 2.4, 50));
    picture_files.push(file_gb("Portfolio Print Masters.psb", 3.8, 28));
    let pictures = dir("Pictures", 1, picture_files);

    let mut music_files = music(160, &mut rng);
    music_files.push(file_gb("Sample Packs/Orchestral Tools Vol 1.wav", 6.4, 300));
    music_files.push(file_gb("Ableton Live Set - Album Master.als", 1.8, 80));
    let music_dir = dir("Music", 3, music_files);

    let documents = dir(
        "Documents",
        2,
        vec![
            dir(
                "Invoices 2026",
                40,
                (0..36).map(|i| file_mb(&format!("INV-2026-{:03}.pdf", i + 1), rng.range(120, 900) / 100, 40 + i)).collect(),
            ),
            dir(
                "Contracts",
                120,
                (0..18).map(|i| file_mb(&format!("Contract_{:02}_signed.pdf", i + 1), rng.range(400, 1600) / 100, 120 + i)).collect(),
            ),
            file_gb("Brand Guidelines 2026 - Final.indd", 1.2, 50),
            file_gb("Archive - Project Documentation.zip", 4.6, 200),
        ],
    );

    let downloads = dir(
        "Downloads",
        1,
        vec![
            file_gb("Windows11_24H2_English_x64.iso", 6.8, 40),
            file_gb("DaVinci_Resolve_Studio_20.dmg", 4.2, 30),
            file_gb("Adobe_Creative_Cloud_Installer.exe", 1.1, 10),
            file_gb("Game_Installer_2026.zip", 12.4, 12),
            file_gb("Ubuntu-26.04-desktop-amd64.iso", 5.2, 16),
            file_mb("invoice_scan_batch.pdf", 284, 2),
        ],
    );

    let appdata = dir(
        "AppData",
        0,
        vec![
            dir("Local/Temp", 0, vec![file_gb("Adobe Premiere Temp/cache_01.tmp", 6.8, 0), file_gb("CrashDumps/renderer.dmp", 2.4, 1)]),
            dir("Local/Packages", 0, vec![file_gb("Microsoft.WindowsStore/cache.bin", 1.8, 0), file_gb("Spotify/Storage/cache.bin", 2.2, 0)]),
            dir("Roaming", 0, vec![file_gb("Slack/Cache/Cache_Data.bin", 3.4, 0), file_gb("Discord/Cache/code_cache.bin", 1.9, 1)]),
        ],
    );

    let users_alex = dir(
        "Alex",
        0,
        vec![
            videos,
            creative,
            pictures,
            music_dir,
            documents,
            downloads,
            appdata,
            dir(
                "Projects",
                3,
                vec![
                    dir("space-analyzer", 0, code_files(140, &mut rng)),
                    dir("render-engine", 20, code_files(90, &mut rng)),
                    dir("website-2026", 12, code_files(60, &mut rng)),
                ],
            ),
            dir(
                "Virtual Machines",
                30,
                vec![
                    file_gb("Windows 11 Dev/Windows 11 Dev.vhdx", 42.8, 30),
                    file_gb("Ubuntu Build Server/Ubuntu Build Server.vmdk", 18.6, 25),
                    file_gb("Snapshots/Windows 11 Dev - clean.vhd", 12.2, 30),
                ],
            ),
        ],
    );
    let users = dir(
        "Users",
        0,
        vec![
            users_alex,
            dir("Public", 40, vec![file_gb("Public Documents/Shared Assets.zip", 2.8, 40)]),
            dir("Shared", 38, vec![file_gb("Media/Team Footage 2026.mov", 8.2, 38), file_gb("Media/Brand Assets.zip", 3.4, 38)]),
        ],
    );

    let backups = dir(
        "Backups",
        6,
        vec![
            dir("2026-11 Full", 6, vec![file_gb("Workstation.vhdx", 42.4, 6), file_gb("FileHistory.zip", 8.6, 6)]),
            dir("2026-10 Full", 36, vec![file_gb("Workstation.vhdx", 39.8, 36), file_gb("FileHistory.zip", 7.9, 36)]),
            dir("2026-09 Full", 66, vec![file_gb("Workstation.vhdx", 38.2, 66), file_gb("FileHistory.zip", 7.4, 66)]),
            dir("2026-08 Full", 96, vec![file_gb("Workstation.vhdx", 37.1, 96), file_gb("FileHistory.zip", 7.1, 96)]),
            dir("2026-07 Full", 126, vec![file_gb("Workstation.vhdx", 35.4, 126)]),
        ],
    );

    let mut program_files = Vec::new();
    let vendors = [
        ("Adobe", 31.2, 160),
        ("Microsoft Office", 8.4, 220),
        ("Blackmagic Design", 12.6, 90),
        ("Blender Foundation", 4.2, 140),
        ("Epic Games", 18.4, 60),
        ("JetBrains", 6.8, 200),
        ("Mozilla Firefox", 1.8, 20),
    ];
    for (vendor, gb, age) in vendors {
        let mut entries = Vec::new();
        let count = if gb > 10.0 { 40 } else { 16 };
        for index in 0..count {
            entries.push(file_mb(
                &format!("{}/bin/component_{:03}.dll", vendor, index + 1),
                rng.range(20, 380),
                age,
            ));
        }
        entries.push(file_gb(&format!("{vendor}/resources/media.pak"), gb * 0.45, age));
        program_files.push(dir(vendor, age, entries));
    }

    let windows = dir(
        "Windows",
        5,
        vec![
            dir(
                "WinSxS",
                5,
                (0..80)
                    .map(|i| file_mb(&format!("amd64_component_{:04}/component.dll", i + 1), rng.range(18, 220), 5))
                    .collect(),
            ),
            dir(
                "System32",
                5,
                (0..90)
                    .map(|i| file_mb(&format!("driver_{:03}.sys", i + 1), rng.range(40, 180), 5))
                    .collect(),
            ),
            dir("Installer", 200, (0..30).map(|i| file_mb(&format!("{:08}.msp", i * 17), rng.range(60, 320), 200)).collect()),
            dir("Logs", 1, (0..24).map(|i| file_mb(&format!("CBS/CbsPersist_{:02}.log", i + 1), rng.range(20, 90), 1 + i)).collect()),
            file_gb("MEMORY.DMP", 2.8, 8),
            file_gb("Installer Cache/Windows Update.bin", 3.4, 15),
        ],
    );

    let program_data = dir(
        "ProgramData",
        4,
        vec![
            dir("Docker", 4, vec![file_gb("Desktop/Data/docker_data.vhdx", 18.6, 4)]),
            dir("Package Cache", 60, (0..40).map(|i| file_mb(&format!("package_{:03}.msi", i + 1), rng.range(80, 620), 60)).collect()),
            dir("NVIDIA Corporation", 20, vec![file_gb("Downloader/installer_cache.bin", 2.4, 20)]),
            file_gb("Microsoft/Windows Defender/Definition Updates.bin", 1.8, 0),
        ],
    );

    let media = dir("Media Vault", 12, media_vault());

    let mut root_children = vec![
        users,
        dir("Games", 2, games()),
        media,
        backups,
        dir("Program Files", 12, program_files),
        windows,
        program_data,
        dir(
            "System Volume Information",
            7,
            vec![file_gb("{3808876b}.vhdx", 6.8, 7)],
        ),
    ];
    root_children.push(dir(
        "Studio Archive",
        80,
        vec![
            file_gb("2025 Showreel - Master.mov", 9.8, 80),
            file_gb("Client Deliverables.zip", 18.4, 80),
            file_gb("RAW Archive - Q4.tar", 22.6, 80),
            dir("Audio Stems", 80, (0..40).map(|i| file_mb(&format!("stem_{:02}.wav", i + 1), rng.range(40, 180), 80)).collect()),
        ],
    ));

    let root = TreeNode::dir("Demo volume", demo_time(0), root_children);
    let mut root = root;
    root.sort_children();
    let elapsed = std::time::Duration::from_millis(8_420);
    Tree::from_tree(
        root,
        PathBuf::from("Demo volume"),
        "Demo volume".to_string(),
        elapsed,
        0,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn scans_a_small_fixture() {
        let base = std::env::temp_dir().join(format!(
            "spaceanalyzer-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(base.join("nested")).unwrap();
        fs::write(base.join("hello.txt"), b"hello world").unwrap();
        let mut f = fs::File::create(base.join("nested/data.bin")).unwrap();
        f.write_all(&vec![7u8; 4096]).unwrap();
        drop(f);

        let progress = Arc::new(ScanProgress::default());
        let cancel = Arc::new(AtomicBool::new(false));
        let tree = scan_path(
            base.clone(),
            "Fixture".to_string(),
            progress.clone(),
            cancel,
        )
        .unwrap();
        assert_eq!(tree.total_files, 2);
        assert_eq!(tree.total_dirs, 1);
        assert!(tree.total_size >= 4107);
        assert_eq!(tree.root_label, "Fixture");
        let base_name = base.file_name().unwrap().to_string_lossy().to_string();
        assert!(tree.path(tree.root).ends_with(&base_name));
        fs::remove_dir_all(base).ok();
    }

    #[test]
    fn demo_is_nonempty_and_large() {
        let tree = demo_tree();
        assert!(tree.demo);
        assert!(tree.total_size > 500 * GB);
        assert!(tree.total_files > 500);
        assert!(tree.total_dirs > 30);
        assert!(!tree.ext_stats.is_empty());
        assert!(tree.file_ids.len() as u64 == tree.total_files);
    }

    #[test]
    fn category_is_only_used_for_files() {
        let tree = demo_tree();
        for id in &tree.file_ids {
            let node = tree.node(*id);
            assert!(!node.is_dir);
            assert!(node.category.index() < crate::model::Category::ALL.len());
        }
    }
}
