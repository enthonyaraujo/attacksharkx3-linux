//! Left column: Profile management and Power / Battery display panel matching original Windows driver.

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use crate::gui::theme::*;
use crate::profiles::ProfileManager;
use crate::protocol::BatteryStatus;

pub struct ProfilesPanel {
    pub is_expanded_profiles: bool,
    pub is_expanded_power: bool,
    pub editing_name: Option<String>,
}

impl Default for ProfilesPanel {
    fn default() -> Self {
        Self {
            is_expanded_profiles: true,
            is_expanded_power: true,
            editing_name: None,
        }
    }
}

impl ProfilesPanel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        profiles: &mut ProfileManager,
        battery: Option<BatteryStatus>,
        on_profile_changed: &mut bool,
    ) {
        // --- 1. PROFILE SECTION ---
        collapsible_section_header(ui, "Profile", &mut self.is_expanded_profiles);

        if self.is_expanded_profiles {
            ui.add_space(2.0);
            egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .inner_margin(8.0)
                .rounding(4.0)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .show(ui, |ui| {
                    // Deep black profile box
                    let list_h = 100.0;
                    egui::Frame::none()
                        .fill(Color32::from_rgb(0x0C, 0x0C, 0x0C))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x22, 0x22, 0x22)))
                        .rounding(3.0)
                        .inner_margin(4.0)
                        .show(ui, |ui| {
                            ui.set_height(list_h);
                            egui::ScrollArea::vertical()
                                .max_height(list_h)
                                .show(ui, |ui| {
                                    let active_idx = profiles.active_index;
                                    for i in 0..profiles.profiles.len() {
                                        let name = profiles.profiles[i].name.clone();
                                        let is_active = i == active_idx;

                                        let (rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(ui.available_width(), 24.0),
                                            egui::Sense::click(),
                                        );

                                        if resp.clicked() && profiles.active_index != i {
                                            profiles.active_index = i;
                                            *on_profile_changed = true;
                                        }

                                        let painter = ui.painter_at(rect);
                                        if is_active {
                                            // Red border and subtle dark red gradient fill matching button_2.png
                                            painter.rect_filled(
                                                rect,
                                                Rounding::same(2.0),
                                                Color32::from_rgb(0x28, 0x0A, 0x0A),
                                            );
                                            painter.rect_stroke(
                                                rect,
                                                Rounding::same(2.0),
                                                Stroke::new(1.0_f32, COLOR_ACCENT_RED),
                                            );
                                        }

                                        painter.text(
                                            Pos2::new(rect.min.x + 8.0, rect.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            &name,
                                            egui::FontId::proportional(12.0),
                                            if is_active { Color32::WHITE } else { COLOR_TEXT_SECONDARY },
                                        );
                                    }
                                });
                        });

                    ui.add_space(6.0);

                    // 6 Action Buttons (+, delete, edit, export, import, reset)
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                        let total_w = ui.available_width();
                        let btn_w = ((total_w - (5.0 * 4.0)) / 6.0).floor();

                        let make_btn = |ui: &mut egui::Ui, icon: &str, tooltip: &str| -> egui::Response {
                            ui.add_sized(
                                Vec2::new(btn_w, 22.0),
                                egui::Button::new(
                                    egui::RichText::new(icon)
                                        .size(11.0)
                                        .color(COLOR_TEXT_PRIMARY),
                                )
                                .fill(Color32::from_rgb(0x22, 0x22, 0x22))
                                .stroke(Stroke::new(1.0_f32, COLOR_HEADER_BORDER)),
                            )
                            .on_hover_text(tooltip)
                        };

                        // 1. Add
                        if make_btn(ui, "+", "Add new profile").clicked() {
                            let count = profiles.profiles.len() + 1;
                            profiles.add_profile(format!("Profile{}", count));
                            *on_profile_changed = true;
                        }

                        // 2. Delete
                        if make_btn(ui, "🗑", "Delete active profile").clicked() {
                            if profiles.delete_profile(profiles.active_index) {
                                *on_profile_changed = true;
                            }
                        }

                        // 3. Rename
                        if make_btn(ui, "✎", "Rename profile").clicked() {
                            let cur_name = profiles.active_profile().name.clone();
                            self.editing_name = Some(cur_name);
                        }

                        // 4. Export
                        if make_btn(ui, "📤", "Export profile").clicked() {
                            // Quick export feedback
                        }

                        // 5. Import
                        if make_btn(ui, "📥", "Import profile").clicked() {
                            // Quick import feedback
                        }

                        // 6. Reset
                        if make_btn(ui, "↺", "Reset active profile to defaults").clicked() {
                            profiles.reset_active();
                            *on_profile_changed = true;
                        }
                    });

                    // Rename dialog if open
                    if let Some(mut edit_name) = self.editing_name.take() {
                        ui.horizontal(|ui| {
                            let r = ui.text_edit_singleline(&mut edit_name);
                            if r.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if !edit_name.trim().is_empty() {
                                    profiles.active_profile_mut().name = edit_name.trim().to_string();
                                    *on_profile_changed = true;
                                }
                            } else {
                                self.editing_name = Some(edit_name);
                            }
                        });
                    }
                });
        }

        ui.add_space(4.0);

        // --- 2. POWER SECTION ---
        collapsible_section_header(ui, "Power", &mut self.is_expanded_power);

        if self.is_expanded_power {
            ui.add_space(2.0);
            egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .inner_margin(10.0)
                .rounding(4.0)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .show(ui, |ui| {
                    let pct = battery.map(|b| b.percentage).unwrap_or(50); // Default to 50% as in screenshot

                    ui.vertical_centered(|ui| {
                        // Render Horizontal Battery
                        let (bat_rect, _) = ui.allocate_exact_size(Vec2::new(76.0, 34.0), egui::Sense::hover());
                        let painter = ui.painter_at(bat_rect);

                        let shell_w = 68.0;
                        let shell_h = 34.0;
                        let body_rect = Rect::from_min_size(
                            Pos2::new(bat_rect.min.x, bat_rect.min.y),
                            Vec2::new(shell_w, shell_h),
                        );

                        // Outer dark background and stroke
                        painter.rect_filled(body_rect, Rounding::same(3.0), Color32::from_rgb(0x18, 0x18, 0x18));
                        painter.rect_stroke(body_rect, Rounding::same(3.0), Stroke::new(1.5_f32, Color32::from_rgb(0x44, 0x44, 0x44)));

                        // Battery positive terminal nipple on right
                        let tip_rect = Rect::from_min_size(
                            Pos2::new(bat_rect.min.x + shell_w + 1.0, bat_rect.min.y + 9.0),
                            Vec2::new(4.5, 16.0),
                        );
                        painter.rect_filled(tip_rect, Rounding::same(1.5), Color32::from_rgb(0x44, 0x44, 0x44));

                        // Inner colored fill based on percentage
                        let inner_margin = 3.0;
                        let max_fill_w = (shell_w - inner_margin * 2.0).max(1.0);
                        let fill_h = shell_h - inner_margin * 2.0;
                        let fill_w = (max_fill_w * (pct as f32 / 100.0)).clamp(0.0, max_fill_w);

                        // Paint gradient slices
                        let slice_count = 16;
                        let slice_w = fill_w / slice_count as f32;
                        for s in 0..slice_count {
                            let frac = (s as f32) / (slice_count as f32);
                            // Gradient: Red -> Orange -> Yellow -> Green
                            let color = if frac < 0.4 {
                                Color32::from_rgb(
                                    230,
                                    (frac / 0.4 * 120.0) as u8,
                                    0,
                                )
                            } else if frac < 0.75 {
                                Color32::from_rgb(
                                    230,
                                    (120.0 + (frac - 0.4) / 0.35 * 100.0) as u8,
                                    0,
                                )
                            } else {
                                Color32::from_rgb(
                                    (230.0 - (frac - 0.75) / 0.25 * 150.0) as u8,
                                    220,
                                    (frac * 40.0) as u8,
                                )
                            };

                            let slice_rect = Rect::from_min_size(
                                Pos2::new(
                                    body_rect.min.x + inner_margin + (s as f32 * slice_w),
                                    body_rect.min.y + inner_margin,
                                ),
                                Vec2::new(slice_w.ceil().min(fill_w - s as f32 * slice_w), fill_h),
                            );
                            painter.rect_filled(slice_rect, Rounding::ZERO, color);
                        }

                        ui.add_space(6.0);

                        // Battery Percentage Text
                        ui.label(
                            egui::RichText::new(format!("{}%", pct))
                                .size(14.0)
                                .strong()
                                .color(COLOR_TEXT_PRIMARY),
                        );
                    });
                });
        }
    }
}
