# GMMK 2 (96%) HID protocol (GmmkSeries)

This document describes the vendor HID protocol used by Glorious CORE 2.x for the **GMMK 2 Full Size 96%** keyboard, as recovered from behavioral analysis of Glorious CORE **2.1.3** (Electron) on macOS. It is a clean-room reference for **gmmk-core-mac**; it does not reproduce Glorious UI assets or native binaries.

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
- **Payload length:** `264` bytes for GMMK 2 96% (`KeyMatrixLength > 120`)
- **Byte 0:** report ID (`7`) in buffers passed to Glorious native code; **node-hid** expects the first byte to be the report ID.

Command byte is **`buf[1]`** (Glorious sets `buf[0] = 7` then uses `buf[1]` as subcommand).

## Subcommands (partial)

| `buf[1]` | Purpose |
|----------|---------|
| `2` | LED effect parameters (`SetLEDEffectToDevice`) |
| `6` | Per-key LED layout chunk (`SetLEDLayoutToDevice`) |
| `7` | Profile/layer lighting mode (`SetLEDTypeToDevice`) |
| `8` | Polling rate / EP2 flags (`SetPollingRatetoDevice`) |
| `16` | RGB sync matrix chunk |

### Polling rate (`buf[1] === 8`)

```text
buf[8] = index into [1000, 500, 250, 125] Hz
buf[9] = EP2Enable (1 = on)
buf[10] = LEDNoChange (1 = do not reset lighting)
```

### LED preset effect (`buf[1] === 2`)

```text
buf[2]  = effect index (see LED_EFFECT_IDS in src/protocol/gmmk-v2-96.mjs)
buf[8]  = speed (scaled from UI 0–100)
buf[9]  = brightness (UI 0–100 → device via * 20 / 100)
buf[15..17] = RGB primary color
```

### Profile / layer mode (`buf[1] === 7`)

```text
buf[2] = profile index + 1
buf[3] = layer index + 1
buf[4] = effect index
buf[7] = side light (0 = on, 1 = off when sideLightSwitch)
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

- Glorious CORE 2.1.3 `GmmkSeries` logic (behavioral source for this doc).
- Community OpenRGB / QMK work uses different stacks (QMK raw HID); stock Glorious firmware uses this vendor feature protocol.
