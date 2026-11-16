use eframe::egui;
use egui::{
    Align, Align2, Color32, CursorIcon, Frame, Id, Layout, Margin, Pos2, Rect, RichText, Rounding,
    ScrollArea, Sense, Stroke, Ui, Vec2,
};
use egui_phosphor::regular as ph;

use crate::app::{Action, App};
use crate::dupes::{DupFile, DupJob, DupResults};
use crate::model::{Category, NodeId, Tree};
use crate::theme::{self, Palette};
use crate::util::{format_bytes, format_count, truncate_middle};

pub(crate) fn duplicate_idle(
    ui: &mut Ui,
    palette: &Palette,
    card: Rect,
    actions: &mut Vec<Action>,
) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(card.shrink(24.0))
            .layout(Layout::top_down(Align::Center)),
    );
    child.add_space((card.height() * 0.19 - 30.0).clamp(16.0, 140.0));
    let (art, _) = child.allocate_exact_size(Vec2::new(92.0, 74.0), Sense::hover());
    let back = Rect::from_min_size(
        Pos2::new(art.left() + 16.0, art.top() + 6.0),
        Vec2::new(52.0, 62.0),
    );
    let front = Rect::from_min_size(
        Pos2::new(art.left() + 30.0, art.top() + 16.0),
        Vec2::new(52.0, 58.0),
    );
    child.painter().rect_filled(
        back,
        Rounding::same(9.0),
        palette.category_soft(Category::Document),
    );
    child.painter().rect_stroke(
        back,
        Rounding::same(9.0),
        Stroke::new(
            1.0,
            theme::with_alpha(palette.category(Category::Document), 100),
        ),
    );
    child
        .painter()
        .rect_filled(front, Rounding::same(9.0), palette.surface_3);
    child.painter().rect_stroke(
        front,
        Rounding::same(9.0),
        Stroke::new(1.0, palette.border_strong),
    );
    child.painter().text(
        front.center(),
        Align2::CENTER_CENTER,
        ph::COPY,
        theme::font_icon(21.0),
        palette.accent,
    );
    child.add_space(14.0);
    child.label(
        RichText::new("Find duplicate files")
            .font(theme::font_semibold(18.0))
            .color(palette.text),
    );
    child.add_space(6.0);
    child.label(
        RichText::new(
            "Files are grouped by exact size, then compared by content hash. Nothing is deleted without your confirmation.",
        )
        .font(theme::font_regular(10.8))
        .color(palette.muted),
    );
    child.add_space(18.0);
    if theme::primary_button(
        &mut child,
        palette,
        "Search for duplicates",
        Some(ph::COPY),
        Some(190.0),
        36.0,
    )
    .clicked()
    {
        actions.push(Action::StartDuplicates);
    }
    child.add_space(7.0);
    child.label(
        RichText::new("Only files larger than 1 MB are compared")
            .font(theme::font_regular(9.0))
            .color(palette.faint),
    );
}

pub(crate) fn duplicate_running(
    ui: &mut Ui,
    palette: &Palette,
    card: Rect,
    job: &DupJob,
    actions: &mut Vec<Action>,
) {
    let progress = job.progress.snapshot();
    let fraction = if progress.files_total == 0 {
        0.0
    } else {
        (progress.files_done as f32 / (progress.files_total as f32 * 2.0)).min(0.98)
    };
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(card.shrink(24.0))
            .layout(Layout::top_down(Align::Center)),
    );
    child.add_space((card.height() * 0.22 - 30.0).clamp(16.0, 150.0));
    let (art, _) = child.allocate_exact_size(Vec2::new(64.0, 64.0), Sense::hover());
    child
        .painter()
        .circle_filled(art.center(), 31.0, palette.accent_soft);
    child.painter().text(
        art.center(),
        Align2::CENTER_CENTER,
        ph::COPY,
        theme::font_icon(24.0),
        palette.accent,
    );
    child.add_space(14.0);
    child.label(
        RichText::new("Comparing file contents")
            .font(theme::font_semibold(17.0))
            .color(palette.text),
    );
    child.add_space(4.0);
    child.label(
        RichText::new(&progress.phase)
            .font(theme::font_medium(10.5))
            .color(palette.accent),
    );
    child.add_space(14.0);
    let bar = Rect::from_min_size(
        child.cursor().min,
        Vec2::new(340.0_f32.min(child.available_width()), 6.0),
    );
    indeterminate_bar(
        child.painter(),
        bar,
        child.input(|input| input.time),
        palette,
    );
    child.add_space(12.0);
    child.horizontal(|ui| {
        ui.label(
            RichText::new(format!(
                "{} hashes · {} read",
                format_count(progress.files_done),
                format_bytes(progress.bytes_read)
            ))
            .font(theme::font_regular(10.0))
            .color(palette.muted),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{:.0}%", fraction * 100.0))
                    .font(theme::font_medium(10.0))
                    .color(palette.text),
            );
        });
    });
    child.add_space(4.0);
    child.label(
        RichText::new(truncate_middle(&progress.current, 72))
            .font(theme::font_regular(9.0))
            .color(palette.faint),
    );
    child.add_space(16.0);
    if theme::secondary_button(
        &mut child,
        palette,
        "Cancel",
        Some(ph::X),
        Some(104.0),
        32.0,
    )
    .clicked()
    {
        actions.push(Action::CancelDuplicates);
    }
}

pub(crate) fn duplicate_results(
    ui: &mut Ui,
    app: &App,
    tree: &Tree,
    results: &DupResults,
    actions: &mut Vec<Action>,
    width: f32,
    height: f32,
    margin: f32,
) {
    let palette = &app.palette;
    let gap = 10.0;
    let summary_w = (width - gap * 2.0) / 3.0;
    ui.horizontal(|ui| {
        ui.add_space(margin);
        ui.spacing_mut().item_spacing.x = gap;
        dup_summary(
            ui,
            palette,
            summary_w,
            "DUPLICATE GROUPS",
            &format_count(results.groups.len() as u64),
            &format!("{} files compared", format_count(results.candidates)),
            palette.accent,
            ph::COPY,
        );
        dup_summary(
            ui,
            palette,
            summary_w,
            "REDUNDANT FILES",
            &format_count(results.duplicate_files()),
            "identical copies beyond the first",
            palette.warning,
            ph::FILES,
        );
        dup_summary(
            ui,
            palette,
            summary_w,
            "RECOVERABLE",
            &format_bytes(results.wasted()),
            "if extra copies are removed",
            palette.success,
            ph::DATABASE,
        );
    });
    ui.add_space(9.0);
    let list_h = (height - 94.0).max(160.0);
    ui.horizontal(|ui| {
        ui.add_space(margin);
        let (list_rect, _) = ui.allocate_exact_size(Vec2::new(width, list_h), Sense::hover());
        let mut list = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(list_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut list, |ui| {
                for (group_index, group) in results.groups.iter().enumerate() {
                    Frame::none()
                        .fill(palette.surface)
                        .stroke(Stroke::new(1.0, palette.border))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(13.0, 11.0))
                        .show(ui, |ui| {
                            ui.set_width((width - 26.0).max(280.0));
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{} copies", group.files.len()))
                                        .font(theme::font_semibold(11.5))
                                        .color(palette.text),
                                );
                                ui.label(
                                    RichText::new(format!("· {} each", format_bytes(group.size)))
                                        .font(theme::font_regular(9.5))
                                        .color(palette.muted),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    theme::pill(
                                        ui,
                                        palette,
                                        &format!("{} recoverable", format_bytes(group.wasted())),
                                        palette.success,
                                    );
                                });
                            });
                            ui.add_space(5.0);
                            for (file_index, file) in group.files.iter().enumerate() {
                                let response = duplicate_file_row(
                                    ui,
                                    palette,
                                    group_index,
                                    file_index,
                                    file,
                                    app.selection,
                                    actions,
                                );
                                if response.double_clicked() {
                                    actions.push(Action::Reveal(file.node));
                                } else if response.clicked() {
                                    actions.push(Action::Select(file.node));
                                }
                            }
                        });
                    ui.add_space(7.0);
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "Duplicates are matched by exact size and xxHash3 content hash. Verify a file before removing it.",
                    )
                    .font(theme::font_regular(9.0))
                    .color(palette.faint),
                );
            });
    });
}

#[allow(clippy::too_many_arguments)]
fn duplicate_file_row(
    ui: &mut Ui,
    palette: &Palette,
    group_index: usize,
    file_index: usize,
    file: &DupFile,
    selected: Option<NodeId>,
    actions: &mut Vec<Action>,
) -> egui::Response {
    let (row, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
    if selected == Some(file.node) {
        ui.painter()
            .rect_filled(row, Rounding::same(6.0), palette.accent_soft);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(row, Rounding::same(6.0), palette.surface_2);
    }
    let painter = ui.painter();
    painter.text(
        Pos2::new(row.left() + 6.0, row.center().y),
        Align2::LEFT_CENTER,
        ph::FILE,
        theme::font_icon(12.0),
        palette.category(Category::Other),
    );
    let reveal_w = 25.0;
    let trash_w = 25.0;
    let text_right = row.right() - reveal_w - trash_w - 16.0;
    painter
        .with_clip_rect(Rect::from_min_max(
            Pos2::new(row.left() + 22.0, row.top()),
            Pos2::new(text_right, row.bottom()),
        ))
        .text(
            Pos2::new(row.left() + 22.0, row.center().y),
            Align2::LEFT_CENTER,
            truncate_middle(&file.path.to_string_lossy(), 82),
            theme::font_regular(9.2),
            palette.muted,
        );
    painter.text(
        Pos2::new(text_right, row.center().y),
        Align2::RIGHT_CENTER,
        format_bytes(file.size),
        theme::font_medium(9.2),
        palette.text,
    );
    let reveal_rect = Rect::from_center_size(
        Pos2::new(row.right() - trash_w - 14.0, row.center().y),
        Vec2::splat(23.0),
    );
    let trash_rect = Rect::from_center_size(
        Pos2::new(row.right() - 13.0, row.center().y),
        Vec2::splat(23.0),
    );
    let reveal_response = ui.interact(
        reveal_rect,
        Id::new(("dup_reveal", group_index, file_index)),
        Sense::click(),
    );
    let trash_response = ui.interact(
        trash_rect,
        Id::new(("dup_trash", group_index, file_index)),
        Sense::click(),
    );
    if reveal_response.hovered() {
        painter.rect_filled(reveal_rect, Rounding::same(5.0), palette.surface_3);
    }
    if trash_response.hovered() {
        painter.rect_filled(trash_rect, Rounding::same(5.0), palette.danger_soft);
    }
    painter.text(
        reveal_rect.center(),
        Align2::CENTER_CENTER,
        ph::ARROW_SQUARE_OUT,
        theme::font_icon(11.0),
        palette.muted,
    );
    painter.text(
        trash_rect.center(),
        Align2::CENTER_CENTER,
        ph::TRASH,
        theme::font_icon(11.0),
        if trash_response.hovered() {
            palette.danger
        } else {
            palette.faint
        },
    );
    if reveal_response.clicked() {
        actions.push(Action::Reveal(file.node));
    }
    if trash_response.clicked() {
        actions.push(Action::DeleteDuplicate {
            node: file.node,
            group: group_index,
        });
    }
    response.context_menu(|ui| {
        if ui.button("Open file location").clicked() {
            actions.push(Action::Reveal(file.node));
            ui.close_menu();
        }
        if ui.button("Open file").clicked() {
            actions.push(Action::OpenItem(file.node));
            ui.close_menu();
        }
        if ui.button("Copy path").clicked() {
            actions.push(Action::CopyPath(file.node));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Delete…").clicked() {
            actions.push(Action::DeleteDuplicate {
                node: file.node,
                group: group_index,
            });
            ui.close_menu();
        }
    });
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn dup_summary(
    ui: &mut Ui,
    palette: &Palette,
    width: f32,
    label: &str,
    value: &str,
    subtitle: &str,
    accent: Color32,
    icon: &str,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 84.0), Sense::hover());
    crate::views::paint_card(ui.painter(), palette, rect);
    let icon_rect = Rect::from_min_size(
        Pos2::new(rect.right() - 37.0, rect.top() + 13.0),
        Vec2::splat(24.0),
    );
    ui.painter()
        .rect_filled(icon_rect, Rounding::same(6.0), theme::with_alpha(accent, 24));
    ui.painter().text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        icon,
        theme::font_icon(12.0),
        accent,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 13.0, rect.top() + 15.0),
        Align2::LEFT_CENTER,
        label,
        theme::font_semibold(8.5),
        palette.faint,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 13.0, rect.top() + 37.0),
        Align2::LEFT_CENTER,
        value,
        theme::font_semibold(20.0),
        palette.text,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 13.0, rect.top() + 61.0),
        Align2::LEFT_CENTER,
        truncate_middle(subtitle, 34),
        theme::font_regular(9.0),
        palette.faint,
    );
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
            Rect::from_min_max(
                Pos2::new(start, rect.top()),
                Pos2::new(end, rect.bottom()),
            ),
            Rounding::same(rect.height() * 0.5),
            palette.accent,
        );
    }
}
