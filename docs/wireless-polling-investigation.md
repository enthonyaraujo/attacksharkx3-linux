# Wireless polling-rate investigation

## Reproduction reported on 2026-11-08

- Polling-rate changes work over the USB cable when measured with `evhz`.
- With only the 2.4 GHz receiver connected, the measured rate stays at 1000 Hz.
- Fully quitting the application and reopening it with only the receiver does
  not resolve the issue.
- DPI changes work with only the receiver connected.

These are user-reported hardware results. They narrow the failure to polling-rate
configuration over the receiver; they do not establish its cause. The libinput
click-filtering workaround discussed in the issue addresses a separate symptom.

## Static evidence

Before the fix, the Linux application built Report `0x06` and sent it through
HID Feature Reports, with 15 ms between configuration reports. It did not check
a receiver acknowledgement or measure the resulting input-event rate.

The locally extracted reference application was inspected with GNU `objdump`
(`objdump -d -Mintel`). No reference executable was run. Native decompilation
through REA was unavailable because no native analysis engine was configured.

| Artifact | SHA-256 |
| --- | --- |
| `X3.exe` | `db7e366f06aa6b06e73cd3a76c7c4d02cf8927e0d9b80d6a838a179d4c52ab41` |
| Both `hiddriver_1.dll` and `hiddriver_2.dll` | `f9efbf0630218b3e13e894b132d77ceb16113acef1bc841e25af8ce57eac39f4` |

Relevant static locations in those exact files:

- `X3.exe:0x413e9f..0x413efd` builds the same 9-byte polling report and rate
  codes as the Linux implementation.
- `X3.exe:0x40ef0c..0x40f000` associates the DLL function tables and connection
  flags with wired PID `0xfa61` and receiver PID `0xfa60`.
- `X3.exe:0x413570..0x413917` routes configuration writes. The receiver branch
  at `0x41386d..0x4138cb` waits 500 ms between attempts and checks a flag updated
  by a status notification. At `0x4138f6..0x4138ff`, successful sends have a
  further 500 ms wait, including the wired path.
- `hiddriver_1.dll:0x10002340..0x1000235c` passes the supplied buffer and length
  to `HidD_SetFeature` via `0x100023b0`.

This establishes a difference in pacing and response handling. Static evidence
alone did not prove that increasing delays would fix the receiver. The hardware
experiment below supplied that additional evidence; no different polling packet
was introduced.

## Controlled timing experiment

An initial diagnostic build accepted `ATTACK_SHARK_WIRELESS_REPORT_DELAY_MS`
to change only receiver configuration-burst pacing. The user tested 500 ms
spacing and reported that polling-rate changes worked. Their `evhz` output
shows sustained plateaus at 125 Hz (average approximately 124–125), 250 Hz,
500 Hz, and approximately 1000 Hz, with a return to 1000 Hz after the lower
rates. This was a physical test on the user's Fedora system with the Beken
2.4G receiver; it was not reproduced on hardware by the coding agent.

The `evhz` average covers the last 64 samples, so transitions and movement
pauses affect it. The stable plateaus, rather than isolated instantaneous
readings or the final average, demonstrate that rate changes took effect.

## Implemented correction

Configuration bursts now wait 500 ms after each report on `Wireless24G` and
15 ms on wired USB. Packet bytes and report ordering remain unchanged. The
diagnostic environment variable was removed; no special launch configuration
is required. An apply over the receiver takes about 2.5 seconds. Macro uploads
continue to use their existing, separate path.

To check the normal build:

1. Fully quit any other running instance using the tray's Quit action.
2. Disconnect the USB cable and leave only the receiver connected.
3. From the repository root, build and launch:

   ```sh
   cargo build --offline
   RUST_LOG=info ./target/debug/attack-shark-x3
   ```

4. Select 125 Hz, wait for the apply operation to finish, and measure continuous
   movement with `evhz`. Repeat at 250, 500 and 1000 Hz.
5. The terminal should show `Wireless24G`, `delay_ms=500`, and a different
   `polling_report` for each rate. For 125 Hz the report is
   `[6, 9, 1, 8, 247, 0, 0, 0, 0]`; for 1000 Hz it is
   `[6, 9, 1, 1, 254, 0, 0, 0, 0]`.

The experiment establishes that 500 ms spacing resolves the reported failure
on the tested unit. The minimum required delay, exact firmware/radio mechanism,
and compatibility with other firmware versions remain unknown. Receiver
acknowledgement handling was not added. The current apply path runs on the UI
thread, so the interface may pause for about 2.5 seconds during wireless apply.

## Initial checkpoint

- Hardware reproduction narrowed by the user; packet construction compared
  against the reference binary.
- Added an opt-in timing experiment and burst logging; no hardware settings
  were changed during this investigation.
- Validation: `cargo test --offline` passed all 17 existing tests;
  `cargo build --offline` and `git diff --check` passed. These checks do not
  exercise receiver timing or prove a physical fix.
- Physical validation of the timing hypothesis is pending. A separate cached
  HID-handle issue was observed in the code, but restarting did not resolve this
  polling-rate reproduction, so it was not changed as part of this experiment.

## Follow-up checkpoint — 2026-11-08

- User supplied `evhz` hardware evidence for all four rates after the timing
  experiment. The earlier physical-validation pending item above is resolved
  for that experiment on this unit.
- Made 500 ms receiver pacing the default and removed the diagnostic override;
  wired pacing remains 15 ms. Added transport regression tests with simulated
  I/O and time, covering all rate packets, ordering, delays, and write failures.
- Before the fix, the wireless timing regression failed at the second report:
  actual send time 15 ms, required 500 ms. No device writes were made by tests.
- Validation after the fix: all 20 tests passed with `cargo test --offline`;
  `cargo build --offline`, focused Rust formatting, and `git diff --check`
  passed. The final default build uses the same report bytes, sequence and
  500 ms waits as the user-validated diagnostic configuration.
