use eframe::egui;
use egui::{
    Align, Align2, CursorIcon, Layout, Pos2, Rect, RichText, Rounding, ScrollArea, Sense, Stroke,
    Ui, Vec2,
};
use egui_phosphor::regular as ph;

use crate::app::{Action, App};
use crate::model::{extension_of, Category, NodeId, Tree};
use crate::theme::{self, Palette};
use crate::util::{format_bytes, format_count, format_percent, truncate_middle};

pub(crate) fn paint_stacked_categories(
    painter: &egui::Painter,
    palette: &Palette,
    tree: &Tree,
    rect: Rect,
    total: u64,
) {
    painter.rect_filled(rect, Rounding::same(rect.height() * 0.5), palette.surface_3);
    if total == 0 {
        return;
    }
    let inner = rect.shrink(1.5);
    let gap = 1.2;
    let mut x = inner.left();
    for category in Category::ALL {
        let size = tree.cat_sizes[category.index()];
        if size == 0 {
            continue;
        }
        let width = inner.width() * (size as f32 / total as f32);
        if width < 0.5 {
            continue;
        }
        let segment = Rect::from_min_size(
            Pos2::new(x, inner.top()),
            Vec2::new((width - gap).max(0.5).min(inner.right() - x), inner.height()),
        );
        if segment.width() > 0.0 {
            painter.rect_filled(segment, Rounding::same(4.0), palette.category(category));
        }
        x += width;
        if x >= inner.right() {
            break;
        }
    }
}

pub(crate) fn category_chip(
    ui: &mut Ui,
    palette: &Palette,
    category: Category,
    size: u64,
    total: u64,
    selected: bool,
) -> egui::Response {
    let width = 112.0;
    let height = 58.0;
    let sense = if size > 0 {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), sense);
    let fill = if selected {
        palette.accent_soft
    } else if response.hovered() && size > 0 {
        palette.surface_2
    } else {
        palette.surface
    };
    ui.painter().rect_filled(rect, Rounding::same(9.0), fill);
    ui.painter().rect_stroke(
        rect,
        Rounding::same(9.0),
        Stroke::new(
            1.0,
            if selected {
                theme::with_alpha(palette.accent, 140)
            } else {
                palette.border
            },
        ),
    );
    let color = if size == 0 {
        palette.faint
    } else {
        palette.category(category)
    };
    ui.painter().circle_filled(
        Pos2::new(rect.left() + 12.0, rect.top() + 15.0),
        4.0,
        color,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 22.0, rect.top() + 15.0),
        Align2::LEFT_CENTER,
        category.label(),
        theme::font_medium(9.0),
        if size == 0 { palette.faint } else { palette.text },
    );
    ui.painter().text(
        Pos2::new(rect.left() + 10.0, rect.top() + 34.0),
        Align2::LEFT_CENTER,
        format_bytes(size),
        theme::font_semibold(10.5),
        if size == 0 { palette.faint } else { palette.text },
    );
    ui.painter().text(
        Pos2::new(rect.right() - 9.0, rect.top() + 34.0),
        Align2::RIGHT_CENTER,
        format_percent(size, total),
        theme::font_regular(8.5),
        palette.faint,
    );
    let bar = Rect::from_min_size(
        Pos2::new(rect.left() + 10.0, rect.bottom() - 10.0),
        Vec2::new(rect.width() - 20.0, 3.0),
    );
    theme::progress_bar(
        ui.painter(),
        bar,
        size as f32 / total.max(1) as f32,
        color,
        palette.surface_3,
    );
    if size > 0 {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

pub(crate) fn ext_table(
    ui: &mut Ui,
    app: &App,
    tree: &Tree,
    actions: &mut Vec<Action>,
    sort: crate::app::SortState,
) {
    let palette = &app.palette;
    let width = ui.available_width();
    let mut stats: Vec<&crate::model::ExtStat> = tree
        .ext_stats
        .iter()
        .filter(|stat| {
            app.selected_category
                .map(|category| stat.category == category)
                .unwrap_or(true)
        })
        .collect();
    match sort.key {
        crate::app::SortKey::Name => stats.sort_unstable_by(|a, b| a.ext.cmp(&b.ext)),
        crate::app::SortKey::Count => stats.sort_unstable_by(|a, b| a.count.cmp(&b.count)),
        _ => stats.sort_unstable_by(|a, b| a.size.cmp(&b.size)),
    }
    if sort.descending {
        stats.reverse();
    }
    let left = 14.0;
    let ext_w = 96.0;
    let category_w = 104.0;
    let files_w = 74.0;
    let avg_w = 84.0;
    let size_w = 90.0;
    let right = width - 14.0;
    let share_x = left + ext_w + category_w + files_w + avg_w + size_w;
    let share_w = (right - share_x - 8.0).max(40.0);
    let (header, _) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::hover());
    let painter = ui.painter();
    let font = theme::font_semibold(8.8);
    painter.text(
        Pos2::new(left, header.center().y),
        Align2::LEFT_CENTER,
        "EXTENSION",
        font.clone(),
        palette.faint,
    );
    painter.text(
        Pos2::new(left + ext_w, header.center().y),
        Align2::LEFT_CENTER,
        "CATEGORY",
        font.clone(),
        palette.faint,
    );
    painter.text(
        Pos2::new(left + ext_w + category_w + files_w - 6.0, header.center().y),
        Align2::RIGHT_CENTER,
        "FILES",
        font.clone(),
        palette.faint,
    );
    painter.text(
        Pos2::new(
            left + ext_w + category_w + files_w + avg_w - 6.0,
            header.center().y,
        ),
        Align2::RIGHT_CENTER,
        "AVG",
        font.clone(),
        palette.faint,
    );
    painter.text(
        Pos2::new(
            left + ext_w + category_w + files_w + avg_w + size_w - 6.0,
            header.center().y,
        ),
        Align2::RIGHT_CENTER,
        "SIZE",
        font.clone(),
        palette.faint,
    );
    painter.text(
        Pos2::new(share_x, header.center().y),
        Align2::LEFT_CENTER,
        "SHARE",
        font,
        palette.faint,
    );
    painter.line_segment(
        [
            Pos2::new(left - 4.0, header.bottom()),
            Pos2::new(right, header.bottom()),
        ],
        Stroke::new(1.0, palette.border),
    );
    if stats.is_empty() {
        empty_state(
            ui,
            palette,
            Rect::from_min_size(
                Pos2::new(0.0, ui.cursor().top() + 40.0),
                Vec2::new(width, 200.0),
            ),
            ph::FILE,
            "No file types in this category",
            "Select a different category or clear the filter.",
        );
        return;
    }
    ui.add_space(2.0);
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, 34.0, stats.len(), |ui, range| {
            for index in range {
                let stat = stats[index];
                let (row, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 34.0),
                    Sense::click(),
                );
                if response.hovered() {
                    ui.painter()
                        .rect_filled(row, Rounding::ZERO, palette.surface_2);
                }
                let painter = ui.painter();
                let badge = Rect::from_min_size(
                    Pos2::new(left, row.center().y - 9.0),
                    Vec2::new(
                        (stat.ext.chars().count() as f32 * 5.6 + 14.0).clamp(42.0, 88.0),
                        18.0,
                    ),
                );
                painter.rect_filled(
                    badge,
                    Rounding::same(5.0),
                    palette.category_soft(stat.category),
                );
                painter.text(
                    badge.center(),
                    Align2::CENTER_CENTER,
                    truncate_middle(&stat.ext, 12),
                    theme::font_medium(9.0),
                    palette.category(stat.category),
                );
                painter.text(
                    Pos2::new(left + ext_w, row.center().y),
                    Align2::LEFT_CENTER,
                    stat.category.label(),
                    theme::font_regular(9.5),
                    palette.muted,
                );
                painter.text(
                    Pos2::new(left + ext_w + category_w + files_w - 6.0, row.center().y),
                    Align2::RIGHT_CENTER,
                    format_count(stat.count),
                    theme::font_regular(9.5),
                    palette.muted,
                );
                let avg = if stat.count == 0 {
                    0
                } else {
                    stat.size / stat.count
                };
                painter.text(
                    Pos2::new(
                        left + ext_w + category_w + files_w + avg_w - 6.0,
                        row.center().y,
                    ),
                    Align2::RIGHT_CENTER,
                    format_bytes(avg),
                    theme::font_regular(9.5),
                    palette.faint,
                );
                painter.text(
                    Pos2::new(
                        left + ext_w + category_w + files_w + avg_w + size_w - 6.0,
                        row.center().y,
                    ),
                    Align2::RIGHT_CENTER,
                    format_bytes(stat.size),
                    theme::font_medium(10.0),
                    palette.text,
                );
                let bar = Rect::from_min_size(
                    Pos2::new(share_x, row.center().y - 2.5),
                    Vec2::new(share_w, 5.0),
                );
                theme::progress_bar(
                    painter,
                    bar,
                    stat.size as f32 / tree.total_size.max(1) as f32,
                    palette.category(stat.category),
                    palette.surface_3,
                );
                painter.line_segment(
                    [
                        Pos2::new(left - 4.0, row.bottom()),
                        Pos2::new(right, row.bottom()),
                    ],
                    Stroke::new(1.0, theme::with_alpha(palette.border, 90)),
                );
                if response.clicked() {
                    if let Some(id) = file_with_extension(tree, &stat.ext) {
                        actions.push(Action::Select(id));
                    }
                }
                response.on_hover_cursor(CursorIcon::PointingHand);
            }
        });
}

fn file_with_extension(tree: &Tree, ext_label: &str) -> Option<NodeId> {
    let ext = ext_label.trim_start_matches('.');
    for &id in &tree.file_ids {
        let node = tree.node(id);
        let node_ext = extension_of(&node.name);
        let matches = if ext_label.eq_ignore_ascii_case("No extension") {
            node_ext.is_none()
        } else {
            node_ext
                .map(|value| value.eq_ignore_ascii_case(ext))
                .unwrap_or(false)
        };
        if matches {
            return Some(id);
        }
    }
    None
}

pub(crate) fn empty_state(
    ui: &mut Ui,
    palette: &Palette,
    rect: Rect,
    icon: &str,
    title: &str,
    text: &str,
) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(Align::Center)),
    );
    let top = (rect.height() * 0.28 - 20.0).clamp(18.0, 150.0);
    child.add_space(top);
    let (icon_rect, _) = child.allocate_exact_size(Vec2::splat(54.0), Sense::hover());
    child
        .painter()
        .rect_filled(icon_rect, Rounding::same(14.0), palette.accent_soft);
    child.painter().text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        icon,
        theme::font_icon(23.0),
        palette.accent,
    );
    child.add_space(14.0);
    child.label(
        RichText::new(title)
            .font(theme::font_semibold(15.0))
            .color(palette.text),
    );
    child.add_space(5.0);
    child.label(
        RichText::new(text)
            .font(theme::font_regular(10.5))
            .color(palette.muted),
    );
}
