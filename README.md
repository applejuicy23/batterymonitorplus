# BatteryMonitor+

Customizable Windows 11 taskbar tiles showing your peripherals' battery — no hovering, no vendor apps.

> Work in progress. Right now there is a device detector with per-device settings and a console tool; the taskbar tiles are next.

## What it detects

- **This PC** — the laptop battery, when there is one.
- **Bluetooth devices** — every paired device, with the same charge Windows shows in Settings → Bluetooth, and whether it is connected right now (so a stale value from a switched-off device isn't shown as live).
- **Razer devices** over the 2.4 GHz dongle or cable, using the OpenRazer HID protocol (read-only commands).
- **YUNZII keyboards** over their 2.4 GHz dongle (`056D:C077`). The dongle reports the value the keyboard sent when it last connected.

Each device gets a kind (mouse, keyboard, headset, gamepad, laptop, other) that picks its default icon.

## Choosing what the tiles show

Settings live in `%APPDATA%\BatteryMonitorPlus\config.toml`. Until there's a UI, use `bmp-probe`
(devices can be picked by id or by part of the name):

```powershell
cargo run -p bmp-probe                          # all devices, shown or hidden
cargo run -p bmp-probe -- tiles                 # only what the tiles will show
cargo run -p bmp-probe -- tiles --json          # the same, as JSON
cargo run -p bmp-probe -- hide yunzii
cargo run -p bmp-probe -- show yunzii
cargo run -p bmp-probe -- rename viper "Mouse"
cargo run -p bmp-probe -- kind yunzii keyboard
cargo run -p bmp-probe -- order viper 1
cargo run -p bmp-probe -- reset viper           # forget your settings for it
```

By default every connected device that reports a battery is shown; a switched-off device with an old value stays hidden unless you `show` it.

## Build

Requires Rust (MSVC toolchain) and Visual Studio Build Tools with the C++ workload.

| Crate | What it does |
|---|---|
| `bmp-battery` | detects devices and reads their battery |
| `bmp-config` | the user's per-device settings |
| `bmp-probe` | console tool for both |

## License

[GPL-3.0](LICENSE)
