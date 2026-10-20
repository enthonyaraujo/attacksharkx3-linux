#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../common.sh"
prepare_binary

WORK="${PACKAGE_DIR}/appimage"
APPDIR="${WORK}/AppDir"
BASELINE="${APPIMAGE_GLIBC_MAX:-2.35}"
rm -rf "${APPDIR}"
stage_files "${APPDIR}"
mkdir -p "${APPDIR}/usr/lib"
cp "${PROJECT_ROOT}/packaging/desktop/attack-shark-x3.desktop" "${APPDIR}/"
cp "${PROJECT_ROOT}/assets/icons/icon.png" "${APPDIR}/attack-shark-x3.png"

check_glibc() {
    local needed
    needed="$(glibc_required "$1")"
    if [[ -n "${needed}" && "$(printf '%s\n' "${BASELINE}" "${needed}" | sort -V | tail -n 1)" != "${BASELINE}" ]]; then
        echo "$1 requires glibc ${needed}, newer than baseline ${BASELINE}. Build on Ubuntu 22.04 or an equivalent older sysroot." >&2
        exit 1
    fi
}
check_glibc "${BINARY}"

# Bundle both ELF dependencies and the libraries loaded dynamically by winit /
# glutin. Keep glibc and the host's graphics drivers outside the bundle.
copy_library() {
    local soname="$1" path="$2" dependencies dep_name dep_path
    case "${soname}" in
        ld-linux*|libc.so.*|libm.so.*|libpthread.so.*|libdl.so.*|librt.so.*|libresolv.so.*|libnss_*) return ;;
    esac
    [[ -f "${APPDIR}/usr/lib/${soname}" ]] && return
    check_glibc "${path}"
    install -m 0755 "${path}" "${APPDIR}/usr/lib/${soname}"
    dependencies="$(LC_ALL=C ldd "${path}")"
    if [[ "${dependencies}" == *"not found"* ]]; then
        echo "Unresolved runtime dependency of ${path}: ${dependencies}" >&2
        exit 1
    fi
    while read -r dep_name dep_path; do
        [[ -n "${dep_name}" ]] && copy_library "${dep_name}" "${dep_path}"
    done < <(awk '/=> \// {print $1, $3}' <<< "${dependencies}")
    return 0
}

LDCONFIG_CACHE="$(ldconfig -p)"
for library in libX11.so.6 libX11-xcb.so.1 libxcb.so.1 libXcursor.so.1 libXi.so.6 libXrandr.so.2 libxkbcommon.so.0 libxkbcommon-x11.so.0 libGL.so.1 libEGL.so.1; do
    path="$(awk -v name="${library}" '$1 == name {print $NF; exit}' <<< "${LDCONFIG_CACHE}")"
    [[ -n "${path}" ]] || { echo "Missing runtime library: ${library}" >&2; exit 1; }
    copy_library "${library}" "${path}"
done
dependencies="$(LC_ALL=C ldd "${BINARY}")"
if [[ "${dependencies}" == *"not found"* ]]; then
    echo "Unresolved binary dependencies: ${dependencies}" >&2
    exit 1
fi
while read -r name path; do
    copy_library "${name}" "${path}"
done < <(awk '/=> \// {print $1, $3}' <<< "${dependencies}")

cat > "${APPDIR}/AppRun" <<'EOF'
#!/bin/sh
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export APPDIR="$HERE"
export LD_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PATH="$HERE/usr/bin:$PATH"
exec "$HERE/usr/bin/attack-shark-x3" "$@"
EOF
chmod 0755 "${APPDIR}/AppRun"
if [[ "${APPIMAGE_STAGE_ONLY:-0}" == 1 ]]; then
    echo "AppDir staged at ${APPDIR} (glibc <= ${BASELINE})"
    exit 0
fi

TOOL="${APPIMAGETOOL:-$(command -v appimagetool || true)}"
if [[ -z "${TOOL}" && -x "${PROJECT_ROOT}/target/appimagetool" ]]; then
    TOOL="${PROJECT_ROOT}/target/appimagetool"
fi
[[ -n "${TOOL}" ]] || { echo "Set APPIMAGETOOL to a verified appimagetool executable." >&2; exit 1; }
OUTPUT="${WORK}/Attack_Shark_X3-${VERSION}-${MACHINE}.AppImage"
OPTIONS=()
if [[ -n "${APPIMAGE_RUNTIME:-}" ]]; then
    OPTIONS+=(--runtime-file "${APPIMAGE_RUNTIME}")
fi
ARCH="${MACHINE}" APPIMAGE_EXTRACT_AND_RUN=1 "${TOOL}" "${OPTIONS[@]}" "${APPDIR}" "${OUTPUT}"
echo "Built ${OUTPUT} (glibc <= ${BASELINE})"
