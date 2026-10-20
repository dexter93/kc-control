# Keychron M6 — WebHID Protocol Reference

Reverse-engineered from the Keychron Launcher web bundle (`kcm6main.txt`, `2.js`) and a live
capture from a Keychron M6 (VID `0x3434`, PID `0xD060`, firmware 1.0.3, connected through the
2.4 GHz receiver).

Line references (`kcm6main.txt:NNNNN`) point at the minified bundle so each claim can be re-checked.
The bundles themselves are Keychron's code and aren't included in this repository; save the launcher's
JavaScript from your browser's DevTools if you want to follow the references.

> **Status legend**
> - ✅ **Verified** — seen in the live capture from the test M6.
> - 📖 **From code** — read from the launcher source for the `"8k"` protocol, not yet tested on hardware.
> - ⚠️ **Partial** — the meaning of some bytes is inferred, see the note.

---

## 1. Transport

| Item | Value |
|---|---|
| Vendor ID | `0x3434` |
| Product ID | `0xD060` (Keychron M6). The M6 8K is `0xD049`. |
| HID collection | `usagePage 0xFFC1` (65473), `usage 0x01` |
| Launcher protocol name | `"8k"` (the default branch in every command class) |

The launcher chooses the protocol from the collection the mouse exposes (`kcm6main.txt:132386`):
`0xFFC1` → `"8k"`, `0xFF0A` → `"4k"` or `"8k_nordic"` (probed), `usagePage 140` → `"1k"` (dongle bridge).
A per-model JSON (`https://launcher.keychron.com/…/device/<vid<<16|pid>/json/v3.json`) can override it.
This document covers only `"8k"`, which is what the M6 uses.

### Report IDs ✅

| Direction | Report ID | Payload size | Used for |
|---|---|---|---|
| OUT | `0xB5` (181) | 20 bytes | "Set"-style commands and some reads |
| IN  | `0xB6` (182) | 63 bytes | Replies to `0xB5` |
| OUT | `0xB3` (179) | 63 bytes | "Get"-style commands, button/macro writes |
| IN  | `0xB4` (180) | 63 bytes | Replies to `0xB3` |

Byte 0 of every OUT payload is the **command opcode**. Pad the rest with zeros.
With WebHID, call `device.sendReport(reportId, payload)`. The report ID is **not** part of `payload`.
With hidapi or other native libraries, prepend the report ID as the first byte you write.

### Generic acknowledgement ✅

Most set commands are acknowledged with:

```
E4 <status> <opcode> …
```

- `status` = `00` means success. The pairing-combo setter checks this byte explicitly.
- `opcode` echoes the command you sent (e.g. `E4 00 0E` after a profile switch).

Some commands answer with their own opcode in byte 0 instead (`0x40`, `0x41`), so the launcher
accepts either `E4 …` or `<opcode> …` for those.

### Flow control

On wireless (`workMode 1`) the launcher uses a serial queue (`kcm6main.txt:93548`, class `T`):
- It keeps one command in flight at a time.
- It waits for any input report before sending the next command.
- It times out after 1000 ms and retries up to 3 times.

Copy that behaviour. If you fire commands back to back, the receiver drops them.

---

## 2. Command index

| Opcode | Report | Name | Section |
|---|---|---|---|
| `0x02` | B5 → B6 | Device info / protocol version / feature flags | [3.1](#31-device-info-0x02) |
| `0x03` | B5 → B6 | Receiver / bonded device list | [3.2](#32-receiver-state-0x03) |
| `0x04` | B3 → B4 | Firmware version string | [3.3](#33-firmware-version-string-0x04) |
| `0x06` | B3 → B4 | **Base info** (profile, DPI, polling, battery, settings…) | [3.4](#34-base-info-0x06) |
| `0x08` | B5 | Shell / wheel colour material IDs | [3.18](#318-colour-materials-0x08) |
| `0x0A` | B5 → B6 | Sleep timeout get/set | [3.13](#313-sleep-timeout-0x0a) |
| `0x0B` | B5 → B6 | Receiver pairing button combo get/set | [3.14](#314-pairing-button-combo-0x0b) |
| `0x0E` | B5 → B6 | **Select profile** / set profile count | [3.5](#35-select-profile-0x0e) |
| `0x0F` | B5 → B6 | Factory reset (selective) | [3.15](#315-factory-reset-0x0f) |
| `0x23` / `0x24` | B5 | Lighting get / set | [3.12](#312-lighting-0x23--0x24) |
| `0x26` | B5 → B6 | Indicator setting get/set | [3.17](#317-indication-0x26) |
| `0x40` | B5 → B6 | DPI levels set | [3.7](#37-dpi-0x40) |
| `0x41` | B5 → B6 | Polling rate set | [3.8](#38-polling-rate-0x41) |
| `0x42` | B5 → B6 | System features (LOD, sensor toggles, scroll direction, angle) | [3.10](#310-system-features--angle-snapping-0x42) |
| `0x43` | B5 → B6 | Debounce | [3.9](#39-debounce-0x43) |
| `0x45` | B5 | Scroll wheel parameters | [3.11](#311-scroll-wheel-0x45) |
| `0x48` / `0x49` | B3 → B4 | DPI with separate X/Y (set / get) | [3.7.1](#371-separate-xy-dpi-0x48--0x49) |
| `0x4A` / `0x4B` | B5 → B6 | Polling rate per connection mode (set / get) | [3.8.1](#381-per-mode-polling-rate-0x4a--0x4b) |
| `0x52` | B3 → B4 | Set button function | [3.6.3](#363-set-button-0x52) |
| `0x54` | B3 → B4 | Write macro to button | [3.6.4](#364-macros-0x54--0x71--0x64) |
| `0x55` / `0x65` | B3 → B4 | Hall-effect trigger set / get (HE mice only) | [3.19](#319-hall-effect-trigger-0x55--0x65) |
| `0x59` / `0x69` | B5 → B6 | Haptic motor set / get | [3.16](#316-haptic-motor-0x59--0x69) |
| `0x61` | B3 → B4 | List remapped buttons | [3.6.1](#361-list-remapped-buttons-0x61) |
| `0x62` | B3 → B4 | Get one button's function | [3.6.2](#362-get-button-0x62) |
| `0x64` | B3 → B4 | Read macro | [3.6.4](#364-macros-0x54--0x71--0x64) |
| `0x71` / `0x72` | B3 → B4 | Long-packet chunk / chunk ack (macros) | [3.6.4](#364-macros-0x54--0x71--0x64) |
| `0xE1`, `0xE2`, `0xE3`, `0xE5` | IN only | Unsolicited notifications | [4](#4-unsolicited-notifications) |

---

## 3. Commands

### 3.1 Device info (`0x02`) ✅

`kcm6main.txt:82089`

**Request** — report `0xB5`, 20 bytes:

| Byte | Value |
|---|---|
| 0 | `0x02` |
| 1 | `workMode` (connection mode, `0` = wired, `1` = 2.4 GHz receiver) |

**Reply** — report `0xB6`, byte 0 = `0x02`:

| Byte(s) | Meaning |
|---|---|
| 1–2 | Protocol version, little-endian. **Test M6: 4.** Version ≥ 6 changes some layouts. |
| 3–4 | VID, little-endian |
| 5–6 | PID, little-endian |
| 7 | Firmware minor (high nibble) and patch (low nibble) |
| 8 | Firmware major → version string `b8.(b7>>4).(b7&0xF)` |
| 9 | Low 3 bits = current `workMode` |
| 11 | `feature1Flag` (only used when version ≥ 6) |
| 12 | `feature2Flag` (version ≥ 6) |
| 13 | `feature3Flag` (version ≥ 6) |
| 15 | `feature4Flag` (version ≥ 6) |

Capture: `OUT B5 02 01` → `IN B6 02 04 00 34 34 60 D0 03 01 11 …` = protocol v4, `3434:D060`, firmware 1.0.3, workMode 1.

### 3.2 Receiver state (`0x03`) 📖

`kcm6main.txt:87254`

**Request:** `B5 [0x03]`

**Reply** (`0x03`):
- Byte 1 = `n`, the number of bonded devices.
- Then `n` records of 5 bytes starting at byte 2: `vid lo, vid hi, pid lo, pid hi, state`.
- `state == 1` marks the active/connected device. If none is active, the launcher reports "device sleeping".

### 3.3 Firmware version string (`0x04`) 📖

`kcm6main.txt:82231`

**Request:** `B3 [0x04, workMode]`

**Reply** (`0x04`): byte 1 = length `L`, bytes 2…(2+L−1) = ASCII version string.

### 3.4 Base info (`0x06`) ✅

`kcm6main.txt:82612`. This is the main "read everything" call.

**Request:** `B3 [0x06]` (63 bytes)

**Reply** (`B4`, byte 0 = `0x06`):

Connection-mode-indexed fields hold three slots: `[0]` wired, `[1]` 2.4 GHz, `[2]` third mode
(probably Bluetooth). Index with the current `workMode`.

| Byte(s) | Field | Notes / test M6 value |
|---|---|---|
| 1 | **Current profile** (0-based) | `01` → Profile 2 |
| 2, 3, 4 | Low nibble = **DPI level index**, high nibble = **polling level index**, one byte per connection mode | `22 22 02` → DPI level 2, polling level 2 |
| 5–14 | Up to 5 DPI values, `uint16` little-endian | `0190 0320 0640 0C80 1388` = 400 / 800 / 1600 / 3200 / 5000 |
| 15 | System feature bits: `[1:0]` LOD, bit2 wave/ripple, bit3 line (angle snapping), bit4 motion sync, bit6 scroll reversed | `0x15` → LOD 1, wave on, line off, motion on, scroll normal |
| 16 | Number of DPI levels | `05` |
| 17 | Global debounce (ms) | `00` |
| 18 | Sleep timeout (minutes) | `0A` = 10 min |
| 19 | Battery: bits `[6:0]` = percent, bit 7 = state flag (likely charging) | `4E` = 78 % |
| 26 | `feature1Flag` (protocol < 6) | `37` |
| 27 | Scroll speed | |
| 28 | Scroll inertia | |
| 29 | Scroll "spl" (smooth/precision flag) | |
| 30–39 | Debounce values (the last five match the per-button debounce of buttons 0–4) | `… 04 04 04 04 04` |
| 40–41 | Max DPI, `uint16` LE (0 = not reported) | |
| 42 | DPI step (0 → default 50) | |
| 43–48 | Polling-rate **codes** for each level (see the table below) | `00 01 02 …` = 125 / 500 / 1000 Hz |
| 49 | Number of polling levels (0 → 6) | `03` |
| 50 | **Number of profiles** | `05` |
| 51 | Wake sources: bits 0–3 = supported (key, scroll, move, side-scroll), bits 4–7 = enabled (same order) | |
| 52 | bit0 = 20k FPS mode | |
| 53 | `feature2Flag` (protocol < 6) | |
| 55 | Angle-snapping angle, signed (`>90` → `value−256`) | |
| 57, 58, 59 | Colour material IDs: shell, middle wheel, side wheel | |
| 60 | `feature3Flag` (protocol < 6) | |
| 61 | Low nibble = LOD level, high nibble = LOD level count | |

**Polling-rate codes** (`kcm6main.txt:136726`):

| Code | 0 | 6 | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|---|
| Hz | 125 | 250 | 500 | 1000 | 2000 | 4000 | 8000 |

**Feature flags.** Protocol < 6 reads them from base-info bytes 26 / 53 / 60. Protocol ≥ 6 reads them from the `0x02` reply.

| Flag | Bit | Meaning |
|---|---|---|
| feature1 | 0 | Scroll settings supported |
| feature1 | 1 | Debounce supported |
| feature1 | 3 | DPI max/step reported |
| feature1 | 4 | Polling "gears" (level list) supported |
| feature1 | 5 | **Profiles supported** |
| feature1 | 6 | Wake-source settings supported |
| feature1 | 7 | 20k FPS supported |
| feature2 | 1 | Pairing-combo setting supported |
| feature2 | 2 | Angle snapping supported |
| feature2 | 3 / 5 / 6 | Trigger sensing: ALL / LIGHT / MAGNETIC |
| feature3 | 0 | **Separate X/Y DPI** (→ use `0x48` instead of `0x40`) |
| feature3 | 1 | **Separate polling per mode** (→ use `0x4A` instead of `0x41`) |
| feature3 | 7 | 7-level polling |
| feature4 | 2 | Haptic motor |
| feature4 | 4 | LOD level supported |

Test M6: `feature1 = 0x37` (scroll, debounce, max/step, polling gears, profiles), and feature2 and feature3 are 0.
So it uses the plain `0x40` DPI and `0x41` polling commands.

### 3.5 Select profile (`0x0E`) ✅

`kcm6main.txt:84450`. Called from `2.js:968` (`switchProfileLevel`).

**Request** — `B5`, 20 bytes:

| Byte | Value |
|---|---|
| 0 | `0x0E` |
| 1 | Profile index (0-based) |
| 2 | **Total profile count** (base-info byte 50) |

**Reply:** `B6 E4 00 0E`

> ⚠️ Byte 2 also **sets** the number of profiles. The launcher uses this same command to change the count
> (`2.js:986`). Always pass the current count unless you mean to change it.

Capture: `OUT B5 0E 01 05` → `IN B6 E4 00 0E`.

### 3.6 Buttons

Function types (`EFunKey`, `kcm6main.txt:88836` — the enum module used for `"8k"`):

| Type | Name | Data bytes (after the type byte) |
|---|---|---|
| 0 | Remove (restore default) | none |
| 1 | Mouse (basic button) | 3 bytes, big-endian value from **Basic keys** |
| 2 | Keyboard | `[modifierMask, keycode1, keycode2]` |
| 3 | Media | 2 bytes, **little-endian** consumer usage |
| 4 | Macro | written via `0x54`, see 3.6.4 |
| 5 | DPI | 2 bytes LE: `1` loop, `2` next, `3` prev |
| 6 | Light | 1 byte: `1` toggle, `2` speed, `3` colour, `4` brightness+, `5` brightness− |
| 7 | Game (rapid fire) | `[kind(1=mouse,2=keyboard), key, rate lo, rate hi, count lo, count hi]` |
| 8 | Shortcut | 3 bytes, big-endian value from **Shortcuts** |
| 9 | Disable | none |
| 10 | Profile | `[profileIndex]` |
| 13 | Polling rate | 2 bytes LE: `1` loop, `2` next, `3` prev |
| 15 | Custom function | `[3 (IrHeMode), target, action]` (Hall-effect mice only) |

**Basic keys** (type 1, 3-byte big-endian):

| Name | Value | Bytes |
|---|---|---|
| left | 65536 | `01 00 00` |
| right | 131072 | `02 00 00` |
| middle | 262144 | `04 00 00` |
| backward | 524288 | `08 00 00` |
| forward | 1048576 | `10 00 00` |
| leftDouble | 8388608 | `80 00 00` |
| upScroll | 512 | `00 02 00` |
| downScroll | 65024 | `00 FE 00` |
| leftScroll | 254 | `00 00 FE` |
| rightScroll | 2 | `00 00 02` |
| leftScrollSustain | 4194559 | `40 00 FF` |
| rightScrollSustain | 4194305 | `40 00 01` |

**Modifier mask** (keyboard type): `1` LCtrl, `2` LShift, `4` LAlt, `8` LWin/Cmd, `16` RCtrl, `32` RShift, `64` RAlt, `128` RWin.

**Keycodes** are standard USB HID keyboard usage IDs (`KC_A = 0x04` … `KC_Z = 0x1D`, `KC_1 = 0x1E` … `KC_0 = 0x27`, and so on).

**Media** (type 3, LE): volumeUp 233 (`E9 00`), volumeDown 234, mute 226, player 387 (`83 01`), pause 205, previous 182, next 181, stop 183.

**Shortcuts** (type 8, 3-byte BE value):

| Name | Value | Name | Value |
|---|---|---|---|
| brightnessUp | 814848 | copy | 459014 |
| brightnessDown | 815104 | cut | 459035 |
| calculate | 823809 | paste | 459033 |
| myComputer | 824321 | copyMac | 460806 |
| homePage | 795394 | cutMac | 460827 |
| email | 821761 | pasteMac | 460825 |
| refresh | 458814 | screenshot | 461334 |
| switchApp | 459819 | screenshotMac | 461345 |
| task | 827138 | webLessen / webMagnify | 459053 / 459054 |
| launchPad | 827394 | webLessenMac / webMagnifyMac | 460845 / 460846 |
| home | 460807 | | |

**Rapid fire mouse keys** (type 7, kind 1): left 1, right 2, middle 4, front 8, back 16.

**Button indices.** The model JSON lists the buttons; the test M6 has 11. The launcher's UI enum uses
left 0, middle 1, right 2, backward 3, forward 4 for the first five. Higher indices (5+) are
model-specific; the capture shows indices up to 14.

#### 3.6.1 List remapped buttons (`0x61`) ⚠️

**Request:** `B3 [0x61]`

**Reply** (`0x61`):
- Byte 1 = `n`, the number of entries.
- Then `n` variable-length entries. Each entry's first byte `h` holds the button index in `h & 0x1F`, and the entry is `2 + (h >> 5)` bytes long.

The launcher uses this list **only for indices**. It then calls `0x62` for every listed index and for every index > 4.
Don't rely on the other bytes of each entry: in the capture they don't line up with the `0x62` results.

#### 3.6.2 Get button (`0x62`) ✅

**Request:** `B3 [0x62, buttonIndex]`

**Reply:** `B4 [0x62, buttonIndex, 0x00, type, data…]` (decode `data` with the table above)

Capture: `62 09 00 02 00 10` → button 9 = Keyboard, no modifier, `KC_M` (0x10).
`62 0E 00 00` → button 14 = default.

#### 3.6.3 Set button (`0x52`) 📖

`kcm6main.txt:84669`

**Request** — `B3`, 63 bytes:

| Byte | Value |
|---|---|
| 0 | `0x52` |
| 1 | Button index |
| 2 | `0x00` |
| 3 | Function type |
| 4… | Data bytes (see the table) |

**Reply:** `E4 00 52`

Examples:
- Button 3 → left click: `52 03 00 01 01 00 00`
- Button 9 → Ctrl+C: `52 09 00 02 01 06 00`
- Button 5 → DPI loop: `52 05 00 05 01 00`
- Button 6 → Profile 3: `52 06 00 0A 02`
- Button 6 → default: `52 06 00 00`

#### 3.6.4 Macros (`0x54` / `0x71` / `0x64`) 📖

`kcm6main.txt:84993`. A macro is written **directly onto a button index**. Writing it also binds it to that button.
The launcher keeps the macro library and names only in `localStorage`.

**Macro packet:**

| Byte | Value |
|---|---|
| 0 | `0x54` |
| 1 | Button index |
| 2 | `0x00` |
| 3–4 | `stepBytes + 6`, `uint16` LE |
| 5 | `0x01` |
| 6–7 | `loopMode << 13 | loopCount`, `uint16` LE |
| 8–9 | Global delay (ms), `uint16` LE |
| 10 | Number of steps |
| 11… | Steps, 4 bytes each |

`loopMode`: `0` loop N times, `1` until release, `2` until next press, `3` until double press.

Step encoding (`serializeMacro`, `kcm6main.txt:85368`):

| Step | Bytes |
|---|---|
| Key press/release | `00, (press?0x80:0) | 0x01, hidKeycode, 00` |
| Modifier press/release | `00, (press?0x80:0) | 0x02, modBit, 00` (modBit as in the modifier mask; HID 224–231 map to 1…128) |
| Mouse button | `00, (press?0x80:0) | 0x08, button, 00` (left 1, right 2, middle 4, forward 8, backward 16) |
| Delay | `00, 0x0F, ms lo, ms hi` |

**Sending:**
- If the whole packet is **≤ 62 bytes** (≤ 12 steps), send it as one `B3` report and wait for any reply.
- Otherwise split the *entire packet* (including the `0x54` header) into chunks of ≤ 59 bytes. Send each chunk as
  `B3 [0x71, (seq << 2) | more, chunkLen, chunk…]`, where `more = 1` on every chunk except the last.
  Wait for a reply with byte 0 = `0x72` before sending the next chunk.
- The launcher caps macros at 200 steps.

**Reading:** `B3 [0x64, buttonIndex]`
- Short reply: `[0x64, ?, ?, len lo, len hi, data…]`, where the data starts at byte 5.
- Long reply: a sequence of chunks `[?, (seq<<2)|more, len, data…]`, collected until `more == 0`.

### 3.7 DPI (`0x40`) 📖

`kcm6main.txt:85511`. Use this when separate X/Y DPI is **not** supported, which is the case for the M6.

**Request** — `B5`, 20 bytes:

| Byte(s) | Value |
|---|---|
| 0 | `0x40` |
| 1, 2, 3 | Active DPI level index (the launcher writes the same value for all three connection modes) |
| 4–13 | Up to 5 DPI values, `uint16` LE |
| 14 | Number of enabled levels |

**Reply:** `E4 …` or `40 …`

Example — levels 400/800/1600/3200/5000, select 1600:
`40 02 02 02 90 01 20 03 40 06 80 0C 88 13 05`

#### 3.7.1 Separate X/Y DPI (`0x48` / `0x49`) 📖

Only when feature3 bit0 is set.
- **Get:** `B3 [0x49]`.
- **Set:** `B3 [0x48, …]`, acknowledged with `E4 xx 48`.

| Byte(s) | Protocol < 6 | Protocol ≥ 6 |
|---|---|---|
| 1–3 | `(yLevel << 4) | xLevel` per connection mode | Level per connection mode |
| 4 | Level count | Level count |
| 5–20 | X values, `uint16` LE (up to 8) | Same |
| 21–36 | Y values, `uint16` LE (up to 8) | Same |
| 37–44 | Per-level "separate Y enabled" flags, 1 byte each | Byte 37 = bitmask |

### 3.8 Polling rate (`0x41`) 📖

`kcm6main.txt:85831`

**Request** — `B5`, 20 bytes:

| Byte(s) | Value |
|---|---|
| 0 | `0x41` |
| 1, 2 | Active level index (the same value is written for both modes) |
| 3–8 | Rate codes per level (see the code table) |
| 9 | Level count |

**Reply:** `E4 …` or `41 …`

Example — the test M6 table (125/500/1000), select 1000 Hz: `41 02 02 00 01 02 00 00 00 03`

#### 3.8.1 Per-mode polling rate (`0x4A` / `0x4B`) 📖

Only when feature3 bit1 is set.
- **Get:** `B5 [0x4B]`.
- **Set:** `B5 [0x4A, …]`.

| Byte(s) | Value |
|---|---|
| 1 | Level, mode 0 (wired) |
| 2 | Level, mode 1 (2.4 GHz) |
| 3 | Count, mode 0 |
| 4 | Count, mode 1 |
| 5–10 | Codes, mode 0 |
| 11–16 | Codes, mode 1 |

### 3.9 Debounce (`0x43`) 📖

`kcm6main.txt:86080`

| Use | Request (`B5`) |
|---|---|
| Global | `[0x43, ms]` |
| One button (index 0–4) | `[0x43, ms, 0x11, buttonIndex, ms]` |

Range: 0–20 ms. **Reply:** `E4 00 43`

### 3.10 System features / angle snapping (`0x42`) 📖

`kcm6main.txt:86246`

**Set features** — `B5`, 20 bytes:

| Byte | Value |
|---|---|
| 0 | `0x42` |
| 1 | LOD index (model-specific, e.g. `1` = 1 mm, `2` = 2 mm, `3` = 0.7 mm on a 3950 sensor) |
| 2 | Wave/ripple: `1` on, `2` off |
| 3 | Line / angle snapping: `1` on, `2` off |
| 4 | Motion sync: `1` on, `2` off |
| 6 | Scroll direction: `1` normal, `2` reversed |
| 8 | 20k FPS: `1` off, `2` on (value + 1) |
| 11 | LOD level (if supported) |

The launcher always sends all fields together, so read base info first and send back the values you aren't changing.

**Set angle** — `B5 [0x42, 0,0,0,0,0,0,0,0, enable ? 2 : 0, angle]`

`angle` is a signed byte, −90…90. The launcher zeroes the other fields here, so zero most likely means "leave unchanged".

**Reply:** `E4 00 42`

### 3.11 Scroll wheel (`0x45`) ⚠️

`kcm6main.txt:86893`

**Request:** `B5 [0x45, a, b, c]`

Base info stores the scroll values as speed (27), inertia (28), spl (29), so the order is very likely `[speed, inertia, spl]`.
The launcher waits for a reply with byte 0 = `0xB5`, which probably never arrives. Don't block on it.

### 3.12 Lighting (`0x23` / `0x24`) 📖

`kcm6main.txt:86820`. Only applies if your model has LEDs.

| Action | Request (`B5`) | Reply |
|---|---|---|
| Get | `[0x23]` | `[0x21, mode, brightness, speed, r, g, b]` |
| Set | `[0x24, mode, brightness, speed, r, g, b]` | none waited on |

Mode `0` = off. Brightness and speed are 0–255.

### 3.13 Sleep timeout (`0x0A`) 📖

`kcm6main.txt:86924`

| Action | Request (`B5`) | Reply |
|---|---|---|
| Get | `[0x0A, 0x02]` | `[0x0A, ?, minutes]` |
| Set | `[0x0A, 0x01, minutes]` | `E4 00 0A` |

Range 1–240 minutes. A value of 0 is sent as `0xFF`.

### 3.14 Pairing button combo (`0x0B`) 📖

`kcm6main.txt:86568`. This is the button combination and hold time that put the mouse into receiver-pairing mode.

| Action | Request (`B5`) | Reply |
|---|---|---|
| Get | `[0x0B, 0x02]` | `[0x0B, ?, mask, time]` |
| Set | `[0x0B, 0x01, mask, time]` | success when `byte1 == 0` |

`mask` bits: 0 left, 1 middle, 2 right, 3 forward, 4 backward. On PID `0xD054` bits 1 and 2 are swapped.
- Default mask when 0 is sent: `0x11` (left + backward).
- A reply mask of 0 means the defaults are in use (left + forward).

### 3.15 Factory reset (`0x0F`) 📖

`kcm6main.txt:87144`

| Scope | Request (`B5`) |
|---|---|
| Everything, all profiles | `[0x0F, 0xFF]` |
| Selected areas, all profiles | `[0x0F, mask]` |
| Selected areas, one profile | `[0x0F, 0x00, profileIndex, mask, mask2]` |

`mask` bits: 0 buttons, 1 DPI, 2 lighting, 3 sensor, 4 profile, 5 scroll, 6 polling rate.
`mask2` bits: 0 debounce, 1 Hall effect, 2 XY acceleration.

**Reply:** `E4 00 0F`

The launcher's "reset buttons" action sends `[0x0F, 0x00, currentProfile, 0x01, 0x00]`.

### 3.16 Haptic motor (`0x59` / `0x69`) 📖

Only when feature4 bit2 is set.

| Action | Request (`B5`) | Reply |
|---|---|---|
| Get | `[0x69]` | `[0x69, levelTotal, levelCount, l0a, l0b, l1a, l1b]` |
| Set | `[0x59, levelCount (default 6), l0a, l0b, l1a, l1b]` | `E4 00 59` |

### 3.17 Indication (`0x26`) 📖

| Action | Request (`B5`) | Reply |
|---|---|---|
| Get | `[0x26, 0x00]` | `[0x26, ?, value]` |
| Set | `[0x26, 0x01, value]` | `[0x26, 0x01, …]` |

What the value means isn't documented in the bundle (likely an indicator LED behaviour).

### 3.18 Colour materials (`0x08`) 📖

**Set:** `B5 [0x08, shellColor, middleWheelColor, sideWheelColor]`. To read, use base-info bytes 57–59.
These are cosmetic IDs the launcher uses to render the mouse picture, not RGB lighting.

### 3.19 Hall-effect trigger (`0x55` / `0x65`) 📖

Only for magnetic-switch mice (feature2 bits 3/6). The M6 doesn't have these switches.
- **Get:** `B3 [0x65]`.
- **Set:** `B3 [0x55, n, sections…]`, acknowledged with `E4 00 55`.

Each section is 22 bytes:

| Offset | Field |
|---|---|
| 0 | Button index |
| 1–2 | Dead band |
| 3–4 | Percent points |
| 5–8 | Scale points, 2 × `uint16` LE |
| 9 | Percent value |
| 10–11 | Scale value, `uint16` LE |
| 12 | `(rtEnable << 4) | triggerMode` |
| 13–16 | Rapid-trigger values, 2 × `uint16` LE |
| 17 | ADC flag |
| 18–21 | ADC data |

---

## 4. Unsolicited notifications

`kcm6main.txt:84272`. These are input reports the mouse sends on its own. Listen on every input report ID.

| Byte 0 | Event | Payload |
|---|---|---|
| `0xE5` | **Profile changed** (e.g. by the on-mouse button) | `[1]` = new profile index |
| `0xE2` | Base status changed | `[1]` workMode, `[2]` connected (1 = yes), `[3]` power state, `[4]` battery %, `[5]` DPI level, `[6]` polling level, `[7]` DPI level count |
| `0xE1` | Lighting changed | `[1]` mode, `[3]` speed, `[4]` brightness, `[5..7]` RGB |
| `0xE3` | Hall-effect mode changed (when `[1] == 3`) | `[2]` button index, `[3]` trigger mode |
| `0xE4` | Command acknowledgement | `[1]` status, `[2]` opcode |

---

## 5. Reference implementation (WebHID)

```js
const VID = 0x3434, PID = 0xD060;
const OUT_SET = 0xB5, IN_SET = 0xB6, OUT_GET = 0xB3, IN_GET = 0xB4;

export async function open() {
  const [dev] = await navigator.hid.requestDevice({
    filters: [{ vendorId: VID, productId: PID, usagePage: 0xFFC1, usage: 1 }],
  });
  if (!dev.opened) await dev.open();
  return dev;
}

// Serialises commands: one in flight, 1 s timeout, 3 retries (mirrors the launcher).
let chain = Promise.resolve();
function cmd(dev, outId, payload, match, { size = outId === OUT_SET ? 20 : 63, timeout = 1000, retries = 3 } = {}) {
  const run = () => new Promise((resolve, reject) => {
    let attempt = 0, timer;
    const onReport = e => {
      const d = new Uint8Array(e.data.buffer);
      if (match(d, e.reportId)) { cleanup(); resolve(d); }
    };
    const cleanup = () => { clearTimeout(timer); dev.removeEventListener('inputreport', onReport); };
    const send = () => {
      const buf = new Uint8Array(size); buf.set(payload);
      dev.sendReport(outId, buf).catch(err => { cleanup(); reject(err); });
      timer = setTimeout(() => (attempt++ < retries ? send() : (cleanup(), reject(new Error('timeout')))), timeout);
    };
    dev.addEventListener('inputreport', onReport);
    send();
  });
  return (chain = chain.then(run, run));
}
const ack = op => d => d[0] === 0xE4 && d[2] === op;

export async function getBaseInfo(dev) {
  const d = await cmd(dev, OUT_GET, [0x06], d => d[0] === 0x06);
  const u16 = i => d[i] | (d[i + 1] << 8);
  return {
    profile: d[1], profileCount: d[50],
    dpiLevel: d[2] & 0xF, pollingLevel: d[2] >> 4,
    dpiValues: [5, 7, 9, 11, 13].map(u16).slice(0, d[16]),
    pollingCodes: [...d.slice(43, 49)].slice(0, d[49] || 6),
    lod: d[15] & 3, wave: (d[15] >> 2) & 1, line: (d[15] >> 3) & 1, motion: (d[15] >> 4) & 1, scrollReversed: (d[15] >> 6) & 1,
    debounce: d[17], sleepMinutes: d[18], battery: d[19] & 0x7F, raw: d,
  };
}

export async function setProfile(dev, index) {
  const { profileCount } = await getBaseInfo(dev);
  if (index < 0 || index >= profileCount) throw new RangeError(`profile 0..${profileCount - 1}`);
  await cmd(dev, OUT_SET, [0x0E, index, profileCount], ack(0x0E));
}

export async function setDpi(dev, values, activeLevel) {
  const p = [0x40, activeLevel, activeLevel, activeLevel];
  values.slice(0, 5).forEach(v => p.push(v & 0xFF, v >> 8));
  while (p.length < 14) p.push(0);
  p[14] = values.length;
  await cmd(dev, OUT_SET, p, d => d[0] === 0xE4 || d[0] === 0x40);
}

export async function setPollingLevel(dev, level) {
  const b = await getBaseInfo(dev);
  const codes = [...b.raw.slice(43, 49)];
  await cmd(dev, OUT_SET, [0x41, level, level, ...codes, b.raw[49]], d => d[0] === 0xE4 || d[0] === 0x41);
}

export async function getButton(dev, index) {
  const d = await cmd(dev, OUT_GET, [0x62, index], d => d[0] === 0x62 && d[1] === index);
  return { index, type: d[3], data: [...d.slice(4, 10)] };
}

export async function setButton(dev, index, type, data = []) {
  await cmd(dev, OUT_GET, [0x52, index, 0x00, type, ...data], ack(0x52));
}

export function onNotify(dev, handlers) {
  dev.addEventListener('inputreport', e => {
    const d = new Uint8Array(e.data.buffer);
    if (d[0] === 0xE5) handlers.profile?.(d[1]);
    if (d[0] === 0xE2) handlers.status?.({ workMode: d[1], connected: d[2] === 1, battery: d[4], dpiLevel: d[5], pollingLevel: d[6] });
  });
}
```

### hidapi (Python) note

```python
import hid
# open the interface whose usage_page == 0xFFC1
dev.write(bytes([0xB5, 0x0E, idx, count] + [0] * 17))   # report ID first
reply = dev.read(64)                                      # may include 0xB6 as reply[0] depending on backend
```

---

## 6. Capturing more traffic

Paste this into the launcher tab's DevTools console. It logs every OUT/IN report so you can confirm a command
before relying on it:

```js
const P = HIDDevice.prototype, orig = P.sendReport;
P.sendReport = function (id, data) {
  const hex = a => [...a].map(x => x.toString(16).padStart(2, '0')).join(' ');
  console.log('%cOUT', 'color:orange', '0x' + id.toString(16), hex(new Uint8Array(data.buffer ?? data)));
  if (!this.__hooked) {
    this.__hooked = true;
    this.addEventListener('inputreport', e => console.log('%cIN ', 'color:green', '0x' + e.reportId.toString(16), hex(new Uint8Array(e.data.buffer))));
  }
  return orig.call(this, id, data);
};
```

Remember that replies arrive asynchronously: in the log, each `IN` line usually shows up after the *next* `OUT`.

## 7. Out of scope

- Firmware update / bootloader (`blhost`, `kcm6main.txt:~93000`). This can brick the device, so it's not documented here.
- The `"1k"`, `"4k"`, `"8k_nordic"` and `"kc_lite"` protocol variants. Each command class in `kcm6main.txt:81977–87650` has a
  branch per protocol if you need to support other Keychron mice.
