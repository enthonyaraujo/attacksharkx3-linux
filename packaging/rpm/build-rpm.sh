#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../common.sh"
command -v rpmbuild >/dev/null || { echo "rpmbuild is required" >&2; exit 1; }
prepare_binary

TOPDIR="${PACKAGE_DIR}/rpm"
SOURCE_NAME="attack-shark-x3-${VERSION}"
STAGE="${TOPDIR}/source/${SOURCE_NAME}"
rm -rf "${STAGE}"
mkdir -p "${TOPDIR}"/{BUILD,RPMS,SOURCES,SPECS,SRPMS,tmp}
stage_files "${STAGE}"
tar -czf "${TOPDIR}/SOURCES/${SOURCE_NAME}.tar.gz" -C "${TOPDIR}/source" "${SOURCE_NAME}"
cp "${PROJECT_ROOT}/packaging/rpm/attack-shark-x3.spec" "${TOPDIR}/SPECS/"
rpmbuild --define "_topdir ${TOPDIR}" --define "_tmppath ${TOPDIR}/tmp" --define "app_version ${VERSION}" --target "${MACHINE}" -bb "${TOPDIR}/SPECS/attack-shark-x3.spec"
find "${TOPDIR}/RPMS" -type f -name '*.rpm' -print
