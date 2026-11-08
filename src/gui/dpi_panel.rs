//! Right column: DPI Settings panel matching the official Attack Shark X3 Windows driver.
//! Features 6 vertical DPI sliders, editable direct input fields (e.g. click 800 -> type 900 -> Apply),
//! stage radio indicators, and the centered Apply button.

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use crate::dpi::clamp_dpi;
use crate::gui::theme::*;
use crate::protocol::sensor::SensorConfig;

pub struct DpiPanel {
    pub is_expanded: bool,
    pub pending_apply: bool,
    /// Buffers for each stage's text input field
    pub text_buffers: Vec<String>,
}

impl Default for DpiPanel {
    fn default() -> Self {
        Self {
            is_expanded: true,
            pending_apply: false,
            text_buffers: Vec::new(),
        }
    }
}

impl DpiPanel {
    fn sync_buffers(&mut self, sensor: &SensorConfig) {
        if self.text_buffers.len() != sensor.dpi_stages.len() {
            self.text_buffers = sensor
                .dpi_stages
                .iter()
                .map(|&dpi| format!("{}", dpi))
                .collect();
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, sensor: &mut SensorConfig) {
        self.sync_buffers(sensor);

        // Header matching original Attack Shark title bar
        collapsible_section_header(ui, "DPI Settings", &mut self.is_expanded);

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
                let stage_count = sensor.dpi_stages.len().min(6);
                let avail_w = ui.available_width();
                let col_w = (avail_w / stage_count as f32).floor();
                let slider_h = 120.0;
                let track_w = 7.0;
                let thumb_radius = 6.5;

                // 6 Columns for DPI Stages
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);

                    for i in 0..stage_count {
                        let is_active = sensor.active_stage == i;
                        let current_dpi = sensor.dpi_stages[i];

                        ui.vertical(|ui| {
                            ui.set_width(col_w);

                            // 1. Top label: 26000
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("26000")
                                        .size(9.5)
                                        .color(COLOR_TEXT_MUTED),
                                );
                            });

                            ui.add_space(3.0);

                            // 2. Custom Vertical Slider
                            let (slider_rect, slider_resp) = ui.allocate_exact_size(
                                Vec2::new(col_w, slider_h),
                                egui::Sense::click_and_drag(),
                            );

                            let painter = ui.painter_at(slider_rect);
                            let track_x = slider_rect.center().x;
                            let track_top = slider_rect.min.y + thumb_radius;
                            let track_bottom = slider_rect.max.y - thumb_radius;
                            let usable_track_h = (track_bottom - track_top).max(1.0);

                            // Normalized value (0.0 at bottom = 50 DPI, 1.0 at top = 26000 DPI)
                            let mut norm = ((current_dpi as f32 - 50.0) / (26000.0 - 50.0)).clamp(0.0, 1.0);

                            if slider_resp.dragged() || slider_resp.clicked() {
                                if let Some(pointer_pos) = slider_resp.interact_pointer_pos() {
                                    let new_norm = 1.0 - ((pointer_pos.y - track_top) / usable_track_h).clamp(0.0, 1.0);
                                    let calc_dpi = 50.0 + new_norm * (26000.0 - 50.0);
                                    let clamped = clamp_dpi(calc_dpi.round() as u32);
                                    sensor.dpi_stages[i] = clamped;
                                    if let Some(buf) = self.text_buffers.get_mut(i) {
                                        *buf = format!("{}", clamped);
                                    }
                                    norm = ((clamped as f32 - 50.0) / (26000.0 - 50.0)).clamp(0.0, 1.0);
                                }
                            }

                            // Thumb Y position
                            let thumb_y = track_bottom - (norm * usable_track_h);

                            // Draw full track background
                            let track_rect = Rect::from_min_max(
                                Pos2::new(track_x - track_w / 2.0, track_top),
                                Pos2::new(track_x + track_w / 2.0, track_bottom),
                            );
                            painter.rect_filled(
                                track_rect,
                                Rounding::same(track_w / 2.0),
                                Color32::from_rgb(0x28, 0x28, 0x28),
                            );

                            // Draw active red filled track from bottom up to thumb
                            let fill_rect = Rect::from_min_max(
                                Pos2::new(track_x - track_w / 2.0, thumb_y),
                                Pos2::new(track_x + track_w / 2.0, track_bottom),
                            );
                            painter.rect_filled(
                                fill_rect,
                                Rounding::same(track_w / 2.0),
                                COLOR_ACCENT_RED,
                            );

                            // Draw white circular thumb knob
                            let thumb_center = Pos2::new(track_x, thumb_y);
                            painter.circle_filled(
                                thumb_center,
                                thumb_radius,
                                Color32::from_rgb(0xEA, 0xEA, 0xEA),
                            );
                            painter.circle_stroke(
                                thumb_center,
                                thumb_radius,
                                Stroke::new(1.0_f32, Color32::from_rgb(0x88, 0x88, 0x88)),
                            );

                            ui.add_space(3.0);

                            // 3. Bottom label: 50
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("50")
                                        .size(9.5)
                                        .color(COLOR_TEXT_MUTED),
                                );
                            });

                            ui.add_space(4.0);

                            // 4. Editable DPI Text Box
                            // The user can click directly and type any value (e.g. 800 -> 900)
                            ui.vertical_centered(|ui| {
                                if let Some(buf) = self.text_buffers.get_mut(i) {
                                    let box_w = (col_w - 6.0).clamp(32.0, 46.0);
                                    let edit_resp = ui.add_sized(
                                        Vec2::new(box_w, 20.0),
                                        egui::TextEdit::singleline(buf)
                                            .font(egui::FontId::proportional(11.0))
                                            .desired_width(box_w)
                                            .horizontal_align(egui::Align::Center)
                                            .vertical_align(egui::Align::Center),
                                    );

                                    let is_focused = edit_resp.has_focus();
                                    let enter_pressed = is_focused && ui.input(|inp| inp.key_pressed(egui::Key::Enter));

                                    if edit_resp.lost_focus() || enter_pressed {
                                        if let Ok(val) = buf.trim().parse::<u32>() {
                                            let clamped = clamp_dpi(val);
                                            sensor.dpi_stages[i] = clamped;
                                            *buf = format!("{}", clamped);
                                        } else {
                                            *buf = format!("{}", sensor.dpi_stages[i]);
                                        }
                                    } else if !is_focused {
                                        // Keep synchronized when not typing
                                        *buf = format!("{}", sensor.dpi_stages[i]);
                                    }
                                }
                            });

                            ui.add_space(5.0);

                            // 5. Radio Indicator Dot
                            // Clicking the dot selects this stage as active DPI
                            ui.vertical_centered(|ui| {
                                let (dot_rect, dot_resp) = ui.allocate_exact_size(
                                    Vec2::new(col_w, 14.0),
                                    egui::Sense::click(),
                                );

                                if dot_resp.clicked() {
                                    sensor.active_stage = i;
                                }

                                let dot_painter = ui.painter_at(dot_rect);
                                let dot_center = dot_rect.center();

                                if is_active {
                                    // Solid bright red active dot with subtle glow
                                    dot_painter.circle_filled(
                                        dot_center,
                                        5.5,
                                        Color32::from_rgba_premultiplied(226, 33, 28, 70),
                                    );
                                    dot_painter.circle_filled(
                                        dot_center,
                                        4.5,
                                        COLOR_ACCENT_RED,
                                    );
                                } else {
                                    // Inactive dark charcoal dot with subtle border
                                    dot_painter.circle_filled(
                                        dot_center,
                                        4.0,
                                        Color32::from_rgb(0x1E, 0x1E, 0x1E),
                                    );
                                    dot_painter.circle_stroke(
                                        dot_center,
                                        4.0,
                                        Stroke::new(1.0_f32, Color32::from_rgb(0x35, 0x35, 0x35)),
                                    );
                                }
                            });
                        });
                    }
                });

                ui.add_space(10.0);

                // 6. Centered Apply Button
                ui.vertical_centered(|ui| {
                    let apply_btn = ui.add_sized(
                        Vec2::new(72.0, 24.0),
                        egui::Button::new(
                            egui::RichText::new("Apply")
                                .color(COLOR_TEXT_PRIMARY)
                                .size(11.5),
                        )
                        .fill(Color32::from_rgb(0x2A, 0x2A, 0x2A))
                        .stroke(Stroke::new(1.0_f32, COLOR_HEADER_BORDER)),
                    );

                    if apply_btn.clicked() {
                        // Commit any text box inputs
                        for (idx, buf) in self.text_buffers.iter_mut().enumerate() {
                            if let Ok(val) = buf.trim().parse::<u32>() {
                                let clamped = clamp_dpi(val);
                                if idx < sensor.dpi_stages.len() {
                                    sensor.dpi_stages[idx] = clamped;
                                }
                                *buf = format!("{}", clamped);
                            }
                        }
                        self.pending_apply = true;
                    }
                });
            });
    }
}
