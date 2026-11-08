//! Device connection and HID interaction module.

pub mod manager;
mod transport;

pub use manager::{ConnectionType, DeviceManager};
