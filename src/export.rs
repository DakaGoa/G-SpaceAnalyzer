// Shared export helpers used by the app and the headless CLI.

use std::path::Path;

use crate::model::{NodeId, Tree};
use crate::util::{format_bytes, format_count, format_mtime};

pub fn write_largest_csv(tree: &Tree, ids: &[NodeId], path: &Path) -> Result<u64, String> {
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

pub fn write_folders_csv(tree: &Tree, limit: usize, path: &Path) -> Result<u64, String> {
    let mut folders: Vec<NodeId> = tree
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.is_dir && !node.is_empty())
        .map(|(index, _)| index as NodeId)
        .collect();
    folders.sort_unstable_by(|a, b| tree.node(*b).size.cmp(&tree.node(*a).size));
    folders.truncate(limit);

    let mut writer = csv::Writer::from_path(path).map_err(|err| err.to_string())?;
    writer
        .write_record(["path", "name", "size_bytes", "size", "files", "folders", "modified"])
        .map_err(|err| err.to_string())?;
    let mut rows = 0u64;
    for &id in &folders {
        let node = tree.node(id);
        writer
            .write_record([
                tree.path(id),
                node.name.clone(),
                node.size.to_string(),
                format_bytes(node.size),
                format_count(node.files),
                format_count(node.dirs),
                format_mtime(node.mtime),
            ])
            .map_err(|err| err.to_string())?;
        rows += 1;
    }
    writer.flush().map_err(|err| err.to_string())?;
    Ok(rows)
}

pub fn write_ext_csv(tree: &Tree, path: &Path) -> Result<u64, String> {
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

/// Writes the whole arena. Used by the CLI and by snapshots.
pub fn write_json(tree: &Tree, path: &Path) -> Result<u64, String> {
    let file = std::fs::File::create(path).map_err(|err| err.to_string())?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer(writer, tree).map_err(|err| err.to_string())?;
    Ok(tree.nodes.len() as u64)
}
