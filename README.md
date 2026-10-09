# BatteryMonitor+

Customizable Windows 11 taskbar tiles showing your peripherals' battery — no hovering, no vendor apps.

> Work in progress. Right now there is a battery reader and a console probe; the taskbar tiles are next.

## What it reads

- **Bluetooth devices** — the same charge Windows shows in Settings → Bluetooth.
- **Razer devices** over the 2.4 GHz dongle or cable, using the OpenRazer HID protocol (read-only commands).

## Build

Requires Rust (MSVC toolchain) and Visual Studio Build Tools with the C++ workload.

```powershell
cargo run -p bmp-battery --bin bmp-probe            # table
cargo run -p bmp-battery --bin bmp-probe -- --json  # JSON
```

## License

[GPL-3.0](LICENSE)
