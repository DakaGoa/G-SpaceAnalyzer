use std::path::PathBuf;

use eframe::egui;
use egui::{
    Align, Align2, Color32, CursorIcon, Frame, Id, Layout, Margin, Order, Pos2, Rect, RichText,
    Rounding, ScrollArea, Sense, Stroke, Ui, Vec2,
};
use egui_phosphor::regular as ph;

use crate::app::{Action, App, DupState, SortKey};
use crate::dupe_ui::{duplicate_idle, duplicate_results, duplicate_running};
use crate::table_ui::{category_chip, empty_state, ext_table, paint_stacked_categories};
use crate::model::{Category, NodeId, Tree};
use crate::platform::DriveInfo;
use crate::theme::{self, Palette};
use crate::treemap;
use crate::util::{format_bytes, format_count, format_percent, truncate_middle};

impl App {
    pub(crate) fn view_overview(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let Some(tree) = &self.tree else {
            return;
        };
        let palette = &self.palette;
        let current = tree.node(self.current);
        let margin = 20.0;
        let width = (ui.available_width() - margin * 2.0).max(320.0);
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(margin);
            ui.allocate_ui_with_layout(
                Vec2::new(width, ui.available_height()),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let gap = 10.0;
                    let card_w = ((width - gap * 3.0) / 4.0).max(120.0);
                    let volume = self.volume.as_ref();
                    let total = tree.total_size.max(1);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        stat_card(
                            ui,
                            palette,
                            card_w,
                            "TOTAL SIZE",
                            &format_bytes(current.size),
                            &format!(
                                "{} of {}",
                                format_percent(current.size, total),
                                tree.root_label
                            ),
                            ph::DATABASE,
                            palette.accent,
                            current.size as f32 / total as f32,
                        );
                        stat_card(
                            ui,
                            palette,
                            card_w,
                            "FILES",
                            &format_count(current.files),
                            &format!("{} folders inside", format_count(current.dirs)),
                            ph::FILES,
                            palette.category(Category::Document),
                            0.0,
                        );
                        stat_card(
                            ui,
                            palette,
                            card_w,
                            "FOLDERS",
                            &format_count(current.dirs),
                            if current.dirs == 0 {
                                "No subfolders"
                            } else {
                                "Nested below this folder"
                            },
                            ph::FOLDERS,
                            palette.category(Category::Image),
                            0.0,
                        );
                        let (free, capacity) = volume
                            .map(|volume| (volume.free, volume.total))
                            .unwrap_or((0, total));
                        stat_card(
                            ui,
                            palette,
                            card_w,
                            "FREE SPACE",
                            &format_bytes(free),
                            &format!("of {} capacity", format_bytes(capacity)),
                            ph::HARD_DRIVE,
                            palette.success,
                            if capacity == 0 { 0.0 } else { free as f32 / capacity as f32 },
                        );
                    });
                    ui.add_space(12.0);
                    let main_h = ui.available_height().max(260.0);
                    let show_rail = width > 820.0 && main_h > 330.0;
                    let rail_w = 286.0;
                    let map_w = if show_rail {
                        width - rail_w - 12.0
                    } else {
                        width
                    };
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        treemap_card(
                            ui,
                            self,
                            tree,
                            actions,
                            map_w,
                            main_h,
                        );
                        if show_rail {
                            overview_rail(ui, self, tree, actions, rail_w, main_h);
                        }
                    });
                },
            );
        });
    }

    pub(crate) fn view_welcome(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let palette = &self.palette;
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let margin = 34.0;
                let width = (ui.available_width() - margin * 2.0).max(360.0);
                ui.add_space(36.0);
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.allocate_ui_with_layout(
                        Vec2::new(width, 350.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            let text_w = (width * 0.53).max(390.0);
                            ui.allocate_ui_with_layout(
                                Vec2::new(text_w, 330.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    theme::pill(
                                        ui,
                                        palette,
                                        "Disk space intelligence",
                                        palette.accent,
                                    );
                                    ui.add_space(16.0);
                                    ui.label(
                                        RichText::new("Find what's eating")
                                            .font(theme::font_bold(40.0))
                                            .color(palette.text),
                                    );
                                    ui.label(
                                        RichText::new("your disk.")
                                            .font(theme::font_bold(40.0))
                                            .color(palette.accent),
                                    );
                                    ui.add_space(14.0);
                                    ui.label(
                                        RichText::new(
                                            "A fast, private Windows analyzer with a visual map of every folder, file and byte. No license key, no telemetry, no cloud.",
                                        )
                                        .font(theme::font_regular(14.0))
                                        .color(palette.muted),
                                    );
                                    ui.add_space(24.0);
                                    ui.horizontal(|ui| {
                                        let scan = theme::primary_button(
                                            ui,
                                            palette,
                                            "Scan this PC",
                                            Some(ph::HARD_DRIVES),
                                            Some(178.0),
                                            42.0,
                                        );
                                        if scan.clicked() {
                                            let root = crate::platform::system_drive();
                                            let label = crate::platform::label_for_path(&root);
                                            actions.push(Action::ScanPath(root, label));
                                        }
                                        let choose = theme::secondary_button(
                                            ui,
                                            palette,
                                            "Choose a folder",
                                            Some(ph::FOLDER_OPEN),
                                            Some(178.0),
                                            42.0,
                                        );
                                        if choose.clicked() {
                                            actions.push(Action::ChooseFolder);
                                        }
                                    });
                                    ui.add_space(12.0);
                                    let sample = theme::ghost_button(
                                        ui,
                                        palette,
                                        "or explore a sample volume",
                                        Some(ph::LIGHTNING),
                                        26.0,
                                    );
                                    if sample.clicked() {
                                        actions.push(Action::LoadDemo);
                                    }
                                },
                            );
                            ui.add_space(20.0);
                            ui.allocate_ui_with_layout(
                                Vec2::new(ui.available_width(), 330.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    hero_illustration(ui, palette);
                                },
                            );
                        },
                    );
                });

                ui.add_space(22.0);
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.allocate_ui_with_layout(
                        Vec2::new(width, 20.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            theme::section_label(ui, palette, "Your drives");
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new("Click a drive to start a scan")
                                        .font(theme::font_regular(10.0))
                                        .color(palette.faint),
                                );
                            });
                        },
                    );
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    let drive_w = if self.disks.len() >= 3 {
                        (width - 20.0) / 3.0
                    } else {
                        (width - 10.0) / 2.0
                    };
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for drive in self.disks.iter().take(3) {
                        let root = PathBuf::from(&drive.root);
                        let label = drive.display_name();
                        if welcome_drive_card(ui, palette, drive, drive_w).clicked() {
                            actions.push(Action::ScanPath(root, label));
                        }
                    }
                    if self.disks.is_empty() {
                        Frame::none()
                            .fill(palette.surface)
                            .stroke(Stroke::new(1.0, palette.border))
                            .rounding(Rounding::same(12.0))
                            .inner_margin(Margin::same(20.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("No drives were detected. Choose a folder instead.")
                                        .font(theme::font_regular(12.0))
                                        .color(palette.muted),
                                );
                            });
                    }
                });

                ui.add_space(22.0);
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.allocate_ui_with_layout(
                        Vec2::new(width, 100.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            let feature_w = (width - 20.0) / 3.0;
                            feature_tile(
                                ui,
                                palette,
                                ph::LIGHTNING,
                                "Parallel scanning",
                                "Walk every folder at once and turn a 1 TB drive into a map in seconds.",
                                feature_w,
                            );
                            feature_tile(
                                ui,
                                palette,
                                ph::CUBE,
                                "Visual treemap",
                                "See relative size at a glance. Click any tile to inspect it.",
                                feature_w,
                            );
                            feature_tile(
                                ui,
                                palette,
                                ph::COPY,
                                "Duplicate finder",
                                "Hash matching files to reclaim space without deleting originals.",
                                feature_w,
                            );
                        },
                    );
                });
                ui.add_space(34.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(
                            "Private by design · Everything stays on this PC · Free and open source",
                        )
                        .font(theme::font_regular(10.0))
                        .color(palette.faint),
                    );
                });
                ui.add_space(28.0);
            });
    }

    pub(crate) fn view_scanning(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let Some(job) = &self.scan else {
            return;
        };
        let palette = &self.palette;
        let progress = job.progress.snapshot();
        let time = ui.input(|input| input.time);
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.14).clamp(30.0, 110.0));
            let width = 590.0f32.min(ui.available_width() - 40.0);
            Frame::none()
                .fill(palette.surface)
                .stroke(Stroke::new(1.0, palette.border))
                .rounding(Rounding::same(16.0))
                .inner_margin(Margin::same(28.0))
                .show(ui, |ui| {
                    ui.set_width(width - 56.0);
                    ui.vertical_centered(|ui| {
                        let (logo_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
                        theme::paint_logo(ui.painter(), logo_rect, palette);
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Mapping your drive")
                                .font(theme::font_semibold(21.0))
                                .color(palette.text),
                        );
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(&job.label)
                                .font(theme::font_medium(12.0))
                                .color(palette.accent),
                        );
                        ui.add_space(18.0);
                        scanning_blocks(ui, palette, time, width - 70.0);
                        ui.add_space(18.0);
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().size(14.0).color(palette.accent));
                            ui.label(
                                RichText::new(format!(
                                    "{} files · {} folders",
                                    format_count(progress.files),
                                    format_count(progress.dirs)
                                ))
                                .font(theme::font_medium(11.5))
                                .color(palette.text),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format_bytes(progress.bytes))
                                        .font(theme::font_semibold(12.0))
                                        .color(palette.text),
                                );
                            });
                        });
                        ui.add_space(8.0);
                        let bar_rect = Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(ui.available_width(), 5.0),
                        );
                        indeterminate_bar(ui.painter(), bar_rect, time, palette);
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(truncate_middle(&progress.current, 72))
                                .font(theme::font_regular(9.5))
                                .color(palette.faint),
                        );
                        ui.add_space(18.0);
                        let cancel = theme::secondary_button(
                            ui,
                            palette,
                            if self.scan_cancel_requested {
                                "Cancelling…"
                            } else {
                                "Cancel scan"
                            },
                            Some(ph::X),
                            Some(132.0),
                            32.0,
                        );
                        if cancel.clicked() {
                            actions.push(Action::CancelScan);
                        }
                    });
                });
        });
    }
}

fn stat_card(
    ui: &mut Ui,
    palette: &Palette,
    width: f32,
    label: &str,
    value: &str,
    subtitle: &str,
    icon: &str,
    accent: Color32,
    fraction: f32,
) {
    let height = 88.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(11.0), palette.surface);
    ui.painter()
        .rect_stroke(rect, Rounding::same(11.0), Stroke::new(1.0, palette.border));
    let icon_rect = Rect::from_min_size(
        Pos2::new(rect.right() - 39.0, rect.top() + 13.0),
        Vec2::splat(26.0),
    );
    ui.painter()
        .rect_filled(icon_rect, Rounding::same(7.0), theme::with_alpha(accent, 26));
    ui.painter().text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        icon,
        theme::font_icon(14.0),
        accent,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.top() + 16.0),
        Align2::LEFT_CENTER,
        label,
        theme::font_semibold(8.8),
        palette.faint,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.top() + 38.0),
        Align2::LEFT_CENTER,
        value,
        theme::font_semibold(21.0),
        palette.text,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.top() + 62.0),
        Align2::LEFT_CENTER,
        truncate_middle(subtitle, 34),
        theme::font_regular(9.5),
        palette.faint,
    );
    if fraction > 0.0 {
        let bar = Rect::from_min_size(
            Pos2::new(rect.left() + 14.0, rect.bottom() - 9.0),
            Vec2::new(rect.width() - 28.0, 3.0),
        );
        theme::progress_bar(
            ui.painter(),
            bar,
            fraction,
            accent,
            palette.surface_3,
        );
    }
}

fn treemap_card(
    ui: &mut Ui,
    app: &App,
    tree: &Tree,
    actions: &mut Vec<Action>,
    width: f32,
    height: f32,
) {
    let palette = &app.palette;
    let (card, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    ui.painter()
        .rect_filled(card, Rounding::same(12.0), palette.surface);
    ui.painter()
        .rect_stroke(card, Rounding::same(12.0), Stroke::new(1.0, palette.border));
    let header_h = 51.0;
    ui.painter().text(
        Pos2::new(card.left() + 16.0, card.top() + 20.0),
        Align2::LEFT_CENTER,
        "Space map",
        theme::font_semibold(14.0),
        palette.text,
    );
    ui.painter().text(
        Pos2::new(card.left() + 16.0, card.top() + 36.0),
        Align2::LEFT_CENTER,
        "Every rectangle is a file or folder, sized by storage",
        theme::font_regular(9.8),
        palette.faint,
    );
    // Map / list switch, right-aligned beside the colour chip.
    let list_mode = app.settings.overview_list;
    let mode_w = 46.0;
    let color_w = if list_mode { 0.0 } else { 92.0 };
    let group_left = card.right() - 16.0 - color_w - (mode_w * 2.0 + 4.0);
    let map_chip = Rect::from_min_size(
        Pos2::new(group_left, card.top() + 16.0),
        Vec2::new(mode_w, 22.0),
    );
    let list_chip = Rect::from_min_size(
        Pos2::new(group_left + mode_w + 4.0, card.top() + 16.0),
        Vec2::new(mode_w, 22.0),
    );
    if mode_chip(ui, palette, map_chip, "Map", !list_mode, "overview_mode_map") {
        actions.push(Action::SetOverviewList(false));
    }
    if mode_chip(ui, palette, list_chip, "List", list_mode, "overview_mode_list") {
        actions.push(Action::SetOverviewList(true));
    }
    let mut legend_x = group_left - 14.0;
    if !list_mode {
        let chip_w = 84.0;
        let chip_rect = Rect::from_min_size(
            Pos2::new(card.right() - 16.0 - chip_w, card.top() + 16.0),
            Vec2::new(chip_w, 22.0),
        );
        let chip_response = ui.interact(chip_rect, Id::new("treemap_color_mode"), Sense::click());
        ui.painter()
            .rect_filled(chip_rect, Rounding::same(6.0), palette.surface_2);
        ui.painter()
            .rect_stroke(chip_rect, Rounding::same(6.0), Stroke::new(1.0, palette.border));
        ui.painter().text(
            chip_rect.center(),
            Align2::CENTER_CENTER,
            format!("Color: {}", app.settings.treemap_color.label()),
            theme::font_regular(9.0),
            if chip_response.hovered() {
                palette.text
            } else {
                palette.muted
            },
        );
        if chip_response.clicked() {
            actions.push(Action::CycleTreemapColor);
        }
    }
    let categories = Category::ALL
        .iter()
        .copied()
        .filter(|category| tree.cat_sizes[category.index()] > 0)
        .take(3)
        .collect::<Vec<_>>();
    for category in categories.iter().rev() {
        let label = category.label();
        let width = ui
            .painter()
            .layout_no_wrap(
                label.to_string(),
                theme::font_regular(9.5),
                palette.muted,
            )
            .size()
            .x;
        legend_x -= width;
        ui.painter().text(
            Pos2::new(legend_x, card.top() + 27.0),
            Align2::LEFT_CENTER,
            label,
            theme::font_regular(9.5),
            palette.muted,
        );
        legend_x -= 12.0;
        ui.painter().circle_filled(
            Pos2::new(legend_x, card.top() + 27.0),
            3.0,
            palette.category(*category),
        );
        legend_x -= 16.0;
    }

    let map_rect = Rect::from_min_max(
        Pos2::new(card.left() + 12.0, card.top() + header_h),
        Pos2::new(card.right() - 12.0, card.bottom() - 32.0),
    );
    if list_mode {
        child_list(ui, app, tree, map_rect, actions);
        return;
    }
    ui.painter()
        .rect_filled(map_rect, Rounding::same(8.0), palette.treemap_bg);
    let current = tree.node(app.current);
    if current.children.is_empty() {
        ui.painter().text(
            map_rect.center(),
            Align2::CENTER_CENTER,
            "This folder is empty",
            theme::font_medium(12.0),
            palette.faint,
        );
    } else {
        // A short scale-in makes folder navigation feel continuous.
        let zoom = app
            .treemap_zoom
            .map(|start| {
                let t = ((ui.input(|input| input.time) - start) / 0.22).clamp(0.0, 1.0) as f32;
                1.06 - 0.06 * t
            })
            .unwrap_or(1.0);
        let layout_rect = Rect::from_center_size(map_rect.center(), map_rect.size() * zoom);
        let layout = treemap::build(tree, app.current, layout_rect.shrink(4.0), 6000);
        let color_mode = app.settings.treemap_color;
        let highlight: Option<std::collections::HashSet<NodeId>> =
            if !app.search.trim().is_empty() && !app.search_results.is_empty() {
                Some(app.search_results.iter().copied().collect())
            } else {
                None
            };
        let pointer = ui.input(|input| input.pointer.hover_pos());
        let response = ui.interact(map_rect, Id::new("treemap_canvas"), Sense::click());
        let hovered_index = if response.hovered() {
            pointer.and_then(|position| layout.hit_test(position))
        } else {
            None
        };
        let painter = ui.painter().with_clip_rect(map_rect);
        for (index, tile) in layout.tiles.iter().enumerate() {
            let hovered = hovered_index == Some(index);
            if tile.group_count > 0 {
                let fill = theme::mix(palette.surface_2, palette.accent, 0.18);
                painter.rect_filled(tile.rect, Rounding::same(3.0), fill);
                painter.rect_stroke(
                    tile.rect,
                    Rounding::same(3.0),
                    Stroke::new(1.0, theme::with_alpha(palette.accent, 80)),
                );
            } else if tile.is_dir {
                let mut color = treemap_tint(palette, tree, tile, color_mode);
                if let Some(matches) = &highlight {
                    if !matches.contains(&tile.node) {
                        color = theme::mix(color, palette.treemap_bg, 0.55);
                    }
                }
                let fill = if tile.expanded {
                    theme::mix(palette.treemap_container, color, 0.06)
                } else {
                    theme::mix(palette.treemap_container, color, 0.28)
                };
                painter.rect_filled(tile.rect, Rounding::same(3.0), fill);
                painter.rect_stroke(
                    tile.rect,
                    Rounding::same(3.0),
                    Stroke::new(
                        1.0,
                        theme::with_alpha(color, if tile.expanded { 55 } else { 120 }),
                    ),
                );
            } else {
                let mut fill = treemap_tint(palette, tree, tile, color_mode);
                if tile.depth > 2 {
                    fill = theme::mix(fill, Color32::BLACK, 0.10);
                }
                if let Some(matches) = &highlight {
                    if !matches.contains(&tile.node) {
                        fill = theme::mix(fill, palette.treemap_bg, 0.6);
                    }
                }
                if hovered {
                    fill = theme::mix(fill, Color32::WHITE, 0.16);
                }
                painter.rect_filled(tile.rect, Rounding::same(2.5), fill);
            }
            if hovered {
                painter.rect_stroke(
                    tile.rect.shrink(0.5),
                    Rounding::same(3.0),
                    Stroke::new(1.5, theme::with_alpha(Color32::WHITE, 210)),
                );
            }
        }
        if app.settings.treemap_labels {
            paint_treemap_labels(&painter, ui, palette, tree, &layout, hovered_index);
        }
        if let Some(index) = hovered_index {
            let tile = &layout.tiles[index];
            if let Some(position) = pointer {
                treemap_tooltip(ui, palette, tree, tile, current.size, position);
            }
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if response.double_clicked() {
            if let Some(index) = hovered_index {
                let tile = &layout.tiles[index];
                if tile.is_dir {
                    actions.push(Action::Navigate(tile.node));
                } else {
                    actions.push(Action::Reveal(tile.node));
                }
            }
        } else if response.clicked() {
            if let Some(index) = hovered_index {
                actions.push(Action::Select(layout.tiles[index].node));
            }
        }
        let _ = response.context_menu(|ui| {
            if let Some(index) = hovered_index {
                let node = layout.tiles[index].node;
                if ui.button("Inspect").clicked() {
                    actions.push(Action::Select(node));
                    ui.close_menu();
                }
                if tree.node(node).is_dir && ui.button("Open folder").clicked() {
                    actions.push(Action::Navigate(node));
                    ui.close_menu();
                }
                if ui.button("Show in Explorer").clicked() {
                    actions.push(Action::Reveal(node));
                    ui.close_menu();
                }
                if ui.button("Delete").clicked() {
                    actions.push(Action::Delete(node));
                    ui.close_menu();
                }
            }
        });
        let footer = if layout.truncated {
            "Small items are grouped to keep the map fast"
        } else {
            "Click to inspect · Double-click to open · Right-click for actions"
        };
        ui.painter().text(
            Pos2::new(card.left() + 16.0, card.bottom() - 16.0),
            Align2::LEFT_CENTER,
            footer,
            theme::font_regular(9.0),
            palette.faint,
        );
        if !layout.tiles.is_empty() {
            let count = layout.tiles.iter().filter(|tile| !tile.is_dir).count();
            ui.painter().text(
                Pos2::new(card.right() - 16.0, card.bottom() - 16.0),
                Align2::RIGHT_CENTER,
                format!("{} visible items", format_count(count as u64)),
                theme::font_regular(9.0),
                palette.faint,
            );
        }
    }
}

/// Right-click actions shared by every file and folder row.
pub(crate) fn file_menu(
    response: &egui::Response,
    tree: &Tree,
    node: NodeId,
    actions: &mut Vec<Action>,
) {
    response.context_menu(|ui| {
        if tree.node(node).is_dir {
            if ui.button("Open folder").clicked() {
                actions.push(Action::Navigate(node));
                ui.close_menu();
            }
        } else {
            if ui.button("Open file").clicked() {
                actions.push(Action::OpenItem(node));
                ui.close_menu();
            }
        }
        if ui.button("Open file location").clicked() {
            actions.push(Action::Reveal(node));
            ui.close_menu();
        }
        if ui.button("Copy path").clicked() {
            actions.push(Action::CopyPath(node));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Delete…").clicked() {
            actions.push(Action::Delete(node));
            ui.close_menu();
        }
    });
}

/// A small on/off chip used for the Map / List switch.
fn mode_chip(
    ui: &Ui,
    palette: &Palette,
    rect: Rect,
    label: &str,
    active: bool,
    id: &str,
) -> bool {
    let response = ui.interact(rect, Id::new(id), Sense::click());
    let fill = if active {
        palette.accent_soft
    } else if response.hovered() {
        palette.surface_3
    } else {
        palette.surface_2
    };
    ui.painter().rect_filled(rect, Rounding::same(6.0), fill);
    ui.painter().rect_stroke(
        rect,
        Rounding::same(6.0),
        Stroke::new(
            1.0,
            if active {
                theme::with_alpha(palette.accent, 150)
            } else {
                palette.border
            },
        ),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        theme::font_regular(9.0),
        if active { palette.text } else { palette.muted },
    );
    response.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// The list form of the space map: every child of the current folder in size order.
fn child_list(
    ui: &mut Ui,
    app: &App,
    tree: &Tree,
    area: Rect,
    actions: &mut Vec<Action>,
) {
    let palette = &app.palette;
    let node = tree.node(app.current);
    let mut ids = node.children.clone();
    ids.sort_unstable_by(|a, b| tree.node(*b).size.cmp(&tree.node(*a).size));
    let total = node.size.max(1);
    let mut body = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(area.shrink(2.0))
            .layout(Layout::top_down(Align::Min)),
    );
    if ids.is_empty() {
        body.centered_and_justified(|ui| {
            ui.label(
                RichText::new("This folder has no children to list")
                    .font(theme::font_medium(11.5))
                    .color(palette.faint),
            );
        });
        return;
    }
    let row_h = 34.0;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(&mut body, row_h, ids.len(), |ui, range| {
            ui.set_max_width((ui.available_width() - 10.0).max(120.0));
            for index in range {
                let id = ids[index];
                let child = tree.node(id);
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), row_h - 4.0),
                    Sense::click(),
                );
                if response.hovered() {
                    ui.painter()
                        .rect_filled(rect, Rounding::same(7.0), palette.surface_2);
                }
                let tint = palette.category(child.category);
                ui.painter().text(
                    Pos2::new(rect.left() + 10.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    if child.is_dir { ph::FOLDER } else { ph::FILE },
                    theme::font_icon(12.0),
                    tint,
                );
                let pct_w = 46.0;
                let size_w = 74.0;
                let bar_w = (rect.width() - 40.0 - pct_w - size_w).max(60.0);
                ui.painter().text(
                    Pos2::new(rect.left() + 30.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    truncate_middle(&child.name, 52),
                    theme::font_medium(11.0),
                    palette.text,
                );
                let fraction = child.size as f32 / total as f32;
                let bar = Rect::from_min_size(
                    Pos2::new(rect.left() + 30.0, rect.bottom() - 3.0),
                    Vec2::new(bar_w, 2.5),
                );
                theme::progress_bar(ui.painter(), bar, fraction, tint, palette.surface_3);
                ui.painter().text(
                    Pos2::new(rect.right() - size_w - 12.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    format!("{:.1}%", fraction * 100.0),
                    theme::font_regular(9.5),
                    palette.muted,
                );
                ui.painter().text(
                    Pos2::new(rect.right() - 12.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    format_bytes(child.size),
                    theme::font_semibold(11.0),
                    palette.text,
                );
                if response.clicked() {
                    actions.push(Action::Navigate(id));
                }
                file_menu(&response, tree, id, actions);
            }
        });
}

fn treemap_tint(
    palette: &Palette,
    tree: &Tree,
    tile: &treemap::Tile,
    mode: crate::settings::TreemapColor,
) -> Color32 {
    let node = tree.node(tile.node);
    use crate::settings::TreemapColor;
    match mode {
        TreemapColor::Category => palette.category(node.category),
        TreemapColor::Age => {
            let now = chrono::Local::now().timestamp();
            let days = if node.mtime > 0 {
                ((now - node.mtime).max(0) / 86_400) as u32
            } else {
                u32::MAX
            };
            if days < 30 {
                palette.success
            } else if days < 180 {
                palette.accent
            } else if days < 730 {
                palette.warning
            } else {
                palette.danger
            }
        }
        TreemapColor::Folder => {
            // Stable pseudo-colour per top-level branch.
            let mut hash = 0u32;
            for byte in node.name.as_bytes() {
                hash = hash.wrapping_mul(31).wrapping_add(u32::from(*byte));
            }
            let index = (hash as usize) % Category::ALL.len();
            palette.category(Category::ALL[index])
        }
    }
}

fn paint_treemap_labels(
    painter: &egui::Painter,
    _ui: &Ui,
    palette: &Palette,
    tree: &Tree,
    layout: &treemap::TreemapLayout,
    hovered: Option<usize>,
) {
    for (index, tile) in layout.tiles.iter().enumerate() {
        if tile.depth > 1 {
            continue;
        }
        if tile.rect.width() < 58.0 || tile.rect.height() < 22.0 {
            continue;
        }
        if Some(index) == hovered && !tile.is_dir {
            continue;
        }
        let node = tree.node(tile.node);
        let (label, sub) = if tile.group_count > 0 {
            (format!("{} smaller items", format_count(tile.group_count as u64)), None)
        } else if tile.is_dir {
            (node.name.clone(), None)
        } else {
            (node.name.clone(), Some(format_bytes(node.size)))
        };
        let font = if tile.is_dir {
            theme::font_semibold(10.0)
        } else {
            theme::font_medium(9.5)
        };
        let color = if tile.is_dir {
            palette.text
        } else {
            palette.category_text(node.category)
        };
        let max_chars = ((tile.rect.width() - 12.0) / 5.7).max(4.0) as usize;
        let text = truncate_middle(&label, max_chars);
        let text_pos = Pos2::new(tile.rect.left() + 5.0, tile.rect.top() + 10.0);
        if tile.is_dir {
            let galley = painter.layout_no_wrap(text.clone(), font.clone(), color);
            let pill = Rect::from_min_size(
                Pos2::new(tile.rect.left() + 3.0, tile.rect.top() + 3.0),
                Vec2::new((galley.size().x + 8.0).min(tile.rect.width() - 6.0), 16.0),
            );
            painter.rect_filled(
                pill,
                Rounding::same(4.0),
                theme::with_alpha(palette.treemap_bg, 205),
            );
            painter.with_clip_rect(pill).text(
                Pos2::new(pill.left() + 4.0, pill.center().y),
                Align2::LEFT_CENTER,
                text,
                font,
                color,
            );
        } else {
            painter.with_clip_rect(tile.rect.shrink(3.0)).text(
                text_pos,
                Align2::LEFT_CENTER,
                text,
                font,
                color,
            );
            if let Some(sub) = sub {
                if tile.rect.height() > 34.0 {
                    painter.with_clip_rect(tile.rect.shrink(3.0)).text(
                        Pos2::new(tile.rect.left() + 5.0, tile.rect.top() + 23.0),
                        Align2::LEFT_CENTER,
                        truncate_middle(&sub, max_chars),
                        theme::font_regular(8.5),
                        theme::with_alpha(color, 210),
                    );
                }
            }
        }
    }
}

fn treemap_tooltip(
    ui: &Ui,
    palette: &Palette,
    tree: &Tree,
    tile: &treemap::Tile,
    current_size: u64,
    pointer: Pos2,
) {
    let node = tree.node(tile.node);
    let name = if tile.group_count > 0 {
        format!("{} smaller items", format_count(tile.group_count as u64))
    } else {
        node.name.clone()
    };
    let path = tree.path(tile.node);
    let size = tile.size;
    let percent = if current_size == 0 {
        "0%".to_string()
    } else {
        format_percent(size, current_size)
    };
    let screen = ui.ctx().screen_rect();
    let pos = Pos2::new(
        (pointer.x + 16.0).min(screen.right() - 320.0),
        (pointer.y + 16.0).min(screen.bottom() - 110.0),
    );
    egui::Area::new(Id::new("treemap_tooltip"))
        .order(Order::Tooltip)
        .fixed_pos(pos)
        .interactable(false)
        .show(ui.ctx(), |ui| {
            Frame::none()
                .fill(palette.surface_3)
                .stroke(Stroke::new(1.0, palette.border_strong))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::symmetric(10.0, 8.0))
                .show(ui, |ui| {
                    ui.set_max_width(300.0);
                    ui.label(
                        RichText::new(truncate_middle(&name, 46))
                            .font(theme::font_semibold(11.5))
                            .color(palette.text),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} · {} of this folder",
                            format_bytes(size),
                            percent
                        ))
                        .font(theme::font_medium(10.0))
                        .color(palette.accent),
                    );
                    ui.label(
                        RichText::new(truncate_middle(&path, 62))
                            .font(theme::font_regular(9.0))
                            .color(palette.faint),
                    );
                });
        });
}

fn overview_rail(
    ui: &mut Ui,
    app: &App,
    tree: &Tree,
    actions: &mut Vec<Action>,
    width: f32,
    height: f32,
) {
    let palette = &app.palette;
    let top_h = (height * 0.54).max(150.0);
    let file_h = (height - top_h - 12.0).max(130.0);
    ui.allocate_ui_with_layout(
        Vec2::new(width, height),
        Layout::top_down(Align::Min),
        |ui| {
            rail_card(ui, palette, width, top_h, "Top folders", |ui, inner_h| {
                let node = tree.node(app.current);
                let folders = node
                    .children
                    .iter()
                    .filter(|id| tree.node(**id).is_dir)
                    .copied()
                    .take(((inner_h - 30.0) / 43.0).max(2.0) as usize)
                    .collect::<Vec<_>>();
                if folders.is_empty() {
                    empty_rail_text(ui, palette, "No subfolders here");
                    return;
                }
                for id in folders {
                    let response = overview_folder_row(ui, palette, tree, id);
                    if response.double_clicked() {
                        actions.push(Action::Navigate(id));
                    } else if response.clicked() {
                        actions.push(Action::Select(id));
                    }
                    ui.add_space(3.0);
                }
            });
            ui.add_space(12.0);
            rail_card(ui, palette, width, file_h, "Largest files", |ui, inner_h| {
                let files = tree.largest_files(((inner_h - 26.0) / 42.0).max(2.0) as usize);
                if files.is_empty() {
                    empty_rail_text(ui, palette, "No files scanned");
                    return;
                }
                for id in files {
                    let response = overview_file_row(ui, palette, tree, id);
                    if response.double_clicked() {
                        actions.push(Action::Reveal(id));
                    } else if response.clicked() {
                        actions.push(Action::Select(id));
                    }
                    ui.add_space(2.0);
                }
            });
        },
    );
}

fn rail_card(
    ui: &mut Ui,
    palette: &Palette,
    width: f32,
    height: f32,
    title: &str,
    contents: impl FnOnce(&mut Ui, f32),
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(12.0), palette.surface);
    ui.painter()
        .rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, palette.border));
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.top() + 21.0),
        Align2::LEFT_CENTER,
        title,
        theme::font_semibold(12.0),
        palette.text,
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(
                Pos2::new(rect.left() + 14.0, rect.top() + 38.0),
                Pos2::new(rect.right() - 14.0, rect.bottom() - 12.0),
            ))
            .layout(Layout::top_down(Align::Min)),
    );
    let inner_h = child.available_height();
    contents(&mut child, inner_h);
}

fn overview_folder_row(ui: &mut Ui, palette: &Palette, tree: &Tree, id: NodeId) -> egui::Response {
    let node = tree.node(id);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 40.0),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, Rounding::same(7.0), palette.surface_2);
    }
    let icon_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 5.0, rect.center().y - 10.0),
        Vec2::splat(20.0),
    );
    ui.painter().rect_filled(
        icon_rect,
        Rounding::same(5.0),
        palette.category_soft(node.category),
    );
    ui.painter().text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        ph::FOLDER,
        theme::font_icon(11.0),
        palette.category(node.category),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 32.0, rect.center().y - 6.0),
        Align2::LEFT_CENTER,
        truncate_middle(&node.name, 24),
        theme::font_medium(10.5),
        palette.text,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 32.0, rect.center().y + 8.0),
        Align2::LEFT_CENTER,
        format!("{} · {}", format_bytes(node.size), format_count(node.files)),
        theme::font_regular(8.8),
        palette.faint,
    );
    let bar = Rect::from_min_size(
        Pos2::new(rect.left() + 32.0, rect.bottom() - 3.0),
        Vec2::new(rect.width() - 36.0, 2.0),
    );
    let total = tree.total_size.max(1);
    theme::progress_bar(
        ui.painter(),
        bar,
        node.size as f32 / total as f32,
        palette.category(node.category),
        palette.surface_3,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn overview_file_row(ui: &mut Ui, palette: &Palette, tree: &Tree, id: NodeId) -> egui::Response {
    let node = tree.node(id);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 37.0),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, Rounding::same(7.0), palette.surface_2);
    }
    let ext_badge = Rect::from_min_size(
        Pos2::new(rect.left() + 5.0, rect.center().y - 9.0),
        Vec2::splat(18.0),
    );
    ui.painter().rect_filled(
        ext_badge,
        Rounding::same(5.0),
        palette.category_soft(node.category),
    );
    ui.painter().text(
        ext_badge.center(),
        Align2::CENTER_CENTER,
        node.category.icon(),
        theme::font_icon(10.0),
        palette.category(node.category),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 31.0, rect.center().y - 5.0),
        Align2::LEFT_CENTER,
        truncate_middle(&node.name, 24),
        theme::font_medium(10.2),
        palette.text,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 31.0, rect.center().y + 8.5),
        Align2::LEFT_CENTER,
        format!(
            "{} · {}",
            format_bytes(node.size),
            crate::model::extension_label(&node.name)
        ),
        theme::font_regular(8.6),
        palette.faint,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn empty_rail_text(ui: &mut Ui, palette: &Palette, text: &str) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(text)
            .font(theme::font_regular(10.0))
            .color(palette.faint),
    );
}

fn hero_illustration(ui: &mut Ui, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().min(455.0), 310.0),
        Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, Rounding::same(16.0), palette.surface);
    ui.painter()
        .rect_stroke(rect, Rounding::same(16.0), Stroke::new(1.0, palette.border));
    theme::paint_vertical_gradient(
        ui.painter(),
        rect.shrink(1.0),
        theme::with_alpha(palette.accent, 10),
        Color32::TRANSPARENT,
    );
    let inner = rect.shrink(22.0);
    let gap = 5.0;
    let left_w = inner.width() * 0.54;
    let right_w = inner.width() - left_w - gap;
    let h = inner.height();
    let blocks = [
        (Rect::from_min_size(inner.min, Vec2::new(left_w, h * 0.63)), Category::Video, "Video", "246 GB"),
        (
            Rect::from_min_size(
                Pos2::new(inner.left() + left_w + gap, inner.top()),
                Vec2::new(right_w, h * 0.31),
            ),
            Category::Image,
            "Photos",
            "82 GB",
        ),
        (
            Rect::from_min_size(
                Pos2::new(inner.left() + left_w + gap, inner.top() + h * 0.31 + gap),
                Vec2::new(right_w, h * 0.32 - gap * 0.5),
            ),
            Category::Archive,
            "Games",
            "358 GB",
        ),
        (
            Rect::from_min_size(
                Pos2::new(inner.left() + left_w + gap, inner.top() + h * 0.63 + gap),
                Vec2::new(right_w, h * 0.37 - gap),
            ),
            Category::Document,
            "Backups",
            "189 GB",
        ),
    ];
    for (block, category, label, size) in blocks {
        let color = palette.category(category);
        ui.painter()
            .rect_filled(block, Rounding::same(7.0), theme::mix(color, Color32::BLACK, 0.08));
        ui.painter().text(
            Pos2::new(block.left() + 9.0, block.top() + 16.0),
            Align2::LEFT_CENTER,
            label,
            theme::font_semibold(10.0),
            palette.category_text(category),
        );
        ui.painter().text(
            Pos2::new(block.left() + 9.0, block.top() + 30.0),
            Align2::LEFT_CENTER,
            size,
            theme::font_regular(8.5),
            theme::with_alpha(palette.category_text(category), 220),
        );
    }
    let chip_w = 188.0;
    let chip = Rect::from_min_size(
        Pos2::new(rect.left() + 18.0, rect.bottom() - 52.0),
        Vec2::new(chip_w, 34.0),
    );
    ui.painter()
        .rect_filled(chip, Rounding::same(8.0), palette.surface_3);
    ui.painter()
        .rect_stroke(chip, Rounding::same(8.0), Stroke::new(1.0, palette.border_strong));
    ui.painter().circle_filled(
        Pos2::new(chip.left() + 14.0, chip.center().y),
        3.2,
        palette.success,
    );
    ui.painter().text(
        Pos2::new(chip.left() + 24.0, chip.center().y),
        Align2::LEFT_CENTER,
        "1.24 TB mapped in 8.4s",
        theme::font_medium(9.8),
        palette.text,
    );
}

fn welcome_drive_card(
    ui: &mut Ui,
    palette: &Palette,
    drive: &DriveInfo,
    width: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 100.0), Sense::click());
    ui.painter()
        .rect_filled(rect, Rounding::same(11.0), palette.surface);
    ui.painter()
        .rect_stroke(rect, Rounding::same(11.0), Stroke::new(1.0, palette.border));
    let icon = Rect::from_min_size(
        Pos2::new(rect.left() + 16.0, rect.top() + 18.0),
        Vec2::splat(32.0),
    );
    ui.painter()
        .rect_filled(icon, Rounding::same(8.0), palette.accent_soft);
    ui.painter().text(
        icon.center(),
        Align2::CENTER_CENTER,
        ph::HARD_DRIVE,
        theme::font_icon(16.0),
        palette.accent,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 58.0, rect.top() + 26.0),
        Align2::LEFT_CENTER,
        drive.display_name(),
        theme::font_semibold(11.5),
        palette.text,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 58.0, rect.top() + 43.0),
        Align2::LEFT_CENTER,
        format!(
            "{} free of {} · {}",
            format_bytes(drive.free),
            format_bytes(drive.total),
            drive.filesystem
        ),
        theme::font_regular(9.2),
        palette.faint,
    );
    let bar = Rect::from_min_size(
        Pos2::new(rect.left() + 16.0, rect.bottom() - 21.0),
        Vec2::new(rect.width() - 32.0, 5.0),
    );
    theme::progress_bar(
        ui.painter(),
        bar,
        drive.used_fraction(),
        if drive.used_fraction() > 0.9 {
            palette.danger
        } else {
            palette.accent
        },
        palette.surface_3,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 16.0, rect.bottom() - 32.0),
        Align2::LEFT_CENTER,
        format!("{}% used", (drive.used_fraction() * 100.0).round() as u32),
        theme::font_regular(8.8),
        palette.faint,
    );
    ui.painter().text(
        Pos2::new(rect.right() - 16.0, rect.bottom() - 32.0),
        Align2::RIGHT_CENTER,
        "Click to scan →",
        theme::font_medium(8.8),
        palette.accent,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn feature_tile(
    ui: &mut Ui,
    palette: &Palette,
    icon: &str,
    title: &str,
    description: &str,
    width: f32,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 92.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(10.0), palette.surface);
    ui.painter()
        .rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, palette.border));
    let icon_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 14.0, rect.top() + 16.0),
        Vec2::splat(26.0),
    );
    ui.painter()
        .rect_filled(icon_rect, Rounding::same(7.0), palette.accent_soft);
    ui.painter().text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        icon,
        theme::font_icon(13.0),
        palette.accent,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 50.0, rect.top() + 24.0),
        Align2::LEFT_CENTER,
        title,
        theme::font_semibold(11.0),
        palette.text,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 50.0, rect.top() + 43.0),
        Align2::LEFT_CENTER,
        truncate_middle(description, 64),
        theme::font_regular(9.3),
        palette.faint,
    );
}

fn scanning_blocks(ui: &mut Ui, palette: &Palette, time: f64, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 64.0), Sense::hover());
    let pieces = [
        (0.00, 0.28, Category::Video, 0.0),
        (0.29, 0.18, Category::Image, 0.7),
        (0.48, 0.16, Category::Archive, 1.3),
        (0.65, 0.21, Category::Document, 1.9),
        (0.87, 0.13, Category::Code, 2.5),
    ];
    for (start, size, category, delay) in pieces {
        let pulse = ((time * 1.5 + delay).sin() * 0.5 + 0.5) as f32;
        let block = Rect::from_min_size(
            Pos2::new(rect.left() + width * start, rect.top()),
            Vec2::new(width * size - 3.0, 64.0),
        );
        let color = theme::mix(
            theme::with_alpha(palette.category(category), 110),
            palette.category(category),
            0.25 + pulse * 0.45,
        );
        ui.painter().rect_filled(block, Rounding::same(4.0), color);
    }
}

fn indeterminate_bar(painter: &egui::Painter, rect: Rect, time: f64, palette: &Palette) {
    painter.rect_filled(rect, Rounding::same(rect.height() * 0.5), palette.surface_3);
    let travel = (time * 0.55 % 1.0) as f32;
    let width = rect.width() * 0.28;
    let x = rect.left() + (rect.width() + width) * travel - width;
    let start = x.max(rect.left());
    let end = (x + width).min(rect.right());
    if end > start {
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(start, rect.top()), Pos2::new(end, rect.bottom())),
            Rounding::same(rect.height() * 0.5),
            palette.accent,
        );
    }
}

impl App {
    pub(crate) fn view_largest_files(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let Some(tree) = &self.tree else {
            return;
        };
        let palette = &self.palette;
        let ids = sorted_largest(self, tree);
        let margin = 20.0;
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(margin);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Largest files")
                        .font(theme::font_semibold(22.0))
                        .color(palette.text),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!(
                        "The {} biggest files on {} · {} total",
                        format_count(ids.len() as u64),
                        tree.root_label,
                        format_bytes(tree.total_size)
                    ))
                    .font(theme::font_regular(10.5))
                    .color(palette.muted),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(margin);
                sort_chip(ui, palette, "Modified", SortKey::Modified, self.sort, actions);
                sort_chip(ui, palette, "Name", SortKey::Name, self.sort, actions);
                sort_chip(ui, palette, "Size", SortKey::Size, self.sort, actions);
                ui.label(
                    RichText::new("Sort")
                        .font(theme::font_regular(9.5))
                        .color(palette.faint),
                );
            });
        });
        ui.add_space(10.0);
        let card_w = (ui.available_width() - margin * 2.0).max(300.0);
        let card_h = (ui.available_height() - 10.0).max(180.0);
        ui.horizontal(|ui| {
            ui.add_space(margin);
            let (card, _) = ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::hover());
            paint_card(ui.painter(), palette, card);
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_max(
                        Pos2::new(card.left() + 1.0, card.top() + 34.0),
                        Pos2::new(card.right() - 1.0, card.bottom() - 30.0),
                    ))
                    .layout(Layout::top_down(Align::Min)),
            );
            item_table(&mut content, self, tree, &ids, actions, "No files scanned yet");
            let footer_y = card.bottom() - 16.0;
            ui.painter().text(
                Pos2::new(card.left() + 16.0, footer_y),
                Align2::LEFT_CENTER,
                "Click a row to inspect · Double-click to show in Explorer",
                theme::font_regular(9.0),
                palette.faint,
            );
            ui.painter().text(
                Pos2::new(card.right() - 16.0, footer_y),
                Align2::RIGHT_CENTER,
                format!("{} items", format_count(ids.len() as u64)),
                theme::font_regular(9.0),
                palette.faint,
            );
        });
    }

    pub(crate) fn view_search(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let Some(tree) = &self.tree else {
            return;
        };
        let palette = &self.palette;
        let margin = 20.0;
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(margin);
            ui.vertical(|ui| {
                let title = if self.search.trim().is_empty() {
                    "Search".to_string()
                } else {
                    format!("Search · “{}”", self.search.trim())
                };
                ui.label(
                    RichText::new(title)
                        .font(theme::font_semibold(22.0))
                        .color(palette.text),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(if self.search.trim().is_empty() {
                        "Type at least two characters in the search box above"
                            .to_string()
                    } else {
                        format!(
                            "{} matches in {} · results ranked by size",
                            format_count(self.search_results.len() as u64),
                            tree.root_label
                        )
                    })
                    .font(theme::font_regular(10.5))
                    .color(palette.muted),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(margin);
                sort_chip(ui, palette, "Modified", SortKey::Modified, self.search_sort, actions);
                sort_chip(ui, palette, "Name", SortKey::Name, self.search_sort, actions);
                sort_chip(ui, palette, "Size", SortKey::Size, self.search_sort, actions);
                if !self.search.is_empty() {
                    if theme::secondary_button(
                        ui,
                        palette,
                        "Clear",
                        Some(ph::X),
                        Some(96.0),
                        30.0,
                    )
                    .clicked()
                    {
                        actions.push(Action::ClearSearch);
                    }
                }
            });
        });
        ui.add_space(10.0);
        let card_w = (ui.available_width() - margin * 2.0).max(300.0);
        let card_h = (ui.available_height() - 10.0).max(180.0);
        ui.horizontal(|ui| {
            ui.add_space(margin);
            let (card, _) = ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::hover());
            paint_card(ui.painter(), palette, card);
            if self.search.trim().is_empty() {
                empty_state(
                    ui,
                    palette,
                    card,
                    ph::MAGNIFYING_GLASS,
                    "Search your scan",
                    "Find files and folders by name across the entire scanned volume.",
                );
                return;
            }
            if self.search_results.is_empty() {
                empty_state(
                    ui,
                    palette,
                    card,
                    ph::MAGNIFYING_GLASS,
                    "No matches",
                    "Try a shorter query or a different spelling.",
                );
                return;
            }
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_max(
                        Pos2::new(card.left() + 1.0, card.top() + 1.0),
                        Pos2::new(card.right() - 1.0, card.bottom() - 1.0),
                    ))
                    .layout(Layout::top_down(Align::Min)),
            );
            let mut ids = self.search_results.clone();
            crate::views_extra::sort_ids(tree, &mut ids, self.search_sort);
            item_table(&mut content, self, tree, &ids, actions, "No matches");
        });
    }

    pub(crate) fn view_file_types(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let Some(tree) = &self.tree else {
            return;
        };
        let palette = &self.palette;
        let margin = 20.0;
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(margin);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("File types")
                        .font(theme::font_semibold(22.0))
                        .color(palette.text),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!(
                        "{} file types across {} · click a category to filter",
                        format_count(tree.ext_stats.len() as u64),
                        format_bytes(tree.total_size)
                    ))
                    .font(theme::font_regular(10.5))
                    .color(palette.muted),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(margin);
                sort_chip(ui, palette, "Count", SortKey::Count, self.ext_sort, actions);
                sort_chip(ui, palette, "Name", SortKey::Name, self.ext_sort, actions);
                sort_chip(ui, palette, "Size", SortKey::Size, self.ext_sort, actions);
                if let Some(category) = self.selected_category {
                    if theme::secondary_button(
                        ui,
                        palette,
                        "All types",
                        Some(ph::X),
                        None,
                        30.0,
                    )
                    .clicked()
                    {
                        actions.push(Action::SetCategory(None));
                    }
                    theme::pill(ui, palette, category.label(), palette.category(category));
                }
            });
        });
        ui.add_space(9.0);
        let width = (ui.available_width() - margin * 2.0).max(300.0);

        // Composition card
        let (composition, _) = ui
            .horizontal(|ui| {
                ui.add_space(margin);
                ui.allocate_exact_size(Vec2::new(width, 102.0), Sense::hover())
            })
            .inner;
        paint_card(ui.painter(), palette, composition);
        ui.painter().text(
            Pos2::new(composition.left() + 16.0, composition.top() + 18.0),
            Align2::LEFT_CENTER,
            "Storage by type",
            theme::font_semibold(12.0),
            palette.text,
        );
        let total = tree.total_size.max(1);
        let bar = Rect::from_min_size(
            Pos2::new(composition.left() + 16.0, composition.top() + 34.0),
            Vec2::new(composition.width() - 32.0, 13.0),
        );
        paint_stacked_categories(ui.painter(), palette, tree, bar, total);
        let mut legend_x = composition.left() + 16.0;
        for category in Category::ALL {
            let size = tree.cat_sizes[category.index()];
            if size == 0 {
                continue;
            }
            let label = format!("{} {}", category.label(), format_bytes(size));
            let galley = ui.painter().layout_no_wrap(
                label.clone(),
                theme::font_regular(9.0),
                palette.muted,
            );
            if legend_x + galley.size().x + 22.0 > composition.right() {
                break;
            }
            ui.painter().circle_filled(
                Pos2::new(legend_x + 3.0, composition.bottom() - 22.0),
                3.0,
                palette.category(category),
            );
            ui.painter().text(
                Pos2::new(legend_x + 11.0, composition.bottom() - 22.0),
                Align2::LEFT_CENTER,
                label,
                theme::font_regular(9.0),
                palette.muted,
            );
            legend_x += galley.size().x + 26.0;
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(margin);
            ScrollArea::horizontal()
                .id_salt("category_chips")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 7.0;
                        for category in Category::ALL {
                            let size = tree.cat_sizes[category.index()];
                            if category_chip(
                                ui,
                                palette,
                                category,
                                size,
                                total,
                                self.selected_category == Some(category),
                            )
                            .clicked()
                            {
                                actions.push(Action::SetCategory(if self.selected_category
                                    == Some(category)
                                {
                                    None
                                } else {
                                    Some(category)
                                }));
                            }
                        }
                    });
                });
        });
        ui.add_space(8.0);

        let table_h = (ui.available_height() - 12.0).max(160.0);
        let (table_card, _) = ui.horizontal(|ui| {
            ui.add_space(margin);
            ui.allocate_exact_size(Vec2::new(width, table_h), Sense::hover())
        })
        .inner;
        paint_card(ui.painter(), palette, table_card);
        let mut content = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(table_card.shrink(1.0))
                .layout(Layout::top_down(Align::Min)),
        );
        ext_table(&mut content, self, tree, actions, self.ext_sort);
    }

    pub(crate) fn view_duplicates(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let Some(tree) = &self.tree else {
            return;
        };
        let palette = &self.palette;
        let margin = 20.0;
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(margin);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Duplicate finder")
                        .font(theme::font_semibold(22.0))
                        .color(palette.text),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(
                        "Find identical files by size and content. Nothing is removed without confirmation.",
                    )
                    .font(theme::font_regular(10.5))
                    .color(palette.muted),
                );
                ui.add_space(7.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(
                        RichText::new("Smallest")
                            .font(theme::font_regular(9.5))
                            .color(palette.faint),
                    );
                    for (mb, label) in [(1u64, "1 MB"), (4, "4 MB"), (16, "16 MB"), (64, "64 MB")] {
                        if crate::views_extra::dup_chip(
                            ui,
                            palette,
                            label,
                            self.settings.dup_min_size_mb == mb,
                        ) {
                            actions.push(Action::SetDupMinSize(mb));
                        }
                    }
                    ui.add_space(6.0);
                    let scope_label = if self.settings.dup_current_folder {
                        "This folder"
                    } else {
                        "Whole scan"
                    };
                    if crate::views_extra::dup_chip(
                        ui,
                        palette,
                        scope_label,
                        self.settings.dup_current_folder,
                    ) {
                        actions.push(Action::ToggleDupScope);
                    }
                    if matches!(self.dupes, DupState::Done(_)) {
                        ui.add_space(6.0);
                        if theme::ghost_button(ui, palette, "Keep newest", Some(ph::CLOCK), 24.0)
                            .clicked()
                        {
                            actions.push(Action::DupKeepAll { newest: true });
                        }
                        if theme::ghost_button(ui, palette, "Keep oldest", Some(ph::CLOCK), 24.0)
                            .clicked()
                        {
                            actions.push(Action::DupKeepAll { newest: false });
                        }
                    }
                });
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(margin);
                match &self.dupes {
                    DupState::Running(_) => {
                        if theme::secondary_button(
                            ui,
                            palette,
                            "Cancel",
                            Some(ph::X),
                            Some(100.0),
                            32.0,
                        )
                        .clicked()
                        {
                            actions.push(Action::CancelDuplicates);
                        }
                    }
                    DupState::Done(results) if !results.groups.is_empty() => {
                        if theme::secondary_button(
                            ui,
                            palette,
                            "Scan again",
                            Some(ph::ARROWS_CLOCKWISE),
                            Some(122.0),
                            32.0,
                        )
                        .clicked()
                        {
                            actions.push(Action::StartDuplicates);
                        }
                    }
                    _ => {
                        if theme::primary_button(
                            ui,
                            palette,
                            "Find duplicates",
                            Some(ph::COPY),
                            Some(158.0),
                            32.0,
                        )
                        .clicked()
                        {
                            actions.push(Action::StartDuplicates);
                        }
                    }
                }
            });
        });
        ui.add_space(10.0);
        let width = (ui.available_width() - margin * 2.0).max(320.0);
        let height = (ui.available_height() - 10.0).max(200.0);
        match &self.dupes {
            DupState::Idle => {
                let (card, _) = ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.allocate_exact_size(Vec2::new(width, height), Sense::hover())
                })
                .inner;
                paint_card(ui.painter(), palette, card);
                duplicate_idle(ui, palette, card, actions);
            }
            DupState::Running(job) => {
                let (card, _) = ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.allocate_exact_size(Vec2::new(width, height), Sense::hover())
                })
                .inner;
                paint_card(ui.painter(), palette, card);
                duplicate_running(ui, palette, card, job, actions);
            }
            DupState::Failed(error) => {
                let (card, _) = ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.allocate_exact_size(Vec2::new(width, height), Sense::hover())
                })
                .inner;
                paint_card(ui.painter(), palette, card);
                empty_state(
                    ui,
                    palette,
                    card,
                    ph::WARNING_CIRCLE,
                    "Duplicate scan failed",
                    error,
                );
            }
            DupState::Done(results) => {
                if results.groups.is_empty() {
                    let (card, _) = ui.horizontal(|ui| {
                        ui.add_space(margin);
                        ui.allocate_exact_size(Vec2::new(width, height), Sense::hover())
                    })
                    .inner;
                    paint_card(ui.painter(), palette, card);
                    empty_state(
                        ui,
                        palette,
                        card,
                        ph::CHECK_CIRCLE,
                        "No duplicates found",
                        "Every candidate has unique content. Your drive is tidy.",
                    );
                } else {
                    duplicate_results(ui, self, tree, results, actions, width, height, margin);
                }
            }
        }
    }
}

fn sorted_largest(app: &App, tree: &Tree) -> Vec<NodeId> {
    let mut ids = app.largest_ids.clone();
    let descending = app.sort.descending;
    match app.sort.key {
        SortKey::Size | SortKey::Count => {
            ids.sort_unstable_by(|a, b| tree.node(*a).size.cmp(&tree.node(*b).size));
        }
        SortKey::Modified => {
            ids.sort_unstable_by(|a, b| tree.node(*a).mtime.cmp(&tree.node(*b).mtime));
        }
        SortKey::Name => {
            ids.sort_unstable_by(|a, b| {
                tree.node(*a)
                    .name
                    .to_lowercase()
                    .cmp(&tree.node(*b).name.to_lowercase())
            });
        }
    }
    if descending {
        ids.reverse();
    }
    ids
}

struct Columns {
    left: f32,
    name_x: f32,
    path_x: f32,
    size_x: f32,
    pct_x: f32,
    date_x: f32,
    name_w: f32,
    path_w: f32,
    size_w: f32,
    pct_w: f32,
    date_w: f32,
    right: f32,
    show_date: bool,
    show_pct: bool,
}

fn columns(rect: Rect) -> Columns {
    let width = rect.width();
    let left = rect.left() + 14.0;
    let right = rect.right() - 14.0;
    // Total horizontal spacing between columns; must match the offsets below.
    const GAPS: f32 = 38.0;
    let size_w = 82.0;
    let mut path_w = (width * 0.24).clamp(130.0, 300.0);
    let mut show_date = width > 760.0;
    let mut show_pct = width > 620.0;
    let mut pct_w = if show_pct { 62.0 } else { 0.0 };
    let mut date_w = if show_date { 98.0 } else { 0.0 };
    let mut name_w = right - left - path_w - size_w - pct_w - date_w - GAPS;
    // Narrow windows drop the optional columns rather than spilling past the card.
    if name_w < 120.0 && show_date {
        show_date = false;
        date_w = 0.0;
        name_w = right - left - path_w - size_w - pct_w - GAPS;
    }
    if name_w < 120.0 && show_pct {
        show_pct = false;
        pct_w = 0.0;
        name_w = right - left - path_w - size_w - GAPS;
    }
    if name_w < 120.0 {
        path_w = (path_w * 0.6).max(70.0);
        name_w = (right - left - path_w - size_w - pct_w - date_w - GAPS).max(60.0);
    }
    let name_x = left;
    let path_x = name_x + name_w + 12.0;
    let size_x = path_x + path_w + 10.0;
    let pct_x = size_x + size_w + 8.0;
    let date_x = pct_x + pct_w + 8.0;
    Columns {
        left,
        name_x,
        path_x,
        size_x,
        pct_x,
        date_x,
        name_w,
        path_w,
        size_w,
        pct_w,
        date_w,
        right,
        show_date,
        show_pct,
    }
}

fn item_table(
    ui: &mut Ui,
    app: &App,
    tree: &Tree,
    ids: &[NodeId],
    actions: &mut Vec<Action>,
    empty_text: &str,
) {
    let palette = &app.palette;
    let width = ui.available_width();
    if ids.is_empty() {
        empty_state(
            ui,
            palette,
            Rect::from_min_size(ui.cursor().min, Vec2::new(width, 240.0)),
            ph::FILES,
            "Nothing to show",
            empty_text,
        );
        return;
    }
    let cols = columns(Rect::from_min_size(
        Pos2::new(ui.cursor().left(), ui.cursor().top()),
        Vec2::new(width, 1.0),
    ));
    let (header, _) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::hover());
    let painter = ui.painter();
    let header_font = theme::font_semibold(8.8);
    painter.text(
        Pos2::new(cols.name_x, header.center().y),
        Align2::LEFT_CENTER,
        "NAME",
        header_font.clone(),
        palette.faint,
    );
    painter.text(
        Pos2::new(cols.path_x, header.center().y),
        Align2::LEFT_CENTER,
        "LOCATION",
        header_font.clone(),
        palette.faint,
    );
    painter.text(
        Pos2::new(cols.size_x + cols.size_w, header.center().y),
        Align2::RIGHT_CENTER,
        "SIZE",
        header_font.clone(),
        palette.faint,
    );
    if cols.show_pct {
        painter.text(
            Pos2::new(cols.pct_x + cols.pct_w, header.center().y),
            Align2::RIGHT_CENTER,
            "%",
            header_font.clone(),
            palette.faint,
        );
    }
    if cols.show_date {
        painter.text(
            Pos2::new(cols.date_x, header.center().y),
            Align2::LEFT_CENTER,
            "MODIFIED",
            header_font,
            palette.faint,
        );
    }
    painter.line_segment(
        [
            Pos2::new(cols.left - 8.0, header.bottom()),
            Pos2::new(cols.right + 4.0, header.bottom()),
        ],
        Stroke::new(1.0, palette.border),
    );
    ui.add_space(2.0);
    let row_height = 40.0;
    ui.spacing_mut().item_spacing.y = 0.0;
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, ids.len(), |ui, range| {
            for index in range {
                let id = ids[index];
                let node = tree.node(id);
                let (row, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), row_height),
                    Sense::click(),
                );
                let selected = app.selection == Some(id);
                if selected {
                    ui.painter()
                        .rect_filled(row, Rounding::ZERO, palette.accent_soft);
                    ui.painter().rect_filled(
                        Rect::from_min_size(row.min, Vec2::new(2.0, row.height())),
                        Rounding::ZERO,
                        palette.accent,
                    );
                } else if response.hovered() {
                    ui.painter()
                        .rect_filled(row, Rounding::ZERO, palette.surface_2);
                }
                let painter = ui.painter();
                let icon_color = palette.category(node.category);
                let icon_rect = Rect::from_min_size(
                    Pos2::new(cols.name_x, row.center().y - 8.0),
                    Vec2::splat(16.0),
                );
                painter.text(
                    icon_rect.center(),
                    Align2::CENTER_CENTER,
                    if node.is_dir { ph::FOLDER } else { node.category.icon() },
                    theme::font_icon(12.5),
                    icon_color,
                );
                let name_font = theme::font_medium(11.5);
                let name_chars = ((cols.name_w - 24.0) / 5.9).max(8.0) as usize;
                painter.with_clip_rect(Rect::from_min_max(
                    Pos2::new(cols.name_x + 21.0, row.top()),
                    Pos2::new(cols.name_x + cols.name_w, row.bottom()),
                )).text(
                    Pos2::new(cols.name_x + 21.0, row.center().y),
                    Align2::LEFT_CENTER,
                    truncate_middle(&node.name, name_chars),
                    name_font,
                    palette.text,
                );
                let path = tree.path(id);
                let path_chars = (cols.path_w / 5.5).max(12.0) as usize;
                painter.with_clip_rect(Rect::from_min_max(
                    Pos2::new(cols.path_x, row.top()),
                    Pos2::new(cols.path_x + cols.path_w, row.bottom()),
                )).text(
                    Pos2::new(cols.path_x, row.center().y),
                    Align2::LEFT_CENTER,
                    truncate_middle(&path, path_chars),
                    theme::font_regular(9.3),
                    palette.faint,
                );
                painter.text(
                    Pos2::new(cols.size_x + cols.size_w, row.center().y),
                    Align2::RIGHT_CENTER,
                    format_bytes(node.size),
                    theme::font_medium(10.5),
                    palette.text,
                );
                if cols.show_pct {
                    painter.text(
                        Pos2::new(cols.pct_x + cols.pct_w, row.center().y),
                        Align2::RIGHT_CENTER,
                        format_percent(node.size, tree.total_size),
                        theme::font_regular(9.5),
                        palette.muted,
                    );
                }
                if cols.show_date {
                    painter.text(
                        Pos2::new(cols.date_x, row.center().y),
                        Align2::LEFT_CENTER,
                        crate::util::format_mtime(node.mtime),
                        theme::font_regular(9.2),
                        palette.faint,
                    );
                }
                if response.hovered() {
                    painter.text(
                        Pos2::new(cols.right, row.center().y),
                        Align2::RIGHT_CENTER,
                        ph::CARET_RIGHT,
                        theme::font_icon(11.0),
                        palette.muted,
                    );
                }
                painter.line_segment(
                    [
                        Pos2::new(cols.left - 8.0, row.bottom()),
                        Pos2::new(cols.right + 4.0, row.bottom()),
                    ],
                    Stroke::new(1.0, theme::with_alpha(palette.border, 90)),
                );
                response.clone().on_hover_cursor(CursorIcon::PointingHand);
                if response.double_clicked() {
                    if node.is_dir {
                        actions.push(Action::Navigate(id));
                    } else {
                        actions.push(Action::Reveal(id));
                    }
                } else if response.clicked() {
                    actions.push(Action::Select(id));
                }
                file_menu(&response, tree, id, actions);
                if response.hovered() {
                    response.on_hover_text(truncate_middle(&path, 90));
                }
            }
        });
}

pub(crate) fn paint_card(painter: &egui::Painter, palette: &Palette, rect: Rect) {
    painter.rect_filled(rect, Rounding::same(12.0), palette.surface);
    painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, palette.border));
}

fn sort_chip(
    ui: &mut Ui,
    palette: &Palette,
    label: &str,
    key: SortKey,
    state: crate::app::SortState,
    actions: &mut Vec<Action>,
) {
    let active = state.key == key;
    let text = if active {
        format!("{label} {}", state.arrow())
    } else {
        label.to_string()
    };
    let font = theme::font_medium(10.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.clone(), font.clone(), palette.text);
    let width = galley.size().x + 20.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 27.0), Sense::click());
    let fill = if active {
        palette.accent_soft
    } else if response.hovered() {
        palette.surface_2
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(7.0), fill);
    if active {
        ui.painter().rect_stroke(
            rect,
            Rounding::same(7.0),
            Stroke::new(1.0, theme::with_alpha(palette.accent, 80)),
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        font,
        if active { palette.accent } else { palette.muted },
    );
    if response.clicked() {
        actions.push(Action::SortBy(key));
    }
    response.on_hover_cursor(CursorIcon::PointingHand);
}


