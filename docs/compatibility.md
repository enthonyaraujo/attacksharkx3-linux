# Linux desktop and hardware compatibility

## Supported configuration paths

- Attack Shark X3 USB receiver: VID `1d57`, PID `fa60`.
- Attack Shark X3 wired USB: VID `1d57`, PID `fa61`.
- Bluetooth configuration is not implemented by the HID device manager.

## Desktop integration

The application explicitly uses X11. On GNOME and KDE Wayland sessions this
runs through XWayland. The window manager draws the title bar and its controls,
following the desktop's theme and button placement. This avoids winit 0.30's
generic Wayland client frame and its unsupported window visibility operation.

The tray uses the freedesktop StatusNotifierItem protocol. With an available
host, close hides the window and the tray restores it; Quit exits. If the host
disappears, the window is restored. With no host, close exits normally instead
of leaving an inaccessible background process. GNOME sessions without SNI
support follow this fallback; no Shell extension is installed or enabled by the
application. Pure Wayland sessions without XWayland are outside this build's scope.

References: [winit window support](https://docs.rs/winit/0.30.13/winit/window/struct.Window.html)
and [GNOME status icon support](https://discourse.gnome.org/t/status-of-status-icon-in-gnome-shell/6441).

## Distribution packages

| Format | Intended systems | Installation behavior |
| --- | --- | --- |
| DEB | Debian / Ubuntu and compatible derivatives | Resolves runtime dependencies through the package manager; installs and reloads udev rules |
| RPM | Fedora and RPM systems providing the declared SONAME/file capabilities | Declares graphical libraries explicitly and scans ELF dependencies; installs and reloads udev rules |
| Arch package | Arch Linux and compatible derivatives | Declares pacman dependencies; applies udev permissions through install hooks |
| AppImage | glibc-based Linux desktops matching the build baseline | Bundles application libraries; offers system authentication for first-run device permissions |

The portable build uses Ubuntu 22.04 / glibc 2.35. Native local builds record
their actual ABI requirement, which can be newer. AppImage packaging checks
every bundled library against the baseline and fails if it is too new. An
AppImage is not a way to run a newer glibc binary on an older system.

Standard desktop prerequisites are X11/XWayland, a working graphics driver,
udev and logind managing the active local seat. AppImage's system-level
prerequisites also include a polkit agent / `pkexec` for permission setup and
FUSE for normal mounted execution; extract-and-run is available when FUSE is
not present. These prerequisites are declared by native packages where applicable.
musl/Alpine, declarative NixOS integration, headless sessions and remote HID
access need separate integration and are not certified by this work.

Access is granted to the active local user using `uaccess` in
`70-attack-shark-x3.rules`, before logind's seat ACL processing. No world-writable
`0666` device rules or manual group membership are required. See
[systemd's rule ordering explanation](https://github.com/systemd/systemd/issues/4288).

## Validation evidence

Validated on 2026-10-20:

| Check | Observed result |
| --- | --- |
| Rust tests in the portable build | 8 unit/lifecycle tests and 9 protocol tests passed; repeated successfully from the committed code in a clean checkout |
| Isolated package assembly | 5 tests passed; desktop entry, udev rules and shell/Python/YAML syntax checks passed |
| Ubuntu 24.04 DEB | Installed with apt and started under Xvfb as an unprivileged user |
| Ubuntu 26.04 DEB | Installed with apt and started under Xvfb as an unprivileged user |
| Fedora 44 RPM | Installed with dnf and started under Xvfb as an unprivileged user |
| Arch rolling package | Built with makepkg, installed with pacman and started under Xvfb as an unprivileged user |
| Standalone AppImage on Ubuntu 24.04 | Started unprivileged with bundled libraries, without installing the native package; used extract-and-run because the container has no FUSE device |
| Fedora 44 / GNOME Wayland desktop | XWayland native title bar, two close/restore cycles including a complete hidden background poll, tray Quit, and normal close without a tray all passed |
| AppImage on the real GNOME desktop | Normal FUSE launch, native title bar, close/restore and tray Quit passed |

The portable binary, DEB, RPM and AppImage were built inside Ubuntu 22.04,
using glibc 2.35 as the internal compilation baseline. The Arch package used
that same portable binary. The local container checks used the scripts shipped
in `packaging/ci`; the GitHub Actions matrix itself has not been run remotely.
The first Arch attempt encountered a mirror signature-download timeout; the
repeat completed with signature verification enabled.

KDE/XFCE interaction and first-run polkit authentication/device ACLs on a clean
physical desktop still require manual validation. The unit test verifies that
permission setup stages its helper/rules in a private directory outside the
AppImage mount. The current user's installed device permissions and mouse
configuration were not changed during these checks.
