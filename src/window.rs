//! Window lifecycle shared by the title bar and StatusNotifierItem callbacks.

use eframe::egui::{Context, ViewportCommand};

use crate::tray::TrayEvent;

pub(crate) struct WindowState {
    pub visible: bool,
    pub should_exit: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            visible: true,
            should_exit: false,
        }
    }
}

impl WindowState {
    pub fn set_visible(&mut self, ctx: &Context, visible: bool) {
        self.visible = visible;
        if visible {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        }
        ctx.send_viewport_cmd(ViewportCommand::Visible(visible));
        if visible {
            ctx.send_viewport_cmd(ViewportCommand::Focus);
        }
    }

    pub fn close_requested(&mut self, ctx: &Context, tray_available: bool) {
        if !self.should_exit && tray_available {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.set_visible(ctx, false);
        } else {
            // A desktop without a tray must still have a working close button.
            self.should_exit = true;
        }
    }

    pub fn tray_event(&mut self, ctx: &Context, event: TrayEvent) {
        match event {
            TrayEvent::ToggleVisible => self.set_visible(ctx, !self.visible),
            TrayEvent::Show => self.set_visible(ctx, true),
            TrayEvent::Hide => self.set_visible(ctx, false),
            TrayEvent::Quit => {
                self.should_exit = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{RawInput, ViewportId};

    fn commands(f: impl FnOnce(&Context)) -> Vec<ViewportCommand> {
        let ctx = Context::default();
        let mut f = Some(f);
        let output = ctx.run(RawInput::default(), |ctx| f.take().unwrap()(ctx));
        output.viewport_output[&ViewportId::ROOT].commands.clone()
    }

    #[test]
    fn close_with_a_tray_hides_instead_of_exiting() {
        let mut state = WindowState::default();
        let output = commands(|ctx| state.close_requested(ctx, true));
        assert!(!state.visible);
        assert!(!state.should_exit);
        assert!(output.contains(&ViewportCommand::CancelClose));
        assert!(output.contains(&ViewportCommand::Visible(false)));
    }

    #[test]
    fn close_without_a_tray_allows_the_process_to_exit() {
        let mut state = WindowState::default();
        let output = commands(|ctx| state.close_requested(ctx, false));
        assert!(state.should_exit);
        assert!(!output.contains(&ViewportCommand::CancelClose));
        assert!(!output.contains(&ViewportCommand::Visible(false)));
    }

    #[test]
    fn restore_unminimizes_before_showing_and_focusing() {
        let mut state = WindowState {
            visible: false,
            should_exit: false,
        };
        let output = commands(|ctx| state.tray_event(ctx, TrayEvent::Show));
        assert!(state.visible);
        assert_eq!(output, vec![
            ViewportCommand::Minimized(false),
            ViewportCommand::Visible(true),
            ViewportCommand::Focus,
        ]);
    }

    #[test]
    fn quit_from_the_tray_is_not_cancelled_by_close_handling() {
        let mut state = WindowState::default();
        let output = commands(|ctx| {
            state.tray_event(ctx, TrayEvent::Quit);
            state.close_requested(ctx, true);
        });
        assert!(state.should_exit);
        assert!(output.contains(&ViewportCommand::Close));
        assert!(!output.contains(&ViewportCommand::CancelClose));
    }
}
