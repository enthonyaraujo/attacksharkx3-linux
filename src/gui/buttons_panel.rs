//! Left column: Buttons mapping configuration panel matching original Windows driver.

use eframe::egui::{self, Color32, Rounding, Stroke, Vec2};
use crate::buttons::{available_actions, ButtonAction, ButtonConfig};
use crate::gui::theme::*;

pub struct ButtonsPanel {
    pub is_expanded: bool,
    pub open_macro_dialog: bool,
}

impl Default for ButtonsPanel {
    fn default() -> Self {
        Self {
            is_expanded: true,
            open_macro_dialog: false,
        }
    }
}

impl ButtonsPanel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        btn_cfg: &mut ButtonConfig,
        selected_button: &mut Option<usize>,
    ) {
        collapsible_section_header(ui, "Button Settings", &mut self.is_expanded);

        if !self.is_expanded {
            return;
        }

        ui.add_space(2.0);

        egui::Frame::none()
            .fill(COLOR_PANEL_BG)
            .inner_margin(8.0)
            .rounding(4.0)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);

                render_button_row(ui, 1, &mut btn_cfg.button1_left, selected_button);
                render_button_row(ui, 2, &mut btn_cfg.button2_right, selected_button);
                render_button_row(ui, 3, &mut btn_cfg.button3_middle, selected_button);
                render_button_row(ui, 4, &mut btn_cfg.button4_forward, selected_button);
                render_button_row(ui, 5, &mut btn_cfg.button5_backward, selected_button);

                ui.add_space(6.0);

                // Macro Manager Button
                let macro_btn = ui.add_sized(
                    Vec2::new(ui.available_width(), 26.0),
                    egui::Button::new(
                        egui::RichText::new("Macro Manager")
                            .color(COLOR_TEXT_PRIMARY)
                            .size(12.0),
                    )
                    .fill(Color32::from_rgb(0x25, 0x25, 0x25))
                    .stroke(Stroke::new(1.0_f32, COLOR_HEADER_BORDER)),
                );

                if macro_btn.clicked() {
                    self.open_macro_dialog = true;
                }
            });
    }
}

fn render_button_row(
    ui: &mut egui::Ui,
    idx: usize,
    action: &mut ButtonAction,
    selected: &mut Option<usize>,
) {
    ui.horizontal(|ui| {
        let is_selected = *selected == Some(idx);

        // Square badge with button index
        let (num_rect, num_resp) = ui.allocate_exact_size(Vec2::new(24.0, 24.0), egui::Sense::click());
        if num_resp.clicked() {
            *selected = Some(idx);
        }
        let painter = ui.painter_at(num_rect);
        painter.rect_filled(
            num_rect,
            Rounding::same(3.0),
            if is_selected { COLOR_ACCENT_RED } else { Color32::from_rgb(0x25, 0x25, 0x25) },
        );
        painter.rect_stroke(
            num_rect,
            Rounding::same(3.0),
            Stroke::new(1.0_f32, if is_selected { COLOR_ACCENT_RED_HOVER } else { COLOR_HEADER_BORDER }),
        );
        painter.text(
            num_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}", idx),
            egui::FontId::proportional(11.5),
            Color32::WHITE,
        );

        // Wide Dropdown Combo Button
        let combo_w = ui.available_width();
        egui::ComboBox::from_id_salt(format!("btn_combo_{}", idx))
            .selected_text(action.label())
            .width(combo_w - 8.0)
            .show_ui(ui, |ui| {
                for &act in available_actions() {
                    ui.selectable_value(action, act, act.label());
                }
            });
    });
}
