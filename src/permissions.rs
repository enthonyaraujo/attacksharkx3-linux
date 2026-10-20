//! First-run permission setup for portable packages, through the system's polkit agent.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, TryRecvError};

use eframe::egui::Context;

#[derive(Default)]
pub(crate) struct PermissionSetup {
    attempted: bool,
    result: Option<Receiver<Result<(), String>>>,
    pub message: Option<String>,
}

impl PermissionSetup {
    pub fn is_running(&self) -> bool {
        self.result.is_some()
    }

    pub fn retry(&mut self) {
        self.attempted = false;
    }

    pub fn request(&mut self, ctx: &Context) {
        if self.attempted || self.is_running() {
            return;
        }
        self.attempted = true;
        let helper = std::env::current_exe().ok().and_then(|exe| {
            Some(exe.parent()?.parent()?.join("libexec/attack-shark-x3-install-udev"))
        });
        let Some(helper) = helper.filter(|p| p.is_file()) else {
            self.message = Some("Install a Linux package to enable automatic mouse access setup.".into());
            return;
        };
        let (tx, rx) = channel();
        self.result = Some(rx);
        self.message = Some("Authenticate in the system dialog to enable access to the mouse.".into());
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(install_permissions(helper));
            ctx.request_repaint();
        });
    }

    /// Returns true after the system has applied the rules, so detection can retry.
    pub fn poll(&mut self) -> bool {
        let Some(rx) = &self.result else { return false; };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err("Permission setup stopped unexpectedly. Please try again.".into()),
        };
        self.result = None;
        match result {
            Ok(()) => {
                self.message = None;
                true
            }
            Err(message) => {
                self.message = Some(message);
                false
            }
        }
    }
}

fn install_permissions(helper: PathBuf) -> Result<(), String> {
    // A root process cannot read a normal user's FUSE AppImage mount. Stage
    // both files in a private directory on the regular filesystem first, and
    // retain it until pkexec exits. No fuse.conf changes or root GUI required.
    let staged = stage_permissions(&helper)
        .map_err(|_| "Could not prepare the mouse access helper.".to_string())?;
    let status = Command::new("pkexec")
        .arg(staged.path().join("libexec/attack-shark-x3-install-udev"))
        .status()
        .map_err(|_| "The system authentication service (pkexec) is unavailable.".to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Mouse access was not configured. Grant access to try again.".into())
    }
}

fn stage_permissions(helper: &Path) -> std::io::Result<tempfile::TempDir> {
    let staged = tempfile::Builder::new()
        .prefix("attack-shark-x3-permissions-")
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()?;
    let share = staged.path().join("share/attack-shark-x3");
    let libexec = staged.path().join("libexec");
    fs::create_dir_all(&share)?;
    fs::create_dir_all(&libexec)?;
    let executable = libexec.join("attack-shark-x3-install-udev");
    fs::copy(helper, &executable)?;
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))?;
    let source = helper.parent().and_then(Path::parent)
        .ok_or_else(|| std::io::Error::other("Invalid permission helper location"))?
        .join("share/attack-shark-x3/70-attack-shark-x3.rules");
    fs::copy(source, share.join("70-attack-shark-x3.rules"))?;
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_helper_reads_its_rules_after_the_original_mount_disappears() {
        let original = tempfile::tempdir().unwrap();
        let libexec = original.path().join("libexec");
        let share = original.path().join("share/attack-shark-x3");
        fs::create_dir_all(&libexec).unwrap();
        fs::create_dir_all(&share).unwrap();
        let helper = libexec.join("attack-shark-x3-install-udev");
        fs::write(&helper, "#!/bin/sh\ncat \"$(dirname \"$0\")/../share/attack-shark-x3/70-attack-shark-x3.rules\"\n").unwrap();
        fs::write(share.join("70-attack-shark-x3.rules"), "test device rules\n").unwrap();
        let staged = stage_permissions(&helper).unwrap();
        assert_eq!(fs::metadata(staged.path()).unwrap().permissions().mode() & 0o777, 0o700);
        drop(original);
        let output = Command::new(staged.path().join("libexec/attack-shark-x3-install-udev"))
            .output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"test device rules\n");
    }

    #[test]
    fn completed_setup_requests_device_redetection_once() {
        let (tx, rx) = channel();
        let mut setup = PermissionSetup { result: Some(rx), ..Default::default() };
        tx.send(Ok(())).unwrap();
        assert!(setup.poll());
        assert!(!setup.is_running());
        assert!(!setup.poll());
    }

    #[test]
    fn cancelled_setup_stops_waiting_and_preserves_the_error() {
        let (tx, rx) = channel();
        let mut setup = PermissionSetup { result: Some(rx), ..Default::default() };
        tx.send(Err("Authentication cancelled".into())).unwrap();
        assert!(!setup.poll());
        assert!(!setup.is_running());
        assert_eq!(setup.message.as_deref(), Some("Authentication cancelled"));
    }
}
