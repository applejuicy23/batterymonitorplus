//! The PC's own battery ("This PC"), present on laptops only.

use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

use crate::{Device, DeviceKind, Source, unix_now};

const NO_SYSTEM_BATTERY: u8 = 128;
const CHARGING: u8 = 8;
const UNKNOWN_PERCENT: u8 = 255;

/// Asks Windows for the laptop battery; `None` on desktops or when unknown.
pub fn read_system_battery() -> Option<Device> {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    if status.BatteryFlag & NO_SYSTEM_BATTERY != 0 || status.BatteryLifePercent == UNKNOWN_PERCENT {
        return None;
    }
    Some(Device {
        id: "system:battery".to_owned(),
        name: "This PC".to_owned(),
        kind: DeviceKind::Laptop,
        source: Source::System,
        battery: Some(status.BatteryLifePercent.min(100)),
        charging: Some(status.BatteryFlag & CHARGING != 0),
        connected: Some(true),
        updated_at: Some(unix_now()),
    })
}
