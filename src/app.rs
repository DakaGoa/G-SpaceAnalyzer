use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use eframe::egui;
use egui::{
    Align, Align2, Color32, CursorIcon, Frame, Id, Layout, Margin, Order, Pos2, Rect, Response,
    RichText, Rounding, ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2, ViewportCommand,
};
use egui_phosphor::regular as ph;

use crate::dupes::{self, DupJob, DupResults};
use crate::model::{Category, NodeId, Tree};
use crate::platform::{self, DriveInfo};
use crate::scan::{self, ScanProgress};
use crate::settings::{Settings, ThemeMode};
use crate::theme::{self, Palette};
use crate::util::{format_bytes, format_count, format_duration, format_mtime, format_percent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Overview,
    LargestFiles,
    FileTypes,
    Duplicates,
    Search,
    Cleanup,
    OldFiles,
    Snapshots,
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            View::Overview => "Overview",
            View::LargestFiles => "Largest files",
            View::FileTypes => "File types",
            View::Duplicates => "Duplicates",
            View::Search => "Search",
            View::Cleanup => "Clean up",
            View::OldFiles => "Old files",
            View::Snapshots => "Snapshots",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            View::Overview => ph::CUBE,
            View::LargestFiles => ph::FILES,
            View::FileTypes => ph::CHART_DONUT,
            View::Duplicates => ph::COPY,
            View::Search => ph::MAGNIFYING_GLASS,
            View::Cleanup => ph::TRASH,
            View::OldFiles => ph::CLOCK,
            View::Snapshots => ph::ARCHIVE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Size,
    Name,
    Modified,
    Count,
}

#[derive(Clone, Copy, Debug)]
pub struct SortState {
    pub key: SortKey,
    pub descending: bool,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            key: SortKey::Size,
            descending: true,
        }
    }
}

impl SortState {
    pub fn select(&mut self, key: SortKey) {
        if self.key == key {
            self.descending = !self.descending;
        } else {
            self.key = key;
            self.descending = matches!(key, SortKey::Size | SortKey::Modified);
        }
    }

    pub fn arrow(&self) -> &'static str {
        if self.descending {
            "↓"
        } else {
            "↑"
        }
    }
}

#[derive(Clone, Debug)]
pub struct VolumeInfo {
    pub label: String,
    pub total: u64,
    pub free: u64,
}

#[derive(Clone, Debug)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub kind: ToastKind,
    pub created: f64,
}

#[derive(Clone, Copy, Debug)]
pub enum DeleteTarget {
    Node(NodeId),
    Duplicate { node: NodeId, group: usize },
}

#[derive(Clone, Debug)]
pub enum Modal {
    Settings,
    About,
    Delete(DeleteTarget),
    BulkDelete {
        paths: Vec<PathBuf>,
        bytes: u64,
        label: String,
    },
}

pub enum DupState {
    Idle,
    Running(DupJob),
    Done(DupResults),
    Failed(String),
}

impl Default for DupState {
    fn default() -> Self {
        Self::Idle
    }
}

pub struct ScanJob {
    pub rx: mpsc::Receiver<ScanMessage>,
    pub progress: Arc<ScanProgress>,
    pub cancel: Arc<AtomicBool>,
    pub started: Instant,
    pub root: PathBuf,
    pub label: String,
}

pub enum ScanMessage {
    Done(Box<Tree>),
    Failed(String),
    Cancelled,
}

pub enum Action {
    ScanPath(PathBuf, String),
    CancelScan,
    LoadDemo,
    Select(NodeId),
    Navigate(NodeId),
    Back,
    Up,
    SetView(View),
    ToggleTheme,
    OpenSettings,
    CloseModal,
    Export,
    Reveal(NodeId),
    OpenItem(NodeId),
    Delete(NodeId),
    ConfirmDelete(DeleteTarget),
    CopyPath(NodeId),
    ChooseFolder,
    Rescan,
    FocusSearch,
    ClearSearch,
    StartDuplicates,
    CancelDuplicates,
    DeleteDuplicate { node: NodeId, group: usize },
    SortBy(SortKey),
    SetCategory(Option<Category>),
    CloseDetails,
    DismissToast(u64),
    Toast(String, ToastKind),
    RefreshDisks,
    SetOldYears(u32),
    CycleTreemapColor,
    SaveSnapshot,
    SelectSnapshot(usize),
    CompareSnapshot(usize),
    DeleteSnapshot(usize),
    RefreshSnapshots,
    StartCleanupScan,
    ClearCleanup(usize),
    OpenExternal(PathBuf),
    AdminRescan,
    SetDupMinSize(u64),
    SetOverviewList(bool),
    OpenAbout,
    ToggleDupScope,
    DupKeepAll {
        newest: bool,
    },
    ConfirmBulkDelete {
        paths: Vec<PathBuf>,
        bytes: u64,
        label: String,
    },
}

pub struct App {
    pub(crate) tree: Option<Tree>,
    pub(crate) scan: Option<ScanJob>,
    pub(crate) view: View,
    pub(crate) current: NodeId,
    pub(crate) history: Vec<NodeId>,
    pub(crate) selection: Option<NodeId>,
    pub(crate) search: String,
    pub(crate) search_results: Vec<NodeId>,
    search_computed: String,
    search_changed_at: Option<f64>,
    search_focus: bool,
    pub(crate) sort: SortState,
    pub(crate) largest_ids: Vec<NodeId>,
    pub(crate) settings: Settings,
    pub(crate) palette: Palette,
    pub(crate) disks: Vec<DriveInfo>,
    pub(crate) volume: Option<VolumeInfo>,
    pub(crate) dupes: DupState,
    pub(crate) selected_category: Option<Category>,
    pub(crate) toasts: Vec<Toast>,
    pub(crate) modal: Option<Modal>,
    /// Kind of modal drawn on the previous frame, used to ignore the click
    /// that opened it.
    pub(crate) modal_shown: Option<std::mem::Discriminant<Modal>>,
    pub(crate) generation: u64,
    last_disks_refresh: Instant,
    pub(crate) applied_theme: ThemeMode,
    pub(crate) applied_animations: bool,
    next_toast_id: u64,
    pub(crate) scan_cancel_requested: bool,
    /// Whether the process holds administrator rights (fixed for its lifetime).
    pub(crate) elevated: bool,
    startup_scan: Option<(PathBuf, String)>,
    demo_loaded: bool,
    pub(crate) treemap_hover: Option<usize>,
    pub(crate) treemap_tooltip: Option<(Pos2, String)>,
    pub(crate) ext_sort: SortState,
    pub(crate) search_sort: SortState,
    pub(crate) dup_sort: SortState,
    pub(crate) old_sort: SortState,
    pub(crate) cleanup: Option<CleanupState>,
    pub(crate) snapshots: Vec<crate::snapshots::SnapshotMeta>,
    pub(crate) snapshot_selected: Option<usize>,
    pub(crate) snapshot_diff: Option<Vec<crate::snapshots::DiffRow>>,
    pub(crate) snapshot_note: Option<String>,
    pub(crate) treemap_zoom: Option<f64>,
}

/// Live state for the cleanup-targets view.
pub struct CleanupState {
    pub targets: Vec<crate::cleanup::Target>,
    pub sizes: Vec<Option<(u64, u64)>>,
    pub job: Option<crate::cleanup::CleanupJob>,
    pub progress: String,
}

pub struct Startup {
    pub demo: bool,
    pub scan: Option<PathBuf>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, startup: Startup) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        let mut settings = Settings::load();
        settings.shell_integration = platform::shell_integration_present();
        theme::apply_theme(&cc.egui_ctx, settings.theme, settings.animations);
        let palette = Palette::for_mode(settings.theme);
        let disks = platform::list_drives();
        let startup_scan = startup.scan.map(|path| {
            let label = platform::label_for_path(&path);
            (path, label)
        });
        let applied_theme = settings.theme;
        let applied_animations = settings.animations;
        let mut app = Self {
            tree: None,
            scan: None,
            view: View::Overview,
            current: 0,
            history: Vec::new(),
            selection: None,
            search: String::new(),
            search_results: Vec::new(),
            search_computed: String::new(),
            search_changed_at: None,
            search_focus: false,
            sort: SortState::default(),
            largest_ids: Vec::new(),
            settings,
            palette,
            disks,
            volume: None,
            dupes: DupState::Idle,
            selected_category: None,
            toasts: Vec::new(),
            modal: None,
            modal_shown: None,
            generation: 0,
            last_disks_refresh: Instant::now(),
            applied_theme,
            applied_animations,
            next_toast_id: 1,
            scan_cancel_requested: false,
            elevated: platform::is_elevated(),
            startup_scan,
            demo_loaded: false,
            treemap_hover: None,
            treemap_tooltip: None,
            ext_sort: SortState::default(),
            search_sort: SortState::default(),
            dup_sort: SortState::default(),
            old_sort: SortState::default(),
            cleanup: None,
            snapshots: Vec::new(),
            snapshot_selected: None,
            snapshot_diff: None,
            snapshot_note: None,
            treemap_zoom: None,
        };
        if startup.demo {
            app.load_demo(&cc.egui_ctx);
        } else if let Some((path, label)) = app.startup_scan.take() {
            app.start_scan(&cc.egui_ctx, path, label);
        }
        app
    }

    pub(crate) fn toast(
        &mut self,
        ctx: &egui::Context,
        message: impl Into<String>,
        kind: ToastKind,
    ) {
        let now = ctx.input(|input| input.time);
        let id = self.next_toast_id;
        self.next_toast_id += 1;
        self.toasts.push(Toast {
            id,
            message: message.into(),
            kind,
            created: now,
        });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
        ctx.request_repaint();
    }

    fn load_demo(&mut self, ctx: &egui::Context) {
        if let Some(job) = &self.scan {
            job.cancel.store(true, Ordering::Relaxed);
            self.scan = None;
        }
        let tree = scan::demo_tree();
        let root = tree.root;
        self.largest_ids = tree.largest_files(5000);
        self.tree = Some(tree);
        self.current = root;
        self.history.clear();
        self.selection = None;
        self.search.clear();
        self.search_computed.clear();
        self.search_results.clear();
        self.dupes = DupState::Idle;
        self.view = View::Overview;
        self.generation += 1;
        self.demo_loaded = true;
        self.scan_cancel_requested = false;
        self.volume = Some(VolumeInfo {
            label: "Demo volume".to_string(),
            total: 1536 * 1024 * 1024 * 1024,
            free: 214 * 1024 * 1024 * 1024,
        });
        self.toast(ctx, "Sample volume loaded — explore freely", ToastKind::Info);
    }

    fn start_scan(&mut self, ctx: &egui::Context, path: PathBuf, label: String) {
        if self.scan.is_some() {
            self.toast(ctx, "A scan is already running", ToastKind::Info);
            return;
        }
        if !path.exists() {
            self.toast(ctx, format!("Path not found: {}", path.display()), ToastKind::Error);
            return;
        }
        let progress = Arc::new(ScanProgress::default());
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let worker_progress = progress.clone();
        let worker_cancel = cancel.clone();
        let worker_path = path.clone();
        let worker_label = label.clone();
        let worker_ctx = ctx.clone();
        let worker_hardlinks = self.settings.detect_hardlinks;
        std::thread::spawn(move || {
            let result = scan::scan_path_with(
                worker_path,
                worker_label,
                worker_progress,
                worker_cancel.clone(),
                scan::ScanOptions {
                    detect_hardlinks: worker_hardlinks,
                },
            );
            let message = if worker_cancel.load(Ordering::Relaxed) {
                ScanMessage::Cancelled
            } else {
                match result {
                    Ok(tree) => ScanMessage::Done(Box::new(tree)),
                    Err(err) => {
                        if err.contains("cancelled") {
                            ScanMessage::Cancelled
                        } else {
                            ScanMessage::Failed(err)
                        }
                    }
                }
            };
            let _ = tx.send(message);
            worker_ctx.request_repaint();
        });
        self.scan = Some(ScanJob {
            rx,
            progress,
            cancel,
            started: Instant::now(),
            root: path.clone(),
            label: label.clone(),
        });
        self.scan_cancel_requested = false;
        self.view = View::Overview;
        self.settings.last_path = Some(path.to_string_lossy().into_owned());
        self.settings.save();
        self.demo_loaded = false;
        self.volume = volume_for_path(&path, &self.disks);
        ctx.request_repaint();
    }

    fn refresh_disks(&mut self, ctx: &egui::Context) {
        self.disks = platform::list_drives();
        self.last_disks_refresh = Instant::now();
        if let Some(tree) = &self.tree {
            if !tree.demo {
                self.volume = volume_for_path(&tree.root_path, &self.disks);
            }
        }
        ctx.request_repaint();
    }

    fn poll_scan(&mut self, ctx: &egui::Context) {
        let Some(job) = self.scan.take() else {
            return;
        };
        let mut keep = true;
        while let Ok(message) = job.rx.try_recv() {
            keep = false;
            match message {
                ScanMessage::Done(tree) => {
                    let tree = *tree;
                    self.current = tree.root;
                    self.largest_ids = tree.largest_files(5000);
                    self.selection = None;
                    self.history.clear();
                    self.search.clear();
                    self.search_computed.clear();
                    self.search_results.clear();
                    self.selected_category = None;
                    self.dupes = DupState::Idle;
                    self.volume = volume_for_path(&tree.root_path, &self.disks);
                    if self.volume.is_none() {
                        self.volume = Some(VolumeInfo {
                            label: tree.root_label.clone(),
                            total: tree.total_size,
                            free: 0,
                        });
                    }
                    let duration = tree.elapsed;
                    let files = tree.total_files;
                    let size = tree.total_size;
                    self.tree = Some(tree);
                    self.generation += 1;
                    self.view = View::Overview;
                    self.scan_cancel_requested = false;
                    self.toast(
                        ctx,
                        format!(
                            "Scan complete · {} files · {} in {}",
                            format_count(files),
                            format_bytes(size),
                            format_duration(duration)
                        ),
                        ToastKind::Success,
                    );
                    self.refresh_disks(ctx);
                }
                ScanMessage::Failed(error) => {
                    self.scan_cancel_requested = false;
                    self.toast(ctx, error, ToastKind::Error);
                }
                ScanMessage::Cancelled => {
                    self.scan_cancel_requested = false;
                    self.toast(ctx, "Scan cancelled", ToastKind::Info);
                }
            }
        }
        if keep {
            self.scan = Some(job);
        }
    }

    fn poll_dupes(&mut self, ctx: &egui::Context) {
        let DupState::Running(job) = &self.dupes else {
            return;
        };
        let mut outcome = None;
        while let Ok(message) = job.rx.try_recv() {
            outcome = Some(message);
            break;
        }
        let Some(message) = outcome else {
            return;
        };
        let generation = job.generation;
        match message {
            dupes::DupMessage::Done(results) => {
                if generation != self.generation {
                    self.dupes = DupState::Idle;
                    return;
                }
                let groups = results.groups.len();
                let wasted = results.wasted();
                self.dupes = DupState::Done(*results);
                if groups == 0 {
                    self.toast(ctx, "No duplicates found", ToastKind::Info);
                } else {
                    self.toast(
                        ctx,
                        format!(
                            "Found {} duplicate groups · {} recoverable",
                            format_count(groups as u64),
                            format_bytes(wasted)
                        ),
                        ToastKind::Success,
                    );
                }
            }
            dupes::DupMessage::Failed(error) => {
                self.dupes = DupState::Failed(error.clone());
                self.toast(ctx, error, ToastKind::Error);
            }
            dupes::DupMessage::Cancelled => {
                self.dupes = DupState::Idle;
                self.toast(ctx, "Duplicate search cancelled", ToastKind::Info);
            }
        }
    }

    fn start_duplicates(&mut self, ctx: &egui::Context) {
        if self.demo_loaded || self.tree.as_ref().is_some_and(|tree| tree.demo) {
            self.toast(ctx, "Duplicates are unavailable for sample data", ToastKind::Info);
            return;
        }
        if matches!(self.dupes, DupState::Running(_)) {
            return;
        }
        let Some(tree) = &self.tree else {
            return;
        };
        let min_bytes = self.settings.dup_min_size_mb.max(1) * 1024 * 1024;
        let scope_root = if self.settings.dup_current_folder {
            Some(self.current)
        } else {
            None
        };
        let mut counts: std::collections::HashMap<u64, u32> = std::collections::HashMap::new();
        for &id in &tree.file_ids {
            let node = tree.node(id);
            if node.size < min_bytes {
                continue;
            }
            if let Some(scope) = scope_root {
                if id != scope && !is_descendant(tree, id, scope) {
                    continue;
                }
            }
            *counts.entry(node.size).or_insert(0) += 1;
        }
        let mut candidates = Vec::new();
        for &id in &tree.file_ids {
            let node = tree.node(id);
            if node.size < min_bytes || counts.get(&node.size).copied().unwrap_or(0) < 2 {
                continue;
            }
            if let Some(scope) = scope_root {
                if id != scope && !is_descendant(tree, id, scope) {
                    continue;
                }
            }
            candidates.push(dupes::DupCandidate {
                node: id,
                path: tree.path_buf(id),
                size: node.size,
                modified: node.mtime,
            });
        }
        if candidates.is_empty() {
            self.dupes = DupState::Done(DupResults::default());
            self.toast(ctx, "No duplicate candidates found", ToastKind::Info);
            return;
        }
        let total = candidates.len();
        let generation = self.generation;
        self.dupes = DupState::Running(dupes::spawn(candidates, generation, ctx.clone()));
        self.toast(
            ctx,
            format!("Checking {total} candidate files for duplicates"),
            ToastKind::Info,
        );
    }

    // --- Cleanup targets ---------------------------------------------------

    fn start_cleanup(&mut self, ctx: &egui::Context) {
        let targets = crate::cleanup::targets();
        if targets.is_empty() {
            self.toast(ctx, "No cleanup targets were found on this PC", ToastKind::Info);
            return;
        }
        let job = crate::cleanup::spawn(targets.clone(), ctx.clone());
        self.cleanup = Some(CleanupState {
            sizes: vec![None; targets.len()],
            targets,
            job: Some(job),
            progress: "Measuring…".to_string(),
        });
        ctx.request_repaint();
    }

    fn poll_cleanup(&mut self, ctx: &egui::Context) {
        let mut summary = None;
        {
            let Some(state) = &mut self.cleanup else {
                return;
            };
            let Some(job) = state.job.take() else {
                return;
            };
            let mut done = false;
            let mut progress = None;
            let mut measured = Vec::new();
            while let Ok(message) = job.rx.try_recv() {
                match message {
                    crate::cleanup::CleanupMessage::Progress(text) => progress = Some(text),
                    crate::cleanup::CleanupMessage::Measured(entry) => measured.push(entry),
                    crate::cleanup::CleanupMessage::Done => done = true,
                }
            }
            for entry in measured {
                if let Some(slot) = state.sizes.get_mut(entry.index) {
                    *slot = Some((entry.bytes, entry.files));
                }
            }
            if let Some(text) = progress {
                state.progress = text;
            }
            if done {
                state.progress = "Measured".to_string();
                let total: u64 = state.sizes.iter().flatten().map(|entry| entry.0).sum();
                summary = Some((total, state.targets.len()));
            } else {
                state.job = Some(job);
            }
        }
        if let Some((total, count)) = summary {
            self.toast(
                ctx,
                format!(
                    "{} reclaimable across {} targets",
                    format_bytes(total),
                    format_count(count as u64)
                ),
                ToastKind::Success,
            );
        }
    }

    fn clear_cleanup(&mut self, ctx: &egui::Context, index: usize) {
        let Some(state) = &self.cleanup else {
            return;
        };
        let Some(target) = state.targets.get(index) else {
            return;
        };
        let path = target.path.clone();
        let title = target.title;
        let permanent = self.settings.delete_permanently;
        match crate::cleanup::clear(&path, permanent) {
            Ok((bytes, count)) => {
                let message = if permanent {
                    format!(
                        "{} freed from {} · {} items erased permanently",
                        format_bytes(bytes),
                        title,
                        format_count(count)
                    )
                } else {
                    format!(
                        "{} freed from {} · {} items moved to the Recycle Bin",
                        format_bytes(bytes),
                        title,
                        format_count(count)
                    )
                };
                self.toast(ctx, message, ToastKind::Success);
                self.start_cleanup(ctx);
            }
            Err(error) => self.toast(ctx, error, ToastKind::Error),
        }
    }

    // --- Snapshots ---------------------------------------------------------

    fn save_snapshot(&mut self, ctx: &egui::Context) {
        let Some(tree) = &self.tree else {
            self.toast(ctx, "Scan a drive before saving a snapshot", ToastKind::Info);
            return;
        };
        if tree.demo {
            self.toast(ctx, "Sample data cannot be saved as a snapshot", ToastKind::Info);
            return;
        }
        let label = format!(
            "{} · {}",
            tree.root_label,
            chrono::Local::now().format("%d %b %H:%M")
        );
        let compact = self.settings.snapshots_compact;
        let result = crate::snapshots::save(tree, &label, compact);
        match result {
            Ok(path) => {
                self.snapshots = crate::snapshots::list();
                let size = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
                self.snapshot_note = Some(format!(
                    "Saved {} · {}{}",
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    crate::util::format_bytes(size),
                    if compact { " · compact" } else { "" }
                ));
                self.toast(ctx, "Snapshot saved", ToastKind::Success);
            }
            Err(error) => self.toast(
                ctx,
                format!("Could not save the snapshot: {error}"),
                ToastKind::Error,
            ),
        }
    }

    fn compare_snapshot(&mut self, ctx: &egui::Context, index: usize) {
        let Some(meta) = self.snapshots.get(index).cloned() else {
            return;
        };
        let Some(tree) = &self.tree else {
            self.toast(ctx, "Scan a drive first", ToastKind::Info);
            return;
        };
        match crate::snapshots::load(&meta.path) {
            Ok(previous) => {
                let rows = crate::snapshots::diff(&previous, tree);
                self.snapshot_diff = Some(rows);
                self.snapshot_selected = Some(index);
            }
            Err(error) => self.toast(
                ctx,
                format!("Could not read that snapshot: {error}"),
                ToastKind::Error,
            ),
        }
    }

    fn delete_snapshot(&mut self, ctx: &egui::Context, index: usize) {
        let Some(meta) = self.snapshots.get(index).cloned() else {
            return;
        };
        match crate::snapshots::delete(&meta.path) {
            Ok(()) => {
                self.snapshots = crate::snapshots::list();
                self.snapshot_selected = None;
                self.snapshot_diff = None;
                self.toast(ctx, "Snapshot deleted", ToastKind::Info);
            }
            Err(error) => self.toast(
                ctx,
                format!("Could not delete that snapshot: {error}"),
                ToastKind::Error,
            ),
        }
    }

    // --- Bulk duplicate removal --------------------------------------------

    fn plan_dup_bulk(&mut self, ctx: &egui::Context, newest: bool) {
        if !matches!(self.dupes, DupState::Done(_)) {
            self.toast(ctx, "Run the duplicate finder first", ToastKind::Info);
            return;
        }
        let mut paths = Vec::new();
        let mut bytes = 0u64;
        if let DupState::Done(results) = &self.dupes {
            for group in &results.groups {
                if group.files.len() < 2 {
                    continue;
                }
                let keep = if newest {
                    group.files.iter().max_by_key(|file| file.modified)
                } else {
                    group.files.iter().min_by_key(|file| file.modified)
                };
                let Some(keep) = keep else {
                    continue;
                };
                for file in &group.files {
                    if file.path != keep.path {
                        paths.push(file.path.clone());
                        bytes = bytes.saturating_add(file.size);
                    }
                }
            }
        }
        if paths.is_empty() {
            self.toast(ctx, "Nothing to remove", ToastKind::Info);
            return;
        }
        self.modal = Some(Modal::BulkDelete {
            paths,
            bytes,
            label: if newest {
                "Keeping the newest copy of every duplicate".to_string()
            } else {
                "Keeping the oldest copy of every duplicate".to_string()
            },
        });
    }

    pub(crate) fn bulk_delete(&mut self, ctx: &egui::Context, paths: Vec<PathBuf>, bytes: u64, label: String) {
        let mut removed = 0u64;
        let mut failures = 0u64;
        let permanent = self.settings.delete_permanently;
        for path in &paths {
            match platform::delete_item(path, permanent) {
                Ok(()) => {
                    removed += 1;
                    if let Some(tree) = &mut self.tree {
                        tree.remove_path(path);
                    }
                }
                Err(_) => failures += 1,
            }
        }
        if let Some(tree) = &mut self.tree {
            self.largest_ids = tree.largest_files(5000);
        }
        if let DupState::Done(results) = &mut self.dupes {
            results.remove_paths(&paths);
        }
        self.generation += 1;
        let message = if failures == 0 {
            format!(
                "{label} · {} freed from {} files",
                format_bytes(bytes),
                format_count(removed)
            )
        } else {
            format!(
                "{label} · {} files removed, {} failed",
                format_count(removed),
                format_count(failures)
            )
        };
        self.toast(
            ctx,
            message,
            if failures == 0 {
                ToastKind::Success
            } else {
                ToastKind::Error
            },
        );
        self.refresh_disks(ctx);
    }

    fn update_search(&mut self, ctx: &egui::Context) {
        let query = self.search.trim().to_string();
        if query.is_empty() {
            self.search_results.clear();
            self.search_computed.clear();
            if self.view == View::Search {
                self.view = View::Overview;
            }
            return;
        }
        let now = ctx.input(|input| input.time);
        if self.search_computed == query {
            return;
        }
        if let Some(changed) = self.search_changed_at {
            if now - changed < 0.16 {
                ctx.request_repaint_after(Duration::from_millis(170));
                return;
            }
        }
        let Some(tree) = &self.tree else {
            return;
        };
        let lowered = query.to_lowercase();
        let mut results = Vec::new();
        for (index, node) in tree.nodes.iter().enumerate() {
            if node.name.is_empty() || index == tree.root as usize {
                continue;
            }
            if node.name.to_lowercase().contains(&lowered) {
                results.push(index as NodeId);
                if results.len() >= 600 {
                    break;
                }
            }
        }
        results.sort_unstable_by(|a, b| tree.node(*b).size.cmp(&tree.node(*a).size));
        results.truncate(400);
        self.search_results = results;
        self.search_computed = query;
        self.search_changed_at = None;
        ctx.request_repaint();
    }

    fn apply_shortcuts(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let wants_keyboard = ctx.wants_keyboard_input();
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
            actions.push(Action::ChooseFolder);
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            actions.push(Action::FocusSearch);
        }
        if !wants_keyboard
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F5))
        {
            actions.push(Action::Rescan);
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::E)) {
            actions.push(Action::Export);
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowLeft))
            || (!wants_keyboard
                && ctx.input_mut(|input| {
                    input.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)
                }))
        {
            actions.push(Action::Back);
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            if self.modal.is_some() {
                self.modal = None;
            } else if !self.search.is_empty() {
                self.search.clear();
                self.search_computed.clear();
            } else {
                self.selection = None;
            }
        }
    }

    fn handle_actions(&mut self, ctx: &egui::Context, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::ScanPath(path, label) => self.start_scan(ctx, path, label),
                Action::CancelScan => {
                    if let Some(job) = &self.scan {
                        job.cancel.store(true, Ordering::Relaxed);
                        self.scan_cancel_requested = true;
                        self.toast(ctx, "Cancelling scan…", ToastKind::Info);
                    }
                }
                Action::LoadDemo => self.load_demo(ctx),
                Action::Select(node) => {
                    self.selection = Some(node);
                }
                Action::Navigate(node) => {
                    if let Some(tree) = &self.tree {
                        if tree.node(node).is_dir {
                            self.history.push(self.current);
                            self.current = node;
                            self.selection = None;
                            self.view = View::Overview;
                            self.treemap_zoom = Some(ctx.input(|input| input.time));
                        } else {
                            self.selection = Some(node);
                        }
                    }
                }
                Action::Back => {
                    if let Some(previous) = self.history.pop() {
                        self.current = previous;
                        self.selection = None;
                    }
                }
                Action::Up => {
                    if let Some(tree) = &self.tree {
                        if let Some(parent) = tree.node(self.current).parent {
                            self.history.push(self.current);
                            self.current = parent;
                            self.selection = None;
                        }
                    }
                }
                Action::SetView(view) => {
                    self.view = view;
                    if view == View::Search {
                        self.search_focus = true;
                    }
                    if view == View::Snapshots {
                        self.snapshots = crate::snapshots::list();
                    }
                    if view == View::Cleanup && self.cleanup.is_none() {
                        self.start_cleanup(ctx);
                    }
                }
                Action::ToggleTheme => {
                    self.settings.theme = self.settings.theme.toggled();
                    self.settings.save();
                }
                Action::OpenSettings => self.modal = Some(Modal::Settings),
                Action::CloseModal => self.modal = None,
                Action::Export => self.export_current(ctx),
                Action::Reveal(node) => {
                    if self.demo_loaded {
                        self.toast(ctx, "This is sample data — there is nothing to reveal", ToastKind::Info);
                    } else if let Some(tree) = &self.tree {
                        let path = tree.path_buf(node);
                        platform::reveal(&path, tree.node(node).is_dir);
                    }
                }
                Action::OpenItem(node) => {
                    if self.demo_loaded {
                        self.toast(ctx, "This is sample data — there is nothing to open", ToastKind::Info);
                    } else if let Some(tree) = &self.tree {
                        let path = tree.path_buf(node);
                        platform::open_path(&path);
                    }
                }
                Action::Delete(node) => {
                    if self.demo_loaded {
                        self.toast(ctx, "Sample data cannot be deleted", ToastKind::Info);
                    } else {
                        self.modal = Some(Modal::Delete(DeleteTarget::Node(node)));
                    }
                }
                Action::ConfirmDelete(target) => {
                    self.perform_delete(ctx, target);
                    self.modal = None;
                }
                Action::CopyPath(node) => {
                    if let Some(tree) = &self.tree {
                        let path = tree.path(node);
                        ctx.copy_text(path);
                        self.toast(ctx, "Path copied to clipboard", ToastKind::Success);
                    }
                }
                Action::ChooseFolder => {
                    let mut dialog = rfd::FileDialog::new();
                    if let Some(last) = &self.settings.last_path {
                        let path = PathBuf::from(last);
                        let start = if path.is_dir() {
                            path
                        } else {
                            path.parent().map(PathBuf::from).unwrap_or_default()
                        };
                        if start.exists() {
                            dialog = dialog.set_directory(start);
                        }
                    }
                    if let Some(folder) = dialog.pick_folder() {
                        let label = platform::label_for_path(&folder);
                        self.start_scan(ctx, folder, label);
                    }
                }
                Action::Rescan => {
                    if self.demo_loaded || self.tree.as_ref().is_some_and(|tree| tree.demo) {
                        self.load_demo(ctx);
                    } else if let Some(tree) = &self.tree {
                        let path = tree.root_path.clone();
                        let label = tree.root_label.clone();
                        self.start_scan(ctx, path, label);
                    }
                }
                Action::FocusSearch => {
                    self.view = View::Search;
                    self.search_focus = true;
                }
                Action::ClearSearch => {
                    self.search.clear();
                    self.search_computed.clear();
                    self.search_results.clear();
                    if self.view == View::Search {
                        self.view = View::Overview;
                    }
                }
                Action::StartDuplicates => self.start_duplicates(ctx),
                Action::CancelDuplicates => {
                    if let DupState::Running(job) = &self.dupes {
                        job.cancel.store(true, Ordering::Relaxed);
                    }
                }
                Action::DeleteDuplicate { node, group } => {
                    self.modal = Some(Modal::Delete(DeleteTarget::Duplicate { node, group }));
                }
                Action::SortBy(key) => match self.view {
                    View::OldFiles => self.old_sort.select(key),
                    View::Search => self.search_sort.select(key),
                    View::FileTypes => self.ext_sort.select(key),
                    View::Duplicates => self.dup_sort.select(key),
                    _ => self.sort.select(key),
                },
                Action::SetCategory(category) => self.selected_category = category,
                Action::CloseDetails => self.selection = None,
                Action::DismissToast(id) => self.toasts.retain(|toast| toast.id != id),
                Action::Toast(message, kind) => self.toast(ctx, message, kind),
                Action::RefreshDisks => self.refresh_disks(ctx),
                Action::SetOldYears(years) => {
                    self.settings.old_file_years = years;
                    self.settings.save();
                }
                Action::CycleTreemapColor => {
                    self.settings.treemap_color = self.settings.treemap_color.next();
                    self.settings.save();
                }
                Action::SaveSnapshot => self.save_snapshot(ctx),
                Action::SelectSnapshot(index) => {
                    self.snapshot_selected = Some(index);
                    self.snapshot_diff = None;
                }
                Action::CompareSnapshot(index) => self.compare_snapshot(ctx, index),
                Action::DeleteSnapshot(index) => self.delete_snapshot(ctx, index),
                Action::RefreshSnapshots => {
                    self.snapshots = crate::snapshots::list();
                    self.snapshot_note = Some(format!(
                        "{} saved scan{}",
                        format_count(self.snapshots.len() as u64),
                        if self.snapshots.len() == 1 { "" } else { "s" }
                    ));
                }
                Action::StartCleanupScan => self.start_cleanup(ctx),
                Action::ClearCleanup(index) => self.clear_cleanup(ctx, index),
                Action::OpenExternal(path) => crate::platform::open_path(&path),
                Action::AdminRescan => {
                    let path = self.tree.as_ref().map(|tree| tree.root_path.clone());
                    match platform::relaunch_as_admin(path) {
                        Ok(()) => {
                            self.toast(ctx, "Restarting with administrator rights", ToastKind::Info);
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                        Err(error) => self.toast(ctx, error, ToastKind::Error),
                    }
                }
                Action::OpenAbout => self.modal = Some(Modal::About),
                Action::SetOverviewList(list) => {
                    self.settings.overview_list = list;
                    self.settings.save();
                }
                Action::SetDupMinSize(mb) => {
                    self.settings.dup_min_size_mb = mb;
                    self.settings.save();
                }
                Action::ToggleDupScope => {
                    self.settings.dup_current_folder = !self.settings.dup_current_folder;
                    self.settings.save();
                }
                Action::DupKeepAll { newest } => self.plan_dup_bulk(ctx, newest),
                Action::ConfirmBulkDelete { paths, bytes, label } => {
                    self.bulk_delete(ctx, paths, bytes, label);
                    self.modal = None;
                }
            }
        }
    }

    pub(crate) fn perform_delete(&mut self, ctx: &egui::Context, target: DeleteTarget) {
        let node = match target {
            DeleteTarget::Node(node) => node,
            DeleteTarget::Duplicate { node, .. } => node,
        };
        let Some(tree) = &self.tree else {
            return;
        };
        if tree.demo {
            self.toast(ctx, "Sample data cannot be deleted", ToastKind::Info);
            return;
        }
        let path = tree.path_buf(node);
        let name = tree.node(node).name.clone();
        let is_dir = tree.node(node).is_dir;
        let parent = tree.node(node).parent.unwrap_or(tree.root);
        let current_inside = is_descendant(tree, self.current, node);
        let selection_inside = self
            .selection
            .map(|selected| is_descendant(tree, selected, node) || selected == node)
            .unwrap_or(false);
        let permanent = self.settings.delete_permanently;
        let result = platform::delete_item(&path, permanent);
        match result {
            Ok(()) => {
                if let Some(tree) = &mut self.tree {
                    tree.remove_subtree(node);
                    self.largest_ids = tree.largest_files(5000);
                }
                if current_inside {
                    self.current = parent;
                    self.history.clear();
                }
                if selection_inside {
                    self.selection = None;
                }
                if let DupState::Done(results) = &mut self.dupes {
                    results.remove_file(node);
                }
                self.generation += 1;
                let kind = if is_dir { "Folder" } else { "File" };
                self.toast(
                    ctx,
                    if permanent {
                        format!("{kind} “{name}” deleted permanently")
                    } else {
                        format!("{kind} “{name}” moved to the Recycle Bin")
                    },
                    ToastKind::Success,
                );
                self.refresh_disks(ctx);
            }
            Err(error) => {
                self.toast(ctx, format!("Could not delete “{name}”: {error}"), ToastKind::Error);
            }
        }
    }

    fn export_current(&mut self, ctx: &egui::Context) {
        let Some(tree) = &self.tree else {
            self.toast(ctx, "Scan a drive first", ToastKind::Info);
            return;
        };
        let default_name = match self.view {
            View::FileTypes => "G-SpaceAnalyzer-file-types.csv",
            View::Duplicates => "G-SpaceAnalyzer-duplicates.csv",
            _ => "G-SpaceAnalyzer-largest-files.csv",
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(default_name)
            .add_filter("CSV", &["csv"])
            .save_file()
        else {
            return;
        };
        let result = match self.view {
            View::FileTypes => export_extensions(tree, &path),
            View::Duplicates => export_duplicates(&self.dupes, &path),
            _ => export_largest(tree, &self.largest_ids, &path),
        };
        match result {
            Ok(rows) => self.toast(
                ctx,
                format!("Exported {} rows to {}", format_count(rows), path.display()),
                ToastKind::Success,
            ),
            Err(error) => self.toast(ctx, format!("Export failed: {error}"), ToastKind::Error),
        }
    }

    fn selected_drive(&self) -> Option<&DriveInfo> {
        let tree = self.tree.as_ref()?;
        if tree.demo {
            return None;
        }
        let root = tree.root_path.to_string_lossy().to_lowercase();
        self.disks
            .iter()
            .find(|drive| root.starts_with(&drive.root.to_lowercase()))
    }

    pub(crate) fn free_space_text(&self) -> String {
        if let Some(volume) = &self.volume {
            format_bytes(volume.free)
        } else if let Some(drive) = self.selected_drive() {
            format_bytes(drive.free)
        } else {
            "—".to_string()
        }
    }

    pub(crate) fn volume_capacity(&self) -> u64 {
        self.volume
            .as_ref()
            .map(|volume| volume.total)
            .or_else(|| self.selected_drive().map(|drive| drive.total))
            .unwrap_or_else(|| self.tree.as_ref().map(|tree| tree.total_size).unwrap_or(0))
    }
}

fn volume_for_path(path: &std::path::Path, disks: &[DriveInfo]) -> Option<VolumeInfo> {
    let text = path.to_string_lossy().to_lowercase();
    disks
        .iter()
        .find(|drive| text.starts_with(&drive.root.to_lowercase()))
        .map(|drive| VolumeInfo {
            label: drive.display_name(),
            total: drive.total,
            free: drive.free,
        })
}

fn is_descendant(tree: &Tree, candidate: NodeId, ancestor: NodeId) -> bool {
    let mut current = candidate;
    let mut guard = 0;
    while let Some(parent) = tree.node(current).parent {
        if parent == ancestor {
            return true;
        }
        current = parent;
        guard += 1;
        if guard > 4096 {
            break;
        }
    }
    false
}

fn export_largest(tree: &Tree, ids: &[NodeId], path: &std::path::Path) -> Result<u64, String> {
    let mut writer = csv::Writer::from_path(path).map_err(|err| err.to_string())?;
    writer
        .write_record(["path", "name", "size_bytes", "size", "modified"])
        .map_err(|err| err.to_string())?;
    let mut rows = 0u64;
    for &id in ids {
        let node = tree.node(id);
        writer
            .write_record([
                tree.path(id),
                node.name.clone(),
                node.size.to_string(),
                format_bytes(node.size),
                format_mtime(node.mtime),
            ])
            .map_err(|err| err.to_string())?;
        rows += 1;
    }
    writer.flush().map_err(|err| err.to_string())?;
    Ok(rows)
}

fn export_extensions(tree: &Tree, path: &std::path::Path) -> Result<u64, String> {
    let mut writer = csv::Writer::from_path(path).map_err(|err| err.to_string())?;
    writer
        .write_record(["extension", "category", "files", "size_bytes", "size"])
        .map_err(|err| err.to_string())?;
    let mut rows = 0u64;
    for stat in &tree.ext_stats {
        writer
            .write_record([
                stat.ext.clone(),
                stat.category.label().to_string(),
                stat.count.to_string(),
                stat.size.to_string(),
                format_bytes(stat.size),
            ])
            .map_err(|err| err.to_string())?;
        rows += 1;
    }
    writer.flush().map_err(|err| err.to_string())?;
    Ok(rows)
}

fn export_duplicates(state: &DupState, path: &std::path::Path) -> Result<u64, String> {
    let DupState::Done(results) = state else {
        return Err("Run the duplicate finder first".to_string());
    };
    let mut writer = csv::Writer::from_path(path).map_err(|err| err.to_string())?;
    writer
        .write_record(["group", "path", "size_bytes", "modified"])
        .map_err(|err| err.to_string())?;
    let mut rows = 0u64;
    for (group_index, group) in results.groups.iter().enumerate() {
        for file in &group.files {
            writer
                .write_record([
                    (group_index + 1).to_string(),
                    file.path.to_string_lossy().into_owned(),
                    file.size.to_string(),
                    format_mtime(file.modified),
                ])
                .map_err(|err| err.to_string())?;
            rows += 1;
        }
    }
    writer.flush().map_err(|err| err.to_string())?;
    Ok(rows)
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.settings.theme != self.applied_theme || self.settings.animations != self.applied_animations {
            theme::apply_theme(ctx, self.settings.theme, self.settings.animations);
            self.palette = Palette::for_mode(self.settings.theme);
            self.applied_theme = self.settings.theme;
            self.applied_animations = self.settings.animations;
        }
        if self.last_disks_refresh.elapsed() > Duration::from_secs(12) {
            self.refresh_disks(ctx);
        }
        self.poll_scan(ctx);
        self.poll_dupes(ctx);
        self.poll_cleanup(ctx);
        self.update_search(ctx);

        let mut actions = Vec::new();
        self.apply_shortcuts(ctx, &mut actions);
        self.titlebar(ctx, &mut actions);
        self.status_bar(ctx, &mut actions);
        let (sidebar_width, sidebar_edge) = self.sidebar(ctx, &mut actions);
        let mut details_pane = None;
        if let Some(tree) = &self.tree {
            if self.selection.is_some() && self.settings.show_details_on_select {
                if self.selection == Some(tree.root) {
                    self.selection = None;
                } else {
                    details_pane = self.details_panel(ctx, &mut actions);
                }
            }
        }
        // Remember a pane size only when a drag actually ends on its divider,
        // otherwise every click would rewrite the settings file.
        if ctx.input(|input| input.pointer.any_released()) {
            if let Some(pointer) = ctx.input(|input| input.pointer.latest_pos()) {
                let mut persist = false;
                if (pointer.x - sidebar_edge).abs() <= 6.0
                    && (sidebar_width - self.settings.sidebar_width).abs() > 0.5
                {
                    self.settings.sidebar_width = sidebar_width;
                    persist = true;
                }
                if let Some((edge, width)) = details_pane {
                    if (pointer.x - edge).abs() <= 6.0
                        && (width - self.settings.details_width).abs() > 0.5
                    {
                        self.settings.details_width = width;
                        persist = true;
                    }
                }
                if persist {
                    self.settings.save();
                }
            }
        }
        self.central(ctx, &mut actions);
        self.handle_actions(ctx, actions);
        self.modals(ctx);
        self.draw_toasts(ctx);
        self.window_border(ctx);

        if self.scan.is_some()
            || matches!(self.dupes, DupState::Running(_))
            || !self.toasts.is_empty()
            || self.search_changed_at.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
    }
}

impl App {
    fn titlebar(&self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(theme::TITLEBAR_HEIGHT)
            .frame(Frame::none().fill(self.palette.sidebar))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                let drag_rect = Rect::from_min_max(
                    full.min,
                    Pos2::new((full.right() - 224.0).max(full.left()), full.bottom()),
                );
                let drag = ui.interact(drag_rect, Id::new("titlebar_drag"), Sense::click_and_drag());
                if drag.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
                ui.horizontal_centered(|ui| {
                    ui.add_space(14.0);
                    let (logo_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(23.0), Sense::hover());
                    theme::paint_logo(ui.painter(), logo_rect, &self.palette);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("G-SpaceAnalyzer")
                            .font(theme::font_semibold(14.5))
                            .color(self.palette.text),
                    );
                    ui.add_space(2.0);
                    theme::pill(
                        ui,
                        &self.palette,
                        concat!("v", env!("CARGO_PKG_VERSION")),
                        self.palette.faint,
                    );
                    if let Some(tree) = &self.tree {
                        ui.add_space(6.0);
                        if tree.demo {
                            theme::pill(ui, &self.palette, "Sample", self.palette.warning);
                        } else if self.scan.is_some() {
                            theme::pill(ui, &self.palette, "Scanning", self.palette.accent);
                        } else {
                            theme::pill(ui, &self.palette, "Live", self.palette.success);
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(4.0);
                        if window_button(ui, ctx, &self.palette, WindowButton::Close).clicked() {
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                        let maximized =
                            ctx.input(|input| input.viewport().maximized.unwrap_or(false));
                        if window_button(
                            ui,
                            ctx,
                            &self.palette,
                            if maximized {
                                WindowButton::Restore
                            } else {
                                WindowButton::Maximize
                            },
                        )
                        .clicked()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                        }
                        if window_button(ui, ctx, &self.palette, WindowButton::Minimize).clicked() {
                            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                        }
                        ui.add_space(2.0);
                        let (line_rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 18.0), Sense::hover());
                        ui.painter().rect_filled(line_rect, Rounding::ZERO, self.palette.border);
                        ui.add_space(2.0);
                        if theme::icon_button(ui, &self.palette, ph::GEAR, 32.0, "Settings").clicked() {
                            actions.push(Action::OpenSettings);
                        }
                        if theme::icon_button(
                            ui,
                            &self.palette,
                            ph::INFO,
                            32.0,
                            "About G-SpaceAnalyzer",
                        )
                        .clicked()
                        {
                            actions.push(Action::OpenAbout);
                        }
                        let theme_icon = if self.settings.theme == ThemeMode::Dark {
                            ph::SUN
                        } else {
                            ph::MOON
                        };
                        if theme::icon_button(
                            ui,
                            &self.palette,
                            theme_icon,
                            32.0,
                            if self.settings.theme == ThemeMode::Dark {
                                "Light appearance"
                            } else {
                                "Dark appearance"
                            },
                        )
                        .clicked()
                        {
                            actions.push(Action::ToggleTheme);
                        }
                    });
                });
                let bottom = ui.max_rect().bottom();
                ui.painter().line_segment(
                    [Pos2::new(full.left(), bottom - 0.5), Pos2::new(full.right(), bottom - 0.5)],
                    Stroke::new(1.0, self.palette.border),
                );
            });
    }

    fn sidebar(&self, ctx: &egui::Context, actions: &mut Vec<Action>) -> (f32, f32) {
        let response = egui::SidePanel::left("sidebar")
            .default_width(self.settings.sidebar_width)
            .width_range(212.0..=360.0)
            .resizable(true)
            .frame(
                Frame::none()
                    .fill(self.palette.sidebar)
                    .inner_margin(Margin::symmetric(14.0, 0.0)),
            )
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Reserve room for the scroll bar so cards and badges are
                        // never clipped by the panel edge.
                        ui.set_max_width((ui.available_width() - 10.0).max(80.0));
                        ui.add_space(16.0);
                        theme::section_label(ui, &self.palette, "Scan");
                        ui.add_space(7.0);
                        let label = if self.scan.is_some() {
                            "Scanning…"
                        } else {
                            "Scan this PC"
                        };
                        let response = theme::primary_button(
                            ui,
                            &self.palette,
                            label,
                            Some(ph::HARD_DRIVES),
                            None,
                            38.0,
                        );
                        if response.clicked() {
                            let root = platform::system_drive();
                            let label = platform::label_for_path(&root);
                            actions.push(Action::ScanPath(root, label));
                        }
                        ui.add_space(2.0);
                        let choose = theme::secondary_button(
                            ui,
                            &self.palette,
                            "Choose a folder",
                            Some(ph::FOLDER_OPEN),
                            None,
                            32.0,
                        );
                        if choose.clicked() {
                            actions.push(Action::ChooseFolder);
                        }

                        ui.add_space(18.0);
                        ui.horizontal(|ui| {
                            theme::section_label(ui, &self.palette, "Storage");
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if theme::icon_button(
                                    ui,
                                    &self.palette,
                                    ph::ARROWS_CLOCKWISE,
                                    22.0,
                                    "Refresh drives",
                                )
                                .clicked()
                                {
                                    actions.push(Action::RefreshDisks);
                                }
                            });
                        });
                        ui.add_space(4.0);
                        if self.disks.is_empty() {
                            ui.label(
                                RichText::new("No drives detected")
                                    .font(theme::font_regular(11.5))
                                    .color(self.palette.faint),
                            );
                        }
                        let scanning_root = self.scan.as_ref().map(|job| job.root.clone());
                        for drive in &self.disks {
                            let active = scanning_root
                                .as_ref()
                                .map(|root| root.to_string_lossy().eq_ignore_ascii_case(&drive.root))
                                .unwrap_or(false);
                            let root = PathBuf::from(&drive.root);
                            let label = drive.display_name();
                            if drive_card(
                                ui,
                                &self.palette,
                                drive,
                                active,
                                self.scan_cancel_requested,
                            )
                            .clicked()
                            {
                                actions.push(Action::ScanPath(root, label));
                            }
                            ui.add_space(5.0);
                        }

                        ui.add_space(16.0);
                        theme::section_label(ui, &self.palette, "Analyze");
                        ui.add_space(4.0);
                        let enabled = self.tree.is_some();
                        ui.add_enabled_ui(enabled, |ui| {
                            if nav_item(
                                ui,
                                &self.palette,
                                ph::CUBE,
                                "Overview",
                                None,
                                self.view == View::Overview,
                            )
                            .clicked()
                            {
                                actions.push(Action::SetView(View::Overview));
                            }
                            let largest_badge = self
                                .tree
                                .as_ref()
                                .map(|tree| format_count(tree.total_files.min(5000)));
                            if nav_item(
                                ui,
                                &self.palette,
                                ph::FILES,
                                "Largest files",
                                largest_badge.as_deref(),
                                self.view == View::LargestFiles,
                            )
                            .clicked()
                            {
                                actions.push(Action::SetView(View::LargestFiles));
                            }
                            let type_badge = self
                                .tree
                                .as_ref()
                                .map(|tree| format_count(tree.ext_stats.len() as u64));
                            if nav_item(
                                ui,
                                &self.palette,
                                ph::CHART_DONUT,
                                "File types",
                                type_badge.as_deref(),
                                self.view == View::FileTypes,
                            )
                            .clicked()
                            {
                                actions.push(Action::SetView(View::FileTypes));
                            }
                            let dup_badge = match &self.dupes {
                                DupState::Done(results) => {
                                    Some(format_count(results.groups.len() as u64))
                                }
                                DupState::Running(_) => Some("…".to_string()),
                                _ => None,
                            };
                            if nav_item(
                                ui,
                                &self.palette,
                                ph::COPY,
                                "Duplicates",
                                dup_badge.as_deref(),
                                self.view == View::Duplicates,
                            )
                            .clicked()
                            {
                                actions.push(Action::SetView(View::Duplicates));
                            }
                            if nav_item(
                                ui,
                                &self.palette,
                                ph::MAGNIFYING_GLASS,
                                "Search",
                                None,
                                self.view == View::Search,
                            )
                            .clicked()
                            {
                                actions.push(Action::FocusSearch);
                            }
                            if nav_item(
                                ui,
                                &self.palette,
                                ph::TRASH,
                                "Clean up",
                                None,
                                self.view == View::Cleanup,
                            )
                            .clicked()
                            {
                                actions.push(Action::SetView(View::Cleanup));
                            }
                            let old_badge = self.tree.as_ref().map(|_| {
                                format!("{}y+", self.settings.old_file_years)
                            });
                            if nav_item(
                                ui,
                                &self.palette,
                                ph::CLOCK,
                                "Old files",
                                old_badge.as_deref(),
                                self.view == View::OldFiles,
                            )
                            .clicked()
                            {
                                actions.push(Action::SetView(View::OldFiles));
                            }
                            let snapshot_badge = (!self.snapshots.is_empty())
                                .then(|| format_count(self.snapshots.len() as u64));
                            if nav_item(
                                ui,
                                &self.palette,
                                ph::ARCHIVE,
                                "Snapshots",
                                snapshot_badge.as_deref(),
                                self.view == View::Snapshots,
                            )
                            .clicked()
                            {
                                actions.push(Action::SetView(View::Snapshots));
                            }
                        });

                        ui.add_space(18.0);
                        if let Some(tree) = &self.tree {
                            session_card(ui, &self.palette, tree, &self.volume);
                        } else if self.scan.is_some() {
                            scanning_sidebar_card(ui, &self.palette, self.scan.as_ref().unwrap());
                        } else {
                            ui.add_space(4.0);
                            theme::inset_card(&self.palette).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(
                                    RichText::new("Nothing scanned yet")
                                        .font(theme::font_medium(11.5))
                                        .color(self.palette.text),
                                );
                                ui.add_space(3.0);
                                ui.label(
                                    RichText::new("Pick a drive above to map your storage.")
                                        .font(theme::font_regular(10.5))
                                        .color(self.palette.faint),
                                );
                            });
                        }
                        ui.add_space(16.0);
                        let sample = theme::ghost_button(
                            ui,
                            &self.palette,
                            "Preview sample volume",
                            Some(ph::LIGHTNING),
                            28.0,
                        );
                        if sample.clicked() {
                            actions.push(Action::LoadDemo);
                        }
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Free & open source")
                                    .font(theme::font_regular(9.5))
                                    .color(self.palette.faint),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                                        .font(theme::font_regular(9.5))
                                        .color(self.palette.faint),
                                );
                            });
                        });
                        ui.add_space(12.0);
                    });
                let rect = ui.max_rect();
                ui.painter().line_segment(
                    [rect.right_top(), rect.right_bottom()],
                    Stroke::new(1.0, self.palette.border),
                );
            });
        let rect = response.response.rect;
        (rect.width(), rect.right())
    }

    fn status_bar(&self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(theme::STATUS_HEIGHT)
            .frame(
                Frame::none()
                    .fill(self.palette.sidebar)
                    .inner_margin(Margin::symmetric(14.0, 0.0)),
            )
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().line_segment(
                    [rect.left_top(), rect.right_top()],
                    Stroke::new(1.0, self.palette.border),
                );
                ui.horizontal_centered(|ui| {
                    if let Some(job) = &self.scan {
                        ui.add(egui::Spinner::new().size(12.0).color(self.palette.accent));
                        let progress = job.progress.snapshot();
                        let label = if self.scan_cancel_requested {
                            "Cancelling".to_string()
                        } else {
                            format!("Scanning {}", job.label)
                        };
                        ui.label(
                            RichText::new(label)
                                .font(theme::font_medium(10.5))
                                .color(self.palette.text),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} files · {}",
                                format_count(progress.files),
                                format_bytes(progress.bytes)
                            ))
                            .font(theme::font_regular(10.5))
                            .color(self.palette.muted),
                        );
                        if progress.errors > 0 {
                            ui.label(
                                RichText::new(format!("{} skipped", format_count(progress.errors)))
                                    .font(theme::font_regular(10.5))
                                    .color(self.palette.warning),
                            );
                        }
                        ui.add_space(6.0);
                        if theme::ghost_button(
                            ui,
                            &self.palette,
                            if self.scan_cancel_requested { "Stopping…" } else { "Stop" },
                            Some(ph::X),
                            22.0,
                        )
                        .on_hover_text("Stop scanning")
                        .clicked()
                            && !self.scan_cancel_requested
                        {
                            actions.push(Action::CancelScan);
                        }
                    } else if matches!(self.dupes, DupState::Running(_)) {
                        ui.add(egui::Spinner::new().size(12.0).color(self.palette.accent));
                        ui.label(
                            RichText::new("Finding duplicates")
                                .font(theme::font_medium(10.5))
                                .color(self.palette.text),
                        );
                    } else if let Some(tree) = &self.tree {
                        let (color, text) = if tree.demo {
                            (self.palette.warning, "Sample data".to_string())
                        } else {
                            (self.palette.success, "Scan complete".to_string())
                        };
                        theme::dot(ui.painter(), Pos2::new(ui.cursor().left() + 3.0, ui.cursor().center().y), 3.0, color);
                        ui.add_space(9.0);
                        ui.label(
                            RichText::new(text)
                                .font(theme::font_medium(10.5))
                                .color(self.palette.muted),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} files · {} folders · {}",
                                format_count(tree.total_files),
                                format_count(tree.total_dirs),
                                format_bytes(tree.total_size)
                            ))
                            .font(theme::font_regular(10.5))
                            .color(self.palette.faint),
                        );
                        if tree.errors > 0 {
                            ui.label(
                                RichText::new(format!("{} inaccessible", format_count(tree.errors)))
                                    .font(theme::font_regular(10.5))
                                    .color(self.palette.warning),
                            );
                        }
                    } else {
                        theme::dot(ui.painter(), Pos2::new(ui.cursor().left() + 3.0, ui.cursor().center().y), 3.0, self.palette.faint);
                        ui.add_space(9.0);
                        ui.label(
                            RichText::new("Ready")
                                .font(theme::font_medium(10.5))
                                .color(self.palette.muted),
                        );
                        ui.label(
                            RichText::new("Select a drive to begin")
                                .font(theme::font_regular(10.5))
                                .color(self.palette.faint),
                        );
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(concat!(
                                "G-SpaceAnalyzer ",
                                env!("CARGO_PKG_VERSION")
                            ))
                                .font(theme::font_regular(9.5))
                                .color(self.palette.faint),
                        );
                        if let Some(tree) = &self.tree {
                            ui.label(
                                RichText::new(format!(
                                    "{} scanned in {}",
                                    format_bytes(tree.total_size),
                                    format_duration(tree.elapsed)
                                ))
                                .font(theme::font_regular(9.5))
                                .color(self.palette.faint),
                            );
                        }
                        ui.add_space(8.0);
                        if self.elevated {
                            ui.horizontal(|ui| {
                                theme::icon_text(
                                    ui,
                                    ph::SHIELD_CHECK,
                                    "Administrator",
                                    self.palette.success,
                                    self.palette.success,
                                    9.5,
                                );
                            })
                            .response
                            .on_hover_text(
                                "Running as administrator: Windows exposes every folder, so scans are complete.",
                            );
                        } else if theme::ghost_button(
                            ui,
                            &self.palette,
                            "Restart as admin",
                            Some(ph::SHIELD_WARNING),
                            20.0,
                        )
                        .on_hover_text(
                            "Windows hides protected folders from a standard account, so scans miss space and take longer. Restarting as administrator scans everything.",
                        )
                        .clicked()
                        {
                            actions.push(Action::AdminRescan);
                        }
                    });
                });
            });
    }

    fn central(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        egui::CentralPanel::default()
            .frame(Frame::none().fill(self.palette.bg))
            .show(ctx, |ui| {
                if self.tree.is_some() {
                    self.content_header(ui, actions);
                    match self.view {
                        View::Overview => self.view_overview(ui, actions),
                        View::LargestFiles => self.view_largest_files(ui, actions),
                        View::FileTypes => self.view_file_types(ui, actions),
                        View::Duplicates => self.view_duplicates(ui, actions),
                        View::Search => self.view_search(ui, actions),
                        View::Cleanup => self.view_cleanup(ui, actions),
                        View::OldFiles => self.view_old_files(ui, actions),
                        View::Snapshots => self.view_snapshots(ui, actions),
                    }
                } else if self.scan.is_some() {
                    self.view_scanning(ui, actions);
                } else {
                    self.view_welcome(ui, actions);
                }
            });
    }

    fn content_header(&mut self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let Some(tree) = &self.tree else {
            return;
        };
        let current = self.current;
        let crumbs = tree.breadcrumbs(current);
        let node = tree.node(current);
        let size = node.size;
        let files = node.files;
        let dirs = node.dirs;
        let elapsed = tree.elapsed;
        let root_label = tree.root_label.clone();
        let demo = tree.demo;
        let scanning = self.scan.is_some();
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            let back_enabled = !self.history.is_empty();
            ui.add_enabled_ui(back_enabled, |ui| {
                if theme::secondary_button(
                    ui,
                    &self.palette,
                    "Back",
                    Some(ph::ARROW_LEFT),
                    Some(78.0),
                    28.0,
                )
                .on_hover_text("Back to the previous folder (Alt+Left)")
                .clicked()
                {
                    actions.push(Action::Back);
                }
            });
            let up_enabled = current != tree.root;
            if icon_toggle(ui, &self.palette, ph::ARROW_UP, 30.0, up_enabled, "Up one level") {
                actions.push(Action::Up);
            }
            ui.add_space(2.0);
            let available = ui.available_width() - 380.0;
            ui.allocate_ui_with_layout(
                Vec2::new(available.max(180.0), 26.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    if crumbs.len() > 4 {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("…")
                                        .font(theme::font_medium(12.0))
                                        .color(self.palette.faint),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            actions.push(Action::Navigate(crumbs[crumbs.len() - 4].0));
                        }
                        ui.label(
                            RichText::new(ph::CARET_RIGHT)
                                .font(theme::font_icon(9.0))
                                .color(self.palette.faint),
                        );
                    }
                    let start = crumbs.len().saturating_sub(3);
                    for (index, (id, label)) in crumbs.iter().enumerate().skip(start) {
                        let is_last = index + 1 == crumbs.len();
                        let text = if id == &tree.root {
                            if demo {
                                label.clone()
                            } else {
                                root_label.clone()
                            }
                        } else {
                            label.clone()
                        };
                        let text = if is_last {
                            text
                        } else {
                            crate::util::truncate_middle(&text, 22)
                        };
                        let response = ui.add(
                            egui::Button::new(
                                RichText::new(text)
                                    .font(if is_last {
                                        theme::font_semibold(15.0)
                                    } else {
                                        theme::font_regular(12.5)
                                    })
                                    .color(if is_last {
                                        self.palette.text
                                    } else {
                                        self.palette.muted
                                    }),
                            )
                            .frame(false),
                        );
                        if response.clicked() {
                            actions.push(Action::Navigate(*id));
                        }
                        if !is_last {
                            ui.label(
                                RichText::new(ph::CARET_RIGHT)
                                    .font(theme::font_icon(9.0))
                                    .color(self.palette.faint),
                            );
                        }
                    }
                },
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::icon_button(ui, &self.palette, ph::EXPORT, 30.0, "Export CSV (Ctrl+E)")
                    .clicked()
                {
                    actions.push(Action::Export);
                }
                if theme::icon_button(
                    ui,
                    &self.palette,
                    ph::ARROWS_CLOCKWISE,
                    30.0,
                    "Rescan (F5)",
                )
                .clicked()
                {
                    actions.push(Action::Rescan);
                }
                if self.tree.is_some()
                    && theme::icon_button(
                        ui,
                        &self.palette,
                        ph::SHIELD_WARNING,
                        30.0,
                        "Rescan as administrator",
                    )
                    .clicked()
                {
                    actions.push(Action::AdminRescan);
                }
                ui.add_space(2.0);
                let search_frame = Frame::none()
                    .fill(self.palette.surface_2)
                    .stroke(Stroke::new(1.0, self.palette.border))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(Margin::symmetric(9.0, 5.0));
                let inner = search_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.label(
                            RichText::new(ph::MAGNIFYING_GLASS)
                                .font(theme::font_icon(13.0))
                                .color(self.palette.faint),
                        );
                        let response = ui.add(
                            TextEdit::singleline(&mut self.search)
                                .hint_text("Search files and folders")
                                .desired_width(174.0)
                                .frame(false)
                                .font(theme::font_regular(12.0))
                                .text_color(self.palette.text),
                        );
                        if self.search_focus {
                            response.request_focus();
                            self.search_focus = false;
                        }
                        if response.changed() {
                            self.search_changed_at = Some(ui.input(|input| input.time));
                            if self.search.trim().len() >= 2 {
                                self.view = View::Search;
                            }
                        }
                        if !self.search.is_empty() {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(ph::X)
                                            .font(theme::font_icon(11.0))
                                            .color(self.palette.faint),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                actions.push(Action::ClearSearch);
                            }
                        }
                    });
                });
                if inner.response.clicked() && !self.search_focus {
                    // Clicking the padding around the field focuses the field on the next frame.
                    self.search_focus = true;
                }
            });
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(64.0);
            let (dot_color, dot_text) = if scanning {
                (self.palette.accent, "Refreshing…".to_string())
            } else if demo {
                (self.palette.warning, "Sample data".to_string())
            } else {
                (self.palette.success, "Scan complete".to_string())
            };
            theme::dot(
                ui.painter(),
                Pos2::new(ui.cursor().left() + 3.0, ui.cursor().center().y),
                2.8,
                dot_color,
            );
            ui.add_space(9.0);
            ui.label(
                RichText::new(dot_text)
                    .font(theme::font_medium(10.0))
                    .color(self.palette.muted),
            );
            ui.label(
                RichText::new(format!(
                    "{} · {} files · {} folders · scanned in {}",
                    format_percent(size, tree.total_size),
                    format_count(files),
                    format_count(dirs),
                    format_duration(elapsed)
                ))
                .font(theme::font_regular(10.0))
                .color(self.palette.faint),
            );
        });
        ui.add_space(9.0);
        let line_rect = Rect::from_min_max(
            Pos2::new(ui.max_rect().left() + 20.0, ui.cursor().top()),
            Pos2::new(ui.max_rect().right() - 20.0, ui.cursor().top() + 1.0),
        );
        ui.painter().rect_filled(line_rect, Rounding::ZERO, self.palette.border);
        ui.add_space(8.0);
    }

    fn details_panel(
        &self,
        ctx: &egui::Context,
        actions: &mut Vec<Action>,
    ) -> Option<(f32, f32)> {
        let tree = self.tree.as_ref()?;
        let id = self.selection?;
        if id as usize >= tree.nodes.len() || tree.node(id).is_empty() {
            return None;
        }
        let node = tree.node(id);
        let palette = &self.palette;
        let response = egui::SidePanel::right("details")
            .default_width(self.settings.details_width)
            .width_range(250.0..=430.0)
            .resizable(true)
            .frame(
                Frame::none()
                    .fill(palette.surface)
                    .inner_margin(Margin::symmetric(18.0, 0.0)),
            )
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            theme::section_label(ui, palette, "Inspector");
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if theme::icon_button(ui, palette, ph::X, 24.0, "Close").clicked() {
                                    actions.push(Action::CloseDetails);
                                }
                            });
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::hover());
                            ui.painter()
                                .rect_filled(rect, Rounding::same(9.0), palette.category_soft(node.category));
                            ui.painter().text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                if node.is_dir { ph::FOLDER } else { node.category.icon() },
                                theme::font_icon(19.0),
                                palette.category(node.category),
                            );
                            ui.add_space(2.0);
                            ui.vertical(|ui| {
                                theme::pill(
                                    ui,
                                    palette,
                                    if node.is_dir { "Folder" } else { node.category.label() },
                                    palette.category(node.category),
                                );
                                ui.add_space(3.0);
                                ui.label(
                                    RichText::new(crate::util::truncate_middle(&node.name, 34))
                                        .font(theme::font_semibold(15.0))
                                        .color(palette.text),
                                );
                            });
                        });
                        ui.add_space(12.0);
                        let path = tree.path(id);
                        Frame::none()
                            .fill(palette.surface_2)
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::symmetric(9.0, 7.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(1.0);
                                    ui.label(
                                        RichText::new(crate::util::truncate_middle(&path, 46))
                                            .font(theme::font_regular(10.0))
                                            .color(palette.muted),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if theme::icon_button(
                                            ui,
                                            palette,
                                            ph::COPY,
                                            20.0,
                                            "Copy path",
                                        )
                                        .clicked()
                                        {
                                            actions.push(Action::CopyPath(id));
                                        }
                                    });
                                });
                            });
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new(format_bytes(node.size))
                                .font(theme::font_bold(27.0))
                                .color(palette.text),
                        );
                        let total = tree.total_size.max(1);
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(format!(
                                "{} of {} · {}",
                                format_percent(node.size, total),
                                tree.root_label,
                                if node.is_dir {
                                    format!("{} files", format_count(node.files))
                                } else {
                                    "single file".to_string()
                                }
                            ))
                            .font(theme::font_regular(10.5))
                            .color(palette.muted),
                        );
                        ui.add_space(7.0);
                        let bar = Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(ui.available_width(), 5.0),
                        );
                        theme::progress_bar(
                            ui.painter(),
                            bar,
                            node.size as f32 / total as f32,
                            palette.accent,
                            palette.surface_3,
                        );
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(10.0);
                        stat_line(ui, palette, "Type", if node.is_dir { "Folder" } else { node.category.label() });
                        if node.is_dir {
                            stat_line(ui, palette, "Files", &format_count(node.files));
                            stat_line(ui, palette, "Subfolders", &format_count(node.dirs));
                        }
                        stat_line(ui, palette, "Modified", &format_mtime(node.mtime));
                        ui.add_space(14.0);
                        let demo = tree.demo;
                        ui.horizontal(|ui| {
                            let open = theme::secondary_button(
                                ui,
                                palette,
                                if node.is_dir { "Open" } else { "Reveal" },
                                Some(if node.is_dir { ph::FOLDER_OPEN } else { ph::ARROW_SQUARE_OUT }),
                                None,
                                32.0,
                            );
                            if open.clicked() {
                                if node.is_dir {
                                    actions.push(Action::Navigate(id));
                                } else {
                                    actions.push(Action::Reveal(id));
                                }
                            }
                            let delete = theme::secondary_button(
                                ui,
                                palette,
                                "Delete",
                                Some(ph::TRASH),
                                None,
                                32.0,
                            );
                            if delete.clicked() && !demo {
                                actions.push(Action::Delete(id));
                            }
                        });
                        if demo {
                            ui.add_space(5.0);
                            ui.label(
                                RichText::new("Actions are disabled for sample data")
                                    .font(theme::font_regular(9.5))
                                    .color(palette.faint),
                            );
                        }
                        if !node.children.is_empty() {
                            ui.add_space(18.0);
                            theme::section_label(ui, palette, "Largest contents");
                            ui.add_space(6.0);
                            for child_id in node.children.iter().take(6) {
                                let child = tree.node(*child_id);
                                let row = contents_row(ui, palette, tree, *child_id);
                                if row.clicked() {
                                    actions.push(Action::Select(*child_id));
                                }
                                if row.double_clicked() {
                                    actions.push(Action::Navigate(*child_id));
                                }
                                let _ = child;
                                ui.add_space(2.0);
                            }
                        }
                        ui.add_space(20.0);
                    });
                let rect = ui.max_rect();
                ui.painter().line_segment(
                    [rect.left_top(), rect.left_bottom()],
                    Stroke::new(1.0, self.palette.border),
                );
            });
        let rect = response.response.rect;
        Some((rect.width(), rect.left()))
    }

    fn window_border(&self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            Order::Middle,
            Id::new("window_border"),
        ));
        painter.rect_stroke(
            screen.shrink(0.5),
            Rounding::ZERO,
            Stroke::new(1.0, self.palette.border),
        );
    }
}

#[derive(Clone, Copy)]
enum WindowButton {
    Minimize,
    Maximize,
    Restore,
    Close,
}

fn window_button(
    ui: &mut Ui,
    _ctx: &egui::Context,
    palette: &Palette,
    kind: WindowButton,
) -> Response {
    let size = Vec2::new(46.0, theme::TITLEBAR_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    if hovered {
        let fill = match kind {
            WindowButton::Close => Color32::from_rgb(196, 43, 28),
            _ => palette.surface_2,
        };
        ui.painter().rect_filled(rect, Rounding::ZERO, fill);
    }
    let color = if hovered && matches!(kind, WindowButton::Close) {
        Color32::WHITE
    } else {
        palette.muted
    };
    let center = rect.center();
    let stroke = Stroke::new(1.1, color);
    match kind {
        WindowButton::Minimize => {
            ui.painter().line_segment(
                [
                    Pos2::new(center.x - 5.0, center.y + 0.5),
                    Pos2::new(center.x + 5.0, center.y + 0.5),
                ],
                stroke,
            );
        }
        WindowButton::Maximize => {
            ui.painter().rect_stroke(
                Rect::from_center_size(center, Vec2::splat(9.0)),
                Rounding::ZERO,
                stroke,
            );
        }
        WindowButton::Restore => {
            ui.painter().rect_stroke(
                Rect::from_center_size(Pos2::new(center.x - 1.0, center.y + 1.0), Vec2::splat(8.0)),
                Rounding::ZERO,
                stroke,
            );
            ui.painter().rect_stroke(
                Rect::from_center_size(Pos2::new(center.x + 1.5, center.y - 1.5), Vec2::splat(8.0)),
                Rounding::ZERO,
                stroke,
            );
        }
        WindowButton::Close => {
            ui.painter().line_segment(
                [
                    Pos2::new(center.x - 4.5, center.y - 4.5),
                    Pos2::new(center.x + 4.5, center.y + 4.5),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    Pos2::new(center.x + 4.5, center.y - 4.5),
                    Pos2::new(center.x - 4.5, center.y + 4.5),
                ],
                stroke,
            );
        }
    }
    response.on_hover_cursor(CursorIcon::Default)
}

fn icon_toggle(
    ui: &mut Ui,
    palette: &Palette,
    icon: &str,
    size: f32,
    enabled: bool,
    tooltip: &str,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hovered = response.hovered() && enabled;
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, hovered, 0.12);
    if t > 0.0 {
        ui.painter().rect_filled(
            rect,
            Rounding::same(7.0),
            theme::mix(palette.bg, palette.surface_2, t),
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        icon,
        theme::font_icon(14.0),
        if enabled {
            theme::mix(palette.muted, palette.text, t)
        } else {
            palette.faint
        },
    );
    if enabled {
        response.clone().on_hover_text(tooltip);
        response.on_hover_cursor(CursorIcon::PointingHand).clicked()
    } else {
        false
    }
}

fn nav_item(
    ui: &mut Ui,
    palette: &Palette,
    icon: &str,
    label: &str,
    badge: Option<&str>,
    selected: bool,
) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::click());
    let hovered = response.hovered();
    let bg = if selected {
        palette.accent_soft
    } else if hovered {
        palette.surface_2
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(8.0), bg);
    if selected {
        let bar = Rect::from_min_size(
            Pos2::new(rect.left() + 1.0, rect.center().y - 8.0),
            Vec2::new(3.0, 16.0),
        );
        ui.painter().rect_filled(bar, Rounding::same(2.0), palette.accent);
    }
    let color = if selected {
        palette.text
    } else if hovered {
        palette.text
    } else {
        palette.muted
    };
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        icon,
        theme::font_icon(15.0),
        if selected { palette.accent } else { color },
    );
    ui.painter().text(
        Pos2::new(rect.left() + 36.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        theme::font_medium(12.5),
        color,
    );
    if let Some(badge) = badge {
        let galley = ui.painter().layout_no_wrap(
            badge.to_string(),
            theme::font_regular(9.5),
            palette.faint,
        );
        let badge_rect = Rect::from_min_size(
            Pos2::new(rect.right() - galley.size().x - 12.0, rect.center().y - 8.0),
            Vec2::new(galley.size().x + 12.0, 16.0),
        );
        ui.painter()
            .rect_filled(badge_rect, Rounding::same(8.0), palette.surface_2);
        ui.painter().text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            badge,
            theme::font_regular(9.5),
            palette.faint,
        );
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn drive_card(
    ui: &mut Ui,
    palette: &Palette,
    drive: &DriveInfo,
    scanning: bool,
    cancel_requested: bool,
) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 58.0), Sense::click());
    let hovered = response.hovered();
    let border = if scanning {
        palette.accent
    } else if hovered {
        palette.border_strong
    } else {
        palette.border
    };
    ui.painter()
        .rect_filled(rect, Rounding::same(9.0), palette.surface_2);
    ui.painter()
        .rect_stroke(rect, Rounding::same(9.0), Stroke::new(1.0, border));
    let icon_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 9.0, rect.center().y - 15.0),
        Vec2::splat(30.0),
    );
    ui.painter()
        .rect_filled(icon_rect, Rounding::same(8.0), palette.surface_3);
    ui.painter().text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        if scanning { ph::ARROWS_CLOCKWISE } else { ph::HARD_DRIVE },
        theme::font_icon(15.0),
        if scanning { palette.accent } else { palette.muted },
    );
    let text_left = rect.left() + 47.0;
    ui.painter().text(
        Pos2::new(text_left, rect.top() + 16.0),
        Align2::LEFT_CENTER,
        format!("{}:", drive.letter),
        theme::font_semibold(11.5),
        palette.text,
    );
    let label = if drive.label.is_empty() {
        drive.kind_label().to_string()
    } else {
        drive.label.clone()
    };
    ui.painter().text(
        Pos2::new(text_left + 20.0, rect.top() + 16.0),
        Align2::LEFT_CENTER,
        crate::util::truncate_middle(&label, 20),
        theme::font_regular(10.0),
        palette.muted,
    );
    let free_text = if scanning {
        if cancel_requested {
            "Cancelling…".to_string()
        } else {
            "Scanning…".to_string()
        }
    } else {
        format!("{} free of {}", format_bytes(drive.free), format_bytes(drive.total))
    };
    ui.painter().text(
        Pos2::new(text_left, rect.top() + 33.0),
        Align2::LEFT_CENTER,
        free_text,
        theme::font_regular(9.5),
        if scanning { palette.accent } else { palette.faint },
    );
    let bar = Rect::from_min_size(
        Pos2::new(text_left, rect.bottom() - 13.0),
        Vec2::new(rect.right() - text_left - 9.0, 3.0),
    );
    let fill = if drive.used_fraction() > 0.92 {
        palette.danger
    } else if drive.used_fraction() > 0.8 {
        palette.warning
    } else {
        palette.accent
    };
    theme::progress_bar(ui.painter(), bar, drive.used_fraction(), fill, palette.surface_3);
    if hovered && !scanning {
        ui.painter().text(
            Pos2::new(rect.right() - 13.0, rect.top() + 16.0),
            Align2::CENTER_CENTER,
            ph::PLAY,
            theme::font_icon(12.0),
            palette.accent,
        );
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn session_card(ui: &mut Ui, palette: &Palette, tree: &Tree, volume: &Option<VolumeInfo>) {
    Frame::none()
        .fill(palette.surface_2)
        .stroke(Stroke::new(1.0, palette.border))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                theme::dot(
                    ui.painter(),
                    Pos2::new(ui.cursor().left() + 3.0, ui.cursor().center().y),
                    2.8,
                    if tree.demo { palette.warning } else { palette.success },
                );
                ui.add_space(9.0);
                ui.label(
                    RichText::new(if tree.demo { "Sample volume" } else { "Last scan" })
                        .font(theme::font_medium(10.5))
                        .color(palette.text),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format_duration(tree.elapsed))
                            .font(theme::font_regular(9.5))
                            .color(palette.faint),
                    );
                });
            });
            ui.add_space(9.0);
            ui.horizontal(|ui| {
                mini_stat(ui, palette, "Files", &format_count(tree.total_files));
                mini_stat(ui, palette, "Folders", &format_count(tree.total_dirs));
            });
            ui.add_space(7.0);
            ui.horizontal(|ui| {
                mini_stat(ui, palette, "Size", &format_bytes(tree.total_size));
                mini_stat(
                    ui,
                    palette,
                    "Free",
                    &volume.as_ref().map(|volume| format_bytes(volume.free)).unwrap_or_else(|| "—".into()),
                );
            });
            if tree.errors > 0 {
                ui.add_space(7.0);
                ui.label(
                    RichText::new(format!("{} inaccessible items skipped", format_count(tree.errors)))
                        .font(theme::font_regular(9.5))
                        .color(palette.warning),
                );
            }
        });
}

fn mini_stat(ui: &mut Ui, palette: &Palette, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(label)
                .font(theme::font_regular(9.0))
                .color(palette.faint),
        );
        ui.label(
            RichText::new(value)
                .font(theme::font_semibold(11.0))
                .color(palette.text),
        );
    });
    ui.add_space(14.0);
}

fn scanning_sidebar_card(ui: &mut Ui, palette: &Palette, job: &ScanJob) {
    let progress = job.progress.snapshot();
    Frame::none()
        .fill(palette.surface_2)
        .stroke(Stroke::new(1.0, palette.border))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(12.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(12.0).color(palette.accent));
                ui.label(
                    RichText::new("Scanning")
                        .font(theme::font_medium(10.5))
                        .color(palette.text),
                );
            });
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "{} files · {}",
                    format_count(progress.files),
                    format_bytes(progress.bytes)
                ))
                .font(theme::font_regular(10.0))
                .color(palette.muted),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(crate::util::truncate_middle(&progress.current, 38))
                    .font(theme::font_regular(9.0))
                    .color(palette.faint),
            );
        });
}

fn stat_line(ui: &mut Ui, palette: &Palette, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .font(theme::font_regular(10.5))
                .color(palette.faint),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(value)
                    .font(theme::font_medium(10.5))
                    .color(palette.text),
            );
        });
    });
    ui.add_space(5.0);
}

fn contents_row(ui: &mut Ui, palette: &Palette, tree: &Tree, id: NodeId) -> Response {
    let node = tree.node(id);
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, Rounding::same(7.0), palette.surface_2);
    }
    let (dot, _) = ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
    let _ = dot;
    ui.painter().circle_filled(
        Pos2::new(rect.left() + 8.0, rect.center().y),
        3.0,
        palette.category(node.category),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 18.0, rect.center().y),
        Align2::LEFT_CENTER,
        crate::util::truncate_middle(&node.name, 24),
        theme::font_regular(10.5),
        palette.text,
    );
    ui.painter().text(
        Pos2::new(rect.right() - 6.0, rect.center().y),
        Align2::RIGHT_CENTER,
        format_bytes(node.size),
        theme::font_regular(9.5),
        palette.muted,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

