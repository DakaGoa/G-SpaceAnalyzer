// Saved scans ("snapshots") and the folder-level diff between two scans.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::Tree;
use crate::util::app_data_dir;

const VERSION: u32 = 2;

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub label: String,
    pub created: i64,
    #[serde(default)]
    pub root_label: String,
    #[serde(default)]
    pub total_size: u64,
    #[serde(default)]
    pub total_files: u64,
    /// Present in full snapshots, absent in compact ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<Tree>,
    /// Folder path and size, present in compact snapshots only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<(String, u64)>,
}

/// A snapshot as the diff needs it, whatever form it was saved in.
pub struct LoadedSnapshot {
    pub root_label: String,
    pub total_size: u64,
    pub total_files: u64,
    pub folders: HashMap<String, u64>,
    pub full: bool,
}

#[derive(Clone, Debug)]
pub struct SnapshotMeta {
    pub path: PathBuf,
    pub label: String,
    pub created: i64,
    pub total_size: u64,
    pub total_files: u64,
    pub root_label: String,
    /// Bytes the snapshot file itself takes on disk.
    pub file_size: u64,
}

pub fn directory() -> PathBuf {
    app_data_dir().join("snapshots")
}

pub fn save(tree: &Tree, label: &str, compact: bool) -> Result<PathBuf, String> {
    let dir = directory();
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let now = chrono::Local::now();
    let stamp = now.format("%Y%m%d-%H%M%S").to_string();
    let safe: String = label
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let path = dir.join(format!("{stamp}-{safe}.json"));

    write(&path, &build(tree, label, now.timestamp(), compact))?;
    Ok(path)
}

/// A compact snapshot keeps folder totals only; a full one keeps every file.
fn build(tree: &Tree, label: &str, created: i64, compact: bool) -> Snapshot {
    Snapshot {
        version: VERSION,
        label: label.to_string(),
        created,
        root_label: tree.root_label.clone(),
        total_size: tree.total_size,
        total_files: tree.total_files,
        tree: if compact { None } else { Some(tree.clone()) },
        folders: if compact { folder_list(tree) } else { Vec::new() },
    }
}

fn write(path: &Path, snapshot: &Snapshot) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|err| err.to_string())?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer(writer, snapshot).map_err(|err| err.to_string())
}

pub fn list() -> Vec<SnapshotMeta> {
    let Ok(entries) = std::fs::read_dir(directory()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let file_size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        // A corrupt or half-written snapshot must not break the list.
        let Ok(file) = std::fs::File::open(&path) else {
            continue;
        };
        let Ok(snapshot) = serde_json::from_reader::<_, Snapshot>(std::io::BufReader::new(file))
        else {
            continue;
        };
        let tree = snapshot.tree.as_ref();
        out.push(SnapshotMeta {
            path,
            label: snapshot.label,
            created: snapshot.created,
            total_size: if snapshot.total_size > 0 {
                snapshot.total_size
            } else {
                tree.map(|tree| tree.total_size).unwrap_or(0)
            },
            total_files: if snapshot.total_files > 0 {
                snapshot.total_files
            } else {
                tree.map(|tree| tree.total_files).unwrap_or(0)
            },
            root_label: if snapshot.root_label.is_empty() {
                tree.map(|tree| tree.root_label.clone()).unwrap_or_default()
            } else {
                snapshot.root_label
            },
            file_size,
        });
    }
    out.sort_by(|a, b| b.created.cmp(&a.created));
    out
}

pub fn load(path: &Path) -> Result<LoadedSnapshot, String> {
    let file = std::fs::File::open(path).map_err(|err| err.to_string())?;
    let snapshot: Snapshot =
        serde_json::from_reader(std::io::BufReader::new(file)).map_err(|err| err.to_string())?;
    let tree = snapshot.tree.as_ref();
    let mut folders: HashMap<String, u64> = snapshot.folders.into_iter().collect();
    if folders.is_empty() {
        if let Some(tree) = tree {
            folders = folder_sizes(tree);
        }
    }
    Ok(LoadedSnapshot {
        root_label: if snapshot.root_label.is_empty() {
            tree.map(|tree| tree.root_label.clone()).unwrap_or_default()
        } else {
            snapshot.root_label
        },
        total_size: if snapshot.total_size > 0 {
            snapshot.total_size
        } else {
            tree.map(|tree| tree.total_size).unwrap_or(0)
        },
        total_files: if snapshot.total_files > 0 {
            snapshot.total_files
        } else {
            tree.map(|tree| tree.total_files).unwrap_or(0)
        },
        folders,
        full: tree.is_some(),
    })
}

pub fn delete(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|err| err.to_string())
}

#[derive(Clone, Debug)]
pub struct DiffRow {
    pub path: String,
    pub is_dir: bool,
    pub old_size: u64,
    pub new_size: u64,
}

impl DiffRow {
    pub fn delta(&self) -> i64 {
        self.new_size as i64 - self.old_size as i64
    }

    pub fn change_ratio(&self) -> f32 {
        if self.old_size == 0 {
            if self.new_size > 0 {
                1.0
            } else {
                0.0
            }
        } else {
            (self.delta() as f64 / self.old_size as f64) as f32
        }
    }
}

fn folder_sizes(tree: &Tree) -> HashMap<String, u64> {
    let mut map = HashMap::with_capacity(tree.nodes.len() / 4);
    for (index, node) in tree.nodes.iter().enumerate() {
        if node.is_empty() || !node.is_dir {
            continue;
        }
        map.insert(tree.path(index as u32), node.size);
    }
    map
}

/// The same folder totals as `folder_sizes`, sorted for compact snapshot files.
fn folder_list(tree: &Tree) -> Vec<(String, u64)> {
    let mut out: Vec<(String, u64)> = Vec::with_capacity(tree.nodes.len() / 4);
    for (index, node) in tree.nodes.iter().enumerate() {
        if node.is_empty() || !node.is_dir {
            continue;
        }
        out.push((tree.path(index as u32), node.size));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Folder-level comparison: what grew and what shrank between two scans.
///
/// Folders (not files) are compared because they survive file churn and keep
/// the diff readable on a terabyte-scale scan.
pub fn diff(previous: &LoadedSnapshot, current: &Tree) -> Vec<DiffRow> {
    let before = &previous.folders;
    let after = folder_sizes(current);
    let mut keys: HashSet<&String> = HashSet::new();
    keys.extend(before.keys());
    keys.extend(after.keys());

    let mut rows: Vec<DiffRow> = keys
        .into_iter()
        .filter_map(|key| {
            let old_size = before.get(key).copied().unwrap_or(0);
            let new_size = after.get(key).copied().unwrap_or(0);
            if old_size == new_size {
                return None;
            }
            Some(DiffRow {
                path: key.clone(),
                is_dir: true,
                old_size,
                new_size,
            })
        })
        .collect();

    // A few KB of churn is noise in a diff view.
    rows.retain(|row| row.delta().unsigned_abs() >= 64 * 1024);
    rows.sort_unstable_by(|a, b| b.delta().abs().cmp(&a.delta().abs()));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TreeNode;
    use std::time::Duration;

    fn make_tree(extra: u64) -> Tree {
        let mut folders = Vec::new();
        for folder in 0..40u64 {
            let mut files = Vec::new();
            for file in 0..25u64 {
                files.push(TreeNode::file(
                    format!("clip-{file}.bin"),
                    (folder + 1) * 1_000 + file + extra,
                    0,
                ));
            }
            folders.push(TreeNode::dir(format!("Folder {folder}"), 0, files));
        }
        Tree::from_tree(
            TreeNode::dir("Root", 0, folders),
            PathBuf::from("C:\\"),
            "Local Disk (C:)".to_string(),
            Duration::from_secs(1),
            0,
            false,
        )
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("spaceanalyzer-snapshot-{name}.json"))
    }

    #[test]
    fn compact_snapshot_is_smaller_and_diffs_the_same() {
        let before = make_tree(0);
        let after = make_tree(500);
        let full_path = temp_path("full");
        let compact_path = temp_path("compact");
        write(&full_path, &build(&before, "test", 1, false)).unwrap();
        write(&compact_path, &build(&before, "test", 1, true)).unwrap();
        let full_size = std::fs::metadata(&full_path).unwrap().len();
        let compact_size = std::fs::metadata(&compact_path).unwrap().len();
        assert!(
            compact_size * 4 < full_size,
            "compact {compact_size} bytes vs full {full_size} bytes"
        );

        let full = load(&full_path).unwrap();
        let compact = load(&compact_path).unwrap();
        assert!(full.full);
        assert!(!compact.full);
        assert_eq!(compact.root_label, "Local Disk (C:)");
        assert_eq!(compact.total_files, before.total_files);
        assert_eq!(compact.total_size, before.total_size);

        let mut full_rows: Vec<(String, i64)> = diff(&full, &after)
            .into_iter()
            .map(|row| {
                let delta = row.delta();
                (row.path, delta)
            })
            .collect();
        let mut compact_rows: Vec<(String, i64)> = diff(&compact, &after)
            .into_iter()
            .map(|row| {
                let delta = row.delta();
                (row.path, delta)
            })
            .collect();
        full_rows.sort();
        compact_rows.sort();
        assert!(!full_rows.is_empty());
        assert_eq!(full_rows, compact_rows);

        println!(
            "compact snapshot {compact_size} bytes vs full {full_size} bytes ({:.1}x smaller)",
            full_size as f64 / compact_size as f64
        );
        let _ = std::fs::remove_file(&full_path);
        let _ = std::fs::remove_file(&compact_path);
    }

    #[test]
    fn old_full_snapshots_still_load() {
        let tree = make_tree(0);
        // Shape of a version 1 file: only version/label/created/tree.
        let text = serde_json::to_value(build(&tree, "old", 1, false)).unwrap();
        let mut value = text;
        if let Some(object) = value.as_object_mut() {
            object.remove("root_label");
            object.remove("total_size");
            object.remove("total_files");
        }
        let path = temp_path("v1");
        std::fs::write(&path, serde_json::to_string(&value).unwrap()).unwrap();

        let loaded = load(&path).unwrap();
        assert!(loaded.full);
        assert_eq!(loaded.root_label, "Local Disk (C:)");
        assert_eq!(loaded.total_files, tree.total_files);
        assert_eq!(loaded.total_size, tree.total_size);
        assert!(!loaded.folders.is_empty());
        let _ = std::fs::remove_file(&path);
    }
}

/// "3 days ago" style label for snapshot lists.
pub fn relative_time(created: i64, now: i64) -> String {
    let seconds = (now - created).max(0);
    if seconds < 60 {
        return "just now".to_string();
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes} min ago");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours} h ago");
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{days} days ago");
    }
    format!("{} months ago", days / 30)
}
