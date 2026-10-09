//! Bluetooth battery levels as Windows reports them (Settings → Bluetooth).
//!
//! Windows stores the charge in a device-node property
//! `{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2` (one byte, 0..=100) on the
//! BTHENUM / BTHLE nodes. We walk all present device nodes with SetupAPI and
//! read that property directly instead of shelling out to PowerShell.

use std::collections::HashSet;

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW,
    SetupDiGetDevicePropertyW,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_DeviceDesc, DEVPKEY_Device_FriendlyName, DEVPROPTYPE,
};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::core::{GUID, PCWSTR};

use crate::{Reading, Source};

const BATTERY_KEY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0x104EA319_6EE2_4701_BD47_8DDBF425BBE5),
    pid: 2,
};

const PREFIXES: [&str; 3] = ["BTHENUM\\", "BTHLE\\", "BTHLEDEVICE\\"];

struct DevInfoList(HDEVINFO);

impl Drop for DevInfoList {
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

/// Walks every present Bluetooth device node and collects the charge Windows
/// knows for it, one reading per physical device (deduplicated by MAC).
pub fn read_bluetooth_batteries() -> windows::core::Result<Vec<Reading>> {
    let list = unsafe {
        DevInfoList(SetupDiGetClassDevsW(
            None,
            PCWSTR::null(),
            None,
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        )?)
    };

    let mut out = Vec::new();
    let mut seen = HashSet::new();
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
        if !PREFIXES.iter().any(|p| upper.starts_with(p)) {
            continue;
        }
        let Some(percent) =
            read_raw_property(&list, &info, &BATTERY_KEY).and_then(|b| b.first().copied())
        else {
            continue;
        };
        let address = extract_mac_address(&upper).unwrap_or(upper);
        if !seen.insert(address.clone()) {
            continue;
        }
        let name = read_string_property(&list, &info, &DEVPKEY_Device_FriendlyName)
            .or_else(|| read_string_property(&list, &info, &DEVPKEY_Device_DeviceDesc))
            .unwrap_or_else(|| address.clone());
        out.push(Reading {
            id: format!("bt:{address}"),
            name,
            source: Source::Bluetooth,
            percent: percent.min(100),
            charging: None,
        });
    }
    Ok(out)
}

/// Reads the device's instance id, e.g. `BTHLE\DEV_D0B9C1A2B3C4\...`.
fn read_instance_id(list: &DevInfoList, info: &SP_DEVINFO_DATA) -> Option<String> {
    let mut buf = [0u16; 512];
    let mut len = 0u32;
    unsafe { SetupDiGetDeviceInstanceIdW(list.0, info, Some(&mut buf), Some(&mut len)) }.ok()?;
    let len = (len as usize).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
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
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::extract_mac_address;

    /// Checks that the MAC is found in both BLE and classic Bluetooth ids.
    #[test]
    fn extracts_mac_from_ble_and_classic_ids() {
        assert_eq!(
            extract_mac_address(r"BTHLE\DEV_D0B9C1A2B3C4\7&1A2B3C4D&0&D0B9C1A2B3C4").as_deref(),
            Some("D0B9C1A2B3C4")
        );
        assert_eq!(
            extract_mac_address(r"BTHENUM\{0000110E-0000-1000-8000-00805F9B34FB}_LOCALMFG&0002\7&2F1E&0&A1B2C3D4E5F6_C00000000")
                .as_deref(),
            Some("A1B2C3D4E5F6")
        );
    }
}
