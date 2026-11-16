// Views added on top of the original five: cleanup targets, old files and
// saved-scan comparison.

use eframe::egui;
use egui::{
    Align, Align2, Color32, CursorIcon, Layout, Pos2, Rect, RichText, Rounding, ScrollArea, Sense,
    Stroke, Ui, Vec2,
};
use egui_phosphor::regular as ph;

use crate::app::{Action, App, SortKey};
use crate::theme::{self, Palette};
use crate::util::{format_bytes, format_count, format_mtime, truncate_middle};

struct Row {
    title: String,
    subtitle: String,
    meta: String,
    right: String,
    fraction: f32,
    color: Color32,
}

fn card(ui: &mut Ui, palette: &Palette, height: f32, width: f32) -> Rect {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(12.0), palette.surface);
    ui.painter()
        .rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, palette.border));
    rect
}

fn heading(ui: &mut Ui, palette: &Palette, title: &str, subtitle: &str, margin: f32) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(margin);
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .font(theme::font_semibold(22.0))
                    .color(palette.text),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(subtitle)
                    .font(theme::font_regular(10.5))
                    .color(palette.muted),
            );
        });
    });
    ui.add_space(10.0);
}

fn chip(ui: &mut Ui, palette: &Palette, text: &str, selected: bool) -> bool {
    let font = theme::font_medium(10.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), palette.muted);
    let size = Vec2::new(galley.size().x + 20.0, 26.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if selected {
        palette.accent_soft
    } else if response.hovered() {
        palette.surface_2
    } else {
        palette.surface
    };
    ui.painter().rect_filled(rect, Rounding::same(7.0), fill);
    ui.painter().rect_stroke(
        rect,
        Rounding::same(7.0),
        Stroke::new(
            1.0,
            if selected {
                theme::with_alpha(palette.accent, 150)
            } else {
                palette.border
            },
        ),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        font,
        if selected { palette.text } else { palette.muted },
    );
    response.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// Paints a scrollable list and returns the row that was clicked this frame.
fn rows(
    ui: &mut Ui,
    tree_rows: &[Row],
    palette: &Palette,
    height: f32,
    empty_text: &str,
    mut menu: impl FnMut(&egui::Response, usize),
) -> Option<usize> {
    if tree_rows.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(height * 0.3);
            ui.label(
                RichText::new(empty_text)
                    .font(theme::font_medium(12.0))
                    .color(palette.faint),
            );
        });
        return None;
    }
    let mut clicked = None;
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, 52.0, tree_rows.len(), |ui, range| {
            ui.set_max_width((ui.available_width() - 10.0).max(60.0));
            for index in range {
                let row = &tree_rows[index];
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 52.0), Sense::click());
                if response.hovered() {
                    ui.painter()
                        .rect_filled(rect.shrink2(Vec2::new(0.0, 2.0)), Rounding::same(7.0), palette.surface_2);
                }
                ui.painter().text(
                    Pos2::new(rect.left() + 12.0, rect.top() + 18.0),
                    Align2::LEFT_CENTER,
                    truncate_middle(&row.title, 56),
                    theme::font_medium(11.5),
                    palette.text,
                );
                ui.painter().text(
                    Pos2::new(rect.left() + 12.0, rect.top() + 34.0),
                    Align2::LEFT_CENTER,
                    truncate_middle(&row.subtitle, 74),
                    theme::font_regular(9.5),
                    palette.faint,
                );
                ui.painter().text(
                    Pos2::new(rect.right() - 12.0, rect.top() + 17.0),
                    Align2::RIGHT_CENTER,
                    &row.right,
                    theme::font_semibold(11.5),
                    palette.text,
                );
                if !row.meta.is_empty() {
                    ui.painter().text(
                        Pos2::new(rect.right() - 12.0, rect.top() + 33.0),
                        Align2::RIGHT_CENTER,
                        &row.meta,
                        theme::font_regular(9.5),
                        palette.muted,
                    );
                }
                if row.fraction > 0.0 {
                    let bar = Rect::from_min_size(
                        Pos2::new(rect.left() + 12.0, rect.bottom() - 7.0),
                        Vec2::new(rect.width() - 24.0, 3.0),
                    );
                    theme::progress_bar(ui.painter(), bar, row.fraction, row.color, palette.surface_3);
                }
                if response.clicked() {
                    clicked = Some(index);
                }
                menu(&response, index);
            }
        });
    clicked
}

impl App {
    // --- Cleanup targets ---------------------------------------------------

    pub(crate) fn view_cleanup(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let palette = &self.palette;
        let margin = 20.0;
        let total_known: u64 = self
            .cleanup
            .as_ref()
            .map(|state| state.sizes.iter().flatten().map(|entry| entry.0).sum())
            .unwrap_or(0);

        heading(
            ui,
            palette,
            "Clean up",
            &format!(
                "{} in caches, temporary folders and other known space hogs",
                format_bytes(total_known)
            ),
            margin,
        );

        ui.horizontal(|ui| {
            ui.add_space(margin);
            if theme::primary_button(
                ui,
                palette,
                if self.cleanup.is_some() { "Measure again" } else { "Measure targets" },
                Some(ph::ARROWS_CLOCKWISE),
                Some(170.0),
                34.0,
            )
            .clicked()
            {
                actions.push(Action::StartCleanupScan);
            }
            if let Some(state) = &self.cleanup {
                if state.job.is_some() {
                    ui.add(egui::Spinner::new().size(13.0).color(palette.accent));
                    ui.label(
                        RichText::new(&state.progress)
                            .font(theme::font_regular(10.0))
                            .color(palette.muted),
                    );
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(margin);
                ui.label(
                    RichText::new(if self.settings.delete_permanently {
                        "Clearing erases files permanently"
                    } else {
                        "Clearing moves files to the Recycle Bin"
                    })
                    .font(theme::font_regular(9.5))
                    .color(palette.faint),
                );
            });
        });
        ui.add_space(10.0);

        let Some(state) = &self.cleanup else {
            let width = (ui.available_width() - margin * 2.0).max(300.0);
            let height = (ui.available_height() - 12.0).max(200.0);
            let rect = ui
                .horizontal(|ui| {
                    ui.add_space(margin);
                    card(ui, palette, height, width)
                })
                .inner;
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Measure the usual suspects: temp folders, browser caches, Windows Update leftovers.",
                theme::font_medium(11.5),
                palette.faint,
            );
            return;
        };

        let width = (ui.available_width() - margin * 2.0).max(300.0);
        let height = (ui.available_height() - 12.0).max(160.0);
        let rect = ui
            .horizontal(|ui| {
                ui.add_space(margin);
                card(ui, palette, height, width)
            })
            .inner;

        let mut content = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink(10.0))
                .layout(Layout::top_down(Align::Min)),
        );

        let mut list = Vec::new();
        let mut clear_requests: Vec<usize> = Vec::new();
        for (index, target) in state.targets.iter().enumerate() {
            let measured = state.sizes.get(index).copied().flatten();
            let (bytes, files) = measured.unwrap_or((0, 0));
            let subtitle = if measured.is_some() {
                format!("{} files · {}", format_count(files), target.path.display())
            } else {
                format!("Measuring… · {}", target.path.display())
            };
            list.push(Row {
                title: target.title.to_string(),
                subtitle,
                meta: target.hint.to_string(),
                right: if measured.is_some() {
                    format_bytes(bytes)
                } else {
                    "—".to_string()
                },
                fraction: if bytes == 0 {
                    0.0
                } else {
                    (bytes as f64 / 1024.0f64.powi(3) as f64).min(1.0) as f32
                },
                color: palette.accent,
            });
        }

        // Clear buttons sit on their own strip so a click cannot be confused
        // with inspecting the row.
        let mut menu = |response: &egui::Response, index: usize| {
            let Some(target) = state.targets.get(index) else {
                return;
            };
            let path = target.path.clone();
            let clearable = target.clearable;
            response.context_menu(|ui| {
                if ui.button("Open file location").clicked() {
                    actions.push(Action::OpenExternal(path.clone()));
                    ui.close_menu();
                }
                if clearable {
                    if ui.button("Clear contents…").clicked() {
                        actions.push(Action::ClearCleanup(index));
                        ui.close_menu();
                    }
                }
            });
        };
        let selected = rows(
            &mut content,
            &list,
            palette,
            height - 46.0,
            "No targets found",
            &mut menu,
        );
        if let Some(index) = selected {
            if let Some(target) = state.targets.get(index) {
                actions.push(Action::OpenExternal(target.path.clone()));
            }
        }
        content.add_space(6.0);
        content.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for (index, target) in state.targets.iter().enumerate() {
                if !target.clearable {
                    continue;
                }
                let label = format!("Clear {}", truncate_middle(target.title, 22));
                if theme::secondary_button(ui, palette, &label, Some(ph::TRASH), None, 26.0).clicked()
                {
                    clear_requests.push(index);
                }
            }
        });
        for index in clear_requests {
            actions.push(Action::ClearCleanup(index));
        }
    }

    // --- Old files ---------------------------------------------------------

    pub(crate) fn view_old_files(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let Some(tree) = &self.tree else {
            return;
        };
        let palette = &self.palette;
        let margin = 20.0;
        let years = self.settings.old_file_years;
        let cutoff = chrono::Local::now().timestamp() - i64::from(years) * 365 * 24 * 3600;
        let mut ids = tree.files_older_than(cutoff, 4000);
        sort_ids(tree, &mut ids, self.old_sort);

        let bytes: u64 = ids.iter().map(|id| tree.node(*id).size).sum();
        heading(
            ui,
            palette,
            "Old files",
            &format!(
                "{} untouched for at least {} year{} · {} files",
                format_bytes(bytes),
                years,
                if years == 1 { "" } else { "s" },
                format_count(ids.len() as u64)
            ),
            margin,
        );

        ui.horizontal(|ui| {
            ui.add_space(margin);
            for option in [1u32, 2, 3, 5, 10] {
                if chip(
                    ui,
                    palette,
                    &format!("{option}y+"),
                    self.settings.old_file_years == option,
                ) {
                    actions.push(Action::SetOldYears(option));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(margin);
                if chip(ui, palette, "Modified", self.old_sort.key == SortKey::Modified) {
                    actions.push(Action::SortBy(SortKey::Modified));
                }
                if chip(ui, palette, "Name", self.old_sort.key == SortKey::Name) {
                    actions.push(Action::SortBy(SortKey::Name));
                }
                if chip(ui, palette, "Size", self.old_sort.key == SortKey::Size) {
                    actions.push(Action::SortBy(SortKey::Size));
                }
            });
        });
        ui.add_space(10.0);

        let width = (ui.available_width() - margin * 2.0).max(300.0);
        let height = (ui.available_height() - 12.0).max(160.0);
        let rect = ui
            .horizontal(|ui| {
                ui.add_space(margin);
                card(ui, palette, height, width)
            })
            .inner;
        let mut content = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink(10.0))
                .layout(Layout::top_down(Align::Min)),
        );

        let list: Vec<Row> = ids
            .iter()
            .map(|id| {
                let node = tree.node(*id);
                Row {
                    title: node.name.clone(),
                    subtitle: tree.path(*id),
                    meta: format_mtime(node.mtime),
                    right: format_bytes(node.size),
                    fraction: 0.0,
                    color: palette.category(node.category),
                }
            })
            .collect();
        let mut menu = |response: &egui::Response, index: usize| {
            if let Some(id) = ids.get(index) {
                crate::views::file_menu(response, tree, *id, actions);
            }
        };
        if let Some(index) = rows(
            &mut content,
            &list,
            palette,
            height - 20.0,
            "Nothing this old",
            &mut menu,
        ) {
            actions.push(Action::Select(ids[index]));
        }
    }

    // --- Snapshots ---------------------------------------------------------

    pub(crate) fn view_snapshots(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let palette = &self.palette;
        let margin = 20.0;
        let now = chrono::Local::now().timestamp();

        let disk_total: u64 = self.snapshots.iter().map(|meta| meta.file_size).sum();
        heading(
            ui,
            palette,
            "Snapshots",
            &format!(
                "{} saved · {} on disk · compare any one with the current scan",
                format_count(self.snapshots.len() as u64),
                format_bytes(disk_total)
            ),
            margin,
        );

        ui.horizontal(|ui| {
            ui.add_space(margin);
            if theme::primary_button(
                ui,
                palette,
                "Save current scan",
                Some(ph::FLOPPY_DISK),
                Some(184.0),
                34.0,
            )
            .clicked()
            {
                actions.push(Action::SaveSnapshot);
            }
            if theme::secondary_button(
                ui,
                palette,
                "Refresh",
                Some(ph::ARROWS_CLOCKWISE),
                Some(104.0),
                34.0,
            )
            .clicked()
            {
                actions.push(Action::RefreshSnapshots);
            }
            if let Some(note) = &self.snapshot_note {
                ui.label(
                    RichText::new(note)
                        .font(theme::font_regular(9.5))
                        .color(palette.faint),
                );
            }
        });
        ui.add_space(10.0);

        if self.snapshots.is_empty() {
            let width = (ui.available_width() - margin * 2.0).max(300.0);
            let height = (ui.available_height() - 12.0).max(200.0);
            let rect = ui
                .horizontal(|ui| {
                    ui.add_space(margin);
                    card(ui, palette, height, width)
                })
                .inner;
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "No snapshots yet. Save one after a scan to track growth over time.",
                theme::font_medium(11.5),
                palette.faint,
            );
            return;
        }

        let width = (ui.available_width() - margin * 2.0).max(240.0);
        let list_h = (self.snapshots.len() as f32 * 52.0 + 16.0)
            .clamp(72.0, (ui.available_height() * 0.42).max(90.0));
        let list_rect = ui
            .horizontal(|ui| {
                ui.add_space(margin);
                card(ui, palette, list_h, width)
            })
            .inner;
        let mut list_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(list_rect.shrink(8.0))
                .layout(Layout::top_down(Align::Min)),
        );

        let rows_data: Vec<Row> = self
            .snapshots
            .iter()
            .map(|meta| Row {
                title: meta.label.clone(),
                subtitle: format!(
                    "{} · {} · {} files · {} on disk",
                    crate::snapshots::relative_time(meta.created, now),
                    meta.root_label,
                    format_count(meta.total_files),
                    format_bytes(meta.file_size)
                ),
                meta: meta
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                right: format_bytes(meta.total_size),
                fraction: 0.0,
                color: palette.accent,
            })
            .collect();

        if let Some(index) = rows(
            &mut list_ui,
            &rows_data,
            palette,
            list_h - 20.0,
            "No snapshots",
            |_, _| {},
        ) {
            actions.push(Action::SelectSnapshot(index));
            actions.push(Action::CompareSnapshot(index));
        }
        // These live on the page rather than in the card, so a long list can
        // never push them across the card edge.
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(margin);
            if let Some(index) = self.snapshot_selected {
                if index < self.snapshots.len() {
                    if theme::secondary_button(
                        ui,
                        palette,
                        "Compare with current",
                        Some(ph::ARROWS_CLOCKWISE),
                        None,
                        26.0,
                    )
                    .clicked()
                    {
                        actions.push(Action::CompareSnapshot(index));
                    }
                    if theme::secondary_button(
                        ui,
                        palette,
                        "Delete",
                        Some(ph::TRASH),
                        None,
                        26.0,
                    )
                    .clicked()
                    {
                        actions.push(Action::DeleteSnapshot(index));
                    }
                }
            }
        });

        ui.add_space(10.0);
        let diff_h = (ui.available_height() - 12.0).max(140.0);
        let diff_rect = ui
            .horizontal(|ui| {
                ui.add_space(margin);
                card(ui, palette, diff_h, width)
            })
            .inner;
        let mut diff_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(diff_rect.shrink(8.0))
                .layout(Layout::top_down(Align::Min)),
        );

        let Some(diff) = &self.snapshot_diff else {
            diff_ui.vertical_centered(|ui| {
                ui.add_space(diff_h * 0.32);
                ui.label(
                    RichText::new("Pick a snapshot to compare folder by folder")
                        .font(theme::font_medium(11.5))
                        .color(palette.faint),
                );
            });
            return;
        };
        if diff.is_empty() {
            diff_ui.vertical_centered(|ui| {
                ui.add_space(diff_h * 0.32);
                ui.label(
                    RichText::new("No meaningful difference since this snapshot")
                        .font(theme::font_medium(11.5))
                        .color(palette.faint),
                );
            });
            return;
        }

        let rows_data: Vec<Row> = diff
            .iter()
            .take(600)
            .map(|row| {
                let delta = row.delta();
                let (color, sign) = if delta >= 0 {
                    (palette.danger, "+")
                } else {
                    (palette.success, "−")
                };
                Row {
                    title: truncate_middle(&row.path, 72),
                    subtitle: format!(
                        "{} → {}",
                        format_bytes(row.old_size),
                        format_bytes(row.new_size)
                    ),
                    meta: format!("{:+.0}%", row.change_ratio() * 100.0),
                    right: format!("{sign}{}", format_bytes(delta.unsigned_abs())),
                    fraction: row.change_ratio().abs().min(1.0),
                    color,
                }
            })
            .collect();
        let _ = rows(&mut diff_ui, &rows_data, palette, diff_h - 16.0, "", |_, _| {});
    }
}

/// Sorting for the plain file lists (largest / old files / search results).
pub(crate) fn sort_ids(
    tree: &crate::model::Tree,
    ids: &mut [crate::model::NodeId],
    sort: crate::app::SortState,
) {
    match sort.key {
        SortKey::Size => ids.sort_unstable_by(|a, b| {
            let ordering = tree.node(*a).size.cmp(&tree.node(*b).size);
            if sort.descending {
                ordering.reverse()
            } else {
                ordering
            }
        }),
        SortKey::Name => ids.sort_unstable_by(|a, b| {
            let ordering = tree
                .node(*a)
                .name
                .to_lowercase()
                .cmp(&tree.node(*b).name.to_lowercase());
            if sort.descending {
                ordering.reverse()
            } else {
                ordering
            }
        }),
        SortKey::Modified | SortKey::Count => ids.sort_unstable_by(|a, b| {
            let ordering = tree.node(*a).mtime.cmp(&tree.node(*b).mtime);
            if sort.descending {
                ordering.reverse()
            } else {
                ordering
            }
        }),
    }
}

/// Small helper used by the duplicates view for its header chips.
pub(crate) fn dup_chip(ui: &mut Ui, palette: &Palette, text: &str, selected: bool) -> bool {
    chip(ui, palette, text, selected)
}

