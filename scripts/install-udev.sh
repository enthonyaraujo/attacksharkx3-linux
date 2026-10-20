#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RULES_SRC="${SCRIPT_DIR}/../packaging/udev/70-attack-shark-x3.rules"
RULES_DEST="/etc/udev/rules.d/70-attack-shark-x3.rules"

echo "==> Installing udev rules for Attack Shark X3..."
sudo install -Dm0644 "${RULES_SRC}" "${RULES_DEST}"
sudo rm -f /etc/udev/rules.d/99-attack-shark-x3.rules
echo "==> Reloading udev rules..."
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw --action=change
sudo udevadm settle --timeout=5
echo "==> Done! You can now configure Attack Shark X3 without root privileges."
