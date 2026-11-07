# GMMK Config

**GMMK Config** (`com.ethannjw.gmmk-config`) is a lightweight **stock-firmware** configuration tool for the **Glorious GMMK 2 Full Size 96% US** (`0x320F:0x504B`). Its vendor HID implementation is source-derived from Glorious CORE; Windows **2.1.23** is the parity reference. Native writes use the keyboard's verified 256-byte HID framing. One lighting Save has owner-confirmed physical behavior; broader hardware acceptance remains incomplete.

The shipping architecture is **native Rust + egui** (no Electron), targeting Windows CORE **2.1.23** appearance and functionality. The current app is **development-stage, not release-ready or hardware-certified**. The legacy Electron prototype remains under `electron/` for reference only.

## Supported hardware

| Field | Value |
|-------|--------|
| Model | GMMK 2 Full Size 96% (US ANSI) |
| Serial key (CORE) | `0x320F0x504B` |
| VID:PID | `0x320F:0x504B` |
| Bootloader (blocked) | `0x320F:0xB00B` |

ISO (`0x505A`) is not enabled in v1.

## Local development

```bash
git clone https://github.com/ethannjw/gmmk-core.git
cd gmmk-core/app
cargo test --locked
cargo build --locked
./target/debug/gmmk-config
```

1. Startup shows a device chooser and automatically detects the supported keyboard. Click its card to open **My Keyboard**. First entry loads explicitly local defaults; it does not download or change onboard settings. Import is optional. Factory templates are not keyboard backups. Returning to **Devices** preserves the in-memory draft.
2. Use **My Keyboard** to choose the local slot, **Performance** for polling/latency, **Lighting** for presets, and **Keybinding** to select and edit a key. These actions do not send device commands.
3. Use a profile menu to rename, import/export or restore that local profile after confirmation. Import/export files contain all three profiles. **Settings** opens device/file tools, including manual Connect/Disconnect and explicit slot/layer activation. Manual Disconnect pauses automatic connection until Connect or Retry. **Refresh info** displays cached USB release metadata without feature reports; **Firmware (USB)** does not claim update status. Export edits before closing the app.
4. Hardware acceptance is still pending: use only an approved, recoverable test profile, and quit Glorious CORE before connecting. **Keybinding → Save to Keyboard writes only the selected layer's key matrix**. **Lighting Save sends the displayed base effect and supplied custom colors, including input latency and slot/layer activation**, without keybindings or polling. Clear custom lighting stages empty pages; Save sends them. **Performance Save still writes the entire selected layer**. The dashboard's **Apply all layers in slot** writes all three layers. All paths validate their payload before transfer. See `docs/PERSISTENCE.md`.

The macOS test suite covers profile validation, report encoding, explicit write boundaries, passive notification parsing, discovery transitions, chooser/navigation, local profile menus and the existing settings editors. Layout checks include default and minimum window sizes, all 99 key legends, lighting controls and error visibility. The [testing reference](app/docs/TESTING.md) records current counts and commands. Native macOS inspection verifies automatic configuration connection, chooser entry and dashboard navigation; the owner confirms the physical lighting changes after Save. Notification access, other hardware behaviors, persistence/recovery, Linux runtime checks, full Windows equivalence and distribution remain pending. Hosted test/build results appear in the repository's Actions tab and do not certify physical keyboard behavior.

Automatic connection and **Settings → Connect** open the configuration interface and attempt a passive slot/layer notification listener. The tools window displays the last valid notification separately from the local draft; it never imports settings or completes a pending Save from that event. Notification access errors remain separate from configuration access. Stable device-open errors wait for explicit retry or removal/reinsertion. No notification-enable write is sent. macOS handles use shared access to avoid seizing the keyboard. Physical notification delivery remains unverified.

For physical testing, follow the [hardware acceptance walkthrough](app/docs/HARDWARE_ACCEPTANCE.md). It separates connection checks from owner-approved writes and requires a tested recovery method. See `docs/DESIGN.md`, `docs/PERSISTENCE.md`, and `docs/PERFORMANCE.md` for implementation details.

## Permissions

### macOS

System Settings → **Privacy & Security** → **Input Monitoring** → enable **gmmk-config** (or the signed app name once packaged).

### Linux

Install `docs/linux-udev.md` rules; user must be in `plugdev` or use `uaccess`.

## Partial implementation (hardware acceptance pending)

| Feature | Status |
|---------|--------|
| Visual key map (96% US) | All 99 keys mapped to unique visual positions and LED addresses |
| Reference-style native screens | Automatic device chooser, profile dashboard and menus, four tabs, licensed fonts and original vector keyboard; remaining full-app parity pending |
| Performance | Polling/latency draft controls; device OS-mode selector unavailable |
| Key remap (HID + modifiers, eight media actions, disable) | Five keyboard categories with 102 mapped choices; five unresolved choices disabled; local editor + scoped Save; hardware acceptance pending |
| Mouse-button/scroll bindings | Not implemented; active Windows identifiers and its legacy encoder map disagree; reference checks document the mismatch |
| Lighting presets + per-key editor | Paint/erase 99 keys; All also includes 20 side LEDs; RGB/HEX, breathing, six other groups and clear; scoped Save; hardware acceptance pending |
| 3 onboard profile slots × 3 layers | Local draft + apply |
| Profile export/import (versioned JSON) | Yes |
| Switch active onboard slot/layer | Yes — explicit |
| Observe keyboard slot/layer changes | Passive notification listener; preserves local drafts; physical acceptance pending |
| Macros | Not implemented; editor, encoding and physical execution acceptance pending |
| Read keymap from keyboard | **Disabled** — no verified full-settings download contract |
| Firmware information | USB release metadata; source-derived formatting, physical comparison pending |
| Firmware update / bootloader | **Not implemented** |

## Legacy Electron prototype

```bash
npm install && npm start   # requires Node; ~150MB+ idle RAM
```

The prototype is research code, not a supported daily-use release or backup tool.

## Docs

- [Native verification](app/docs/TESTING.md) — local commands, CI scope, Windows reference checks and acceptance limits
- [Hardware acceptance walkthrough](app/docs/HARDWARE_ACCEPTANCE.md) — connection checks, approved write tests, recovery and a results template
- `app/docs/research/windows-ui-map.md` — active Windows screens, handlers, appearance inputs and bounded REA evidence
- `app/docs/research/core-parity.md` — reference hashes, alternative projects, release gates and open decisions
- `app/docs/research/windows-native-firmware.md` — native getter trace and read-only metadata acceptance checklist
- [Macro reference checks](app/docs/research/windows-macros.md) — editor/upload evidence, reproduced encoding defects and implementation gates
- `docs/PROTOCOL.md` — HID subcommands
- `docs/DESIGN.md` — architecture choices
- `docs/PERSISTENCE.md` — onboard vs local
- `docs/linux-udev.md` — udev rule

The native UI follows the reference screen structure with licensed Work Sans and Space Grotesk fonts and an original vector keyboard, with incomplete custom lighting, bindings and dialogs. It does not yet match CORE exactly. Source is hosted publicly at `ethannjw/gmmk-core`, with a native test/build workflow for Linux and both Mac architectures. Release packaging, signing and clean-machine acceptance remain pending. Original vendor bundles remain outside this repository; local reference screenshots are Git-ignored.
