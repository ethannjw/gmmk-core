# GMMK 2 (96%) HID protocol (GmmkSeries)

The native app implements a subset of the stock **GMMK 2 Full Size 96% ANSI** vendor HID protocol. Windows CORE **2.1.23** (package **2.1.23.2**) is the source reference. Its builders use 64-byte activation and 264-byte configuration staging buffers; native HID transfers use the keyboard's descriptor-defined 256 bytes, including report ID. Complete hardware acceptance, exact write sequencing and onboard readback remain unverified. Vendor assets and native binaries are not distributed with this app.

## USB identity

| Mode | VID | PID | Product string |
|------|-----|-----|----------------|
| Normal (US ANSI) | `0x320F` | `0x504B` | GMMK 2 Full Size 96% |
| Bootloader (US) | `0x320F` | `0xB00B` | (bootloader) |
| Normal (ISO) | `0x320F` | `0x505A` | GMMK 2 Full Size 96% |
| Bootloader (ISO) | `0x320F` | `0xB00C` | (bootloader) |

## HID interfaces

| Direction | Usage page | Usage | Role |
|-----------|------------|-------|------|
| Host → device (config) | `0xFF01` | `0x01` | Feature reports (primary) |
| Device → host (status) | `0xFF00` | `0xFF00` | Feature / EP2 notifications |

## Feature report framing

- **Report ID:** `7`
- **Windows builder/staging length including report ID:** `64` bytes for command `1`; `264` bytes for commands `2`, `3`, `6`, `7`, `8`.
- **Native HID transfer length including report ID:** `256` bytes for every supported command. Activation is zero-padded; configuration staging bytes 256–263 must be zero before removal. Nonzero overflow rejects the entire batch before any transfer.
- **Byte 0:** report ID (`7`) in both Windows native-API buffers and buffers passed to Rust `hidapi`.

Command byte is **`buf[1]`** (Glorious sets `buf[0] = 7` then uses `buf[1]` as subcommand).

The native send boundary rejects unsupported commands, incorrect staging lengths, incorrect report IDs and invalid profile/layer coordinates before transport. Connection verifies the supported configuration report descriptor before enabling access. An unfamiliar descriptor is rejected rather than guessing a new transport contract. See [framing evidence and acceptance limits](../app/docs/research/hid-report-framing.md). The command allowlist is deliberately limited to implemented writes; it does not claim that other firmware commands do not exist.

## Subcommands (partial)

| `buf[1]` | Purpose |
|----------|---------|
| `1` | Change active profile/layer (`ChangeProfileID`) — `buf[2]=profile+1`, `buf[3]=layer+1` |
| `2` | LED effect parameters (`SetLEDEffectToDevice`) |
| `3` | Key matrix (`SendButtonMatrix2Device`) — 240 bytes at `buf[8..]` |
| `4` | Macro type map — not implemented; rejected by native send validation |
| `5` | Macro payload — not implemented; rejected by native send validation |
| `6` | Per-key LED layout chunk (`SetLEDLayoutToDevice`) — four chunks for 96% |
| `7` | Profile/layer lighting mode (`SetLEDTypeToDevice`) |
| `8` | Polling rate / EP2 flags (`SetPollingRatetoDevice`) |
| `9` | AltGr map — not sent by the inspected Windows GMMK 2 binding update; rejected by native send validation |

Firmware information comes from cached USB release metadata, matching the recovered native `GetFWVersion` path. There is no firmware feature-report query; command `138` remains rejected. See [the native firmware trace](../app/docs/research/windows-native-firmware.md) for exact addresses, the module hash, formatting behavior and pending physical acceptance.

### Multimedia entries inside the key matrix

For a physical key's LED index, the first 120-byte matrix half stores binding type `3`, and the second half stores the following device-native code. These entry codes are separate from the packet's command byte: the complete matrix uses command `3`, a 264-byte staging buffer, a 256-byte native transfer and the scoped binding path's 150 ms delay.

| Media action | Entry code |
| --- | --- |
| Open Media Player | 0 |
| Play / Pause | 1 |
| Mute | 2 |
| Volume Up | 3 |
| Volume Down | 4 |
| Stop | 5 |
| Previous Track | 6 |
| Next Track | 7 |

Source chain: `MultimediaOptions` (`out/main/index.js:6358`) → `getMultimediaBindingValue` (46406) → `DeviceFunctionCodeHIDValueMap` (44618) → `GetKeyAssignmentBuffer$1` (46549). Native decode recognizes only these eight type-3 codes; other imported codes remain opaque and unchanged until explicitly replaced. A disabled key is type `1`, code `0`, not type `3`, code `0`. The reference's None placeholder has no distinct encoded value in this path, so the native UI uses its separate Disable action rather than treating code zero as disabled.

Tests cover all eight actions at every physical key position, exact bytes, round trips, unrelated-byte preservation and invalid input. UI tests cover all media tiles, selection/layer refresh, export/import and explicit-save isolation. Host OS/player behavior still requires physical acceptance on both platforms.

### Polling rate (`buf[1] === 8`)

```text
buf[8] = index into [1000, 500, 250, 125] Hz
buf[9] = EP2Enable (1 = on)
buf[10] = LEDNoChange (1 = do not reset lighting)
```

### LED preset effect (`buf[1] === 2`)

```text
buf[2]  = effect index (see LED_EFFECT_IDS in app/src/protocol/gmmk_v2_96.rs)
buf[8]  = 100 - UI speed
buf[9]  = brightness (UI 0–100 → device via * 20 / 100)
buf[10] = multicolor flag
buf[12] = multicolor flag (same value)
buf[15..17] = RGB primary color
```

### Profile / layer mode (`buf[1] === 7`)

```text
buf[2] = profile index + 1
buf[3] = layer index + 1
buf[4] = effect index
buf[7] = 0 for this target (descriptor sideLightSwitch is true)
buf[8] = brightness
buf[10] = input latency (ms)
```

## LED effect IDs (firmware order)

Glorious maps UI effect values through `arrLEDType`:

`8,0,1,3,4,5,2,6,7,9,10,11,12,13,14,15,16,17,18` → indices `0..18` (Off, Glorious Mode, Wave #1, …).

## macOS notes

1. **Permissions:** System Settings → Privacy & Security → **Input Monitoring** (and sometimes **Accessibility**) for the app binary.
2. **OS support:** Official Glorious CORE 2.1 lists macOS Big Sur through **Sonoma** only; newer macOS versions may break the official app even when this protocol still applies.
3. **Exclusive access:** Only one process should open the vendor interface (`0xFF01`) at a time.

## References

- Windows CORE 2.1.23 extracted `out/main/index.js`: `GmmkBaseDeviceHandler.setActiveProfile` / `setActiveLayer` (46982 / 46997), `readFirmwareVersion` (47033), `GMMKDeviceHandler.sendLightingPropertiesToDevice` (60381), `sendReportToDevice` (60405), `setPollingRate` / `setInputLatency` (60457 / 60477), `setPerKeyLightingState` (60506), `updateAllKeyBinding` (60528).
- [Reference provenance and release gates](../app/docs/research/core-parity.md).
- [Local versus onboard state](PERSISTENCE.md).

## Windows write sequences and remaining differences

Custom-lighting LED addresses are distinct from keybinding entries. GMMK 2 96% ANSI uses 120 keybinding positions, while its lighting map additionally includes 20 side LEDs at indices 120–139. Each custom entry is five bytes: visibility (255 static / 254 breathing), LED index, red, green, blue. Four command-6 pages carry 200 data bytes each, beginning at report offset 8; side LEDs occupy the first 100 payload bytes of page 3. The native validator accepts indices below 140 without changing the 240-byte keybinding matrix. [Shared reference checks](../app/docs/research/windows-ui-map.md#custom-lighting-group-and-report-checks) compare all four complete reports; hardware behavior remains unverified.

Table lengths describe Windows builder buffers and include report ID; native transfers are normalized to 256 bytes. Delays follow each transfer. These sequences are source-derived reference behavior, not successful hardware captures.

| Action | Windows sequence | Native status |
| --- | --- | --- |
| Send profile/layer selection frame | `1` (64 bytes, 550 ms) | Length, bytes, delay and failure boundary covered by injected-transport tests |
| Save keybindings without macros | `3` (264 bytes, 150 ms); no activation or AltGr write | Keybinding tab uses this scoped path; tests cover bytes, delay, target, invalid data, no retries and draft preservation |
| Save preset lighting | `7`, `2` (264 bytes each, 100 ms each), then `1` (64 bytes, 550 ms) | Lighting tab uses this sequence; includes selected layer input latency and activation |
| Save custom lighting | Four `6` pages (264 bytes, 100 ms each), then `1` (64 bytes, 550 ms) | Lighting tab appends this sequence when custom data is supplied; an explicit empty list sends four zero-filled pages |
| Save performance | `8`, then `7` for each of three layers (264 bytes, 550 ms each) | Current apply is whole-layer/slot; reference scope pending |

Windows stores one preset-lighting state per profile; version-1 local files store lighting per layer. Matching Windows performance writes requires an explicit migration/conflict policy rather than silently choosing one layer's lighting for all three. Whole-slot writes also need explicit final active-layer behavior once lighting commits are implemented.

The native **Activate** action sends only the selection frame. Windows `setActiveLayer` additionally calls `updateAllKeyBinding`; that larger operation is not claimed as implemented by the selection-only action. Layer navigation in the native editor remains local by design. Legacy whole-layer/slot apply still uses a fixed 120 ms post-write delay and omits lighting commits; it does not use the scoped Keybinding/Lighting paths.

Lighting Save preflights all mixed-size frames before sending any, stops at the first transport error and never retries automatically. Native Save includes both lighting subpages, but always resends the displayed base effect rather than comparing separate drafts as Windows does. Tests cover sequence, frame contents, delays, targets, clearing versus unspecified colors, malformed final commits and each failure boundary; physical behavior remains unverified.
