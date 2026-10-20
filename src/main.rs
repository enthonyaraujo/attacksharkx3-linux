//! Attack Shark X3 Configuration Driver and GUI Binary Entry Point.

use anyhow::Result;
use attack_shark_x3::AttackSharkApp;
use eframe::egui::Vec2;
use tracing::info;
use winit::platform::x11::EventLoopBuilderExtX11;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting Attack Shark X3 configuration tool v{}", env!("CARGO_PKG_VERSION"));

    let icon_data = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/icon.png")).ok();
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Attack Shark X3")
        .with_app_id("attack-shark-x3")
        .with_decorations(true)
        .with_inner_size(Vec2::new(1127.0, 760.0))
        .with_min_inner_size(Vec2::new(960.0, 680.0));
    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        // Wayland sessions use XWayland: winit cannot hide/restore a Wayland
        // window, and GNOME does not supply server-side Wayland decorations.
        event_loop_builder: Some(Box::new(|builder| {
            builder.with_x11();
        })),
        ..Default::default()
    };

    eframe::run_native(
        "Attack Shark X3",
        native_options,
        Box::new(|cc| Ok(Box::new(AttackSharkApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("Eframe error: {}", e))
}
