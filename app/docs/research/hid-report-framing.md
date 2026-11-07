# Native feature transfers use 256 bytes, including report ID

The connected GMMK 2 96% ANSI (`320f:504b`, USB release `0016`) advertises report ID 7 with 255 payload bytes. The native transport sends 256 bytes to HIDAPI. Windows CORE's 64/264-byte JavaScript buffers remain staging/reference formats, separate from that transport contract. The rebuilt macOS app reports a successful lighting/latency/activation operation, and the owner confirms the physical keyboard's lighting changes correctly after Save. Persistence, other lighting modes and other write categories still require hardware acceptance.

## The physical descriptor establishes the supported framing

Read-only IORegistry inspection on 2026-11-07 returns this configuration descriptor for usage `ff01/0001`:

```text
06 01 ff 09 01 a1 01 85 07 15 00 26 ff 00 09 20
75 08 96 ff 00 b1 02 c0
```

`85 07` selects report 7; `75 08` specifies eight-bit fields; `96 ff 00` specifies 255 fields; `b1 02` defines Feature data. IORegistry independently reports `MaxFeatureReportSize = 256`. The application verifies this exact descriptor on connection. A different descriptor fails closed and needs a reviewed device-specific implementation, even if its VID/PID matches.

The user's original Save error is visible in the native UI: `Report 1/8 failed after 0 completed transfers`, with `IOHIDDeviceSetReport` returning `0xE0005000`. The failing lighting-properties builder creates a 264-byte buffer. The installed HIDAPI macOS implementation forwards the complete buffer to `IOHIDDeviceSetReport` for nonzero report IDs, exceeding the physical descriptor's size. The transport now removes only verified zero staging padding, preserving the report ID and every meaningful byte. Activation buffers are zero-padded to 256 bytes. No alternate length is tried after a failed write.

## The Windows native boundary also differs from its JavaScript builders

Static inspection uses `hiddevice-6b262a24.node`, SHA-256 `6b262a242637c5dd157a092106884a24430c44c1236e2e9a888b5728fcec5057`, from the retained CORE 2.1.23 extract. The module is never executed during this inspection.

- Export registration at `0x180025bea` / `0x180025bf1` associates `SetFeatureReport` with thunk `0x18000167c`, leading to `0x180021b80`.
- The wrapper calls thunk `0x180006852` at `0x1800221a2`, reaching `0x1800785c0`.
- That function compares supplied length with a cached 16-bit report length at object offset `0x20`. The smaller-buffer path copies input and zero-fills the remainder before the native call at `0x1800786c8`; larger buffers pass through unchanged.

This establishes native padding for short Windows buffers, not a successful capture of an oversized Windows transfer or its final USB length. macOS framing follows the actual device descriptor rather than assuming Windows API tolerance applies to IOKit.

## Verification separates transfer success from device behavior

The transport regression fails against the former direct-send behavior (`264` versus `256`). Tests cover all six implemented command types, payload preservation, activation padding, nonzero overflow rejection, malformed staging buffers and descriptor mismatch. Batch preflight rejects a bad later frame before sending an earlier frame. The error-footer regression verifies the complete message fits both 1440×880 and 1000×700 windows with a dirty lighting draft.

The rebuilt verification app detects the physical keyboard after descriptor validation. During the owner's interaction, the UI reports: `Success: Sent lighting, input latency and activation for profile 1 layer 1 (not read back)`. The owner answers yes when asked whether the physical keyboard's lighting changes correctly after that Save. Together these establish successful transport and owner-confirmed visible lighting behavior for this operation. EEPROM persistence, input-latency behavior, settings readback and all-feature acceptance remain unverified. The user's preceding local draft is exported before restart; that file is not an onboard backup. Further UI testing pauses while the owner edits the app, rather than replacing those edits with the preserved draft.

Slot/layer notification opening still fails separately with `0xE00002E2` (not permitted). Read-only inspection of System Settings → Privacy & Security → Input Monitoring shows an enabled `Glorious Core.app` entry but no `GMMK Verification.app` entry. Permission for that other app does not establish permission for the verification wrapper. Granting access and retesting requires owner confirmation; no privacy setting is changed. Full onboard settings download remains unverified; see [Windows startup evidence](windows-hardware-readback.md). Configuration discovery and descriptor inspection do not send settings reports.
