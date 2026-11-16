use eframe::egui;
use egui::{
    Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin, Mesh, Painter,
    Pos2, Rect, Response, Rounding, Sense, Shape, Stroke, TextStyle, Ui, Vec2, Visuals,
};

use crate::model::Category;
use crate::settings::ThemeMode;

pub const SIDEBAR_WIDTH: f32 = 246.0;
pub const TITLEBAR_HEIGHT: f32 = 48.0;
pub const STATUS_HEIGHT: f32 = 30.0;
pub const DETAILS_WIDTH: f32 = 302.0;

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color32,
    pub sidebar: Color32,
    pub surface: Color32,
    pub surface_2: Color32,
    pub surface_3: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_2: Color32,
    pub accent_soft: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub danger_soft: Color32,
    pub treemap_bg: Color32,
    pub treemap_container: Color32,
    pub shadow: Color32,
    pub dark: bool,
}

impl Palette {
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                bg: Color32::from_rgb(8, 10, 15),
                sidebar: Color32::from_rgb(11, 14, 20),
                surface: Color32::from_rgb(15, 19, 26),
                surface_2: Color32::from_rgb(20, 25, 36),
                surface_3: Color32::from_rgb(27, 34, 47),
                border: Color32::from_rgb(33, 41, 55),
                border_strong: Color32::from_rgb(48, 59, 77),
                text: Color32::from_rgb(237, 242, 250),
                muted: Color32::from_rgb(148, 159, 179),
                faint: Color32::from_rgb(103, 113, 132),
                accent: Color32::from_rgb(87, 137, 255),
                accent_2: Color32::from_rgb(139, 92, 246),
                accent_soft: Color32::from_rgba_unmultiplied(87, 137, 255, 34),
                success: Color32::from_rgb(61, 220, 151),
                warning: Color32::from_rgb(245, 185, 66),
                danger: Color32::from_rgb(255, 107, 122),
                danger_soft: Color32::from_rgba_unmultiplied(255, 107, 122, 30),
                treemap_bg: Color32::from_rgb(8, 12, 19),
                treemap_container: Color32::from_rgb(12, 16, 24),
                shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 120),
                dark: true,
            },
            ThemeMode::Light => Self {
                bg: Color32::from_rgb(243, 246, 251),
                sidebar: Color32::from_rgb(255, 255, 255),
                surface: Color32::from_rgb(255, 255, 255),
                surface_2: Color32::from_rgb(246, 248, 252),
                surface_3: Color32::from_rgb(235, 240, 248),
                border: Color32::from_rgb(224, 230, 240),
                border_strong: Color32::from_rgb(201, 211, 225),
                text: Color32::from_rgb(16, 24, 39),
                muted: Color32::from_rgb(91, 103, 122),
                faint: Color32::from_rgb(135, 146, 163),
                accent: Color32::from_rgb(48, 106, 240),
                accent_2: Color32::from_rgb(118, 77, 238),
                accent_soft: Color32::from_rgba_unmultiplied(48, 106, 240, 26),
                success: Color32::from_rgb(14, 145, 95),
                warning: Color32::from_rgb(176, 116, 20),
                danger: Color32::from_rgb(208, 45, 72),
                danger_soft: Color32::from_rgba_unmultiplied(208, 45, 72, 24),
                treemap_bg: Color32::from_rgb(230, 235, 244),
                treemap_container: Color32::from_rgb(218, 225, 237),
                shadow: Color32::from_rgba_unmultiplied(22, 35, 58, 40),
                dark: false,
            },
        }
    }

    pub fn category(&self, category: Category) -> Color32 {
        if self.dark {
            match category {
                Category::Video => Color32::from_rgb(124, 108, 255),
                Category::Image => Color32::from_rgb(242, 92, 138),
                Category::Audio => Color32::from_rgb(242, 169, 59),
                Category::Document => Color32::from_rgb(76, 141, 255),
                Category::Archive => Color32::from_rgb(242, 121, 74),
                Category::Code => Color32::from_rgb(43, 212, 168),
                Category::App => Color32::from_rgb(39, 194, 224),
                Category::System => Color32::from_rgb(100, 116, 139),
                Category::Other => Color32::from_rgb(135, 146, 166),
            }
        } else {
            match category {
                Category::Video => Color32::from_rgb(95, 76, 222),
                Category::Image => Color32::from_rgb(211, 49, 105),
                Category::Audio => Color32::from_rgb(190, 119, 18),
                Category::Document => Color32::from_rgb(43, 102, 214),
                Category::Archive => Color32::from_rgb(202, 82, 38),
                Category::Code => Color32::from_rgb(9, 150, 114),
                Category::App => Color32::from_rgb(12, 142, 170),
                Category::System => Color32::from_rgb(96, 109, 128),
                Category::Other => Color32::from_rgb(116, 128, 146),
            }
        }
    }

    pub fn category_text(&self, category: Category) -> Color32 {
        let color = self.category(category);
        let luma = 0.2126 * color.r() as f32 + 0.7152 * color.g() as f32 + 0.0722 * color.b() as f32;
        if luma > 150.0 {
            Color32::from_rgb(10, 13, 19)
        } else {
            Color32::WHITE
        }
    }

    pub fn category_soft(&self, category: Category) -> Color32 {
        let color = self.category(category);
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), if self.dark { 32 } else { 26 })
    }

    pub fn text_on_accent(&self) -> Color32 {
        Color32::WHITE
    }
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}

pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

pub fn family_regular() -> FontFamily {
    FontFamily::Proportional
}

pub fn family_medium() -> FontFamily {
    FontFamily::Name("inter-medium".into())
}

pub fn family_semibold() -> FontFamily {
    FontFamily::Name("inter-semibold".into())
}

pub fn family_bold() -> FontFamily {
    FontFamily::Name("inter-bold".into())
}

pub fn font_regular(size: f32) -> FontId {
    FontId::new(size, family_regular())
}

pub fn font_medium(size: f32) -> FontId {
    FontId::new(size, family_medium())
}

pub fn font_semibold(size: f32) -> FontId {
    FontId::new(size, family_semibold())
}

pub fn font_bold(size: f32) -> FontId {
    FontId::new(size, family_bold())
}

/// Font family that holds the icon font and nothing else.
pub fn family_icon() -> FontFamily {
    FontFamily::Name("icons".into())
}

pub fn font_icon(size: f32) -> FontId {
    FontId::new(size, family_icon())
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter-regular".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "inter-medium".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Inter-Medium.ttf")).into(),
    );
    fonts.font_data.insert(
        "inter-semibold".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Inter-SemiBold.ttf")).into(),
    );
    fonts.font_data.insert(
        "inter-bold".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Inter-Bold.ttf")).into(),
    );
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    for (key, fallback) in [
        ("inter-medium", family_medium()),
        ("inter-semibold", family_semibold()),
        ("inter-bold", family_bold()),
    ] {
        fonts.families.insert(
            fallback.clone(),
            vec![key.to_owned().into(), "phosphor".to_owned().into()],
        );
    }
    if let Some(proportional) = fonts.families.get_mut(&FontFamily::Proportional) {
        proportional.insert(0, "inter-regular".to_owned().into());
    }
    // Icon glyphs live in the private-use range, and both Inter and egui's
    // fallback fonts ship a few glyphs there, so inside the text families part
    // of the icon set resolves to the wrong symbol. Icons therefore get a
    // family of their own that contains nothing but Phosphor.
    fonts.families.insert(
        family_icon(),
        vec!["phosphor".to_owned().into()],
    );
    ctx.set_fonts(fonts);
}

pub fn apply_theme(ctx: &egui::Context, mode: ThemeMode, animations: bool) {
    let palette = Palette::for_mode(mode);
    let mut visuals = match mode {
        ThemeMode::Dark => Visuals::dark(),
        ThemeMode::Light => Visuals::light(),
    };
    visuals.override_text_color = Some(palette.text);
    visuals.panel_fill = palette.bg;
    visuals.window_fill = palette.surface;
    visuals.window_stroke = Stroke::new(1.0, palette.border);
    visuals.window_rounding = Rounding::same(14.0);
    visuals.menu_rounding = Rounding::same(10.0);
    visuals.extreme_bg_color = palette.bg;
    visuals.faint_bg_color = palette.surface_2;
    visuals.code_bg_color = palette.surface_2;
    visuals.hyperlink_color = palette.accent;
    visuals.selection.bg_fill = palette.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0, palette.accent);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 14.0),
        blur: 36.0,
        spread: 0.0,
        color: Color32::from_rgba_unmultiplied(0, 0, 0, if palette.dark { 150 } else { 55 }),
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 24.0,
        spread: 0.0,
        color: Color32::from_rgba_unmultiplied(0, 0, 0, if palette.dark { 140 } else { 45 }),
    };
    visuals.widgets.noninteractive.bg_fill = palette.surface;
    visuals.widgets.noninteractive.weak_bg_fill = palette.surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.inactive.bg_fill = palette.surface_2;
    visuals.widgets.inactive.weak_bg_fill = palette.surface_2;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.muted);
    visuals.widgets.inactive.rounding = Rounding::same(8.0);
    visuals.widgets.hovered.bg_fill = palette.surface_3;
    visuals.widgets.hovered.weak_bg_fill = palette.surface_3;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_strong);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.hovered.rounding = Rounding::same(8.0);
    visuals.widgets.active.bg_fill = palette.surface_3;
    visuals.widgets.active.weak_bg_fill = palette.surface_3;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.active.rounding = Rounding::same(8.0);
    visuals.widgets.open.bg_fill = palette.surface_3;
    visuals.widgets.open.weak_bg_fill = palette.surface_3;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, palette.border_strong);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.animation_time = if animations { 0.16 } else { 0.0 };
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.spacing.interact_size.y = 30.0;
    style.spacing.scroll.bar_width = 7.0;
    style.spacing.scroll.floating = true;
    style.spacing.scroll.bar_inner_margin = 3.0;
    style.spacing.scroll.bar_outer_margin = 4.0;
    style.spacing.indent = 18.0;
    let mut text_styles = std::collections::BTreeMap::new();
    text_styles.insert(TextStyle::Heading, FontId::new(19.0, family_semibold()));
    text_styles.insert(TextStyle::Body, FontId::new(13.0, family_regular()));
    text_styles.insert(TextStyle::Button, FontId::new(13.0, family_medium()));
    text_styles.insert(TextStyle::Small, FontId::new(11.0, family_regular()));
    text_styles.insert(TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace));
    style.text_styles = text_styles;
    ctx.set_style(style);
}

pub fn card(palette: &Palette) -> Frame {
    Frame::none()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.border))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::same(16.0))
}

pub fn inset_card(palette: &Palette) -> Frame {
    Frame::none()
        .fill(palette.surface_2)
        .stroke(Stroke::new(1.0, palette.border))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(12.0))
}

pub fn section_label(ui: &mut Ui, palette: &Palette, text: &str) {
    ui.label(
        egui::RichText::new(text.to_ascii_uppercase())
            .font(FontId::new(9.5, family_semibold()))
            .color(palette.faint),
    );
}

pub fn divider(ui: &mut Ui, palette: &Palette) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::ZERO, palette.border);
}

pub fn primary_button(
    ui: &mut Ui,
    palette: &Palette,
    label: &str,
    icon: Option<&str>,
    width: Option<f32>,
    height: f32,
) -> Response {
    let enabled = ui.is_enabled();
    let font = font_medium(13.0);
    let text_color = if enabled {
        palette.text_on_accent()
    } else {
        palette.faint
    };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let icon_width = if icon.is_some() { 19.0 } else { 0.0 };
    let content_width = galley.size().x + icon_width;
    let width = width.unwrap_or((content_width + 38.0).max(128.0));
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), sense);
    let hovered = response.hovered() && enabled;
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, hovered && enabled, 0.12);
    let pressed = response.is_pointer_button_down_on();
    let base = if enabled { palette.accent } else { palette.surface_2 };
    let mut fill = mix(base, Color32::WHITE, t * 0.08);
    if pressed && enabled {
        fill = mix(base, Color32::BLACK, 0.12);
    }
    let rounding = Rounding::same(9.0);
    let painter = ui.painter();
    painter.rect_filled(rect, rounding, fill);
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.0, with_alpha(Color32::WHITE, if enabled { 28 } else { 10 })),
    );
    // A soft top highlight keeps the button from looking flat.
    let highlight = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + height * 0.5));
    painter.rect_filled(
        highlight,
        Rounding {
            nw: 9.0,
            ne: 9.0,
            sw: 0.0,
            se: 0.0,
        },
        with_alpha(Color32::WHITE, if enabled { 14 } else { 0 }),
    );
    let content_left = rect.center().x - content_width / 2.0;
    if let Some(icon) = icon {
        painter.text(
            Pos2::new(content_left + 7.5, rect.center().y - 0.5),
            Align2::CENTER_CENTER,
            icon,
            font_icon(16.0),
            text_color,
        );
    }
    painter.text(
        Pos2::new(content_left + icon_width, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        text_color,
    );
    if enabled {
        response.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    if !enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
    }
    response
}

pub fn secondary_button(
    ui: &mut Ui,
    palette: &Palette,
    label: &str,
    icon: Option<&str>,
    width: Option<f32>,
    height: f32,
) -> Response {
    let enabled = ui.is_enabled();
    let font = font_medium(12.5);
    let text_color = if enabled { palette.text } else { palette.faint };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let icon_width = if icon.is_some() { 18.0 } else { 0.0 };
    let content_width = galley.size().x + icon_width;
    let width = width.unwrap_or((content_width + 30.0).max(96.0));
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), sense);
    let hovered = response.hovered() && enabled;
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, hovered && enabled, 0.12);
    let fill = mix(palette.surface_2, palette.surface_3, t);
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(8.0), fill);
    painter.rect_stroke(
        rect,
        Rounding::same(8.0),
        Stroke::new(1.0, if hovered { palette.border_strong } else { palette.border }),
    );
    let content_left = rect.center().x - content_width / 2.0;
    if let Some(icon) = icon {
        painter.text(
            Pos2::new(content_left + 7.0, rect.center().y - 0.5),
            Align2::CENTER_CENTER,
            icon,
            font_icon(15.0),
            text_color,
        );
    }
    painter.text(
        Pos2::new(content_left + icon_width, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        text_color,
    );
    if enabled {
        response.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn ghost_button(
    ui: &mut Ui,
    palette: &Palette,
    label: &str,
    icon: Option<&str>,
    height: f32,
) -> Response {
    let font = font_medium(12.0);
    let text_color = palette.muted;
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let icon_width = if icon.is_some() { 17.0 } else { 0.0 };
    let width = galley.size().x + icon_width + 20.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.12);
    if t > 0.0 {
        ui.painter().rect_filled(
            rect,
            Rounding::same(7.0),
            mix(palette.bg, palette.surface_2, t),
        );
    }
    let content_left = rect.center().x - (galley.size().x + icon_width) / 2.0;
    if let Some(icon) = icon {
        ui.painter().text(
            Pos2::new(content_left + 6.5, rect.center().y - 0.5),
            Align2::CENTER_CENTER,
            icon,
            font_icon(14.5),
            mix(palette.muted, palette.text, t),
        );
    }
    ui.painter().text(
        Pos2::new(content_left + icon_width, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        mix(palette.muted, palette.text, t),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn icon_button(
    ui: &mut Ui,
    palette: &Palette,
    icon: &str,
    size: f32,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.12);
    if t > 0.0 {
        ui.painter().rect_filled(
            rect,
            Rounding::same(7.0),
            mix(palette.bg, palette.surface_2, t),
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        icon,
        font_icon((size * 0.46).clamp(13.0, 18.0)),
        mix(palette.muted, palette.text, t),
    );
    if !tooltip.is_empty() {
        response.clone().on_hover_text(tooltip);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn pill(ui: &mut Ui, palette: &Palette, text: &str, color: Color32) -> Response {
    let font = font_semibold(9.5);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_ascii_uppercase(), font.clone(), color);
    let width = galley.size().x + 16.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 19.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(9.5), with_alpha(color, if palette.dark { 26 } else { 20 }));
    ui.painter().rect_stroke(
        rect,
        Rounding::same(9.5),
        Stroke::new(1.0, with_alpha(color, if palette.dark { 46 } else { 34 })),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text.to_ascii_uppercase(),
        font,
        color,
    );
    response
}

pub fn progress_bar(
    painter: &Painter,
    rect: Rect,
    fraction: f32,
    fill: Color32,
    background: Color32,
) {
    let rounding = Rounding::same(rect.height() * 0.5);
    painter.rect_filled(rect, rounding, background);
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 {
        return;
    }
    let width = (rect.width() * fraction).max(rect.height());
    let filled = Rect::from_min_size(rect.min, Vec2::new(width.min(rect.width()), rect.height()));
    painter.rect_filled(filled, rounding, fill);
}

pub fn paint_logo(painter: &Painter, rect: Rect, palette: &Palette) {
    let gap = rect.width() * 0.09;
    let left_w = rect.width() * 0.56;
    let right_w = rect.width() - left_w - gap;
    let top_h = rect.height() * 0.56;
    let bottom_h = rect.height() - top_h - gap;
    let rounding = Rounding::same(rect.width() * 0.11);
    painter.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(left_w, rect.height())),
        rounding,
        palette.accent,
    );
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(rect.left() + left_w + gap, rect.top()),
            Vec2::new(right_w, top_h),
        ),
        rounding,
        palette.accent_2,
    );
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(rect.left() + left_w + gap, rect.top() + top_h + gap),
            Vec2::new(right_w, bottom_h),
        ),
        rounding,
        palette.category(Category::App),
    );
}

pub fn paint_vertical_gradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(2, 1, 3);
    painter.add(Shape::mesh(mesh));
}

pub fn icon_text(
    ui: &mut Ui,
    icon: &str,
    text: &str,
    icon_color: Color32,
    text_color: Color32,
    size: f32,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(
            egui::RichText::new(icon)
                .font(font_icon(size))
                .color(icon_color),
        );
        ui.label(
            egui::RichText::new(text)
                .font(font_regular(size - 0.5))
                .color(text_color),
        );
    });
}

pub fn truncate_to_width(
    ui: &Ui,
    text: &str,
    font: FontId,
    max_width: f32,
) -> String {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE);
    if galley.size().x <= max_width {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        let galley = ui
            .painter()
            .layout_no_wrap(candidate.clone(), font.clone(), Color32::WHITE);
        if galley.size().x <= max_width {
            return candidate;
        }
    }
    "…".to_string()
}

pub fn category_legend(ui: &mut Ui, palette: &Palette, category: Category) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, Rounding::same(2.0), palette.category(category));
        ui.label(
            egui::RichText::new(category.label())
                .font(font_regular(11.0))
                .color(palette.muted),
        );
    });
}

pub fn dot(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    painter.circle_filled(center, radius, color);
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(1.0, with_alpha(Color32::WHITE, 40)),
    );
}

pub fn tooltip(ui: &mut Ui, palette: &Palette, text: &str) {
    egui::Frame::none()
        .fill(palette.surface_3)
        .stroke(Stroke::new(1.0, palette.border_strong))
        .rounding(Rounding::same(7.0))
        .inner_margin(Margin::symmetric(8.0, 5.0))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .font(font_regular(11.0))
                    .color(palette.text),
            );
        });
}
