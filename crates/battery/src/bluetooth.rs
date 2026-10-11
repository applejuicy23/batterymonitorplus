//! Bluetooth peripherals as Windows knows them (Settings → Bluetooth).
//!
//! Paired devices are `BTHENUM\DEV_<mac>` (classic) or `BTHLE\DEV_<mac>`
//! (Low Energy) device nodes. The charge Windows shows is the property
//! `{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2` (one byte, 0..=100) and
//! `... 7` is when it was last updated. On classic devices it sits on a
//! service node (e.g. hands-free) rather than the `DEV_` node, so every node
//! is matched to its device by MAC address.
//!
//! The `DEV_` node stays "present" while the device is off, so "connected
//! right now" is asked from WinRT. The device kind comes from the HID/audio
//! nodes Windows groups into the same container, falling back to the name.

use std::collections::{BTreeMap, HashMap, HashSet};

use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice, BluetoothLEDevice};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW,
    SetupDiGetDevicePropertyW,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_Class, DEVPKEY_Device_ContainerId, DEVPKEY_Device_DeviceDesc,
    DEVPKEY_Device_FriendlyName, DEVPROPTYPE,
};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::core::{GUID, PCWSTR};

use crate::kind::{guess_kind_from_classes, guess_kind_from_name};
use crate::{Device, Source};

const BATTERY_KEY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0x104EA319_6EE2_4701_BD47_8DDBF425BBE5),
    pid: 2,
};
const BATTERY_UPDATED_KEY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0x104EA319_6EE2_4701_BD47_8DDBF425BBE5),
    pid: 7,
};

const BLUETOOTH_PREFIXES: [&str; 3] = ["BTHENUM\\", "BTHLE\\", "BTHLEDEVICE\\"];
const CLASSIC_DEVICE_PREFIX: &str = "BTHENUM\\DEV_";
const LOW_ENERGY_DEVICE_PREFIX: &str = "BTHLE\\DEV_";

/// Seconds between 1601-01-01 (FILETIME epoch) and 1970-01-01.
const FILETIME_UNIX_OFFSET: u64 = 11_644_473_600;

struct DevInfoList(HDEVINFO);

impl Drop for DevInfoList {
    /// Frees the SetupAPI device list.
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

/// A paired device node (`BTHENUM\DEV_...` / `BTHLE\DEV_...`).
struct PairedNode {
    name: String,
    low_energy: bool,
    container: Option<[u8; 16]>,
}

/// A battery value found on any node of a device.
struct BatteryValue {
    percent: u8,
    updated_at: Option<u64>,
}

/// Everything collected in one pass over the present device nodes.
#[derive(Default)]
struct NodeScan {
    paired: BTreeMap<String, PairedNode>,
    battery_by_mac: HashMap<String, BatteryValue>,
    classes_by_container: HashMap<[u8; 16], HashSet<String>>,
}

/// Finds every paired Bluetooth device, with its charge when Windows knows it,
/// whether it is connected right now, and a best guess of its kind.
pub fn detect_bluetooth_devices() -> windows::core::Result<Vec<Device>> {
    let scan = scan_device_nodes()?;
    let mut out = Vec::new();
    for (mac, node) in scan.paired {
        let battery = scan.battery_by_mac.get(&mac);
        let kind = node
            .container
            .and_then(|c| scan.classes_by_container.get(&c))
            .and_then(guess_kind_from_classes)
            .unwrap_or_else(|| guess_kind_from_name(&node.name));
        out.push(Device {
            id: format!("bt:{mac}"),
            name: node.name,
            kind,
            source: Source::Bluetooth,
            battery: battery.map(|b| b.percent),
            charging: None,
            connected: query_bluetooth_connection(&mac, node.low_energy),
            updated_at: battery.and_then(|b| b.updated_at),
        });
    }
    Ok(out)
}

/// Walks all present device nodes once and collects paired Bluetooth devices,
/// battery values by MAC, and device classes by container.
fn scan_device_nodes() -> windows::core::Result<NodeScan> {
    let list = unsafe {
        DevInfoList(SetupDiGetClassDevsW(
            None,
            PCWSTR::null(),
            None,
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        )?)
    };

    let mut scan = NodeScan::default();
    for index in 0.. {
        let mut info = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        if unsafe { SetupDiEnumDeviceInfo(list.0, index, &mut info) }.is_err() {
            break;
        }
        let Some(instance_id) = read_instance_id(&list, &info) else {
            continue;
        };
        let upper = instance_id.to_ascii_uppercase();
        let container = read_container_id(&list, &info);

        if let (Some(container), Some(class)) = (
            container,
            read_string_property(&list, &info, &DEVPKEY_Device_Class),
        ) {
            scan.classes_by_container
                .entry(container)
                .or_default()
                .insert(class);
        }

        if !BLUETOOTH_PREFIXES.iter().any(|p| upper.starts_with(p)) {
            continue;
        }
        let Some(mac) = extract_mac_address(&upper) else {
            continue;
        };

        if let Some(percent) = read_battery_percent(&list, &info) {
            let updated_at = read_filetime_property(&list, &info, &BATTERY_UPDATED_KEY);
            let newer = scan
                .battery_by_mac
                .get(&mac)
                .is_none_or(|old| updated_at > old.updated_at);
            if newer {
                scan.battery_by_mac.insert(
                    mac.clone(),
                    BatteryValue {
                        percent,
                        updated_at,
                    },
                );
            }
        }

        let low_energy = upper.starts_with(LOW_ENERGY_DEVICE_PREFIX);
        if low_energy || upper.starts_with(CLASSIC_DEVICE_PREFIX) {
            let name = read_string_property(&list, &info, &DEVPKEY_Device_FriendlyName)
                .or_else(|| read_string_property(&list, &info, &DEVPKEY_Device_DeviceDesc))
                .unwrap_or_else(|| mac.clone());
            scan.paired.insert(
                mac,
                PairedNode {
                    name,
                    low_energy,
                    container,
                },
            );
        }
    }
    Ok(scan)
}

/// Asks WinRT whether the device is connected right now; `None` if Windows
/// can't tell (e.g. the Bluetooth radio is off).
fn query_bluetooth_connection(mac: &str, low_energy: bool) -> Option<bool> {
    let address = u64::from_str_radix(mac, 16).ok()?;
    let status = if low_energy {
        BluetoothLEDevice::FromBluetoothAddressAsync(address)
            .ok()?
            .join()
            .ok()?
            .ConnectionStatus()
            .ok()?
    } else {
        BluetoothDevice::FromBluetoothAddressAsync(address)
            .ok()?
            .join()
            .ok()?
            .ConnectionStatus()
            .ok()?
    };
    Some(status == BluetoothConnectionStatus::Connected)
}

/// Reads the device's instance id, e.g. `BTHLE\DEV_D0B9C1A2B3C4\...`.
fn read_instance_id(list: &DevInfoList, info: &SP_DEVINFO_DATA) -> Option<String> {
    let mut buf = [0u16; 512];
    let mut len = 0u32;
    unsafe { SetupDiGetDeviceInstanceIdW(list.0, info, Some(&mut buf), Some(&mut len)) }.ok()?;
    let len = (len as usize).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// Reads the charge Windows stored for this node, 0..=100.
fn read_battery_percent(list: &DevInfoList, info: &SP_DEVINFO_DATA) -> Option<u8> {
    let bytes = read_raw_property(list, info, &BATTERY_KEY)?;
    bytes.first().map(|&p| p.min(100))
}

/// Reads the container id that groups all nodes of one physical device.
fn read_container_id(list: &DevInfoList, info: &SP_DEVINFO_DATA) -> Option<[u8; 16]> {
    let bytes = read_raw_property(list, info, &DEVPKEY_Device_ContainerId)?;
    bytes.try_into().ok()
}

/// Reads a FILETIME property and converts it to unix seconds.
fn read_filetime_property(
    list: &DevInfoList,
    info: &SP_DEVINFO_DATA,
    key: &DEVPROPKEY,
) -> Option<u64> {
    let bytes: [u8; 8] = read_raw_property(list, info, key)?.try_into().ok()?;
    let ticks = u64::from_le_bytes(bytes);
    (ticks / 10_000_000).checked_sub(FILETIME_UNIX_OFFSET)
}

/// Reads one device property as raw bytes; `None` if the device doesn't have it.
fn read_raw_property(
    list: &DevInfoList,
    info: &SP_DEVINFO_DATA,
    key: &DEVPROPKEY,
) -> Option<Vec<u8>> {
    let mut kind = DEVPROPTYPE::default();
    let mut buf = vec![0u8; 512];
    let mut len = 0u32;
    unsafe {
        SetupDiGetDevicePropertyW(
            list.0,
            info,
            key,
            &mut kind,
            Some(&mut buf),
            Some(&mut len),
            0,
        )
    }
    .ok()?;
    buf.truncate(len as usize);
    Some(buf)
}

/// Reads a text property (UTF-16 in Windows) and turns it into a normal string.
fn read_string_property(
    list: &DevInfoList,
    info: &SP_DEVINFO_DATA,
    key: &DEVPROPKEY,
) -> Option<String> {
    let bytes = read_raw_property(list, info, key)?;
    let (pairs, _) = bytes.as_chunks::<2>();
    let wide: Vec<u16> = pairs
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .take_while(|&c| c != 0)
        .collect();
    let s = String::from_utf16_lossy(&wide);
    (!s.is_empty()).then_some(s)
}

/// Pulls the device's MAC out of its instance id: the last 12-hex-digit token,
/// e.g. `D0B9C1A2B3C4` from `BTHLE\DEV_D0B9C1A2B3C4\...`.
fn extract_mac_address(instance_id: &str) -> Option<String> {
    instance_id
        .split(['\\', '_', '&'])
        .rfind(|t| t.len() == 12 && t.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_ascii_uppercase)
}

#[cfg(test)]
mod tests {
    use super::extract_mac_address;

    /// Checks that the MAC is found in BLE, BLE service and classic ids.
    #[test]
    fn extracts_mac_from_ble_and_classic_ids() {
        assert_eq!(
            extract_mac_address(r"BTHLE\DEV_D0B9C1A2B3C4\7&1A2B3C4D&0&D0B9C1A2B3C4").as_deref(),
            Some("D0B9C1A2B3C4")
        );
        assert_eq!(
            extract_mac_address(
                r"BTHLEDEVICE\{0000180F-0000-1000-8000-00805F9B34FB}_DEV_VID&0228E9_PID&2000_REV&0110_E78E379B2DB7\D&F238B83&0&000C"
            )
            .as_deref(),
            Some("E78E379B2DB7")
        );
        assert_eq!(
            extract_mac_address(
                r"BTHENUM\{0000110E-0000-1000-8000-00805F9B34FB}_LOCALMFG&0002\7&2F1E&0&A1B2C3D4E5F6_C00000000"
            )
            .as_deref(),
            Some("A1B2C3D4E5F6")
        );
    }
}
