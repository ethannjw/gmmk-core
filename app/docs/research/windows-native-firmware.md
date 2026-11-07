# Firmware version comes from USB metadata

Windows CORE 2.1.23's GMMK 2 firmware-version display comes from a cached USB release number. The inspected native `GetFWVersion` returns that cached value; it does not implement the speculative command-138 set/get transaction. The Rust app reads the corresponding `hidapi::DeviceInfo::release_number()` during explicit connection and labels it **Firmware (USB)**. This establishes a source-derived metadata path, not full onboard-settings download or hardware acceptance.

## The native trace identifies the value's origin

The JavaScript bridge in `out/main/index.js:4929` forwards `HID.getFirmwareVersion(deviceId)` to `HidLib.GetFWVersion(deviceId)`. The GMMK base handler at line 47033 formats that returned integer for the renderer.

The following addresses are static virtual addresses in the specific PE image below, using image base `0x180000000`. They are not addresses observed in a running process. LLVM's nearest-export labels are unrelated C++ names, so the trace uses exact addresses and string references instead.

| Stage | Static evidence | Derived behavior |
| --- | --- | --- |
| JavaScript export registration | `0x180025c32` loads callback `0x180004543`; `0x180025c39` references the `GetFWVersion` string at `0x180283098` | The callback thunk jumps to `0x180023fd0` |
| Cached-value getter | `0x18002445f` calls `0x180015900`; `0x180024464` reads a 16-bit word at offset 8 in its returned record | The version is a field in cached device attributes |
| Attribute accessor | `0x180015900` copies 12 bytes from the cached object at offset `0x10` | The getter reads the version at object offset `0x18` |
| Device discovery | `FindDevice` calls enumeration via thunk `0x180006a82`, which jumps to `0x180077870`; `0x18001fa85` reads the matching HID record's word at `0x18` | Discovery supplies the release number |
| Cache construction | `0x18001ffb5` copies that value into offset 8 of a six-word attribute record; `0x18001fff4` inserts the attributes and handle into the device map | Firmware lookup uses the discovered value rather than a new USB query |
| Windows HID metadata | `0x18007c936` resolves the `HidD_GetAttributes` string to pointer `0x18031d378`; `0x18007de95` calls it, then `0x18007dec3` / `0x18007dec8` copy the version word into the HID record at `0x18` | The source is Windows HID `VersionNumber` |

The pinned `hidapi` 2.6.7 source independently corroborates the cross-platform mapping: `etc/hidapi/windows/hid.c` fills `release_number` from `attrib.VersionNumber`; `etc/hidapi/mac/hid.c` fills it from `kIOHIDVersionNumberKey`. `src/lib.rs::DeviceInfo::release_number` exposes the stored integer. Linux hardware equivalence remains an acceptance requirement.

## The app displays metadata without claiming update status

- Connection caches the selected configuration interface's release number only after opening that interface succeeds. Failed connection and disconnect clear the worker cache; disconnect/error events clear the displayed value.
- **Refresh info** returns the cached metadata. It accepts no HID handle and issues no feature report. Reconnect after a firmware change to enumerate again.
- Nonzero four-digit decimal-nibble values such as `0x0123` display as `0123`, matching the inspected Windows formatting for these values.
- Windows treats `0x2000` as a special recovery/update marker and displays `0001`. The native app displays `0001 (USB recovery marker 2000)` without offering or attempting a firmware update.
- Zero and non-decimal-nibble values display as unknown; their raw hex value remains in the information message. This deliberately avoids JavaScript's partial decimal parsing of a malformed/non-decimal hex string.
- No remote update check runs. A displayed version never means **Up to Date**, and descriptor metadata does not establish the full running-firmware identity or any saved keymap.

## Evidence is static and reproducible

| Artifact | Identity |
| --- | --- |
| Reference | Windows CORE 2.1.23, package 2.1.23.2; see [reference provenance](core-parity.md) |
| Native module | `out/main/chunks/hiddevice-6b262a24.node`, PE32+ AMD64, 3,411,456 bytes |
| Native SHA-256 | `6b262a242637c5dd157a092106884a24430c44c1236e2e9a888b5728fcec5057` |
| REA PE-import evidence | `ev_ba51c48a2897daa9864fa7c179b50cb5d18ec5312f76d9a1e8f6a8f1ff3d04bf` |
| Local evidence output | `/private/tmp/gmmk-hiddevice-rea-imports.json` |
| Local LLVM disassembly | `/private/tmp/gmmk-hiddevice-disassembly.txt` |

REA 6.8.0's built-in `inspect-pe-imports` runs with the previously available Node 22.23.4 runtime. No native deep-analysis provider is configured for REA, so the instruction-level trace uses the existing Apple LLVM 21 `objdump`. The module is never loaded or executed. No tool installation, GUI inspection, device connection, feature report, reset or firmware update occurs in this investigation. Temporary evidence files are not durable release assets; the module hash and addresses make the trace repeatable from the same official package.

Given an extracted module, reproduce the static inspection with:

```sh
shasum -a 256 /path/to/hiddevice-6b262a24.node
rea inspect-pe-imports /path/to/hiddevice-6b262a24.node --format json
objdump -d --no-show-raw-insn \
  --start-address=0x1800242af --stop-address=0x1800244b0 \
  /path/to/hiddevice-6b262a24.node
```

## Physical comparison is still required

Automated tests cover normal version formatting, unknown/non-decimal values, the recovery marker, the pure cached-info event, connected/disconnected refresh controls, and clearing displayed metadata on disconnect/error. They do not exercise real USB enumeration or compare a connected keyboard with Windows CORE.

For the read-only acceptance check, close other configurators, launch the native app, and select **Device → Connect**. Record **Firmware (USB)** and the raw release number from the information message. Press **Refresh info**, disconnect, and confirm the display becomes unknown. Compare the value with Windows CORE on the same keyboard, recording OS and app versions. Do not click **Save to Keyboard**, **Apply all layers**, activation, reset or update during this check. A full onboard backup remains unavailable; configuration-write acceptance is a separate, explicitly approved step.
