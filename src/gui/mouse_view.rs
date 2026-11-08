//! Central column: Attack Shark X3 mouse visualization with authentic image and Device Sleep overlay.

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use crate::device::ConnectionType;
use crate::gui::theme::*;

pub struct MouseView {
    pub selected_button: Option<usize>,
    mouse_texture: Option<egui::TextureHandle>,
}

impl Default for MouseView {
    fn default() -> Self {
        Self {
            selected_button: None,
            mouse_texture: None,
        }
    }
}

impl MouseView {
    fn ensure_texture(&mut self, ctx: &egui::Context) -> Option<&egui::TextureHandle> {
        if self.mouse_texture.is_none() {
            let bytes = include_bytes!("../../assets/images/ms_1/mouse.png");
            if let Ok(color_img) = load_color_image_from_bytes(bytes) {
                self.mouse_texture = Some(ctx.load_texture(
                    "mouse_view_img",
                    color_img,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        self.mouse_texture.as_ref()
    }

    pub fn show(&mut self, ui: &mut egui::Ui, conn: ConnectionType) {
        let texture_opt = self.ensure_texture(ui.ctx()).cloned();

        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            let mouse_w = 215.0;
            let mouse_h = 398.0;

            let (rect, resp) = ui.allocate_exact_size(
                Vec2::new(mouse_w, mouse_h),
                egui::Sense::click(),
            );

            let painter = ui.painter_at(rect);

            // Draw the official Attack Shark X3 mouse image
            if let Some(tex) = texture_opt {
                painter.image(
                    tex.id(),
                    rect,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }

            // Button click detection
            if resp.clicked() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    let rel_x = pos.x - rect.min.x;
                    let rel_y = pos.y - rect.min.y;

                    // Button 1 (Left click) ~ (40..95, 45..105)
                    if (40.0..=95.0).contains(&rel_x) && (45.0..=105.0).contains(&rel_y) {
                        self.selected_button = Some(1);
                    }
                    // Button 2 (Right click) ~ (120..175, 45..105)
                    else if (120.0..=175.0).contains(&rel_x) && (45.0..=105.0).contains(&rel_y) {
                        self.selected_button = Some(2);
                    }
                    // Button 3 (Middle wheel) ~ (95..120, 45..105)
                    else if (95.0..=120.0).contains(&rel_x) && (45.0..=105.0).contains(&rel_y) {
                        self.selected_button = Some(3);
                    }
                    // Button 4 (Forward side) ~ (0..35, 120..165)
                    else if (0.0..=35.0).contains(&rel_x) && (120.0..=165.0).contains(&rel_y) {
                        self.selected_button = Some(4);
                    }
                    // Button 5 (Backward side) ~ (0..35, 175..220)
                    else if (0.0..=35.0).contains(&rel_x) && (175.0..=220.0).contains(&rel_y) {
                        self.selected_button = Some(5);
                    }
                }
            }

            // "Device Sleep!" Overlay across the mouse
            let overlay_text = match conn {
                ConnectionType::Wireless24G => "Device Connected",
                ConnectionType::WiredUsb => "Wired Mode",
                ConnectionType::PermissionDenied => "Access Denied",
                ConnectionType::Disconnected => "Device Sleep!",
            };

            let overlay_pos = Pos2::new(rect.center().x, rect.min.y + 175.0);
            let pill_rect = Rect::from_center_size(overlay_pos, Vec2::new(150.0, 32.0));
            painter.rect_filled(
                pill_rect,
                Rounding::same(6.0),
                Color32::from_rgba_premultiplied(16, 16, 16, 190),
            );
            painter.rect_stroke(
                pill_rect,
                Rounding::same(6.0),
                Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(90, 90, 90, 120)),
            );
            painter.text(
                overlay_pos,
                egui::Align2::CENTER_CENTER,
                overlay_text,
                egui::FontId::proportional(17.0),
                Color32::WHITE,
            );
        });
    }
}
