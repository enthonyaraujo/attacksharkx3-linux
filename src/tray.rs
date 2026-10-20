//! System Tray integration using KDE/freedesktop StatusNotifierItem (ksni).

use eframe::egui;
use std::sync::mpsc::Sender;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use ksni::blocking::{Handle, TrayMethods};
use ksni::{menu::*, Icon, ToolTip, Tray};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    ToggleVisible,
    Show,
    Hide,
    Quit,
}

pub struct AttackSharkTray {
    pub tx: Sender<TrayEvent>,
    pub egui_ctx: egui::Context,
    pub status_text: String,
    pub battery_text: String,
    pub is_visible: bool,
    icon: Option<Icon>,
    online: Arc<AtomicBool>,
}

pub struct TrayService {
    pub handle: Handle<AttackSharkTray>,
    online: Arc<AtomicBool>,
}

impl TrayService {
    pub fn is_available(&self) -> bool {
        self.online.load(Ordering::Acquire) && !self.handle.is_closed()
    }
}

impl AttackSharkTray {
    pub fn new(tx: Sender<TrayEvent>, egui_ctx: egui::Context) -> Self {
        let icon = load_tray_icon();
        Self {
            tx,
            egui_ctx,
            status_text: "Desconectado".to_string(),
            battery_text: "".to_string(),
            is_visible: true,
            icon,
            online: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn spawn_service(tx: Sender<TrayEvent>, egui_ctx: egui::Context) -> Option<TrayService> {
        let tray = Self::new(tx, egui_ctx);
        let online = Arc::clone(&tray.online);
        match tray.spawn() {
            Ok(handle) => {
                tracing::info!("System tray initialized successfully");
                Some(TrayService { handle, online })
            }
            Err(e) => {
                tracing::warn!("Could not initialize system tray: {}", e);
                None
            }
        }
    }
}

fn load_tray_icon() -> Option<Icon> {
    let bytes = include_bytes!("../assets/icons/icon.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = (img.width() as i32, img.height() as i32);
    // Convert RGBA to ARGB network-byte-order (Big Endian)
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for pixel in img.pixels() {
        let [r, g, b, a] = pixel.0;
        data.push(a);
        data.push(r);
        data.push(g);
        data.push(b);
    }
    Some(Icon {
        width,
        height,
        data,
    })
}

impl Tray for AttackSharkTray {
    fn watcher_online(&self) {
        self.online.store(true, Ordering::Release);
        self.egui_ctx.request_repaint();
    }

    fn watcher_offline(&self, reason: ksni::OfflineReason) -> bool {
        tracing::warn!("System tray host unavailable: {reason:?}");
        self.online.store(false, Ordering::Release);
        // Never leave a hidden application stranded when the desktop tray disappears.
        let _ = self.tx.send(TrayEvent::Show);
        self.egui_ctx.request_repaint();
        true
    }

    fn id(&self) -> String {
        "attack-shark-x3".to_string()
    }

    fn title(&self) -> String {
        "Attack Shark X3".to_string()
    }

    fn icon_name(&self) -> String {
        "attack-shark-x3".to_string()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        if let Some(ref icon) = self.icon {
            vec![icon.clone()]
        } else {
            vec![]
        }
    }

    fn tool_tip(&self) -> ToolTip {
        let desc = if self.battery_text.is_empty() {
            self.status_text.clone()
        } else {
            format!("{} ({})", self.status_text, self.battery_text)
        };
        ToolTip {
            title: "Attack Shark X3".to_string(),
            description: desc,
            icon_name: "attack-shark-x3".to_string(),
            icon_pixmap: vec![],
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayEvent::ToggleVisible);
        self.egui_ctx.request_repaint();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let tx_show = self.tx.clone();
        let tx_hide = self.tx.clone();
        let tx_quit = self.tx.clone();
        let ctx_show = self.egui_ctx.clone();
        let ctx_hide = self.egui_ctx.clone();
        let ctx_quit = self.egui_ctx.clone();

        let status_label = if self.battery_text.is_empty() {
            format!("Status: {}", self.status_text)
        } else {
            format!("Status: {} | {}", self.status_text, self.battery_text)
        };

        vec![
            StandardItem {
                label: "Attack Shark X3".into(),
                enabled: false,
                ..Default::default()
            }.into(),
            StandardItem {
                label: status_label,
                enabled: false,
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            if self.is_visible {
                StandardItem {
                    label: "Ocultar para a bandeja".into(),
                    activate: Box::new(move |_| {
                        let _ = tx_hide.send(TrayEvent::Hide);
                        ctx_hide.request_repaint();
                    }),
                    ..Default::default()
                }.into()
            } else {
                StandardItem {
                    label: "Abrir Attack Shark X3".into(),
                    activate: Box::new(move |_| {
                        let _ = tx_show.send(TrayEvent::Show);
                        ctx_show.request_repaint();
                    }),
                    ..Default::default()
                }.into()
            },
            MenuItem::Separator,
            StandardItem {
                label: "Sair".into(),
                activate: Box::new(move |_| {
                    let _ = tx_quit.send(TrayEvent::Quit);
                    ctx_quit.request_repaint();
                }),
                ..Default::default()
            }.into(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn losing_the_tray_host_restores_the_window() {
        let (tx, rx) = channel();
        let tray = AttackSharkTray::new(tx, egui::Context::default());
        assert!(tray.watcher_offline(ksni::OfflineReason::No));
        assert_eq!(rx.try_recv().ok(), Some(TrayEvent::Show));
    }
}
