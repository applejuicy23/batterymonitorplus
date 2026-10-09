"""Recon: which connected devices expose a battery level, and how.

1. Bluetooth devices — Windows keeps their charge in a PnP property
   (the % shown in Settings → Bluetooth).
2. HID devices — what is plugged in (vendor / product / interface).
3. Razer — ask each Razer interface for its battery via the OpenRazer
   protocol (read-only "get battery" / "get charging" commands).

Run:  .venv\\Scripts\\python.exe probe.py
"""
from __future__ import annotations

import json
import subprocess
import sys

import hid

RAZER_VID = 0x1532
_BT_BATTERY_KEY = "{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2"


def bluetooth_batteries() -> list[dict]:
    ps = rf"""
$ErrorActionPreference = 'SilentlyContinue'
Get-PnpDevice -PresentOnly |
  Where-Object {{ $_.InstanceId -match '^(BTHENUM|BTHLE|BTHLEDEVICE)' }} |
  ForEach-Object {{
    $p = Get-PnpDeviceProperty -InstanceId $_.InstanceId -KeyName '{_BT_BATTERY_KEY}'
    if ($p -and $p.Data -ne $null) {{
      [pscustomobject]@{{ name = $_.FriendlyName; id = $_.InstanceId; battery = [int]$p.Data }}
    }}
  }} | ConvertTo-Json -Compress
"""
    out = subprocess.run(
        ["powershell", "-NoProfile", "-Command", ps],
        capture_output=True,
        text=True,
        timeout=60,
    ).stdout.strip()
    if not out:
        return []
    data = json.loads(out)
    return data if isinstance(data, list) else [data]


def hid_devices() -> list[dict]:
    seen = {}
    for d in hid.enumerate():
        key = (d["vendor_id"], d["product_id"], d["interface_number"], d["usage_page"], d["usage"])
        seen[key] = d
    return sorted(seen.values(), key=lambda d: (d["vendor_id"], d["product_id"], d["interface_number"]))


def _razer_report(transaction_id: int, command_class: int, command_id: int, size: int) -> bytes:
    """90-byte Razer feature report (OpenRazer ``razer_report`` layout)."""
    buf = bytearray(90)
    buf[1] = transaction_id
    buf[5] = size
    buf[6] = command_class
    buf[7] = command_id
    crc = 0
    for b in buf[2:88]:
        crc ^= b
    buf[88] = crc
    return bytes(buf)


def razer_query(path: bytes, command_id: int) -> tuple[int, int] | None:
    """Return (transaction_id, raw 0..255 value) or None."""
    dev = hid.device()
    try:
        dev.open_path(path)
    except OSError:
        return None
    try:
        for tid in (0x1F, 0xFF, 0x3F, 0x9F):
            try:
                dev.send_feature_report(b"\x00" + _razer_report(tid, 0x07, command_id, 0x02))
                import time

                time.sleep(0.06)
                resp = bytes(dev.get_feature_report(0, 91))
            except OSError:
                continue
            if len(resp) >= 91:
                resp = resp[1:]
            # status 0x02 = success; args start at byte 8, value in args[1]
            if len(resp) >= 10 and resp[0] == 0x02 and resp[6] == 0x07 and resp[7] == command_id:
                return tid, resp[9]
        return None
    finally:
        dev.close()


def main() -> int:
    print("=== Bluetooth (Windows battery property) ===")
    bt = bluetooth_batteries()
    if not bt:
        print("  nothing reports a battery level")
    for d in bt:
        print(f"  {d['battery']:>3}%  {d['name']}")

    print("\n=== HID devices ===")
    devices = hid_devices()
    for d in devices:
        name = " ".join(x for x in (d["manufacturer_string"], d["product_string"]) if x) or "?"
        print(
            f"  {d['vendor_id']:04x}:{d['product_id']:04x}  if={d['interface_number']:<2} "
            f"usage={d['usage_page']:04x}/{d['usage']:04x}  {name}"
        )

    print("\n=== Razer battery (OpenRazer protocol) ===")
    razer = [d for d in devices if d["vendor_id"] == RAZER_VID]
    if not razer:
        print("  no Razer HID device")
    done = set()
    for d in razer:
        pid = d["product_id"]
        if pid in done:
            continue
        hit = razer_query(d["path"], 0x80)
        if hit is None:
            continue
        tid, raw = hit
        charging = razer_query(d["path"], 0x84)
        done.add(pid)
        pct = round(raw / 255 * 100)
        chg = "" if charging is None else ("  charging" if charging[1] else "  on battery")
        print(
            f"  {pid:04x} if={d['interface_number']} tid=0x{tid:02x}: "
            f"{pct}% (raw {raw}){chg}  {d['product_string']}"
        )
    if razer and not done:
        print("  Razer device found, but no interface answered the battery query")
    return 0


if __name__ == "__main__":
    sys.exit(main())
