use eframe::egui;
use egui::{
    Align, Align2, Color32, CursorIcon, Frame, Id, Layout, Margin, Order, Pos2, RichText, Rounding,
    Sense, Stroke, Ui, Vec2,
};
use egui_phosphor::regular as ph;

use crate::app::{App, DeleteTarget, DupState, Modal, ToastKind};
use crate::settings::{ThemeMode, TreemapColor};
use crate::theme::{self, Palette};
use crate::util::{format_bytes, format_count, truncate_middle};

impl App {
    pub(crate) fn modals(&mut self, ctx: &egui::Context) {
        let Some(modal) = self.modal.clone() else {
            self.modal_shown = None;
            return;
        };
        let key = std::mem::discriminant(&modal);
        // A modal opened by this frame's click must not be dismissed by that
        // same click, so the outside-click check starts one frame later.
        let already_shown = self.modal_shown == Some(key);
        self.modal_shown = Some(key);
        let screen = ctx.screen_rect();
        let dim = ctx.layer_painter(egui::LayerId::new(
            Order::Foreground,
            Id::new("modal_dim"),
        ));
        dim.rect_filled(
            screen,
            Rounding::ZERO,
            Color32::from_black_alpha(if self.palette.dark { 165 } else { 85 }),
        );
        let area = egui::Area::new(Id::new("modal_card"))
            .order(Order::Tooltip)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                Frame::none()
                    .fill(self.palette.surface)
                    .stroke(Stroke::new(1.0, self.palette.border_strong))
                    .rounding(Rounding::same(14.0))
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, 18.0),
                        blur: 48.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(160),
                    })
                    .inner_margin(Margin::same(22.0))
                    .show(ui, |ui| match &modal {
                        Modal::Settings => self.settings_modal(ui, ctx),
                        Modal::About => self.about_modal(ui, ctx),
                        Modal::Delete(target) => self.delete_modal(ui, ctx, *target),
                        Modal::BulkDelete { .. } => self.bulk_delete_modal(ui, ctx, &modal),
                    });
            });
        if already_shown && ctx.input(|input| input.pointer.any_click()) {
            if let Some(position) = ctx.input(|input| input.pointer.interact_pos()) {
                if !area.response.rect.contains(position) {
                    self.modal = None;
                }
            }
        }
    }

    fn settings_modal(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let palette = self.palette;
        ui.set_width(478.0);
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Settings")
                        .font(theme::font_semibold(19.0))
                        .color(palette.text),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new("Tune how G-SpaceAnalyzer looks and behaves")
                        .font(theme::font_regular(10.0))
                        .color(palette.muted),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::icon_button(ui, &palette, ph::X, 28.0, "Close").clicked() {
                    self.modal = None;
                }
            });
        });
        ui.add_space(15.0);
        egui::ScrollArea::vertical()
            .max_height(600.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
        theme::section_label(ui, &palette, "Appearance");
        ui.add_space(7.0);
        setting_row(
            ui,
            &palette,
            "Theme",
            "Dark is tuned for long analysis sessions",
            |ui| {
                let mut mode = self.settings.theme;
                if theme_segmented(ui, &palette, &mut mode) {
                    self.settings.theme = mode;
                    changed = true;
                }
            },
        );
        ui.add_space(10.0);
        setting_row(
            ui,
            &palette,
            "Interface motion",
            "Subtle transitions and hover feedback",
            |ui| {
                if toggle_switch(ui, &palette, &mut self.settings.animations) {
                    changed = true;
                }
            },
        );
        ui.add_space(10.0);
        setting_row(
            ui,
            &palette,
            "Treemap labels",
            "Show file and folder names inside large tiles",
            |ui| {
                if toggle_switch(ui, &palette, &mut self.settings.treemap_labels) {
                    changed = true;
                }
            },
        );
        ui.add_space(10.0);
        setting_row(
            ui,
            &palette,
            "Treemap colours",
            "Shade tiles by file type, age, or folder",
            |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for mode in [
                    TreemapColor::Category,
                    TreemapColor::Age,
                    TreemapColor::Folder,
                ] {
                    if crate::views_extra::dup_chip(
                        ui,
                        &palette,
                        mode.label(),
                        self.settings.treemap_color == mode,
                    ) {
                        self.settings.treemap_color = mode;
                        changed = true;
                    }
                }
            },
        );
        ui.add_space(16.0);
        theme::section_label(ui, &palette, "Behaviour");
        ui.add_space(7.0);
        setting_row(
            ui,
            &palette,
            "Count hardlinks once",
            "Exact NTFS totals; opens larger files, so scans are a little slower",
            |ui| {
                if toggle_switch(ui, &palette, &mut self.settings.detect_hardlinks) {
                    changed = true;
                }
            },
        );
        ui.add_space(10.0);
        setting_row(
            ui,
            &palette,
            "Compact snapshots",
            "Save folder totals only, so a snapshot takes a fraction of the disk space",
            |ui| {
                if toggle_switch(ui, &palette, &mut self.settings.snapshots_compact) {
                    changed = true;
                }
            },
        );
        ui.add_space(10.0);
        setting_row(
            ui,
            &palette,
            "Permanent delete",
            "Erase deleted items from the disk instead of moving them to the Recycle Bin",
            |ui| {
                if toggle_switch(ui, &palette, &mut self.settings.delete_permanently) {
                    changed = true;
                }
            },
        );
        if self.settings.delete_permanently {
            ui.add_space(8.0);
            Frame::none()
                .fill(palette.danger_soft)
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::symmetric(10.0, 8.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.label(
                            RichText::new(ph::WARNING)
                                .font(theme::font_icon(11.0))
                                .color(palette.danger),
                        );
                        ui.label(
                            RichText::new(
                                "Deleted files and folders are erased immediately and cannot be restored. The Recycle Bin is skipped entirely.",
                            )
                            .font(theme::font_regular(9.3))
                            .color(palette.danger),
                        );
                    });
                });
        }
        ui.add_space(10.0);
        setting_row(
            ui,
            &palette,
            "Explorer context menu",
            "Right-click a folder to analyse it with G-SpaceAnalyzer",
            |ui| {
                if toggle_switch(ui, &palette, &mut self.settings.shell_integration) {
                    let wanted = self.settings.shell_integration;
                    let result = if wanted {
                        crate::platform::install_shell_integration()
                    } else {
                        crate::platform::remove_shell_integration()
                    };
                    match result {
                        Ok(()) => changed = true,
                        Err(error) => {
                            self.settings.shell_integration = !wanted;
                            self.toast(ctx, error, ToastKind::Error);
                        }
                    }
                }
            },
        );
        ui.add_space(10.0);
        setting_row(
            ui,
            &palette,
            "Open inspector on selection",
            "Show the right-hand details panel when an item is clicked",
            |ui| {
                if toggle_switch(
                    ui,
                    &palette,
                    &mut self.settings.show_details_on_select,
                ) {
                    changed = true;
                }
            },
        );
        ui.add_space(18.0);
        Frame::none()
            .fill(palette.surface_2)
            .rounding(Rounding::same(9.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(ph::INFO)
                            .font(theme::font_icon(14.0))
                            .color(palette.accent),
                    );
                    ui.label(
                        RichText::new(
                            "No account, license key, telemetry or network access. Everything is processed on this PC.",
                        )
                        .font(theme::font_regular(9.8))
                        .color(palette.muted),
                    );
                });
            });
        ui.add_space(10.0);
        if theme::secondary_button(
            ui,
            &palette,
            "All options explained",
            Some(ph::INFO),
            None,
            30.0,
        )
        .on_hover_text("Open the About panel")
        .clicked()
        {
            self.modal = Some(Modal::About);
        }
            });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(concat!("G-SpaceAnalyzer ", env!("CARGO_PKG_VERSION")))
                        .font(theme::font_medium(10.0))
                        .color(palette.muted),
                );
                ui.label(
                    RichText::new("Free & open source · MIT licensed")
                        .font(theme::font_regular(9.0))
                        .color(palette.faint),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::primary_button(ui, &palette, "Done", None, Some(96.0), 34.0).clicked() {
                    self.modal = None;
                }
            });
        });
        if changed {
            self.settings.save();
            theme::apply_theme(ctx, self.settings.theme, self.settings.animations);
            self.palette = Palette::for_mode(self.settings.theme);
            self.applied_theme = self.settings.theme;
            self.applied_animations = self.settings.animations;
        }
    }

    fn about_modal(&mut self, ui: &mut Ui, _ctx: &egui::Context) {
        let palette = self.palette;
        ui.set_width(620.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("About G-SpaceAnalyzer")
                        .font(theme::font_semibold(19.0))
                        .color(palette.text),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(concat!(
                        "Version ",
                        env!("CARGO_PKG_VERSION"),
                        " · every option, what it does, and a tip for each"
                    ))
                    .font(theme::font_regular(10.0))
                    .color(palette.muted),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::icon_button(ui, &palette, ph::X, 28.0, "Close").clicked() {
                    self.modal = None;
                }
            });
        });
        ui.add_space(14.0);
        egui::ScrollArea::vertical()
            .max_height(470.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width((ui.available_width() - 10.0).max(200.0));
                about_section(
                    ui,
                    &palette,
                    "Scanning",
                    &[
                        (
                            "Scan this PC",
                            "Maps the drive Windows is installed on.",
                            "F5 rescans whatever folder you are looking at.",
                        ),
                        (
                            "Choose a folder",
                            "Scans any folder or drive you pick.",
                            "Turn on the Explorer menu in Settings to start a scan from a folder's right-click menu.",
                        ),
                        (
                            "Stop",
                            "Appears in the status bar while a scan runs.",
                            "The partial result is kept, so you can still browse what was already scanned.",
                        ),
                        (
                            "Restart as admin",
                            "Relaunches the app with administrator rights and rescans.",
                            "Protected folders are only visible elevated, and scans finish faster.",
                        ),
                    ],
                );
                about_section(
                    ui,
                    &palette,
                    "Moving around",
                    &[
                        (
                            "Back and Up",
                            "Walk your history or step up one folder level.",
                            "Alt+Left or Backspace also goes back.",
                        ),
                        (
                            "Breadcrumbs",
                            "Click any part of the path to jump there.",
                            "Ctrl+F jumps straight to the search box.",
                        ),
                        (
                            "Map / List",
                            "The space map as a treemap or as a size-ordered list.",
                            "List is quicker to read in folders with hundreds of children.",
                        ),
                    ],
                );
                about_section(
                    ui,
                    &palette,
                    "Finding space",
                    &[
                        (
                            "Overview",
                            "Totals, the biggest folders and the space map.",
                            "Double-click a folder to step into it; right-click for actions.",
                        ),
                        (
                            "Largest files",
                            "Every file in the scan, sortable by size, name or date.",
                            "Right-click a row to open its location or delete it.",
                        ),
                        (
                            "File types",
                            "Space and count per extension and category.",
                            "Sort by count to find thousands of small files worth clearing.",
                        ),
                        (
                            "Duplicates",
                            "Files with identical content, found by size then hash.",
                            "Raise \"Smallest\" to 16 MB and scope to this folder to cut noise.",
                        ),
                        (
                            "Search",
                            "Filters the scan by file or folder name.",
                            "Results follow the same sort chips as the largest-files table.",
                        ),
                        (
                            "Clean up",
                            "Temp folders, browser caches and update leftovers.",
                            "Clearing asks first; items stay restorable in the Recycle Bin unless Permanent delete is on.",
                        ),
                        (
                            "Old files",
                            "Files that have not been touched for at least N years.",
                            "Start at 2y+ to find archive candidates without wading through noise.",
                        ),
                        (
                            "Snapshots",
                            "Saved scans you can compare with the current one.",
                            "Turn on Compact snapshots in Settings to save folder totals only, which takes a fraction of the space.",
                        ),
                    ],
                );
                about_section(
                    ui,
                    &palette,
                    "Doing things",
                    &[
                        (
                            "Right-click menu",
                            "Open file or folder, open its location, copy the path, delete.",
                            "Delete always asks first, then either erases the item or moves it to the Recycle Bin.",
                        ),
                        (
                            "Export (Ctrl+E)",
                            "Writes the current scan as CSV or JSON.",
                            "The largest-files export makes a quick inventory of what matters.",
                        ),
                        (
                            "Settings",
                            "Theme, treemap labels and colours, hardlink accuracy, snapshots, delete method, Explorer menu.",
                            "Colouring the treemap by Age finds stale data at a glance.",
                        ),
                        (
                            "Keyboard",
                            "F5 rescan, Ctrl+O open, Ctrl+F search, Ctrl+E export, Alt+Left back, Esc close.",
                            "Esc clears the search first, then closes panels, so it never loses your place.",
                        ),
                    ],
                );
            });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("No account, licence key, telemetry or network access.")
                    .font(theme::font_regular(9.5))
                    .color(palette.faint),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::primary_button(ui, &palette, "Done", None, Some(96.0), 34.0).clicked() {
                    self.modal = None;
                }
            });
        });
    }

    fn bulk_delete_modal(&mut self, ui: &mut Ui, ctx: &egui::Context, modal: &Modal) {
        let Modal::BulkDelete {
            paths,
            bytes,
            label,
        } = modal
        else {
            return;
        };
        let palette = self.palette;
        let permanent = self.settings.delete_permanently;
        let count = paths.len();
        ui.set_width(446.0);
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
        ui.painter()
            .rect_filled(icon_rect, Rounding::same(11.0), palette.danger_soft);
        ui.painter().text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            ph::TRASH,
            theme::font_icon(20.0),
            palette.danger,
        );
        ui.add_space(10.0);
        ui.label(
            RichText::new(if permanent {
                "Delete these duplicates permanently?"
            } else {
                "Move duplicates to the Recycle Bin?"
            })
            .font(theme::font_semibold(17.0))
            .color(palette.text),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(label.clone())
                .font(theme::font_medium(10.5))
                .color(palette.accent),
        );
        ui.add_space(8.0);
        let detail = if permanent {
            format!(
                "{} files · {} will be erased from the disk. This cannot be undone.",
                format_count(count as u64),
                format_bytes(*bytes)
            )
        } else {
            format!(
                "{} files · {} will be moved. Every item can be restored from the Recycle Bin.",
                format_count(count as u64),
                format_bytes(*bytes)
            )
        };
        ui.label(
            RichText::new(detail)
                .font(theme::font_regular(10.0))
                .color(if permanent { palette.danger } else { palette.muted }),
        );
        ui.add_space(16.0);
        let mut confirm = false;
        ui.horizontal(|ui| {
            if theme::secondary_button(ui, &palette, "Cancel", Some(ph::X), Some(112.0), 32.0)
                .clicked()
            {
                self.modal = None;
            }
            if theme::primary_button(
                ui,
                &palette,
                if permanent {
                    "Delete permanently"
                } else {
                    "Move to Recycle Bin"
                },
                Some(ph::TRASH),
                None,
                32.0,
            )
            .clicked()
            {
                confirm = true;
            }
        });
        if confirm {
            self.modal = None;
            self.bulk_delete(ctx, paths.clone(), *bytes, label.clone());
        }
    }

    fn delete_modal(&mut self, ui: &mut Ui, ctx: &egui::Context, target: DeleteTarget) {
        let palette = self.palette;
        let permanent = self.settings.delete_permanently;
        let info = self.delete_info(target);
        let Some((name, path, size, is_dir, is_demo)) = info else {
            self.modal = None;
            return;
        };
        ui.set_width(446.0);
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
        ui.painter()
            .rect_filled(icon_rect, Rounding::same(11.0), palette.danger_soft);
        ui.painter().text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            ph::TRASH,
            theme::font_icon(20.0),
            palette.danger,
        );
        ui.add_space(12.0);
        ui.label(
            RichText::new(if permanent {
                "Delete permanently?"
            } else {
                "Move to the Recycle Bin?"
            })
            .font(theme::font_semibold(18.0))
            .color(palette.text),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "{} · {}{}",
                if is_dir { "Folder" } else { "File" },
                format_bytes(size),
                if is_demo { " · sample data" } else { "" }
            ))
            .font(theme::font_regular(10.0))
            .color(palette.muted),
        );
        ui.add_space(13.0);
        Frame::none()
            .fill(palette.surface_2)
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(truncate_middle(&name, 52))
                        .font(theme::font_medium(11.0))
                        .color(palette.text),
                );
                ui.label(
                    RichText::new(truncate_middle(&path, 68))
                        .font(theme::font_regular(9.0))
                        .color(palette.faint),
                );
            });
        ui.add_space(13.0);
        ui.label(
            RichText::new(if permanent {
                "The item is erased from the disk immediately. It does not go to the Recycle Bin and cannot be restored. Locked or protected files may fail."
            } else {
                "The item is moved to the Windows Recycle Bin and can be restored from there. Locked or protected files may fail to move."
            })
            .font(theme::font_regular(9.8))
            .color(if permanent { palette.danger } else { palette.muted }),
        );
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            if theme::secondary_button(ui, &palette, "Cancel", None, Some(104.0), 34.0).clicked() {
                self.modal = None;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if is_demo {
                    ui.label(
                        RichText::new("Disabled for sample data")
                            .font(theme::font_regular(9.0))
                            .color(palette.faint),
                    );
                } else if danger_button(
                    ui,
                    &palette,
                    if permanent {
                        "Delete permanently"
                    } else {
                        "Move to Recycle Bin"
                    },
                )
                .clicked()
                {
                    self.perform_delete(ctx, target);
                    self.modal = None;
                }
            });
        });
    }

    fn delete_info(&self, target: DeleteTarget) -> Option<(String, String, u64, bool, bool)> {
        match target {
            DeleteTarget::Node(node) => {
                let tree = self.tree.as_ref()?;
                let node_data = tree.node(node);
                Some((
                    node_data.name.clone(),
                    tree.path(node),
                    node_data.size,
                    node_data.is_dir,
                    tree.demo,
                ))
            }
            DeleteTarget::Duplicate { node, group } => {
                let tree = self.tree.as_ref()?;
                let node_data = tree.node(node);
                if let DupState::Done(results) = &self.dupes {
                    if let Some(group) = results.groups.get(group) {
                        if let Some(file) = group.files.iter().find(|file| file.node == node) {
                            return Some((
                                file.path
                                    .file_name()
                                    .map(|name| name.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| node_data.name.clone()),
                                file.path.to_string_lossy().into_owned(),
                                file.size,
                                false,
                                tree.demo,
                            ));
                        }
                    }
                }
                Some((
                    node_data.name.clone(),
                    tree.path(node),
                    node_data.size,
                    node_data.is_dir,
                    tree.demo,
                ))
            }
        }
    }

    pub(crate) fn draw_toasts(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|input| input.time);
        self.toasts.retain(|toast| now - toast.created < 4.6);
        if self.toasts.is_empty() {
            return;
        }
        let screen = ctx.screen_rect();
        let mut y = screen.bottom() - crate::theme::STATUS_HEIGHT - 18.0;
        let mut dismiss = None;
        for toast in self.toasts.iter().rev() {
            y -= 52.0;
            let pos = Pos2::new(screen.right() - 356.0, y);
            egui::Area::new(Id::new(("toast", toast.id)))
                .order(Order::Tooltip)
                .fixed_pos(pos)
                .show(ctx, |ui| {
                    Frame::none()
                        .fill(self.palette.surface_3)
                        .stroke(Stroke::new(1.0, self.palette.border_strong))
                        .rounding(Rounding::same(9.0))
                        .shadow(egui::epaint::Shadow {
                            offset: Vec2::new(0.0, 6.0),
                            blur: 22.0,
                            spread: 0.0,
                            color: Color32::from_black_alpha(120),
                        })
                        .inner_margin(Margin::symmetric(10.0, 8.0))
                        .show(ui, |ui| {
                            ui.set_width(322.0);
                            ui.horizontal(|ui| {
                                let (icon, color) = match toast.kind {
                                    ToastKind::Success => {
                                        (ph::CHECK_CIRCLE, self.palette.success)
                                    }
                                    ToastKind::Error => (ph::X_CIRCLE, self.palette.danger),
                                    ToastKind::Info => (ph::INFO, self.palette.accent),
                                };
                                ui.label(
                                    RichText::new(icon)
                                        .font(theme::font_icon(16.0))
                                        .color(color),
                                );
                                ui.add_space(3.0);
                                ui.label(
                                    RichText::new(&toast.message)
                                        .font(theme::font_regular(10.0))
                                        .color(self.palette.text),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if theme::icon_button(
                                        ui,
                                        &self.palette,
                                        ph::X,
                                        18.0,
                                        "Dismiss",
                                    )
                                    .clicked()
                                    {
                                        dismiss = Some(toast.id);
                                    }
                                });
                            });
                        });
                });
            y -= 8.0;
        }
        if let Some(id) = dismiss {
            self.toasts.retain(|toast| toast.id != id);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
    }
}

/// One block of the About panel: a heading plus one card per option.
fn about_section(ui: &mut Ui, palette: &Palette, title: &str, items: &[(&str, &str, &str)]) {
    theme::section_label(ui, palette, title);
    ui.add_space(6.0);
    for (name, what, tip) in items {
        Frame::none()
            .fill(palette.surface_2)
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(*name)
                        .font(theme::font_medium(11.0))
                        .color(palette.text),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(*what)
                        .font(theme::font_regular(9.8))
                        .color(palette.muted),
                );
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    ui.label(
                        RichText::new(ph::INFO)
                            .font(theme::font_icon(10.0))
                            .color(palette.accent),
                    );
                    ui.label(
                        RichText::new(format!("Tip: {tip}"))
                            .font(theme::font_regular(9.3))
                            .color(palette.faint),
                    );
                });
            });
        ui.add_space(6.0);
    }
    ui.add_space(8.0);
}

fn setting_row(
    ui: &mut Ui,
    palette: &Palette,
    title: &str,
    subtitle: &str,
    control: impl FnOnce(&mut Ui),
) {
    let width = ui.available_width();
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new((width - 150.0).max(200.0), 38.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.label(
                    RichText::new(title)
                        .font(theme::font_medium(10.5))
                        .color(palette.text),
                );
                ui.label(
                    RichText::new(subtitle)
                        .font(theme::font_regular(8.8))
                        .color(palette.faint),
                );
            },
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            control(ui);
        });
    });
}

fn toggle_switch(ui: &mut Ui, palette: &Palette, value: &mut bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(36.0, 20.0), Sense::click());
    let t = ui.ctx().animate_bool_with_time(response.id, *value, 0.14);
    let bg = theme::mix(palette.surface_3, palette.accent, t);
    ui.painter().rect_filled(rect, Rounding::same(10.0), bg);
    let knob_x = rect.left() + 10.0 + (rect.width() - 20.0) * t;
    ui.painter()
        .circle_filled(Pos2::new(knob_x, rect.center().y), 7.0, Color32::WHITE);
    let changed = response.clicked();
    if changed {
        *value = !*value;
    }
    response.on_hover_cursor(CursorIcon::PointingHand);
    changed
}

fn theme_segmented(ui: &mut Ui, palette: &Palette, mode: &mut ThemeMode) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (value, label, icon) in [
            (ThemeMode::Dark, "Dark", ph::MOON),
            (ThemeMode::Light, "Light", ph::SUN),
        ] {
            let selected = *mode == value;
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(74.0, 28.0), Sense::click());
            let fill = if selected {
                palette.accent_soft
            } else if response.hovered() {
                palette.surface_2
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, Rounding::same(7.0), fill);
            if selected {
                ui.painter().rect_stroke(
                    rect,
                    Rounding::same(7.0),
                    Stroke::new(1.0, theme::with_alpha(palette.accent, 90)),
                );
            }
            ui.painter().text(
                Pos2::new(rect.center().x - 12.0, rect.center().y),
                Align2::CENTER_CENTER,
                icon,
                theme::font_icon(11.0),
                if selected { palette.accent } else { palette.faint },
            );
            ui.painter().text(
                Pos2::new(rect.center().x + 7.0, rect.center().y),
                Align2::CENTER_CENTER,
                label,
                theme::font_medium(9.5),
                if selected { palette.text } else { palette.muted },
            );
            if response.clicked() && !selected {
                *mode = value;
                changed = true;
            }
            response.on_hover_cursor(CursorIcon::PointingHand);
        }
    });
    changed
}

fn danger_button(ui: &mut Ui, palette: &Palette, label: &str) -> egui::Response {
    let font = theme::font_medium(11.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(galley.size().x + 28.0, 34.0),
        Sense::click(),
    );
    let hovered = response.hovered();
    ui.painter().rect_filled(
        rect,
        Rounding::same(8.0),
        if hovered {
            theme::mix(palette.danger, Color32::WHITE, 0.10)
        } else {
            palette.danger
        },
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        font,
        Color32::WHITE,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}
