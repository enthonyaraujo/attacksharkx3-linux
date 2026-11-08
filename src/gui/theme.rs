//! Visual theme and design system matching the original Attack Shark X3 Windows application.

use eframe::egui::{self, Color32, Pos2, Rounding, Stroke, Vec2};

pub const COLOR_BG_DARK: Color32 = Color32::from_rgb(0x13, 0x13, 0x13);
pub const COLOR_PANEL_BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
pub const COLOR_CARD_BG: Color32 = Color32::from_rgb(0x21, 0x21, 0x21);
pub const COLOR_HEADER_BG: Color32 = Color32::from_rgb(0x24, 0x24, 0x24);
pub const COLOR_HEADER_BORDER: Color32 = Color32::from_rgb(0x32, 0x32, 0x32);
pub const COLOR_ACCENT_RED: Color32 = Color32::from_rgb(0xE2, 0x21, 0x1C);
pub const COLOR_ACCENT_RED_HOVER: Color32 = Color32::from_rgb(0xFF, 0x35, 0x2B);
pub const COLOR_BORDER: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);
pub const COLOR_BORDER_ACTIVE: Color32 = Color32::from_rgb(0xE2, 0x21, 0x1C);

pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(0xEA, 0xEA, 0xEA);
pub const COLOR_TEXT_SECONDARY: Color32 = Color32::from_rgb(0x94, 0x94, 0x94);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(0x56, 0x56, 0x56);

pub const COLOR_CONNECTED_GREEN: Color32 = Color32::from_rgb(0x34, 0xC7, 0x59);
pub const COLOR_BATTERY_GREEN: Color32 = Color32::from_rgb(0x30, 0xD1, 0x58);

pub fn apply_theme(ctx: &egui::Context) {
    // Load custom HelveticaNeue font
    let font_bytes = include_bytes!("../../assets/fonts/HelveticaNeue.ttf");
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "HelveticaNeue".to_owned(),
        egui::FontData::from_static(font_bytes),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "HelveticaNeue".to_owned());
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();

    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(COLOR_TEXT_PRIMARY);
    style.visuals.panel_fill = COLOR_BG_DARK;
    style.visuals.window_fill = COLOR_PANEL_BG;
    style.visuals.extreme_bg_color = Color32::from_rgb(0x0C, 0x0C, 0x0C);
    style.visuals.faint_bg_color = COLOR_CARD_BG;

    style.visuals.widgets.noninteractive.bg_fill = COLOR_PANEL_BG;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    style.visuals.widgets.noninteractive.rounding = Rounding::same(3.0);

    style.visuals.widgets.inactive.bg_fill = COLOR_CARD_BG;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    style.visuals.widgets.inactive.rounding = Rounding::same(3.0);

    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x2E, 0x2E, 0x2E);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, COLOR_ACCENT_RED_HOVER);
    style.visuals.widgets.hovered.rounding = Rounding::same(3.0);

    style.visuals.widgets.active.bg_fill = COLOR_ACCENT_RED;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, COLOR_ACCENT_RED);
    style.visuals.widgets.active.rounding = Rounding::same(3.0);

    style.spacing.item_spacing = Vec2::new(6.0, 6.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);

    ctx.set_style(style);
}

/// Helper to decode PNG bytes into an egui::ColorImage
pub fn load_color_image_from_bytes(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    Ok(egui::ColorImage::from_rgba_unmultiplied(size, &pixels))
}

/// Header bar matching original header_1.png style with minus icon
pub fn section_header_bar(ui: &mut egui::Ui, title: &str) {
    let desired_size = Vec2::new(ui.available_width(), 24.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(3.0), COLOR_HEADER_BG);
    painter.rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0_f32, COLOR_HEADER_BORDER));
    painter.text(
        Pos2::new(rect.min.x + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(12.0),
        COLOR_TEXT_PRIMARY,
    );
    painter.text(
        Pos2::new(rect.max.x - 12.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        "—",
        egui::FontId::proportional(11.0),
        COLOR_TEXT_MUTED,
    );
}

/// Interactive collapsible header bar
pub fn collapsible_section_header(
    ui: &mut egui::Ui,
    title: &str,
    is_expanded: &mut bool,
) -> egui::Response {
    let desired_size = Vec2::new(ui.available_width(), 24.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *is_expanded = !*is_expanded;
    }

    let is_hovered = response.hovered();
    let bg_color = if is_hovered {
        Color32::from_rgb(0x2E, 0x2E, 0x2E)
    } else {
        COLOR_HEADER_BG
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(3.0), bg_color);
    painter.rect_stroke(
        rect,
        Rounding::same(3.0),
        Stroke::new(1.0_f32, if is_hovered { COLOR_BORDER_ACTIVE } else { COLOR_HEADER_BORDER }),
    );

    painter.text(
        Pos2::new(rect.min.x + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(12.0),
        COLOR_TEXT_PRIMARY,
    );

    let icon_text = if *is_expanded { "—" } else { "+" };
    painter.text(
        Pos2::new(rect.max.x - 12.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        icon_text,
        egui::FontId::proportional(11.0),
        COLOR_TEXT_MUTED,
    );

    response
}
