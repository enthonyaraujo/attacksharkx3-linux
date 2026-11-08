# USB / HID Communication Protocol — Attack Shark X3

## 1. Framing and Report IDs

Communication is handled via **HID Feature Reports** on Usage Page `0x000B`, Usage `0x0000` (Interface 2, `MI_02`).

Every configuration packet begins with a self-describing 3-byte header:
```text
[0] Report ID
[1] Total packet length in bytes
[2] Profile index (0x01)
[3..] Payload
```

### Report Catalog

| Report ID | Length | Description |
|-----------|--------|-------------|
| `0x0C` | 10 bytes | **Commit Burst Header** (leads every apply burst) |
| `0x04` | 56 bytes | **Sensor & DPI Table** (DPI stages, LOD, Ripple, Angle Snap, Motion Sync) |
| `0x05` | 15 bytes | **Power & Debounce** (Idle sleep, deep sleep, key debounce time) |
| `0x06` | 9 bytes | **Polling Rate** (125 Hz, 250 Hz, 500 Hz, 1000 Hz) |
| `0x08` | 59 bytes | **18-slot Button Table** (actions, shortcuts, macros) |
| `0x09` | 64 / 12 bytes | **Macro Upload Chunks** (transfers 128-byte hardware macro block) |

## 2. Integrity & Checksums

### Config Blocks (`0x04`, `0x05`, `0x08`)
Checksum is a 16-bit big-endian sum:
```text
sum = sum(buf[3..check_offset]) & 0xFFFF
buf[check_offset] = (sum >> 8) & 0xFF
buf[check_offset + 1] = sum & 0xFF
```
- Report `0x04`: check offset 50..52
- Report `0x05`: check offset 11..13
- Report `0x08`: check offset 57..59

### Complement Pair Commands (`0x06`, `0x0C`)
Each parameter byte is encoded as `[value, 0xFF - value]`.
- Polling Rate:
  - 1000 Hz: `0x01` (`0xFE`)
  - 500 Hz: `0x02` (`0xFD`)
  - 250 Hz: `0x04` (`0xFB`)
  - 125 Hz: `0x08` (`0xF7`)

## 3. Transmission Sequence (Apply Burst)
When settings are applied, the driver transmits the reports in this strict sequence:
1. Report `0x0C` (Commit lead)
2. Report `0x04` (Sensor & DPI)
3. Report `0x05` (Power & Debounce)
4. Report `0x06` (Polling rate)
5. Report `0x08` (Button mappings)

After each report, including the last, the driver waits **500 ms for the 2.4 GHz
receiver** or **15 ms for wired USB**. The receiver delay prevents rapid
configuration writes from leaving the polling rate unchanged. It was validated
with `evhz` at 125, 250, 500 and 1000 Hz; see the
[investigation and hardware evidence](wireless-polling-investigation.md).
The full wireless apply takes about 2.5 seconds. This delay affects configuration
writes, not the frequency of mouse input events. Macro-upload pacing is separate.

## 4. Hardware Macro Storage (Report `0x09`)
- 128-byte block divided into 3 chunks:
  - Chunk 0 (64 bytes): header `[0x09, 0x40, 0x08, 0x00]` + bytes 0..60
  - Chunk 1 (64 bytes): header `[0x09, 0x40, 0x08, 0x01]` + bytes 60..120
  - Chunk 2 (12 bytes): header `[0x09, 0x0C, 0x08, 0x02]` + bytes 120..128
- Slot 2 must be used for hardware playback (slot 0 is ignored by firmware).

## 5. Battery Reporting (Report `0x03`)
- Received on Usage Page `0x000A` as a 5-byte unsolicited input report:
  `03 10 [voltage_raw] 01 09`
- `voltage_raw / 16.0` gives the cell voltage (e.g. `0x40 / 16 = 4.00 V`), converted via a lithium discharge curve.
