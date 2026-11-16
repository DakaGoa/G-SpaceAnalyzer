use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::util::format_bytes;

pub type NodeId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Category {
    Video = 0,
    Image = 1,
    Audio = 2,
    Document = 3,
    Archive = 4,
    Code = 5,
    App = 6,
    System = 7,
    Other = 8,
}

impl Category {
    pub const ALL: [Category; 9] = [
        Category::Video,
        Category::Image,
        Category::Audio,
        Category::Document,
        Category::Archive,
        Category::Code,
        Category::App,
        Category::System,
        Category::Other,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Category::Video => "Video",
            Category::Image => "Images",
            Category::Audio => "Audio",
            Category::Document => "Documents",
            Category::Archive => "Archives",
            Category::Code => "Code",
            Category::App => "Applications",
            Category::System => "System",
            Category::Other => "Other",
        }
    }

    pub fn icon(self) -> &'static str {
        use egui_phosphor::regular as ph;
        match self {
            Category::Video => ph::FILM_STRIP,
            Category::Image => ph::IMAGE,
            Category::Audio => ph::MUSIC_NOTES,
            Category::Document => ph::FILE_TEXT,
            Category::Archive => ph::FILE_ZIP,
            Category::Code => ph::CODE,
            Category::App => ph::APP_WINDOW,
            Category::System => ph::GEAR,
            Category::Other => ph::FILE,
        }
    }

    pub fn from_ext(ext: &str) -> Category {
        let e = ext.to_ascii_lowercase();
        match e.as_str() {
            // Video
            "mp4" | "mkv" | "mov" | "avi" | "wmv" | "webm" | "m4v" | "mpg" | "mpeg"
            | "flv" | "m2ts" | "mts" | "3gp" | "ogv" | "vob" | "prproj" | "aep" => {
                Category::Video
            }
            // Images
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tif" | "tiff" | "webp" | "heic"
            | "heif" | "raw" | "cr2" | "cr3" | "nef" | "arw" | "dng" | "orf" | "rw2"
            | "psd" | "psb" | "ai" | "svg" | "ico" | "avif" | "jxl" => Category::Image,
            // Audio
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" | "aiff"
            | "aif" | "mid" | "midi" | "caf" => Category::Audio,
            // Documents
            "doc" | "docx" | "pdf" | "txt" | "rtf" | "xls" | "xlsx" | "xlsm" | "ppt"
            | "pptx" | "odt" | "ods" | "odp" | "csv" | "md" | "epub" | "mobi" | "pages"
            | "numbers" | "key" | "tex" | "log" => Category::Document,
            // Archives and disk images
            "zip" | "rar" | "7z" | "tar" | "gz" | "gzip" | "bz2" | "xz" | "zst" | "cab"
            | "iso" | "img" | "vhd" | "vhdx" | "vmdk" | "dmg" | "pkg" | "pak" | "arc"
            | "lz" | "lzma" | "z" | "tgz" | "tbz" | "sitx" => Category::Archive,
            // Code and source
            "rs" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "py" | "pyw" | "c"
            | "cc" | "cpp" | "cxx" | "h" | "hpp" | "cs" | "java" | "go" | "rb" | "php"
            | "swift" | "kt" | "kts" | "html" | "htm" | "css" | "scss" | "sass" | "less"
            | "json" | "jsonc" | "toml" | "yaml" | "yml" | "xml" | "sql" | "sh" | "bash"
            | "zsh" | "ps1" | "bat" | "cmd" | "lua" | "dart" | "vue" | "svelte" | "asm"
            | "gradle" | "cmake" | "make" | "dockerfile" | "gitignore" => Category::Code,
            // Applications and libraries
            "exe" | "msi" | "msix" | "appx" | "dll" | "so" | "dylib" | "com" | "scr"
            | "cpl" | "ocx" => Category::App,
            // System data, caches, databases
            "dat" | "db" | "sqlite" | "sqlite3" | "edb" | "etl" | "evtx" | "dmp" | "mdmp"
            | "cache" | "tmp" | "temp" | "bak" | "old" | "ini" | "inf" | "cfg" | "conf"
            | "reg" | "lock" | "pdb" | "manifest" | "mum" | "cat" | "bin" | "swp" | "sys" | "drv" | "efi" => {
                Category::System
            }
            _ => Category::Other,
        }
    }

    pub fn from_name(name: &str) -> Category {
        if let Some(ext) = extension_of(name) {
            let category = Category::from_ext(ext);
            if category != Category::Other {
                return category;
            }
        }
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "pagefile.sys" | "hiberfil.sys" | "swapfile.sys" | "desktop.ini" | "ntuser.dat"
            | "bootmgr" | "bootnxt" | "ntldr" | "thumbs.db" | "memory.dmp" => {
                Category::System
            }
            "makefile" | "dockerfile" | "gemfile" | "rakefile" | "procfile" => Category::Code,
            "license" | "readme" | "changelog" => Category::Document,
            _ => Category::Other,
        }
    }
}

pub fn extension_of(name: &str) -> Option<&str> {
    let dot = name.rfind('.')?;
    if dot == 0 || dot + 1 >= name.len() {
        return None;
    }
    Some(&name[dot + 1..])
}

pub fn extension_label(name: &str) -> String {
    match extension_of(name) {
        Some(ext) if ext.len() <= 12 => format!(".{}", ext.to_ascii_lowercase()),
        Some(_) => "Complex".to_string(),
        None => "No extension".to_string(),
    }
}

#[derive(Debug)]
pub struct TreeNode {
    pub name: String,
    pub size: u64,
    pub files: u64,
    pub dirs: u64,
    pub mtime: i64,
    pub is_dir: bool,
    pub category: Category,
    pub children: Vec<TreeNode>,
    pub cat_sizes: [u64; 9],
    /// Content shared with another path through an NTFS hardlink.
    pub shared: bool,
}

impl TreeNode {
    pub fn file(name: impl Into<String>, size: u64, mtime: i64) -> Self {
        Self::file_with_share(name, size, mtime, false)
    }

    /// A file whose bytes are already accounted for under another path.
    pub fn shared_file(name: impl Into<String>, size: u64, mtime: i64) -> Self {
        Self::file_with_share(name, size, mtime, true)
    }

    fn file_with_share(name: impl Into<String>, size: u64, mtime: i64, shared: bool) -> Self {
        let name = name.into();
        let category = Category::from_name(&name);
        let mut cat_sizes = [0u64; 9];
        cat_sizes[category.index()] = size;
        Self {
            name,
            size,
            files: 1,
            dirs: 0,
            mtime,
            is_dir: false,
            category,
            children: Vec::new(),
            cat_sizes,
            shared,
        }
    }

    pub fn dir(name: impl Into<String>, mtime: i64, children: Vec<TreeNode>) -> Self {
        let name = name.into();
        let mut size = 0u64;
        let mut files = 0u64;
        let mut dirs = 0u64;
        let mut cat_sizes = [0u64; 9];
        for child in &children {
            // Hardlinked content is attributed to the first path only, so folder
            // totals match what the volume actually stores.
            if !child.shared {
                size = size.saturating_add(child.size);
                for (idx, value) in child.cat_sizes.iter().enumerate() {
                    cat_sizes[idx] = cat_sizes[idx].saturating_add(*value);
                }
            }
            files = files.saturating_add(child.files);
            dirs = dirs.saturating_add(child.dirs + u64::from(child.is_dir));
        }
        let category = cat_sizes
            .iter()
            .enumerate()
            .max_by_key(|(_, value)| **value)
            .map(|(idx, _)| Category::ALL[idx])
            .unwrap_or(Category::Other);
        Self {
            name,
            size,
            files,
            dirs,
            mtime,
            is_dir: true,
            category,
            children,
            cat_sizes,
            shared: false,
        }
    }

    pub fn sort_children(&mut self) {
        self.children
            .sort_unstable_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
        for child in &mut self.children {
            child.sort_children();
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub parent: Option<NodeId>,
    pub size: u64,
    pub files: u64,
    pub dirs: u64,
    pub mtime: i64,
    pub is_dir: bool,
    pub category: Category,
    pub children: Vec<NodeId>,
    #[serde(default)]
    pub shared: bool,
}

impl Node {
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtStat {
    pub ext: String,
    pub size: u64,
    pub count: u64,
    pub category: Category,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub root: NodeId,
    pub root_path: PathBuf,
    pub root_label: String,
    pub total_size: u64,
    pub total_files: u64,
    pub total_dirs: u64,
    pub errors: u64,
    pub elapsed: Duration,
    pub file_ids: Vec<NodeId>,
    pub ext_stats: Vec<ExtStat>,
    pub cat_sizes: [u64; 9],
    pub cat_counts: [u64; 9],
    pub demo: bool,
    /// Bytes skipped because a hardlink already credited them elsewhere.
    #[serde(default)]
    pub hardlinked_bytes: u64,
}

impl Tree {
    pub fn from_tree(
        root: TreeNode,
        root_path: PathBuf,
        root_label: String,
        elapsed: Duration,
        errors: u64,
        demo: bool,
    ) -> Self {
        let total_size = root.size;
        let total_files = root.files;
        let total_dirs = root.dirs;
        let mut builder = TreeBuilder {
            nodes: Vec::new(),
            file_ids: Vec::new(),
            cat_sizes: [0u64; 9],
            cat_counts: [0u64; 9],
        };
        let root_id = builder.push(root, None);
        let mut tree = Self {
            nodes: builder.nodes,
            root: root_id,
            root_path,
            root_label,
            total_size,
            total_files,
            total_dirs,
            errors,
            elapsed,
            file_ids: builder.file_ids,
            ext_stats: Vec::new(),
            cat_sizes: builder.cat_sizes,
            cat_counts: builder.cat_counts,
            demo,
            hardlinked_bytes: 0,
        };
        let nodes = &tree.nodes;
        let file_ids = &mut tree.file_ids;
        file_ids.sort_unstable_by(|a, b| nodes[*b as usize].size.cmp(&nodes[*a as usize].size));
        tree.rebuild_ext_stats();
        tree
    }

    pub fn rebuild_ext_stats(&mut self) {
        let mut map: HashMap<u64, ExtStat> = HashMap::new();
        for &id in &self.file_ids {
            let node = &self.nodes[id as usize];
            if node.is_empty() {
                continue;
            }
            let ext = extension_of(&node.name).unwrap_or("");
            let key = hash_extension(ext);
            let entry = map.entry(key).or_insert_with(|| ExtStat {
                ext: if ext.is_empty() {
                    "No extension".to_string()
                } else {
                    format!(".{}", ext.to_ascii_lowercase())
                },
                size: 0,
                count: 0,
                category: Category::from_ext(ext),
            });
            entry.size = entry.size.saturating_add(node.size);
            entry.count += 1;
        }
        let mut stats: Vec<ExtStat> = map.into_values().collect();
        stats.sort_unstable_by(|a, b| b.size.cmp(&a.size).then_with(|| a.ext.cmp(&b.ext)));
        self.ext_stats = stats;
    }

    pub fn recompute_category_stats(&mut self) {
        self.cat_sizes = [0u64; 9];
        self.cat_counts = [0u64; 9];
        for &id in &self.file_ids {
            let node = &self.nodes[id as usize];
            if node.is_empty() {
                continue;
            }
            let idx = node.category.index();
            self.cat_sizes[idx] = self.cat_sizes[idx].saturating_add(node.size);
            self.cat_counts[idx] += 1;
        }
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id as usize]
    }

    pub fn path(&self, id: NodeId) -> String {
        let mut names: Vec<&str> = Vec::new();
        let mut current = id;
        let mut guard = 0;
        while let Some(parent) = self.nodes[current as usize].parent {
            names.push(&self.nodes[current as usize].name);
            current = parent;
            guard += 1;
            if guard > 4096 {
                break;
            }
        }
        let mut out = self.root_path.to_string_lossy().into_owned();
        for name in names.iter().rev() {
            if !out.ends_with('\\') && !out.ends_with('/') {
                out.push('\\');
            }
            out.push_str(name);
        }
        out
    }

    pub fn path_buf(&self, id: NodeId) -> PathBuf {
        if self.demo {
            return PathBuf::from(self.path(id));
        }
        let mut path = self.root_path.clone();
        let mut names: Vec<&str> = Vec::new();
        let mut current = id;
        while let Some(parent) = self.nodes[current as usize].parent {
            names.push(&self.nodes[current as usize].name);
            current = parent;
        }
        for name in names.iter().rev() {
            path.push(name);
        }
        path
    }

    pub fn display_path(&self, id: NodeId, max_chars: usize) -> String {
        let path = self.path(id);
        if path.chars().count() <= max_chars {
            path
        } else {
            crate::util::truncate_middle(&path, max_chars)
        }
    }

    pub fn breadcrumbs(&self, id: NodeId) -> Vec<(NodeId, String)> {
        let mut chain: Vec<NodeId> = Vec::new();
        let mut current = id;
        loop {
            chain.push(current);
            match self.nodes[current as usize].parent {
                Some(parent) => current = parent,
                None => break,
            }
        }
        chain.reverse();
        chain
            .into_iter()
            .map(|node_id| {
                if node_id == self.root {
                    (node_id, self.root_label.clone())
                } else {
                    (node_id, self.nodes[node_id as usize].name.clone())
                }
            })
            .collect()
    }

    pub fn largest_files(&self, limit: usize) -> Vec<NodeId> {
        self.file_ids.iter().copied().take(limit).collect()
    }

    /// Files modified before `cutoff` (unix seconds), biggest first.
    pub fn files_older_than(&self, cutoff: i64, limit: usize) -> Vec<NodeId> {
        let mut ids: Vec<NodeId> = self
            .file_ids
            .iter()
            .copied()
            .filter(|id| {
                let node = &self.nodes[*id as usize];
                !node.is_empty() && node.mtime > 0 && node.mtime < cutoff
            })
            .collect();
        ids.sort_unstable_by(|a, b| self.nodes[*b as usize].size.cmp(&self.nodes[*a as usize].size));
        ids.truncate(limit);
        ids
    }

    /// Looks up a node by absolute path. Windows paths compare case-insensitively.
    pub fn find_by_path(&self, path: &Path) -> Option<NodeId> {
        if path == self.root_path {
            return Some(self.root);
        }
        let relative = path.strip_prefix(&self.root_path).ok()?;
        let mut current = self.root;
        for component in relative.components() {
            let name = component.as_os_str().to_string_lossy();
            current = self.nodes[current as usize]
                .children
                .iter()
                .copied()
                .find(|id| {
                    let node = &self.nodes[*id as usize];
                    if cfg!(windows) {
                        node.name.eq_ignore_ascii_case(&name)
                    } else {
                        node.name == name
                    }
                })?;
        }
        Some(current)
    }

    pub fn remove_path(&mut self, path: &Path) -> Option<(u64, u64)> {
        let id = self.find_by_path(path)?;
        self.remove_subtree(id)
    }

    pub fn folders_by_size(&self, limit: usize) -> Vec<NodeId> {
        let mut ids: Vec<NodeId> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.is_dir && !node.is_empty())
            .map(|(index, _)| index as NodeId)
            .collect();
        ids.sort_unstable_by(|a, b| self.nodes[*b as usize].size.cmp(&self.nodes[*a as usize].size));
        ids.truncate(limit);
        ids
    }

    pub fn scan_rate(&self) -> f64 {
        let secs = self.elapsed.as_secs_f64();
        if secs <= 0.001 {
            0.0
        } else {
            self.total_size as f64 / secs
        }
    }

    pub fn size_summary(&self) -> String {
        format!(
            "{} across {} files",
            format_bytes(self.total_size),
            crate::util::format_count(self.total_files)
        )
    }

    pub fn remove_subtree(&mut self, id: NodeId) -> Option<(u64, u64)> {
        if id == self.root {
            return None;
        }
        let parent = self.nodes[id as usize].parent?;
        // The subtree root already aggregates its descendants, so taking its
        // totals avoids counting nested folders twice.
        let removed_size = self.nodes[id as usize].size;
        let removed_files = self.nodes[id as usize].files;
        let mut stack = vec![id];
        let mut removed_ids = std::collections::HashSet::new();
        while let Some(current) = stack.pop() {
            let node = &self.nodes[current as usize];
            removed_ids.insert(current);
            for child in &node.children {
                stack.push(*child);
            }
        }
        let removed_dirs = self.nodes[id as usize].dirs + u64::from(self.nodes[id as usize].is_dir);
        self.file_ids.retain(|file_id| !removed_ids.contains(file_id));
        if let Some(pos) = self.nodes[parent as usize]
            .children
            .iter()
            .position(|child| *child == id)
        {
            self.nodes[parent as usize].children.remove(pos);
        }
        let mut ancestor = Some(parent);
        while let Some(current) = ancestor {
            let node = &mut self.nodes[current as usize];
            node.size = node.size.saturating_sub(removed_size);
            node.files = node.files.saturating_sub(removed_files);
            if current != self.root {
                node.dirs = node.dirs.saturating_sub(removed_dirs);
            }
            ancestor = self.nodes[current as usize].parent;
        }
        self.nodes[id as usize].children.clear();
        self.nodes[id as usize].name.clear();
        self.total_size = self.nodes[self.root as usize].size;
        self.total_files = self.nodes[self.root as usize].files;
        self.total_dirs = self.nodes[self.root as usize].dirs;
        self.rebuild_ext_stats();
        self.recompute_category_stats();
        Some((removed_size, removed_files))
    }
}

struct TreeBuilder {
    nodes: Vec<Node>,
    file_ids: Vec<NodeId>,
    cat_sizes: [u64; 9],
    cat_counts: [u64; 9],
}

impl TreeBuilder {
    fn push(&mut self, tree: TreeNode, parent: Option<NodeId>) -> NodeId {
        let id = self.nodes.len() as NodeId;
        let category = tree.category;
        let is_dir = tree.is_dir;
        let size = tree.size;
        self.nodes.push(Node {
            name: tree.name,
            parent,
            size,
            files: tree.files,
            dirs: tree.dirs,
            mtime: tree.mtime,
            is_dir,
            category,
            children: Vec::with_capacity(tree.children.len()),
            shared: tree.shared,
        });
        if !is_dir {
            self.file_ids.push(id);
            let idx = category.index();
            self.cat_sizes[idx] = self.cat_sizes[idx].saturating_add(size);
            self.cat_counts[idx] += 1;
        }
        for child in tree.children {
            let child_id = self.push(child, Some(id));
            self.nodes[id as usize].children.push(child_id);
        }
        id
    }
}

fn hash_extension(ext: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in ext.as_bytes() {
        hash ^= u64::from(byte.to_ascii_lowercase());
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> Tree {
        let root = TreeNode::dir(
            "Root",
            0,
            vec![
                TreeNode::dir(
                    "Videos",
                    0,
                    vec![
                        TreeNode::file("clip.mp4", 400, 0),
                        TreeNode::file("second.mkv", 200, 0),
                    ],
                ),
                TreeNode::file("notes.pdf", 300, 0),
                TreeNode::file("archive.zip", 100, 0),
            ],
        );
        Tree::from_tree(
            root,
            PathBuf::from("C:\\"),
            "Local Disk (C:)".to_string(),
            Duration::from_secs(1),
            0,
            false,
        )
    }

    #[test]
    fn builds_arena_and_stats() {
        let tree = sample_tree();
        assert_eq!(tree.total_size, 1000);
        assert_eq!(tree.total_files, 4);
        assert_eq!(tree.total_dirs, 1);
        assert_eq!(tree.file_ids.len(), 4);
        assert_eq!(tree.cat_sizes[Category::Video.index()], 600);
        assert_eq!(tree.ext_stats[0].ext, ".mp4");
        assert!(tree.path(tree.root).contains("C:"));
    }

    #[test]
    fn paths_follow_parents() {
        let tree = sample_tree();
        let videos = tree.nodes[tree.root as usize].children[0];
        assert!(tree.path(videos).ends_with("Videos"));
        assert_eq!(tree.breadcrumbs(videos).len(), 2);
    }

    #[test]
    fn removal_updates_ancestors() {
        let mut tree = sample_tree();
        let videos = tree.nodes[tree.root as usize].children[0];
        let removed = tree.remove_subtree(videos).unwrap();
        assert_eq!(removed.0, 600);
        assert_eq!(tree.total_size, 400);
        assert_eq!(tree.total_files, 2);
        assert_eq!(tree.nodes[tree.root as usize].children.len(), 2);
    }

    #[test]
    fn removal_by_path_finds_and_removes() {
        let mut tree = sample_tree();
        let clip = tree.root_path.join("Videos").join("clip.mp4");
        assert!(tree.find_by_path(&clip).is_some());
        let removed = tree.remove_path(&clip).unwrap();
        assert_eq!(removed, (400, 1));
        assert_eq!(tree.total_size, 600);
        assert_eq!(tree.total_files, 3);
        assert!(tree.find_by_path(&clip).is_none());
        assert!(tree.remove_path(&tree.root_path.join("ghost.bin")).is_none());
    }

    #[test]
    fn categories_map_common_files() {
        assert_eq!(Category::from_name("movie.mp4"), Category::Video);
        assert_eq!(Category::from_name("photo.JPG"), Category::Image);
        assert_eq!(Category::from_name("pagefile.sys"), Category::System);
        assert_eq!(Category::from_name("main.rs"), Category::Code);
        assert_eq!(Category::from_name("mystery.zzz"), Category::Other);
    }

    #[test]
    fn extension_detection_is_safe() {
        assert_eq!(extension_of(".gitignore"), None);
        assert_eq!(extension_of("archive.tar.gz"), Some("gz"));
        assert_eq!(extension_of("noext"), None);
        assert_eq!(extension_label("MOVIE.MP4"), ".mp4");
    }
}

