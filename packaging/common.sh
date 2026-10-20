#!/usr/bin/env bash
# Shared binary staging for all Linux packages. Source from a package builder.
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE_DIR="${ATTACK_SHARK_PACKAGE_DIR:-${PROJECT_ROOT}/target/packages}"
VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "${PROJECT_ROOT}/Cargo.toml" | head -n 1)"
RULE_NAME="70-attack-shark-x3.rules"

prepare_binary() {
    if [[ -n "${ATTACK_SHARK_BINARY:-}" ]]; then
        BINARY="$(realpath "${ATTACK_SHARK_BINARY}")"
    else
        (cd "${PROJECT_ROOT}" && cargo build --release --locked)
        BINARY="${PROJECT_ROOT}/target/release/attack-shark-x3"
    fi
    [[ -x "${BINARY}" ]] || { echo "Missing executable: ${BINARY}" >&2; exit 1; }
    case "$(LC_ALL=C readelf -h "${BINARY}")" in
        *X86-64*) MACHINE=x86_64; DEB_ARCH=amd64 ;;
        *AArch64*) MACHINE=aarch64; DEB_ARCH=arm64 ;;
        *) echo "Unsupported ELF architecture (expected x86_64 or aarch64)" >&2; exit 1 ;;
    esac
    GLIBC_REQUIRED="$(glibc_required "${BINARY}")"
    [[ -n "${GLIBC_REQUIRED}" ]] || { echo "Cannot determine the binary's glibc requirement" >&2; exit 1; }
}

glibc_required() {
    LC_ALL=C readelf --version-info "$1" | sed -n 's/.*Name: GLIBC_\([0-9][0-9.]*\).*/\1/p' | sort -Vu | tail -n 1
}

stage_files() {
    local stage="$1"
    mkdir -p "${stage}"
    install -Dm755 "${BINARY}" "${stage}/usr/bin/attack-shark-x3"
    install -Dm644 "${PROJECT_ROOT}/packaging/desktop/attack-shark-x3.desktop" "${stage}/usr/share/applications/attack-shark-x3.desktop"
    install -Dm644 "${PROJECT_ROOT}/assets/icons/icon.png" "${stage}/usr/share/icons/hicolor/256x256/apps/attack-shark-x3.png"
    install -Dm644 "${PROJECT_ROOT}/packaging/udev/${RULE_NAME}" "${stage}/usr/lib/udev/rules.d/${RULE_NAME}"
    install -Dm644 "${PROJECT_ROOT}/packaging/udev/${RULE_NAME}" "${stage}/usr/share/attack-shark-x3/${RULE_NAME}"
    install -Dm755 "${PROJECT_ROOT}/packaging/udev/install-permissions.sh" "${stage}/usr/libexec/attack-shark-x3-install-udev"
    install -Dm644 "${PROJECT_ROOT}/LICENSE" "${stage}/usr/share/licenses/attack-shark-x3/LICENSE"
    install -Dm644 "${PROJECT_ROOT}/README.md" "${stage}/usr/share/doc/attack-shark-x3/README.md"
}
