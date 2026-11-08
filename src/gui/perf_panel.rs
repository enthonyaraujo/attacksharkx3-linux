//! Right column: Collapsible accordion panels below DPI settings matching the original Windows driver.
//! Includes Polling Rate, Mouse Attribute, Power Management, Lift Of Distance,
//! Key Response Time, Ripple Control, Angle Snap, and Motion Sync.

use eframe::egui::{self, Color32, Stroke, Vec2};
use crate::gui::theme::*;
use crate::protocol::{
    constants::{LiftOffDistance, PollingRate},
    power::PowerConfig,
    sensor::SensorConfig,
};

pub struct PerfPanel {
    pub expanded_polling: bool,
    pub expanded_attribute: bool,
    pub expanded_power: bool,
    pub expanded_lod: bool,
    pub expanded_debounce: bool,
    pub expanded_ripple: bool,
    pub expanded_angle_snap: bool,
    pub expanded_motion_sync: bool,
}

impl Default for PerfPanel {
    fn default() -> Self {
        Self {
            expanded_polling: false,
            expanded_attribute: false,
            expanded_power: false,
            expanded_lod: false,
            expanded_debounce: false,
            expanded_ripple: false,
            expanded_angle_snap: false,
            expanded_motion_sync: false,
        }
    }
}

impl PerfPanel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        sensor: &mut SensorConfig,
        power: &mut PowerConfig,
        polling_rate: &mut PollingRate,
        pending_apply: &mut bool,
    ) {
        ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);

        // 1. Polling Rate Settings
        collapsible_section_header(ui, "Polling Rate Settings", &mut self.expanded_polling);
        if self.expanded_polling {
            egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .inner_margin(8.0)
                .rounding(4.0)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for &rate in &[
                            PollingRate::Hz125,
                            PollingRate::Hz250,
                            PollingRate::Hz500,
                            PollingRate::Hz1000,
                        ] {
                            let is_selected = *polling_rate == rate;
                            let btn = ui.add_sized(
                                Vec2::new(52.0, 22.0),
                                egui::Button::new(
                                    egui::RichText::new(format!("{}Hz", rate as u16))
                                        .color(if is_selected { Color32::WHITE } else { COLOR_TEXT_SECONDARY })
                                        .size(11.0),
                                )
                                .fill(if is_selected { COLOR_ACCENT_RED } else { COLOR_CARD_BG }),
                            );
                            if btn.clicked() {
                                *polling_rate = rate;
                                *pending_apply = true;
                            }
                        }
                    });
                });
        }

        // 2. Mouse Attribute
        collapsible_section_header(ui, "Mouse Attribute", &mut self.expanded_attribute);
        if self.expanded_attribute {
            egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .inner_margin(8.0)
                .rounding(4.0)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("PAW3395 Optical Sensor (26,000 DPI, 650 IPS, 50G)")
                            .color(COLOR_TEXT_SECONDARY)
                            .size(11.0),
                    );
                });
        }

        // 3. Power Management
        collapsible_section_header(ui, "Power Management", &mut self.expanded_power);
        if self.expanded_power {
            egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .inner_margin(8.0)
                .rounding(4.0)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Sleep:").size(11.5));
                        for &mins in &[1.0_f32, 3.0, 5.0, 10.0] {
                            let is_sel = (power.sleep_minutes - mins).abs() < 0.1;
                            let btn = ui.add_sized(
                                Vec2::new(46.0, 20.0),
                                egui::Button::new(
                                    egui::RichText::new(format!("{:.0}m", mins))
                                        .size(10.5)
                                        .color(if is_sel { Color32::WHITE } else { COLOR_TEXT_SECONDARY }),
                                )
                                .fill(if is_sel { COLOR_ACCENT_RED } else { COLOR_CARD_BG }),
                            );
                            if btn.clicked() {
                                power.sleep_minutes = mins;
                                *pending_apply = true;
                            }
                        }
                    });
                });
        }

        // 4. Lift Of Distance
        collapsible_section_header(ui, "Lift Of Distance", &mut self.expanded_lod);
        if self.expanded_lod {
            egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .inner_margin(8.0)
                .rounding(4.0)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.radio(sensor.lod == LiftOffDistance::OneMm, "1 mm").clicked() {
                            sensor.lod = LiftOffDistance::OneMm;
                            *pending_apply = true;
                        }
                        if ui.radio(sensor.lod == LiftOffDistance::TwoMm, "2 mm").clicked() {
                            sensor.lod = LiftOffDistance::TwoMm;
                            *pending_apply = true;
                        }
                    });
                });
        }

        // 5. Key Response Time
        collapsible_section_header(ui, "Key Response Time", &mut self.expanded_debounce);
        if self.expanded_debounce {
            egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .inner_margin(8.0)
                .rounding(4.0)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Debounce:").size(11.5));
                        let mut val = power.key_debounce_ms as f64;
                        if ui
                            .add(egui::Slider::new(&mut val, 2.0..=50.0).step_by(2.0).suffix(" ms"))
                            .changed()
                        {
                            power.key_debounce_ms = val as u8;
                            *pending_apply = true;
                        }
                    });
                });
        }

        // 6. Ripple Control
        collapsible_section_header(ui, "Ripple Control", &mut self.expanded_ripple);
        if self.expanded_ripple {
            egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .inner_margin(8.0)
                .rounding(4.0)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .show(ui, |ui| {
                    if ui.checkbox(&mut sensor.ripple_control, "Enable Ripple Control").changed() {
                        *pending_apply = true;
                    }
                });
        }

        // 7. Angle Snap
        collapsible_section_header(ui, "Angle Snap", &mut self.expanded_angle_snap);
        if self.expanded_angle_snap {
            egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .inner_margin(8.0)
                .rounding(4.0)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .show(ui, |ui| {
                    if ui.checkbox(&mut sensor.angle_snap, "Enable Angle Snap").changed() {
                        *pending_apply = true;
                    }
                });
        }

        // 8. Motion Sync
        collapsible_section_header(ui, "Motion Sync", &mut self.expanded_motion_sync);
        if self.expanded_motion_sync {
            egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .inner_margin(8.0)
                .rounding(4.0)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .show(ui, |ui| {
                    if ui.checkbox(&mut sensor.motion_sync, "Enable Motion Sync").changed() {
                        *pending_apply = true;
                    }
                });
        }
    }
}
