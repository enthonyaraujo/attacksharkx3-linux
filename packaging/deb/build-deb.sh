#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../common.sh"
prepare_binary

WORK="${PACKAGE_DIR}/deb"
STAGE="${WORK}/attack-shark-x3_${VERSION}_${DEB_ARCH}"
rm -rf "${STAGE}"
stage_files "${STAGE}"
mkdir -p "${STAGE}/DEBIAN"
cat > "${STAGE}/DEBIAN/control" <<EOF
Package: attack-shark-x3
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${DEB_ARCH}
Depends: libc6 (>= ${GLIBC_REQUIRED}), libgcc-s1, libudev1, udev, libx11-6, libx11-xcb1, libxcb1, libxcursor1, libxi6, libxrandr2, libxkbcommon0, libxkbcommon-x11-0, libgl1, libegl1, xwayland, pkexec | policykit-1
Maintainer: Enthony <contact@example.com>
Description: Native Linux configuration utility for the Attack Shark X3 mouse
 Configures DPI, polling rate, sensor, buttons, macros and profiles.
 Uses the desktop's native window decorations and system tray.
EOF

for hook in postinst postrm; do
    cat > "${STAGE}/DEBIAN/${hook}" <<'EOF'
#!/bin/sh
set -e
case "${1:-}" in
    configure|remove|purge)
        if command -v udevadm >/dev/null 2>&1; then
            udevadm control --reload-rules 2>/dev/null || true
            udevadm trigger --subsystem-match=hidraw --action=change 2>/dev/null || true
            udevadm settle --timeout=5 2>/dev/null || true
        fi
        if command -v update-desktop-database >/dev/null 2>&1; then
            update-desktop-database -q /usr/share/applications 2>/dev/null || true
        fi
        ;;
esac
EOF
    chmod 0755 "${STAGE}/DEBIAN/${hook}"
done

OUTPUT="${WORK}/attack-shark-x3_${VERSION}_${DEB_ARCH}.deb"
if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb --root-owner-group -Zgzip --build "${STAGE}" "${OUTPUT}"
else
    TMP="$(mktemp -d)"
    trap 'rm -rf "${TMP}"' EXIT
    printf '2.0\n' > "${TMP}/debian-binary"
    tar --owner=0 --group=0 --numeric-owner -czf "${TMP}/control.tar.gz" -C "${STAGE}/DEBIAN" .
    tar --owner=0 --group=0 --numeric-owner -czf "${TMP}/data.tar.gz" --exclude=./DEBIAN -C "${STAGE}" .
    rm -f "${OUTPUT}"
    ar qcD "${OUTPUT}" "${TMP}/debian-binary" "${TMP}/control.tar.gz" "${TMP}/data.tar.gz"
fi
echo "Built ${OUTPUT} (requires glibc >= ${GLIBC_REQUIRED})"
