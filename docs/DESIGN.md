# GMMK Config — design decisions

GMMK Config is a development-stage stock-firmware configurator for the **GMMK 2 Full Size 96% (US ANSI)** (`0x320F0x504B`). Its target is Windows CORE 2.1.23 appearance and functionality on macOS and Linux. The current implementation provides local profile editing and source-derived HID writes; hardware verification, full feature parity and release packaging remain incomplete.

## Chosen stack

| Decision | Choice | Rationale |
|----------|--------|-----------|
| UI | **egui + eframe** (pure Rust) | You requested native UI over a system WebView. Single process, no Chromium/Electron, predictable cross-platform builds for macOS (Apple Silicon) and Linux. |
| HID | **hidapi** (Rust crate) | Same API family as community tools; works on macOS and Linux with minimal permissions when scoped to vendor interface `0xFF01/0x01`. |
| Electron | **Not used in release** | Legacy `electron/` tree remains for reference only; release builds come from `app/`. |

Alternatives considered: **Tauri 2** (lighter than Electron, reuses HTML UI) was recommended for velocity; **GTK** was deprioritized for macOS parity. egui trades some polish for size and simplicity.

## Product choices (confirmed)

| Topic | Decision |
|-------|----------|
| App name | **GMMK Config** — bundle id **`com.ethannjw.gmmk-config`** |
| Target scope | Windows CORE parity for the initial keyboard, with future devices kept separable. Keys, basic lighting, profiles and performance have partial implementations; macros and broader host features remain required work or explicit scope decisions. |
| Editor state on connect | **Import required** — connecting does not populate keymaps from the keyboard. Import a `.gmmkprofile.json` (or app export) before editing. |
| Auto-write on connect | **Never**. Draft edits and navigation stay local. Apply, save and activation are explicit device actions; information refresh returns cached USB metadata and sends no feature reports. |

## Persistence model

| Storage | What lives there | Travels with keyboard |
|---------|------------------|------------------------|
| **Onboard target** (3 profile slots × 3 layers) | Source-derived key/lighting writes and global polling setting; unrelated AltGr commands are omitted | Persistence after power cycle remains a hardware acceptance gate |
| **Local** (files / memory) | Imported draft and explicit `.gmmkprofile.json` exports | No — files are copied manually; automatic backups are not implemented |

### Read-back and write limitations

- The inspected Windows reference stores profiles locally and subscribes to device notifications. A populated reference UI does not establish a full onboard-settings download.
- No verified full-profile request/response contract exists in this implementation. Download remains disabled, and defaults never substitute for failed reads.
- **Keybinding → Save to Keyboard** sends only the selected layer's key matrix, retaining all unedited bytes. Lighting Save sends the displayed base effect and supplied custom colors, including input latency and activation, without keybindings or polling. Performance Save still writes the entire selected layer. Dashboard apply writes all three layers; performance-only saves and profile-wide lighting migration remain required.
- Firmware information comes from the USB release number cached during connection, matching the recovered Windows native getter. Refresh returns only that cache; disconnect/error clears displayed metadata. There is no feature-report firmware query or update-status claim.
- Explicit activation uses the Windows 64-byte selection frame and 550 ms delay; Keybinding save uses a 264-byte frame and 150 ms delay. Lighting Save uses 264-byte data frames with 100 ms delays and 64-byte selection/commit frames with 550 ms delays. Explicit empty custom data sends four zero pages; unspecified data sends none. Side-light preset byte 7 follows the target descriptor, not the retained legacy file toggle. Bulk-path commits/timing, full side-LED editing and hardware persistence remain required. See `app/docs/research/core-parity.md` for evidence and release gates.

## Native UI

The native app uses the reference's four tabs: **My Keyboard**, **Performance**, **Lighting**, and **Keybinding**. Profile tiles select a local slot without activating hardware. All 99 physical keys appear in a drawn preview; key selection loads the draft binding, and changing layers refreshes the editor. The 1440 × 880 default viewport, dark rail and header follow the captured screen structure. Fonts, artwork, richer dialogs and several controls still differ from CORE.

Headless egui tests drive pointer interactions and observe the command queue. Native-window visual comparison, accessibility acceptance and platform installation tests remain pending. Original vendor renderer code, images and fonts are research inputs, not shipping UI dependencies.

The UI allows one pending write at a time across save, whole-slot apply and activation controls. Submitted payloads and targets are snapshots; continued local editing does not alter them. Completion releases the write controls without clearing unexported changes. Command-queue failures and worker shutdown are surfaced explicitly, and no retry is automatic.

## Safety rules

1. Identify device by **VID/PID + serial key + HID usage page/usage** before any write.
2. Reject bootloader PIDs (`0xB00B`, `0xB00C`) for configuration.
3. Serialize all HID operations on a single worker thread; report transfer errors without automatically retrying. Bounded-timeout and unplug/reconnect acceptance remain pending.
4. Validate profile files (schema version, serial key, buffer lengths) before apply.
5. Patch the imported matrix bytes only at the edited key addresses. This preserves other imported bytes, not unknown settings currently on the keyboard. A factory template is not an onboard backup.
6. No retries on failed writes unless the operation is idempotent (none proven for matrix chunks).

## macOS permissions

- Open vendor HID interface: user may need **Input Monitoring** for the app binary (same class of restriction as CORE / IOKit feature reports).
- Document path: System Settings → Privacy & Security → Input Monitoring → enable **GMMK Config**.

## Linux permissions

- udev rule granting user access to `0x320f:0x504b` on interface with usage page `0xff01` — see `docs/linux-udev.md`.

## Profile file format

- MIME-style type: `gmmk-config-profile`
- Version: `1`
- See `app/src/profile/format.rs` for schema.
