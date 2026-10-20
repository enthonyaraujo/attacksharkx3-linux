#!/usr/bin/env bash
# Install/start a package in a disposable container, without host device mounts.
set -euo pipefail
KIND="${1:?Usage: check-package.sh deb|rpm|appimage IMAGE ARTIFACT}"
IMAGE="${2:?Missing container image}"
ARTIFACT="$(realpath "${3:?Missing package path}")"
ENGINE="${CONTAINER_ENGINE:-docker}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NAME="attack-shark-${KIND}-check-$$"

case "${KIND}" in
    deb)
        COMMAND='apt-get update && apt-get install -y /tmp/package.deb xvfb dbus-x11 python3 libgl1-mesa-dri && test -f /usr/lib/udev/rules.d/70-attack-shark-x3.rules && useradd -m appuser && su -s /bin/sh appuser -c "dbus-run-session -- python3 /tmp/startup_smoke.py /usr/bin/attack-shark-x3"'
        ;;
    rpm)
        COMMAND='dnf install -y /tmp/package.rpm xorg-x11-server-Xvfb dbus-daemon python3 shadow-utils mesa-dri-drivers && test -f /usr/lib/udev/rules.d/70-attack-shark-x3.rules && useradd -m appuser && su -s /bin/sh appuser -c "dbus-run-session -- python3 /tmp/startup_smoke.py /usr/bin/attack-shark-x3"'
        ;;
    appimage)
        COMMAND='apt-get update && apt-get install -y xvfb dbus-x11 python3 libgl1-mesa-dri && chmod +x /tmp/package.appimage && useradd -m appuser && su -s /bin/sh appuser -c "dbus-run-session -- python3 /tmp/startup_smoke.py /tmp/package.appimage"'
        ;;
    *) echo "Unknown package format: ${KIND}" >&2; exit 2 ;;
esac

"${ENGINE}" create --name "${NAME}" "${IMAGE}" sh -ec "${COMMAND}"
trap '"${ENGINE}" rm -f "${NAME}" >/dev/null' EXIT
"${ENGINE}" cp "${ARTIFACT}" "${NAME}:/tmp/package.${KIND}"
"${ENGINE}" cp "${ROOT}/tests/startup_smoke.py" "${NAME}:/tmp/startup_smoke.py"
"${ENGINE}" start --attach "${NAME}"
