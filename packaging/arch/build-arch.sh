#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../common.sh"
command -v makepkg >/dev/null || { echo "Build this package on Arch Linux with makepkg installed." >&2; exit 1; }
prepare_binary
WORK="${PACKAGE_DIR}/arch"
STAGE="${WORK}/source/attack-shark-x3-${VERSION}"
rm -rf "${STAGE}"
stage_files "${STAGE}"
TARBALL="attack-shark-x3-${VERSION}.tar.gz"
tar -czf "${WORK}/${TARBALL}" -C "${WORK}/source" "attack-shark-x3-${VERSION}"
HASH="$(sha256sum "${WORK}/${TARBALL}" | cut -d ' ' -f 1)"
sed -e "s/@VERSION@/${VERSION}/g" -e "s/@ARCH@/${MACHINE}/g" -e "s/@GLIBC@/${GLIBC_REQUIRED}/g" -e "s/@SHA256@/${HASH}/g" "${PROJECT_ROOT}/packaging/arch/PKGBUILD.in" > "${WORK}/PKGBUILD"
cp "${PROJECT_ROOT}/packaging/arch/attack-shark-x3.install" "${WORK}/"
(cd "${WORK}" && makepkg --force)
