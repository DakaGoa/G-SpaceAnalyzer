use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::util::app_data_dir;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::Dark
    }
}

impl ThemeMode {
    pub fn toggled(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TreemapColor {
    Category,
    Age,
    Folder,
}

impl Default for TreemapColor {
    fn default() -> Self {
        Self::Category
    }
}

impl TreemapColor {
    pub fn label(self) -> &'static str {
        match self {
            Self::Category => "Type",
            Self::Age => "Age",
            Self::Folder => "Folder",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Category => Self::Age,
            Self::Age => Self::Folder,
            Self::Folder => Self::Category,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeMode,
    pub last_path: Option<String>,
    pub treemap_labels: bool,
    pub animations: bool,
    pub show_details_on_select: bool,
    pub confirm_delete: bool,
    pub sidebar_width: f32,
    pub details_width: f32,
    pub treemap_color: TreemapColor,
    pub dup_min_size_mb: u64,
    pub dup_current_folder: bool,
    pub old_file_years: u32,
    pub shell_integration: bool,
    pub detect_hardlinks: bool,
    pub overview_list: bool,
    /// Save snapshots as folder totals instead of the full file tree.
    pub snapshots_compact: bool,
    /// Erase deleted items from disk instead of moving them to the Recycle Bin.
    pub delete_permanently: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            last_path: None,
            treemap_labels: true,
            animations: true,
            show_details_on_select: true,
            confirm_delete: true,
            sidebar_width: 246.0,
            details_width: 302.0,
            treemap_color: TreemapColor::Category,
            dup_min_size_mb: 1,
            dup_current_folder: false,
            old_file_years: 2,
            shell_integration: false,
            detect_hardlinks: true,
            overview_list: false,
            snapshots_compact: true,
            delete_permanently: false,
        }
    }
}

impl Settings {
    pub fn path() -> PathBuf {
        app_data_dir().join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }
}
