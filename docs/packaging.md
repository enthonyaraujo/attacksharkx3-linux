# Linux package builds

## Shared inputs and output

Run builders with Bash; each resolves the repository root itself. They use the
version from `Cargo.toml` and inspect the ELF architecture instead of hardcoding
amd64. Native staging supports x86_64 and aarch64; the portable build/CI matrix
currently validates x86_64 only.

```bash
bash packaging/deb/build-deb.sh
bash packaging/rpm/build-rpm.sh
bash packaging/arch/build-arch.sh       # Run on Arch with makepkg available
bash packaging/appimage/build-appimage.sh
```

Output is under `target/packages/{deb,rpm,arch,appimage}`. A package is only a
release candidate after its installation/startup checks pass on the target systems.

Set `ATTACK_SHARK_BINARY=/absolute/path/attack-shark-x3` to package a binary
built elsewhere without recompiling it. `ATTACK_SHARK_PACKAGE_DIR` overrides the
output directory. The RPM builder packages the same binary as DEB/AppImage;
it does not silently rebuild it against a newer Fedora glibc. RPM dependencies
include the ELF requirements and libraries loaded dynamically by the renderer.

DEB assembly uses `dpkg-deb` when available and a root-owned, deterministic `ar`
archive otherwise. The glibc dependency is derived from the binary. Install
through apt or a graphical package manager so dependencies are resolved.

HIDAPI is compiled into the executable. Only the system udev ABI is required;
older distribution HIDAPI packages do not need to provide the newer symbols
used by the Rust wrapper.

## Portable baseline

The container build combines an official Rust 1.90 toolchain with Ubuntu 22.04
and builds/tests the binary and packages there. It installs build dependencies
only inside the container and does not require changing the host distribution.

```bash
docker build -f packaging/ci/Containerfile -t attack-shark-packages .
docker create --name attack-shark-packages-output attack-shark-packages
docker cp attack-shark-packages-output:/src/target/packages ./target/portable-packages
docker rm attack-shark-packages-output
```

Podman accepts the equivalent commands. The AppImage tool and runtime are checked
against SHA-256 digests obtained from their official release metadata. If the
upstream `continuous` assets change, update the URLs/digests together after
verification; never bypass the checksum failure.

The AppImage builder accepts `APPIMAGETOOL` and `APPIMAGE_RUNTIME` paths. It bundles
the ELF dependencies and the X11, keyboard and graphics dispatch libraries loaded
at runtime. Host graphics drivers and glibc are not bundled. The default maximum
glibc requirement is 2.35; `APPIMAGE_STAGE_ONLY=1` validates and prepares an AppDir
without generating the image. `APPIMAGE_GLIBC_MAX` is an explicit override for
local experiments, not a compatibility fix. A package built on a newer host
must not be advertised with the older portable baseline.

Normal AppImage execution uses FUSE. In restricted environments, test using
`APPIMAGE_EXTRACT_AND_RUN=1` instead; native packages are preferable for an
installed desktop experience without AppImage runtime prerequisites. This follows
the [AppImage portability guidance](https://docs.appimage.org/introduction/concepts.html).

## Permissions

Native package hooks reload and trigger the hidraw rules automatically, including
devices already connected. Rules use the active-seat ACL rather than world-write
permissions. The portable app detects denied access and launches only its
packaged `attack-shark-x3-install-udev` helper through polkit. Before authentication,
both the helper and its rules are copied into a private `0700` temporary directory
outside the AppImage FUSE mount, which root cannot normally access. The directory
stays alive until setup finishes and is then removed. The helper installs
the same rule and reapplies permissions; the GUI stays unprivileged. Cancellation
leaves an error and retry action in the application instead of prompting in a loop.

## Verification

```bash
cargo test --locked
python3 tests/packaging_tests.py
udevadm verify packaging/udev/70-attack-shark-x3.rules
desktop-file-validate packaging/desktop/attack-shark-x3.desktop

# A real desktop with a working tray host:
python3 tests/desktop_smoke.py --binary target/release/attack-shark-x3
# Same desktop, isolated session bus without a tray:
dbus-run-session -- python3 tests/desktop_smoke.py --no-tray
# An installed package, headless (requires Xvfb):
dbus-run-session -- python3 tests/startup_smoke.py /usr/bin/attack-shark-x3
```

The Linux packages workflow builds without publishing. Its jobs install artifacts
with apt, dnf and pacman and start the packaged GUI in disposable containers.
These checks do not replace testing physical HID communication, polkit dialogs
or each desktop's tray implementation on real installations.

For the same install checks outside GitHub Actions:

```bash
CONTAINER_ENGINE=podman bash packaging/ci/check-package.sh deb ubuntu:24.04 /path/to/package.deb
CONTAINER_ENGINE=podman bash packaging/ci/check-package.sh rpm fedora:44 /path/to/package.rpm
CONTAINER_ENGINE=podman bash packaging/ci/check-package.sh appimage ubuntu:24.04 /path/to/package.AppImage
CONTAINER_ENGINE=podman bash packaging/ci/check-arch.sh archlinux:base /path/to/portable-binary /path/to/output
```

These scripts create and remove their own containers. The Arch check installs
only package-assembly tools and runtime dependencies, and retries a failed
mirror download once with signature verification preserved.
