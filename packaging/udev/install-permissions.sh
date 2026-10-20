#!/bin/sh
# The AppImage calls only this helper through polkit; the GUI never runs as root.
set -eu
PATH=/usr/sbin:/usr/bin:/sbin:/bin
export PATH
if [ "$(id -u)" != 0 ]; then
    echo "Administrator authentication is required to grant access to the mouse." >&2
    exit 77
fi
if [ "$#" != 0 ]; then
    echo "This helper does not accept arguments." >&2
    exit 64
fi
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
install -Dm0644 "$HERE/../share/attack-shark-x3/70-attack-shark-x3.rules" /etc/udev/rules.d/70-attack-shark-x3.rules
# Remove the legacy manual rule, which granted access to every local user.
rm -f /etc/udev/rules.d/99-attack-shark-x3.rules
udevadm control --reload-rules
udevadm trigger --subsystem-match=hidraw --action=change
udevadm settle --timeout=5
