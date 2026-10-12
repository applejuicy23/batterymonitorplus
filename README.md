# BatteryMonitor+

Customizable Windows 11 taskbar tiles showing your peripherals' battery — no hovering, no vendor apps.

> Work in progress. Right now there is a tray app with a settings window, a device detector and a console tool; the taskbar tiles are next.

## The app

```powershell
cd app
npm install
npm run tauri dev
```

It lives in the tray: left click opens the settings window, the menu has *Refresh now* and *Quit*, and the tooltip lists every shown device with its charge. Closing the window only hides it. Batteries are re-read in the background (every minute by default, configurable in the window).

The settings window previews the tiles and lists every device, where you can show or hide it, rename it, change its kind, reorder it, merge it into another device or reset it.

## What it detects

- **This PC** — the laptop battery, when there is one.
- **Bluetooth devices** — every paired device, with the same charge Windows shows in Settings → Bluetooth, and whether it is connected right now (so a stale value from a switched-off device isn't shown as live).
- **Razer devices** over the 2.4 GHz dongle or cable, using the OpenRazer HID protocol (read-only commands).
- **YUNZII keyboards** over their 2.4 GHz dongle (`056D:C077`). The dongle reports the value the keyboard sent when it last connected.

Each device gets a kind (mouse, keyboard, headset, gamepad, laptop, other) that picks its default icon.

## Choosing what the tiles show

Settings live in `%APPDATA%\BatteryMonitorPlus\config.toml`. Besides the settings window, `bmp-probe`
can change them from the console (devices can be picked by id or by part of the name):

```powershell
cargo run -p bmp-probe                          # all devices, shown or hidden
cargo run -p bmp-probe -- tiles                 # only what the tiles will show
cargo run -p bmp-probe -- tiles --json          # the same, as JSON
cargo run -p bmp-probe -- hide yunzii
cargo run -p bmp-probe -- show yunzii
cargo run -p bmp-probe -- rename viper "Mouse"
cargo run -p bmp-probe -- kind yunzii keyboard
cargo run -p bmp-probe -- order viper 1
cargo run -p bmp-probe -- link E78E yunzii      # same device from two sources, one tile
cargo run -p bmp-probe -- unlink E78E           # keep it as its own tile
cargo run -p bmp-probe -- reset viper           # forget your settings for it
```

By default every connected device that reports a battery is shown; a switched-off device with an old value stays hidden unless you `show` it.

The same device seen by two sources (e.g. a keyboard paired over Bluetooth that also has a 2.4 GHz dongle) is merged into one tile automatically when the kind matches and the names match ignoring case and spaces. The tile takes the freshest battery value of the two.

## Build

Requires Rust (MSVC toolchain), Visual Studio Build Tools with the C++ workload, and Node.js for the app.

| Crate | What it does |
|---|---|
| `bmp-battery` | detects devices and reads their battery |
| `bmp-config` | the user's per-device settings |
| `bmp-probe` | console tool for both |
| `bmp-app` (`app/`) | the Tauri tray app and settings window (Svelte) |

## License

[GPL-3.0](LICENSE)
