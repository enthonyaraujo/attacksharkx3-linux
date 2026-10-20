//! HID Device Manager for Attack Shark X3.

use anyhow::{anyhow, Context, Result};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::thread::sleep;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::protocol::{
    constants::*,
    ApplyBurst, BatteryStatus, DeviceMacro,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Disconnected,
    Wireless24G,
    WiredUsb,
    PermissionDenied,
}

impl ConnectionType {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Wireless24G | Self::WiredUsb)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Disconnected => "Disconnected",
            Self::Wireless24G => "Connected (2.4G Wireless)",
            Self::WiredUsb => "Connected (USB Cable)",
            Self::PermissionDenied => "Access Denied (udev rule needed)",
        }
    }
}

pub struct DeviceManager {
    api: Option<hidapi::HidApi>,
    config_device: Option<hidapi::HidDevice>,
    pub status_device: Option<hidapi::HidDevice>,
    pub connection_type: ConnectionType,
    pub battery_status: Option<BatteryStatus>,
    pub last_error: Option<String>,
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self {
            api: None,
            config_device: None,
            status_device: None,
            connection_type: ConnectionType::Disconnected,
            battery_status: None,
            last_error: None,
        }
    }
}

impl DeviceManager {
    pub fn new() -> Self {
        let mut mgr = Self::default();
        let _ = mgr.init_api();
        mgr
    }

    fn init_api(&mut self) -> Result<()> {
        if self.api.is_none() {
            let api = hidapi::HidApi::new().context("Failed to initialize HID API")?;
            self.api = Some(api);
        }
        Ok(())
    }

    /// Refresh device detection and try to connect to the configuration interface
    pub fn refresh_connection(&mut self) -> ConnectionType {
        if self.init_api().is_err() {
            self.connection_type = ConnectionType::Disconnected;
            return self.connection_type;
        }

        let api = self.api.as_mut().unwrap();
        if let Err(e) = api.refresh_devices() {
            warn!("HID refresh devices failed: {}", e);
        }

        let mut found_wired = false;
        let mut found_wireless = false;
        let mut config_path: Option<CString> = None;
        let mut status_path: Option<CString> = None;
        let mut permission_error = false;

        for device_info in api.device_list() {
            if device_info.vendor_id() == VENDOR_ID {
                let pid = device_info.product_id();
                if pid == PRODUCT_ID_WIRED {
                    found_wired = true;
                } else if pid == PRODUCT_ID_24G {
                    found_wireless = true;
                } else {
                    continue;
                }

                let iface = device_info.interface_number();
                let usage_page = device_info.usage_page();

                debug!(
                    "Found device: VID={:04x} PID={:04x} iface={} usage_page={:04x} path={:?}",
                    device_info.vendor_id(),
                    device_info.product_id(),
                    iface,
                    usage_page,
                    device_info.path()
                );

                // Check for configuration collection (usage_page 0x000B or interface 2)
                if usage_page == CONFIG_USAGE_PAGE || (usage_page == 0 && iface == 2) {
                    if config_path.is_none() {
                        config_path = Some(device_info.path().to_owned());
                    }
                }

                // Check for status/battery collection (usage_page 0x000A)
                if usage_page == STATUS_USAGE_PAGE {
                    if status_path.is_none() {
                        status_path = Some(device_info.path().to_owned());
                    }
                }
            }
        }

        if !found_wired && !found_wireless {
            self.config_device = None;
            self.status_device = None;
            self.connection_type = ConnectionType::Disconnected;
            return self.connection_type;
        }

        // Open config device
        if self.config_device.is_none() {
            if let Some(path) = config_path {
                match api.open_path(&path) {
                    Ok(dev) => {
                        info!("Connected to Attack Shark X3 config interface at {:?}", path);
                        self.config_device = Some(dev);
                    }
                    Err(e) => {
                        warn!("Could not open config interface {:?}: {}", path, e);
                        let open_error = std::fs::OpenOptions::new()
                            .read(true).write(true)
                            .open(std::ffi::OsStr::from_bytes(path.as_bytes()))
                            .err();
                        if open_error.is_some_and(|err| err.kind() == std::io::ErrorKind::PermissionDenied)
                            || e.to_string().to_lowercase().contains("permission")
                        {
                            permission_error = true;
                        }
                    }
                }
            }
        }

        // Open status device (for battery)
        if self.status_device.is_none() {
            if let Some(path) = status_path {
                if let Ok(dev) = api.open_path(&path) {
                    let _ = dev.set_blocking_mode(false);
                    self.status_device = Some(dev);
                }
            }
        }

        if permission_error {
            self.connection_type = ConnectionType::PermissionDenied;
        } else if self.config_device.is_some() || found_wired || found_wireless {
            self.connection_type = if found_wired {
                ConnectionType::WiredUsb
            } else {
                ConnectionType::Wireless24G
            };
        } else {
            self.connection_type = ConnectionType::Disconnected;
        }

        self.connection_type
    }

    /// Read battery input report if available
    pub fn poll_battery(&mut self) -> Option<BatteryStatus> {
        let dev = self.status_device.as_mut().or(self.config_device.as_mut())?;
        let mut buf = [0u8; 16];
        if let Ok(bytes_read) = dev.read_timeout(&mut buf, 10) {
            if bytes_read >= 5 {
                if let Some(status) = BatteryStatus::parse(&buf[..bytes_read]) {
                    self.battery_status = Some(status);
                    return Some(status);
                }
            }
        }
        self.battery_status
    }

    /// Send full configuration burst to mouse
    pub fn send_burst(&mut self, burst: &ApplyBurst) -> Result<()> {
        if self.config_device.is_none() {
            self.refresh_connection();
        }
        let dev = self
            .config_device
            .as_mut()
            .ok_or_else(|| anyhow!("Mouse configuration interface not connected or access denied"))?;

        // Packet 1: 0x0C Commit Lead
        dev.send_feature_report(&burst.commit)
            .context("Failed sending 0x0C Commit packet")?;
        sleep(Duration::from_millis(15));

        // Packet 2: 0x04 Sensor & DPI
        dev.send_feature_report(&burst.sensor)
            .context("Failed sending 0x04 Sensor packet")?;
        sleep(Duration::from_millis(15));

        // Packet 3: 0x05 Power Management & Debounce
        dev.send_feature_report(&burst.power)
            .context("Failed sending 0x05 Power packet")?;
        sleep(Duration::from_millis(15));

        // Packet 4: 0x06 Polling Rate
        dev.send_feature_report(&burst.polling)
            .context("Failed sending 0x06 Polling Rate packet")?;
        sleep(Duration::from_millis(15));

        // Packet 5: 0x08 Buttons
        dev.send_feature_report(&burst.buttons)
            .context("Failed sending 0x08 Buttons packet")?;
        sleep(Duration::from_millis(15));

        info!("Configuration burst successfully sent to Attack Shark X3!");
        Ok(())
    }

    /// Upload a macro to mouse hardware storage
    pub fn upload_macro(&mut self, dev_macro: &DeviceMacro) -> Result<()> {
        if self.config_device.is_none() {
            self.refresh_connection();
        }
        let dev = self
            .config_device
            .as_mut()
            .ok_or_else(|| anyhow!("Mouse configuration interface not connected"))?;

        let (c0, c1, c2) = dev_macro.build_chunks()?;

        dev.send_feature_report(&c0).context("Failed sending macro chunk 0")?;
        sleep(Duration::from_millis(15));

        dev.send_feature_report(&c1).context("Failed sending macro chunk 1")?;
        sleep(Duration::from_millis(15));

        dev.send_feature_report(&c2).context("Failed sending macro chunk 2")?;
        sleep(Duration::from_millis(15));

        info!("Macro '{}' successfully uploaded to slot {}", dev_macro.name, dev_macro.slot);
        Ok(())
    }
}
