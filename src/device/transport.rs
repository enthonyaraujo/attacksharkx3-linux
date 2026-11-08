//! Configuration report delivery, with injectable I/O and waits for regression tests.

use super::manager::ConnectionType;
use crate::protocol::ApplyBurst;
use anyhow::{Context, Result};
use std::time::Duration;
use tracing::info;

pub(super) fn send_burst(
    connection: ConnectionType,
    burst: &ApplyBurst,
    mut send: impl FnMut(&[u8]) -> Result<()>,
    mut wait: impl FnMut(Duration),
) -> Result<()> {
    // The receiver's polling rate stayed unchanged with 15 ms burst spacing.
    // Match the 500 ms reference-app pacing validated with evhz on hardware.
    // This waits between configuration reports; it does not slow input events.
    let report_delay = match connection {
        ConnectionType::Wireless24G => Duration::from_millis(500),
        _ => Duration::from_millis(15),
    };

    info!(?connection, delay_ms = report_delay.as_millis(),
        polling_report = ?burst.polling, "Sending configuration burst");

    for (report, name) in [
        (burst.commit.as_slice(), "0x0C Commit"),
        (burst.sensor.as_slice(), "0x04 Sensor"),
        (burst.power.as_slice(), "0x05 Power"),
        (burst.polling.as_slice(), "0x06 Polling Rate"),
        (burst.buttons.as_slice(), "0x08 Buttons"),
    ] {
        send(report).with_context(|| format!("Failed sending {name} packet"))?;
        wait(report_delay);
    }

    info!("Configuration burst successfully sent to Attack Shark X3!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ButtonConfig, PollingRate, PowerConfig, SensorConfig};
    use std::cell::Cell;

    fn burst(rate: PollingRate) -> ApplyBurst {
        ApplyBurst::build(
            &SensorConfig::default(),
            &PowerConfig::default(),
            rate,
            &ButtonConfig::default(),
        )
        .unwrap()
    }

    fn assert_delivery(connection: ConnectionType, interval: Duration) {
        for rate in [
            PollingRate::Hz125,
            PollingRate::Hz250,
            PollingRate::Hz500,
            PollingRate::Hz1000,
        ] {
            let burst = burst(rate);
            let elapsed = Cell::new(Duration::ZERO);
            let mut writes = Vec::new();

            send_burst(
                connection,
                &burst,
                |report| {
                    writes.push((elapsed.get(), report.to_vec()));
                    Ok(())
                },
                |delay| elapsed.set(elapsed.get() + delay),
            )
            .unwrap();

            let reports = [
                burst.commit.as_slice(),
                burst.sensor.as_slice(),
                burst.power.as_slice(),
                burst.polling.as_slice(),
                burst.buttons.as_slice(),
            ];
            assert_eq!(writes.len(), reports.len());
            for (index, ((time, actual), expected)) in writes.iter().zip(reports).enumerate() {
                assert_eq!(actual, expected);
                assert_eq!(
                    *time,
                    interval * index as u32,
                    "{connection:?}: report 0x{:02X} sent too early",
                    expected[0]
                );
            }
            assert_eq!(elapsed.get(), interval * 5);
        }
    }

    #[test]
    fn wireless_configuration_leaves_receiver_time_to_process_each_report() {
        assert_delivery(ConnectionType::Wireless24G, Duration::from_millis(500));
    }

    #[test]
    fn wired_configuration_keeps_existing_delivery_timing() {
        assert_delivery(ConnectionType::WiredUsb, Duration::from_millis(15));
    }

    #[test]
    fn failed_polling_write_stops_before_buttons_and_reports_the_failure() {
        let burst = burst(PollingRate::Hz125);
        let mut writes = Vec::new();
        let mut waits = Vec::new();
        let result = send_burst(
            ConnectionType::Wireless24G,
            &burst,
            |report| {
                writes.push(report[0]);
                anyhow::ensure!(report[0] != 0x06, "device disconnected");
                Ok(())
            },
            |delay| waits.push(delay),
        );

        let error = result.unwrap_err();
        assert_eq!(writes, [0x0C, 0x04, 0x05, 0x06]);
        assert_eq!(waits.len(), 3);
        assert!(error.to_string().contains("0x06 Polling Rate"));
        assert!(format!("{error:#}").contains("device disconnected"));
    }
}
