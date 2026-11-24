//! HID Device Manager for Attack Shark X3.

use anyhow::{anyhow, Context, Result};
use std::ffi::{CStr, CString};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnumeratedDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub interface_number: i32,
    pub usage_page: u16,
    pub path: CString,
}

#[derive(Debug, Clone)]
pub(crate) struct OpenDeviceError {
    pub message: String,
    pub permission_denied: bool,
}

pub(crate) trait HidHandle {
    fn send_feature_report(&self, report: &[u8]) -> Result<()>;
    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize>;
    fn set_blocking_mode(&self, blocking: bool) -> Result<()>;
}

pub(crate) trait HidBackend {
    fn enumerate(&mut self) -> Result<Vec<EnumeratedDevice>>;
    fn open_path(&mut self, path: &CStr) -> std::result::Result<Box<dyn HidHandle>, OpenDeviceError>;
    fn wait(&self, duration: Duration) {
        sleep(duration);
    }
}

struct SystemHidHandle(hidapi::HidDevice);

impl HidHandle for SystemHidHandle {
    fn send_feature_report(&self, report: &[u8]) -> Result<()> {
        self.0.send_feature_report(report).map_err(Into::into)
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize> {
        self.0.read_timeout(buf, timeout_ms).map_err(Into::into)
    }

    fn set_blocking_mode(&self, blocking: bool) -> Result<()> {
        self.0.set_blocking_mode(blocking).map_err(Into::into)
    }
}

#[derive(Default)]
struct SystemHidBackend {
    api: Option<hidapi::HidApi>,
}

impl SystemHidBackend {
    fn ensure_api(&mut self) -> Result<&mut hidapi::HidApi> {
        if self.api.is_none() {
            let api = hidapi::HidApi::new().context("Failed to initialize HID API")?;
            self.api = Some(api);
        }
        Ok(self.api.as_mut().unwrap())
    }
}

impl HidBackend for SystemHidBackend {
    fn enumerate(&mut self) -> Result<Vec<EnumeratedDevice>> {
        let api = self.ensure_api()?;
        if let Err(e) = api.refresh_devices() {
            warn!("HID refresh devices failed: {}", e);
        }
        Ok(api
            .device_list()
            .map(|info| EnumeratedDevice {
                vendor_id: info.vendor_id(),
                product_id: info.product_id(),
                interface_number: info.interface_number(),
                usage_page: info.usage_page(),
                path: info.path().to_owned(),
            })
            .collect())
    }

    fn open_path(&mut self, path: &CStr) -> std::result::Result<Box<dyn HidHandle>, OpenDeviceError> {
        let api = self.ensure_api().map_err(|e| OpenDeviceError {
            message: e.to_string(),
            permission_denied: false,
        })?;
        match api.open_path(path) {
            Ok(dev) => Ok(Box::new(SystemHidHandle(dev))),
            Err(e) => {
                let open_error = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(std::ffi::OsStr::from_bytes(path.to_bytes()))
                    .err();
                let lower = e.to_string().to_lowercase();
                let permission_denied = open_error
                    .is_some_and(|err| err.kind() == std::io::ErrorKind::PermissionDenied)
                    || lower.contains("permission")
                    || lower.contains("access denied")
                    || lower.contains("not permitted");
                Err(OpenDeviceError {
                    message: e.to_string(),
                    permission_denied,
                })
            }
        }
    }
}

pub struct DeviceManager {
    backend: Box<dyn HidBackend>,
    config_device: Option<Box<dyn HidHandle>>,
    config_path: Option<CString>,
    status_device: Option<Box<dyn HidHandle>>,
    status_path: Option<CString>,
    active_pid: Option<u16>,
    pub connection_type: ConnectionType,
    pub battery_status: Option<BatteryStatus>,
    pub last_error: Option<String>,
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self {
            backend: Box::new(SystemHidBackend::default()),
            config_device: None,
            config_path: None,
            status_device: None,
            status_path: None,
            active_pid: None,
            connection_type: ConnectionType::Disconnected,
            battery_status: None,
            last_error: None,
        }
    }
}

struct SelectedTarget {
    pid: u16,
    connection_type: ConnectionType,
    config_path: Option<CString>,
    status_path: Option<CString>,
}

fn select_target(devices: &[EnumeratedDevice]) -> Option<SelectedTarget> {
    let has_wired = devices
        .iter()
        .any(|d| d.vendor_id == VENDOR_ID && d.product_id == PRODUCT_ID_WIRED);
    let has_wireless = devices
        .iter()
        .any(|d| d.vendor_id == VENDOR_ID && d.product_id == PRODUCT_ID_24G);

    let (pid, connection_type) = if has_wired {
        (PRODUCT_ID_WIRED, ConnectionType::WiredUsb)
    } else if has_wireless {
        (PRODUCT_ID_24G, ConnectionType::Wireless24G)
    } else {
        return None;
    };

    let matching: Vec<&EnumeratedDevice> = devices
        .iter()
        .filter(|d| d.vendor_id == VENDOR_ID && d.product_id == pid)
        .collect();

    // Primary: Usage Page 0x000B (Configuration Feature collection on MI_02).
    // Fallbacks: USB Interface 2 when report descriptor usage is 0, or Usage Page 0x000A
    // which shares the same composite hidraw node on Linux (including uhid where iface == -1).
    let config_path = matching
        .iter()
        .find(|d| d.usage_page == CONFIG_USAGE_PAGE)
        .or_else(|| matching.iter().find(|d| d.interface_number == 2))
        .or_else(|| matching.iter().find(|d| d.usage_page == STATUS_USAGE_PAGE))
        .map(|d| d.path.clone());

    let status_path = matching
        .iter()
        .find(|d| d.usage_page == STATUS_USAGE_PAGE)
        .map(|d| d.path.clone());

    Some(SelectedTarget {
        pid,
        connection_type,
        config_path,
        status_path,
    })
}

impl DeviceManager {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) fn with_backend(backend: Box<dyn HidBackend>) -> Self {
        Self {
            backend,
            ..Self::default()
        }
    }

    fn reset_handles(&mut self) {
        self.config_device = None;
        self.config_path = None;
        self.status_device = None;
        self.status_path = None;
        self.active_pid = None;
    }

    fn missing_config_error(&self) -> anyhow::Error {
        match self.connection_type {
            ConnectionType::PermissionDenied => anyhow!(
                "Mouse access denied: grant device permissions or reload udev rules"
            ),
            _ => {
                if let Some(err) = &self.last_error {
                    anyhow!("Mouse configuration interface unavailable: {}", err)
                } else {
                    anyhow!(
                        "Mouse configuration interface not connected (use the 2.4G receiver or USB cable)"
                    )
                }
            }
        }
    }

    /// Refresh device detection and try to connect to the configuration interface
    pub fn refresh_connection(&mut self) -> ConnectionType {
        let devices = match self.backend.enumerate() {
            Ok(devices) => devices,
            Err(e) => {
                self.reset_handles();
                self.last_error = Some(e.to_string());
                self.connection_type = ConnectionType::Disconnected;
                return self.connection_type;
            }
        };

        for device_info in devices.iter().filter(|d| d.vendor_id == VENDOR_ID) {
            debug!(
                "Found device: VID={:04x} PID={:04x} iface={} usage_page={:04x} path={:?}",
                device_info.vendor_id,
                device_info.product_id,
                device_info.interface_number,
                device_info.usage_page,
                device_info.path
            );
        }

        let Some(target) = select_target(&devices) else {
            self.reset_handles();
            self.last_error = None;
            self.connection_type = ConnectionType::Disconnected;
            return self.connection_type;
        };

        // Drop cached handles whenever the active transport PID or devnode path changes.
        if self.active_pid != Some(target.pid) || self.config_path != target.config_path {
            self.config_device = None;
            self.config_path = None;
        }
        if self.active_pid != Some(target.pid) || self.status_path != target.status_path {
            self.status_device = None;
            self.status_path = None;
        }
        self.active_pid = Some(target.pid);

        let mut permission_error = false;

        // Open config device
        if self.config_device.is_none() {
            if let Some(path) = target.config_path {
                match self.backend.open_path(&path) {
                    Ok(dev) => {
                        info!("Connected to Attack Shark X3 config interface at {:?}", path);
                        self.config_device = Some(dev);
                        self.config_path = Some(path);
                        self.last_error = None;
                    }
                    Err(e) => {
                        warn!("Could not open config interface {:?}: {}", path, e.message);
                        if e.permission_denied {
                            permission_error = true;
                        }
                        self.last_error = Some(e.message);
                    }
                }
            } else {
                self.last_error = Some("Configuration HID collection not ready".into());
            }
        }

        // Open status device (for battery)
        if self.status_device.is_none() {
            if let Some(path) = target.status_path {
                if let Ok(dev) = self.backend.open_path(&path) {
                    let _ = dev.set_blocking_mode(false);
                    self.status_device = Some(dev);
                    self.status_path = Some(path);
                }
            }
        }

        if permission_error {
            self.connection_type = ConnectionType::PermissionDenied;
        } else if self.config_device.is_some() {
            self.connection_type = target.connection_type;
        } else {
            self.connection_type = ConnectionType::Disconnected;
        }

        self.connection_type
    }

    /// Read battery input report if available
    pub fn poll_battery(&mut self) -> Option<BatteryStatus> {
        let mut buf = [0u8; 16];
        let first_read = {
            let dev = self.status_device.as_ref().or(self.config_device.as_ref())?;
            dev.read_timeout(&mut buf, 10)
        };

        let bytes_read = match first_read {
            Ok(n) => n,
            Err(e) => {
                debug!("HID status handle disconnected ({}), refreshing connection", e);
                self.reset_handles();
                self.refresh_connection();
                let dev = self.status_device.as_ref().or(self.config_device.as_ref())?;
                dev.read_timeout(&mut buf, 10).ok()?
            }
        };

        if bytes_read >= 5 {
            if let Some(status) = BatteryStatus::parse(&buf[..bytes_read]) {
                self.battery_status = Some(status);
                return Some(status);
            }
        }
        self.battery_status
    }

    /// Send full configuration burst to mouse
    pub fn send_burst(&mut self, burst: &ApplyBurst) -> Result<()> {
        let was_open = self.config_device.is_some();
        let prev_path = self.config_path.clone();
        self.refresh_connection();
        let reused_cached = was_open && self.config_device.is_some() && self.config_path == prev_path;

        if self.config_device.is_none() {
            return Err(self.missing_config_error());
        }

        let mut reports_sent = 0usize;
        let first_result = {
            let dev = self.config_device.as_ref().unwrap();
            let backend = &self.backend;
            super::transport::send_burst(
                self.connection_type,
                burst,
                |report| {
                    dev.send_feature_report(report)?;
                    reports_sent += 1;
                    Ok(())
                },
                |delay| backend.wait(delay),
            )
        };

        match first_result {
            Ok(()) => Ok(()),
            Err(err) => {
                self.reset_handles();
                if reused_cached && reports_sent == 0 {
                    info!("Cached HID handle failed on initial write; reopening device and retrying");
                    self.refresh_connection();
                    if let Some(dev) = self.config_device.as_ref() {
                        let backend = &self.backend;
                        let retry = super::transport::send_burst(
                            self.connection_type,
                            burst,
                            |report| dev.send_feature_report(report),
                            |delay| backend.wait(delay),
                        );
                        if retry.is_err() {
                            self.reset_handles();
                        }
                        return retry;
                    }
                }
                Err(err)
            }
        }
    }

    /// Upload a macro to mouse hardware storage
    pub fn upload_macro(&mut self, dev_macro: &DeviceMacro) -> Result<()> {
        let (c0, c1, c2) = dev_macro.build_chunks()?;

        let was_open = self.config_device.is_some();
        let prev_path = self.config_path.clone();
        self.refresh_connection();
        let reused_cached = was_open && self.config_device.is_some() && self.config_path == prev_path;

        if self.config_device.is_none() {
            return Err(self.missing_config_error());
        }

        let c0_res = self
            .config_device
            .as_ref()
            .unwrap()
            .send_feature_report(&c0)
            .context("Failed sending macro chunk 0");

        if let Err(err) = c0_res {
            self.reset_handles();
            if reused_cached {
                info!("Cached HID handle failed on macro chunk 0; reopening device and retrying");
                self.refresh_connection();
                let dev = self
                    .config_device
                    .as_ref()
                    .ok_or_else(|| self.missing_config_error())?;
                if let Err(retry_err) = dev
                    .send_feature_report(&c0)
                    .context("Failed sending macro chunk 0")
                {
                    self.reset_handles();
                    return Err(retry_err);
                }
            } else {
                return Err(err);
            }
        }
        self.backend.wait(Duration::from_millis(15));

        if let Err(err) = self
            .config_device
            .as_ref()
            .unwrap()
            .send_feature_report(&c1)
            .context("Failed sending macro chunk 1")
        {
            self.reset_handles();
            return Err(err);
        }
        self.backend.wait(Duration::from_millis(15));

        if let Err(err) = self
            .config_device
            .as_ref()
            .unwrap()
            .send_feature_report(&c2)
            .context("Failed sending macro chunk 2")
        {
            self.reset_handles();
            return Err(err);
        }
        self.backend.wait(Duration::from_millis(15));

        info!("Macro '{}' successfully uploaded to slot {}", dev_macro.name, dev_macro.slot);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        macro_upload::MacroEvent, ButtonConfig, PollingRate, PowerConfig, SensorConfig,
    };
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[derive(Default)]
    struct SharedMockState {
        devices: Vec<EnumeratedDevice>,
        open_results: HashMap<CString, Vec<std::result::Result<Rc<MockHandleState>, OpenDeviceError>>>,
        opened_paths: Vec<CString>,
        waits: Vec<Duration>,
    }

    struct MockHandleState {
        label: String,
        alive: RefCell<bool>,
        writes: RefCell<Vec<Vec<u8>>>,
        reads: RefCell<Vec<Result<Vec<u8>>>>,
    }

    impl MockHandleState {
        fn new(label: &str) -> Rc<Self> {
            Rc::new(Self {
                label: label.to_string(),
                alive: RefCell::new(true),
                writes: RefCell::new(Vec::new()),
                reads: RefCell::new(Vec::new()),
            })
        }

        fn disconnect(&self) {
            *self.alive.borrow_mut() = false;
        }
    }

    struct MockHandle(Rc<MockHandleState>);

    impl HidHandle for MockHandle {
        fn send_feature_report(&self, report: &[u8]) -> Result<()> {
            if !*self.0.alive.borrow() {
                return Err(anyhow!("ioctl(SFEATURE): No such device ({})", self.0.label));
            }
            self.0.writes.borrow_mut().push(report.to_vec());
            Ok(())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize> {
            if !*self.0.alive.borrow() {
                return Err(anyhow!("hid_read_timeout: unexpected poll error (device disconnected)"));
            }
            let mut reads = self.0.reads.borrow_mut();
            if reads.is_empty() {
                return Ok(0);
            }
            let next = reads.remove(0)?;
            let len = next.len().min(buf.len());
            buf[..len].copy_from_slice(&next[..len]);
            Ok(len)
        }

        fn set_blocking_mode(&self, _blocking: bool) -> Result<()> {
            Ok(())
        }
    }

    struct MockBackend(Rc<RefCell<SharedMockState>>);

    impl HidBackend for MockBackend {
        fn enumerate(&mut self) -> Result<Vec<EnumeratedDevice>> {
            Ok(self.0.borrow().devices.clone())
        }

        fn open_path(&mut self, path: &CStr) -> std::result::Result<Box<dyn HidHandle>, OpenDeviceError> {
            let mut state = self.0.borrow_mut();
            let key = path.to_owned();
            state.opened_paths.push(key.clone());
            if let Some(queue) = state.open_results.get_mut(&key) {
                if !queue.is_empty() {
                    let res = if queue.len() > 1 {
                        queue.remove(0)
                    } else {
                        queue[0].clone()
                    };
                    return res.map(|h| Box::new(MockHandle(h)) as Box<dyn HidHandle>);
                }
            }
            Err(OpenDeviceError {
                message: format!("No mock handle configured for {:?}", path),
                permission_denied: false,
            })
        }

        fn wait(&self, duration: Duration) {
            self.0.borrow_mut().waits.push(duration);
        }
    }

    fn dev_entry(pid: u16, iface: i32, usage_page: u16, path: &str) -> EnumeratedDevice {
        EnumeratedDevice {
            vendor_id: VENDOR_ID,
            product_id: pid,
            interface_number: iface,
            usage_page,
            path: CString::new(path).unwrap(),
        }
    }

    fn sample_burst() -> ApplyBurst {
        ApplyBurst::build(
            &SensorConfig::default(),
            &PowerConfig::default(),
            PollingRate::Hz500,
            &ButtonConfig::default(),
        )
        .unwrap()
    }

    #[test]
    fn wired_usb_is_preferred_over_wireless_receiver_even_when_receiver_enumerates_first() {
        let state = Rc::new(RefCell::new(SharedMockState::default()));
        let receiver_handle = MockHandleState::new("receiver");
        let wired_handle = MockHandleState::new("wired");

        {
            let mut s = state.borrow_mut();
            s.devices = vec![
                dev_entry(PRODUCT_ID_24G, 2, CONFIG_USAGE_PAGE, "/dev/hidraw2"),
                dev_entry(PRODUCT_ID_WIRED, 2, CONFIG_USAGE_PAGE, "/dev/hidraw6"),
            ];
            s.open_results
                .insert(CString::new("/dev/hidraw2").unwrap(), vec![Ok(receiver_handle.clone())]);
            s.open_results
                .insert(CString::new("/dev/hidraw6").unwrap(), vec![Ok(wired_handle.clone())]);
        }

        let mut mgr = DeviceManager::with_backend(Box::new(MockBackend(state)));
        assert_eq!(mgr.refresh_connection(), ConnectionType::WiredUsb);
        mgr.send_burst(&sample_burst()).unwrap();

        assert!(receiver_handle.writes.borrow().is_empty(), "must not send wired burst to 2.4G receiver");
        assert_eq!(wired_handle.writes.borrow().len(), 5, "all 5 burst reports must go to wired USB");
    }

    #[test]
    fn unplugging_wired_cable_switches_open_handle_to_wireless_receiver() {
        let state = Rc::new(RefCell::new(SharedMockState::default()));
        let receiver_handle = MockHandleState::new("receiver");
        let wired_handle = MockHandleState::new("wired");

        {
            let mut s = state.borrow_mut();
            s.devices = vec![
                dev_entry(PRODUCT_ID_WIRED, 2, CONFIG_USAGE_PAGE, "/dev/hidraw6"),
                dev_entry(PRODUCT_ID_24G, 2, CONFIG_USAGE_PAGE, "/dev/hidraw2"),
            ];
            s.open_results
                .insert(CString::new("/dev/hidraw6").unwrap(), vec![Ok(wired_handle.clone())]);
            s.open_results
                .insert(CString::new("/dev/hidraw2").unwrap(), vec![Ok(receiver_handle.clone())]);
        }

        let mut mgr = DeviceManager::with_backend(Box::new(MockBackend(state.clone())));
        assert_eq!(mgr.refresh_connection(), ConnectionType::WiredUsb);

        // User unplugs the USB cable while leaving the 2.4G receiver plugged in,
        // and immediately clicks Apply before the 2s background poll fires.
        wired_handle.disconnect();
        state.borrow_mut().devices = vec![dev_entry(PRODUCT_ID_24G, 2, CONFIG_USAGE_PAGE, "/dev/hidraw2")];

        mgr.send_burst(&sample_burst()).unwrap();
        assert_eq!(mgr.connection_type, ConnectionType::Wireless24G);
        assert_eq!(receiver_handle.writes.borrow().len(), 5);
    }

    #[test]
    fn transient_open_failure_does_not_report_connected_and_recovers_on_next_refresh() {
        let state = Rc::new(RefCell::new(SharedMockState::default()));
        let good_handle = MockHandleState::new("receiver-ready");

        {
            let mut s = state.borrow_mut();
            s.devices = vec![dev_entry(PRODUCT_ID_24G, 2, CONFIG_USAGE_PAGE, "/dev/hidraw2")];
            s.open_results.insert(
                CString::new("/dev/hidraw2").unwrap(),
                vec![
                    Err(OpenDeviceError {
                        message: "Device or resource busy".into(),
                        permission_denied: false,
                    }),
                    Ok(good_handle.clone()),
                ],
            );
        }

        let mut mgr = DeviceManager::with_backend(Box::new(MockBackend(state)));
        // First refresh fails to open /dev/hidraw2 -> must NOT falsely claim Connected!
        assert_eq!(mgr.refresh_connection(), ConnectionType::Disconnected);

        // Next refresh succeeds and connects cleanly.
        assert_eq!(mgr.refresh_connection(), ConnectionType::Wireless24G);
        mgr.send_burst(&sample_burst()).unwrap();
        assert_eq!(good_handle.writes.borrow().len(), 5);
    }

    #[test]
    fn stale_same_path_handle_reconnects_automatically_during_send_burst() {
        let state = Rc::new(RefCell::new(SharedMockState::default()));
        let old_handle = MockHandleState::new("old-hidraw2");
        let new_handle = MockHandleState::new("new-hidraw2");

        {
            let mut s = state.borrow_mut();
            s.devices = vec![dev_entry(PRODUCT_ID_24G, 2, CONFIG_USAGE_PAGE, "/dev/hidraw2")];
            s.open_results.insert(
                CString::new("/dev/hidraw2").unwrap(),
                vec![Ok(old_handle.clone()), Ok(new_handle.clone())],
            );
        }

        let mut mgr = DeviceManager::with_backend(Box::new(MockBackend(state)));
        assert_eq!(mgr.refresh_connection(), ConnectionType::Wireless24G);

        // Simulate suspend/resume or USB reset recreating /dev/hidraw2 on the same path.
        old_handle.disconnect();

        mgr.send_burst(&sample_burst()).unwrap();
        assert_eq!(new_handle.writes.borrow().len(), 5);
    }

    #[test]
    fn stale_handle_detected_by_poll_battery_reconnects_immediately() {
        let state = Rc::new(RefCell::new(SharedMockState::default()));
        let old_status = MockHandleState::new("old-status");
        let new_status = MockHandleState::new("new-status");
        new_status
            .reads
            .borrow_mut()
            .push(Ok(vec![0x03, 0x10, 0x40, 0x01, 0x09]));

        let old_cfg = MockHandleState::new("old-cfg");
        let new_cfg = MockHandleState::new("new-cfg");

        {
            let mut s = state.borrow_mut();
            s.devices = vec![
                dev_entry(PRODUCT_ID_24G, 2, STATUS_USAGE_PAGE, "/dev/hidraw2-status"),
                dev_entry(PRODUCT_ID_24G, 2, CONFIG_USAGE_PAGE, "/dev/hidraw2-cfg"),
            ];
            s.open_results.insert(
                CString::new("/dev/hidraw2-status").unwrap(),
                vec![Ok(old_status.clone()), Ok(new_status.clone())],
            );
            s.open_results.insert(
                CString::new("/dev/hidraw2-cfg").unwrap(),
                vec![Ok(old_cfg.clone()), Ok(new_cfg.clone())],
            );
        }

        let mut mgr = DeviceManager::with_backend(Box::new(MockBackend(state)));
        assert_eq!(mgr.refresh_connection(), ConnectionType::Wireless24G);

        old_status.disconnect();
        old_cfg.disconnect();

        let battery = mgr.poll_battery();
        assert!(battery.is_some(), "poll_battery should drop dead handles, reconnect, and read battery");
        mgr.send_burst(&sample_burst()).unwrap();
        assert_eq!(new_cfg.writes.borrow().len(), 5);
    }

    #[test]
    fn permission_denied_and_disconnected_return_distinct_actionable_errors() {
        let state = Rc::new(RefCell::new(SharedMockState::default()));
        let mut mgr = DeviceManager::with_backend(Box::new(MockBackend(state.clone())));

        let disconnected_err = mgr.send_burst(&sample_burst()).unwrap_err().to_string();
        assert!(
            !disconnected_err.to_lowercase().contains("access denied"),
            "disconnected error must not claim access denied: {disconnected_err}"
        );

        {
            let mut s = state.borrow_mut();
            s.devices = vec![dev_entry(PRODUCT_ID_24G, 2, CONFIG_USAGE_PAGE, "/dev/hidraw2")];
            s.open_results.insert(
                CString::new("/dev/hidraw2").unwrap(),
                vec![Err(OpenDeviceError {
                    message: "Permission denied".into(),
                    permission_denied: true,
                })],
            );
        }

        let perm_err = mgr.send_burst(&sample_burst()).unwrap_err().to_string();
        assert_eq!(mgr.connection_type, ConnectionType::PermissionDenied);
        assert!(
            perm_err.to_lowercase().contains("access denied") || perm_err.to_lowercase().contains("permission"),
            "permission error must clearly report access denied: {perm_err}"
        );
        assert_ne!(disconnected_err, perm_err);
    }

    #[test]
    fn upload_macro_reconnects_when_cached_handle_is_stale() {
        let state = Rc::new(RefCell::new(SharedMockState::default()));
        let old_handle = MockHandleState::new("old-macro");
        let new_handle = MockHandleState::new("new-macro");

        {
            let mut s = state.borrow_mut();
            s.devices = vec![dev_entry(PRODUCT_ID_24G, 2, CONFIG_USAGE_PAGE, "/dev/hidraw2")];
            s.open_results.insert(
                CString::new("/dev/hidraw2").unwrap(),
                vec![Ok(old_handle.clone()), Ok(new_handle.clone())],
            );
        }

        let mut mgr = DeviceManager::with_backend(Box::new(MockBackend(state)));
        assert_eq!(mgr.refresh_connection(), ConnectionType::Wireless24G);
        old_handle.disconnect();

        let mut dev_macro = DeviceMacro {
            slot: 2,
            ..Default::default()
        };
        dev_macro.events.push(MacroEvent::press(0x06));
        dev_macro.events.push(MacroEvent::release(0x06));

        mgr.upload_macro(&dev_macro).unwrap();
        assert_eq!(new_handle.writes.borrow().len(), 3);
    }
}
