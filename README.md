# KC Control

Automatically switches your Keychron mouse profile based on the app you're using.

KC Control lives in the system tray. When an app comes to the front, it switches the mouse to the profile you assigned to that app. For example, Photoshop can use one profile, games another, and everything else your everyday profile. It talks to the mouse directly over HID, so you don't need the Keychron Launcher open.

> **Unofficial.** This project isn't affiliated with or endorsed by Keychron. It relies on a reverse-engineered protocol. Use it at your own risk.

## Features

- **Per-app profiles:** map any app to one of the mouse's onboard profiles.
- **Default profile:** choose a profile for all other apps, or leave the profile unchanged.
- **Pick apps easily:** click a recently used app, type an exe name, or browse for the program.
- **Profile names:** name your profiles. The mouse doesn't store names, so KC Control saves them locally.
- **Respects the profile button:** a profile change made on the mouse sticks until you switch apps again.
- **Handles sleep and unplugging:** it waits out a sleeping mouse, reconnects after the receiver is unplugged, and debounces fast alt-tabbing.
- **Start at sign-in:** optionally launch KC Control when you sign in.
- **Light on resources:** a native Rust + Tauri app with a small footprint, no Electron.

## Supported hardware

| Device | Status |
|---|---|
| Keychron M6 (`3434:D060`) via 2.4 GHz receiver | ✅ Tested |
| Keychron M6 wired / Bluetooth | Untested |
| Other Keychron mice using the same `"8k"` protocol (vendor HID usage page `0xFFC1`) | Probably works, untested |

The app finds the mouse by Keychron's vendor ID and the `0xFFC1` HID collection, so other models on the same protocol should be picked up automatically.

## Platforms

| OS | Status |
|---|---|
| Windows 10/11 | ✅ Supported |
| macOS | Planned. Only the focus watcher is missing. |
| Linux (X11) | Planned |
| Linux (Wayland) | Needs a compositor extension (GNOME/KDE) to report the focused app |

## Install

Download the installer from [Releases](../../releases), or build it yourself:

### Build from source

Requirements:

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+ (only for the Tauri CLI)
- Windows: the [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and WebView2 (preinstalled on Windows 11)

```sh
npm install
npm run dev     # run a debug build with logs
npm run build   # produce an installer in src-tauri/target/release/bundle/
```

## Usage

1. Start KC Control. It appears in the system tray.
2. Click the tray icon to open the settings window.
3. Add a rule: type an app's exe name (e.g. `photoshop.exe`), click a recently used app, or **Browse…** for the program. Then pick a profile.
4. Choose what **All other apps** should use.

Settings are saved to `%APPDATA%\com.kc-control.desktop\config.json`.

To quit, right-click the tray icon and choose **Quit**. Closing the window only hides it.

### Close the Keychron Launcher

Both apps can talk to the mouse at the same time and see each other's replies. Close the Launcher tab while KC Control is running to avoid confusing results.

## How it works

```
focus watcher (per OS) ──app id──▶ switcher ──HID──▶ mouse
                                     ▲  │
                     settings window ┘  └─ status ▶ tray + window
```

| Path | Role |
|---|---|
| `src-tauri/src/mouse.rs` | Keychron HID protocol over [hidapi](https://github.com/ruabmbua/hidapi-rs), cross-platform |
| `src-tauri/src/switcher.rs` | Background worker: rules, debounce, reconnects, retries while the mouse is asleep |
| `src-tauri/src/focus/` | Foreground-app detection. `windows.rs` uses `SetWinEventHook`. Add `macos.rs` / `linux.rs` here. |
| `src-tauri/src/config.rs` | Rules and profile names |
| `src-tauri/src/main.rs` | Tauri wiring: tray, window, commands |
| `ui/` | Settings window (plain HTML/CSS/JS, follows the system light/dark theme) |
| `ref/keychron-m6-hid-api.md` | The reverse-engineered protocol reference |

Switching profiles uses two commands: base info (`0x06`) reads the current profile and the profile count, and select profile (`0x0E`) changes it. See the [protocol reference](ref/keychron-m6-hid-api.md) for everything else the mouse supports.

### Does frequent switching wear out the mouse?

Not in practice. The M6 stores its settings in the internal flash of its Telink TLSR8273 chip, which is reportedly rated for about 100k erase cycles. Even if every switch rewrote the same sector, 10 switches a day would take decades to reach that. KC Control only sends a command when the profile actually needs to change.

## Contributing

Issues and pull requests are welcome, especially:

- macOS and Linux focus watchers (`src-tauri/src/focus/`)
- Reports from other Keychron mice. Please include the model, the VID/PID, and whether switching works.

## License

[MIT](LICENSE)

Keychron is a trademark of its respective owner.
