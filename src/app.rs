//! Main eframe application coordinating GUI and device communication.

use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};
use tracing::error;

use crate::config::AppConfig;
use crate::device::DeviceManager;
use crate::gui::*;
use crate::protocol::ApplyBurst;
use crate::tray::{AttackSharkTray, TrayEvent, TrayService};
use crate::window::WindowState;

pub struct AttackSharkApp {
    config: AppConfig,
    device: DeviceManager,
    buttons_panel: ButtonsPanel,
    profiles_panel: ProfilesPanel,
    mouse_view: MouseView,
    dpi_panel: DpiPanel,
    perf_panel: PerfPanel,
    macro_dialog: MacroDialog,

    logo_texture: Option<egui::TextureHandle>,
    last_poll_time: Instant,
    status_message: Option<(String, Instant)>,

    tray: Option<TrayService>,
    tray_rx: Receiver<TrayEvent>,
    window: WindowState,
}

impl AttackSharkApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        theme::apply_theme(&cc.egui_ctx);
        let config = AppConfig::load();
        let mut device = DeviceManager::new();
        device.refresh_connection();

        let (tray_tx, tray_rx) = channel();
        let tray = AttackSharkTray::spawn_service(tray_tx, cc.egui_ctx.clone());

        let app = Self {
            config,
            device,
            buttons_panel: ButtonsPanel::default(),
            profiles_panel: ProfilesPanel::default(),
            mouse_view: MouseView::default(),
            dpi_panel: DpiPanel::default(),
            perf_panel: PerfPanel::default(),
            macro_dialog: MacroDialog::default(),
            logo_texture: None,
            last_poll_time: Instant::now(),
            status_message: Some(("Ready".into(), Instant::now())),
            tray,
            tray_rx,
            window: WindowState::default(),
        };
        app.sync_tray_state();
        app
    }

    fn sync_tray_state(&self) {
        if let Some(ref tray) = self.tray {
            let status = self.device.connection_type.label().to_string();
            let battery = self
                .device
                .battery_status
                .as_ref()
                .map(|b| format!("{}%", b.percentage))
                .unwrap_or_default();
            let visible = self.window.visible;
            let _ = tray.handle.update(|tray| {
                tray.status_text = status;
                tray.battery_text = battery;
                tray.is_visible = visible;
            });
        }
    }

    fn ensure_logo(&mut self, ctx: &egui::Context) -> Option<&egui::TextureHandle> {
        if self.logo_texture.is_none() {
            let bytes = include_bytes!("../assets/images/attack_shark_logo.png");
            if let Ok(color_img) = load_color_image_from_bytes(bytes) {
                self.logo_texture = Some(ctx.load_texture(
                    "attack_shark_logo_header",
                    color_img,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        self.logo_texture.as_ref()
    }

    fn set_status(&mut self, text: impl Into<String>) {
        self.status_message = Some((text.into(), Instant::now()));
    }

    fn apply_current_configuration(&mut self) {
        let profile = self.config.profiles.active_profile();
        match ApplyBurst::build(
            &profile.sensor,
            &profile.power,
            profile.polling_rate,
            &profile.buttons,
        ) {
            Ok(burst) => match self.device.send_burst(&burst) {
                Ok(_) => {
                    self.set_status("Settings applied successfully to mouse!");
                    let _ = self.config.save();
                }
                Err(e) => {
                    error!("Failed to apply settings: {}", e);
                    self.set_status(format!("Error: {}", e));
                }
            },
            Err(e) => {
                error!("Packet generation error: {}", e);
                self.set_status(format!("Config error: {}", e));
            }
        }
    }
}

impl eframe::App for AttackSharkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.tray_rx.try_recv() {
            self.window.tray_event(ctx, event);
            self.sync_tray_state();
        }
        if self.window.should_exit {
            return;
        }

        let tray_available = self.tray.as_ref().is_some_and(TrayService::is_available);
        if !tray_available && !self.window.visible {
            self.window.set_visible(ctx, true);
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.window.close_requested(ctx, tray_available);
            self.sync_tray_state();
        }
        // Schedule before polling: an idle first frame must still start the timer.
        ctx.request_repaint_after(Duration::from_secs(1));

        // Periodic background poll every 2 seconds
        if self.last_poll_time.elapsed() >= Duration::from_secs(2) {
            self.last_poll_time = Instant::now();
            self.device.refresh_connection();
            self.device.poll_battery();
            self.sync_tray_state();
            ctx.request_repaint_after(Duration::from_millis(500));
        }

        let mut trigger_apply = false;
        let mut profile_switched = false;
        let logo_opt = self.ensure_logo(ctx).cloned();

        // Top Header Bar matching official software
        egui::TopBottomPanel::top("header_bar")
            .frame(
                egui::Frame::none()
                    .fill(COLOR_BG_DARK)
                    .inner_margin(egui::Margin::symmetric(16.0, 10.0)),
            )
            .show(ctx, |ui| {
                let avail_w = ui.available_width();
                let header_h = 36.0;

                let (header_rect, header_resp) = ui.allocate_exact_size(
                    Vec2::new(avail_w, header_h),
                    egui::Sense::click_and_drag(),
                );

                if header_resp.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                let painter = ui.painter_at(header_rect);

                // Draw Attack Shark Logo in the top-center
                if let Some(logo) = logo_opt {
                    let logo_w = 140.0;
                    let logo_h = 35.0;
                    let logo_rect = Rect::from_center_size(
                        Pos2::new(header_rect.center().x, header_rect.center().y),
                        Vec2::new(logo_w, logo_h),
                    );
                    painter.image(
                        logo.id(),
                        logo_rect,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            });

        // Bottom Status Notification (if any)
        if let Some((msg, created)) = &self.status_message {
            if created.elapsed() < Duration::from_secs(5) {
                egui::TopBottomPanel::bottom("status_bar")
                    .frame(egui::Frame::none().fill(COLOR_BG_DARK).inner_margin(4.0))
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(msg).color(COLOR_TEXT_SECONDARY).size(11.0));
                        });
                    });
            }
        }

        // Central Panel matching the 3-column layout of the screenshot
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(COLOR_BG_DARK))
            .show(ctx, |ui| {
                let total_w = ui.available_width();
                let col_gap = 18.0;
                let panel_w = 262.0; // Exact width of side panels
                let content_w = panel_w * 2.0 + 380.0 + col_gap * 2.0;
                let margin_x = ((total_w - content_w) / 2.0).max(12.0);

                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if margin_x > 0.0 {
                                ui.add_space(margin_x);
                            }

                            // LEFT COLUMN: Button Settings, Profile, Power
                            ui.vertical(|ui| {
                                ui.set_width(panel_w);
                                let profile = self.config.profiles.active_profile_mut();
                                self.buttons_panel.show(
                                    ui,
                                    &mut profile.buttons,
                                    &mut self.mouse_view.selected_button,
                                );

                                ui.add_space(8.0);
                                self.profiles_panel.show(
                                    ui,
                                    &mut self.config.profiles,
                                    self.device.battery_status,
                                    &mut profile_switched,
                                );
                            });

                            ui.add_space(col_gap);

                            // CENTER COLUMN: Official Mouse Image + Device Sleep overlay
                            ui.vertical(|ui| {
                                let center_w = (total_w - (panel_w * 2.0) - (col_gap * 2.0) - (margin_x * 2.0)).clamp(300.0, 480.0);
                                ui.set_width(center_w);
                                self.mouse_view.show(ui, self.device.connection_type);
                            });

                            ui.add_space(col_gap);

                            // RIGHT COLUMN: DPI Settings & Collapsible Accordion Panels
                            ui.vertical(|ui| {
                                ui.set_width(panel_w);
                                let profile = self.config.profiles.active_profile_mut();
                                self.dpi_panel.show(ui, &mut profile.sensor);
                                if self.dpi_panel.pending_apply {
                                    self.dpi_panel.pending_apply = false;
                                    trigger_apply = true;
                                }

                                ui.add_space(6.0);
                                self.perf_panel.show(
                                    ui,
                                    &mut profile.sensor,
                                    &mut profile.power,
                                    &mut profile.polling_rate,
                                    &mut trigger_apply,
                                );
                            });

                            if margin_x > 0.0 {
                                ui.add_space(margin_x);
                            }
                        });
                    });
            });

        // Macro Manager Dialog Modal
        if self.buttons_panel.open_macro_dialog {
            self.macro_dialog.show(
                ctx,
                &mut self.buttons_panel.open_macro_dialog,
                &mut self.config.macros,
                &mut self.device,
            );
        }

        if profile_switched {
            let _ = self.config.save();
            self.apply_current_configuration();
        } else if trigger_apply {
            self.apply_current_configuration();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.config.save();
        if let Some(tray) = self.tray.take() {
            tray.handle.shutdown().wait();
        }
    }
}
