#!/usr/bin/env bash
# Build/install a native Arch package in an isolated rolling-release container.
set -euo pipefail
IMAGE="${1:-archlinux:base}"
BINARY="$(realpath "${2:?Missing portable binary}")"
OUTPUT="${3:?Missing artifact directory}"
mkdir -p "${OUTPUT}"
OUTPUT="$(realpath "${OUTPUT}")"
ENGINE="${CONTAINER_ENGINE:-docker}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NAME="attack-shark-arch-check-$$"
CONTEXT="$(mktemp -d)"
trap 'rm -rf "${CONTEXT}"; "${ENGINE}" rm -f "${NAME}" >/dev/null 2>&1 || true' EXIT
tar -cf "${CONTEXT}/source.tar" -C "${ROOT}" Cargo.toml packaging assets README.md LICENSE docs tests
"${ENGINE}" create --name "${NAME}" "${IMAGE}" sh -ec '
    for attempt in 1 2; do
        if pacman -Syu --noconfirm --needed fakeroot binutils file zstd systemd libx11 libxcb libxcursor libxi libxrandr libxkbcommon-x11 libglvnd mesa xorg-xwayland polkit python xorg-server-xvfb dbus; then
            break
        fi
        [ "$attempt" -lt 2 ] || exit 1
    done
    mkdir -p /work
    tar -xf /tmp/source.tar -C /work
    install -Dm755 /tmp/portable-binary /work/prebuilt/attack-shark-x3
    useradd -m builder
    chown -R builder:builder /work
    su builder -c "ATTACK_SHARK_BINARY=/work/prebuilt/attack-shark-x3 bash /work/packaging/arch/build-arch.sh"
    pacman -U --noconfirm /work/target/packages/arch/*.pkg.tar.zst
    test -f /usr/lib/udev/rules.d/70-attack-shark-x3.rules
    su -s /bin/sh builder -c "dbus-run-session -- python /work/tests/startup_smoke.py /usr/bin/attack-shark-x3"
    cp /work/target/packages/arch/*.pkg.tar.zst /tmp/attack-shark-x3.pkg.tar.zst
'
"${ENGINE}" cp "${CONTEXT}/source.tar" "${NAME}:/tmp/source.tar"
"${ENGINE}" cp "${BINARY}" "${NAME}:/tmp/portable-binary"
"${ENGINE}" start --attach "${NAME}"
"${ENGINE}" cp "${NAME}:/tmp/attack-shark-x3.pkg.tar.zst" "${OUTPUT}/"
