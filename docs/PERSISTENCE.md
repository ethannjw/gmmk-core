# Local profiles and onboard settings

**GMMK Config cannot currently download a verified full settings snapshot from the stock GMMK 2 96% ANSI.** Import/export operates on local JSON drafts. The native UI disables onboard download. Factory templates and local files are not backups of unknown keyboard settings.

## Configuration write paths

The native UI exposes these explicit write scopes. Navigation, local editing, import, export and information refresh do not send configuration reports.

Only one save/apply/activation can be pending in the UI. While it runs, other writes and Connect/Disconnect/Refresh are disabled; local editing, navigation and export remain available. The pending message identifies the submitted target rather than the newly selected draft. A success or failure result releases the controls without changing the local draft or export-dirty state. No failed operation is retried automatically.

If the worker cannot accept a request, the UI reports that the write was **not queued**. If the worker stops after accepting it but before returning a result, the UI reports that the outcome is **unknown**, disables device commands and asks for an application restart and keyboard verification. Neither case is presented as successful persistence.

| UI action | Current write scope |
| --- | --- |
| Keybinding → Save to Keyboard | One 264-byte command-3 matrix for the selected slot/layer, followed by a 150 ms delay. No lighting, polling, activation, AltGr or macro-payload commands. Unedited opaque matrix bytes are retained; macro storage is not uploaded. |
| Lighting → Save to Keyboard | Base effect and supplied custom colors for the selected slot/layer, including input latency and activation. No keybindings, polling, AltGr or macro writes. Both lighting subpages are included. |
| Performance → Save to Keyboard | Entire selected layer: keys, lighting and polling. Reference-equivalent performance-only writes remain pending. |
| My Keyboard → Apply all layers in slot | Entire selected slot, preflighted before any transfer. |
| Device → Activate | One 64-byte command-1 selection frame, followed by a 550 ms delay; no binding rewrite. |

Lighting Save sends commands `7`, `2`, `1`, followed by four `6` pages and another `1` when custom colors are supplied. Data frames are 264 bytes with a 100 ms delay; selection/commit frames are 64 bytes with a 550 ms delay. **Clear custom lighting** stages an explicit empty list and clears overrides only on Save. **Remove local override data** sets the list to unspecified and omits custom pages on Save. The legacy `side_light_on` file field is preserved for round trips but does not control outgoing preset reports: this target uses capability-derived byte 7 = 0.

Version-1 lighting remains per-layer pending an approved profile-wide migration. Unlike Windows' changed-state comparison, native Lighting Save resends the displayed base state and any supplied custom state. The transport sequence is covered by fake-transport tests; physical effects and persistence remain unverified. Performance and whole-slot apply still use the legacy bulk sequence without lighting commits.

Windows CORE 2.1.23 contains the following operations. Their presence establishes encoding intent, not completed native hardware/persistence acceptance.

| Feature | HID subcommand (`buf[1]`) | Notes |
|---------|---------------------------|--------|
| Active profile/layer switch | `1` | Does not copy profile data; selects slot/layer |
| Key matrix | `3` | 240-byte buffer per profile/layer |
| Macro key types | `4` | Skipped in GMMK Config v1 |
| Macro payload | `5` | Not implemented in v1 |
| Per-key LED layout | `6` | Four 200-byte chunks for 96% |
| Profile lighting mode | `7` | Effect index, brightness, side light |
| Global LED effect params | `2` | Speed, color, brightness |
| Polling rate | `8` | 1000/500/250/125 Hz |
| AltGr map | `9` | Not sent by native apply; unrelated AltGr state is preserved, matching the inspected Windows GMMK 2 key-update path |

Persistence after app exit, USB removal, power cycle or movement to another host remains **unverified** for the native app. A successful send does not prove a setting was accepted or persisted. Apply validates every layer in a requested slot and constructs all reports before the first transfer. Invalid data or coordinates therefore cause no transfers. A transport failure stops the operation, reports the number of completed transfers, warns that device state may be partially changed, and is never retried automatically. It cannot roll back reports already sent.

The pinned `hidapi` 2.6.7 API returns `Result<(), HidError>` for feature writes. Its C backend adapter and Linux native backend already turn incomplete sends into `IncompleteSendError`; the application propagates these errors. There is no byte count returned for the application to check. These dependency checks do not verify hardware persistence.

## Editing keyboard bindings

Select a physical key under **Keybinding**, open **Keyboard Function**, then choose **Alphanumeric**, **F-Row**, **Modifiers**, **Special Characters** or **Others**. The key dropdown contains 102 source-mapped HID choices, including punctuation, left/right modifiers and numpad keys. Five unresolved reference entries remain visible but disabled. Choosing a verified key immediately changes that physical key in the local slot/layer and clears its added modifiers, matching the Windows selection handler. **Save to Keyboard** sends the matrix.

Changing categories cannot assign a key that is absent from the displayed category. Selecting a key or another layer refreshes the picker from the draft. Unsupported imported codes are preserved, and assignment stays unavailable until a verified replacement is selected. Export saves those original bytes if they have not been replaced; it does not read onboard state.

**Include Modifier** reveals Shift, Ctrl, Alt and Meta tiles without changing the draft. Choosing a tile immediately replaces the added modifiers on the selected supported keyboard binding; turning the checkbox off clears those modifiers. These actions do not send device commands. Imported combinations remain intact until an explicit key choice, modifier choice or clear action. Modifier controls cannot replace an unsupported, disabled or media binding with a stale keyboard choice.

## Editing media bindings

Select a key under **Keybinding**, open **Multimedia**, then choose Open Media Player, Play / Pause, Stop, Mute, Volume Up, Volume Down, Next Track or Previous Track. Choosing a tile changes only the selected key in the current local slot/layer. Export preserves the draft; **Save to Keyboard** sends the matrix without lighting, polling or activation writes. **Disable key in draft** uses a separate disabled encoding.

Imported supported media entries select their category automatically when their key or layer is selected. Unknown entries stay intact until explicitly replaced. Media execution, including player-launch behavior, is not yet verified on either operating system.

## Editing custom colors

1. Import a local profile and select its slot/layer.
2. Open **Lighting → Custom Per Key** and choose an RGB color or enter six HEX digits, optionally prefixed with `#`.
3. Choose **Add** or **Erase**, then click a key or a quick group. **Breathing** applies to newly painted keys. Groups are WASD, Arrows, F Keys, Modifiers, Alphas, Numbers and All; Numbers excludes the numpad, and Modifiers excludes Fn. **All includes all 99 keys and 20 side LEDs**, not just the keys shown in the preview. Side Lights remains disabled because the inspected Windows shortcut has no mapped components.
4. Export the draft to preserve it locally. Editing and export send no keyboard commands.
5. Only after agreeing on a recoverable hardware test, use **Save to Keyboard**. It sends both the displayed base effect and supplied custom colors, includes input latency, and activates the selected slot/layer.

Erasing the final painted entry retains an explicit empty list so Save can clear custom colors. Invalid HEX input disables painting but still allows erasing. All preserves unrelated imported entries outside its 119 mapped LEDs. The preview shows static key colors, including for breathing entries; it does not faithfully display side-LED colors. Separate side-light selection, saved-color palettes, per-key brightness and animated/base-effect preview parity remain unimplemented. Profile storage stays per-layer; no migration occurs during editing.

Custom colors accept LED indices 0–139, with side LEDs at 120–139. The keybinding matrix remains 120 entries / 240 bytes. The existing `matrix_index` JSON field and format version are unchanged; older app binaries with the narrower lighting validator reject files containing side-LED indices. Export and import preserve side-color bytes and breathing flags. See the [reference fixtures and scope](../app/docs/research/windows-ui-map.md#custom-lighting-group-and-report-checks) before interpreting local data as verified device behavior.

## Onboard download is unavailable

The inspected Windows CORE 2.1.23 GMMK 2 paths maintain local application profiles and receive hardware notifications. A populated UI does not establish a full onboard download. No verified full-matrix query/response chain is identified in these inspected paths; this is not proof that all firmware lacks readback.

Selecting a profile changes hardware state. A subsequent `GetFeatureReport` has no verified contract returning the selected key matrix. The speculative reader and its factory-default substitution are removed from the native app. The legacy prototype is not a supported backup method.

Firmware information is USB metadata, separate from onboard configuration. The recovered native `GetFWVersion` path returns a cached release number; the Rust app caches `DeviceInfo::release_number()` on connection. **Refresh info** returns that cache without accessing HID. The speculative command `138` reader is removed and command `138` is rejected at the native send boundary. See [native firmware evidence and read-only acceptance](../app/docs/research/windows-native-firmware.md).

**Not downloaded:** full keymaps, macros, per-key RGB layouts or lighting presets. The EP2 parser exists but continuous reading and UI reconciliation are incomplete.

**Export** records the local draft only. It neither reads onboard state nor proves the keyboard accepted an apply operation.

The UI's unexported-change indicator clears after a successful file export or import. Hardware operations never clear it. Export failures remain visible and keep local edits marked as unexported. Successful applies are described as sent, with readback explicitly unverified.

## Host-only (not written)

| Feature | Behavior |
|---------|----------|
| Local `.gmmkprofile.json` | Stays on disk until you export/import |
| Glorious CORE local DB | Separate; may push to device on CORE connect (close CORE when using this app) |

## Safeguards

- Bootloader PIDs rejected (`0xB00B`, `0xB00C`).
- No configuration feature reports on connect or information refresh. Firmware metadata comes from enumeration and is cleared on disconnect or failed connection.
- Matrix edits merge into imported buffers (factory defaults preserved for untouched keys).

## Pending hardware acceptance

1. Record model, firmware and OS; establish a recoverable known configuration using the original app. A native local export cannot back up unknown onboard data.
2. Verify no feature reports on connect/reconnect or information refresh.
3. After write-safety tests pass, approve one minimal change in a disposable slot. Check actual output and unrelated settings.
4. Check app exit, reconnect, power cycle and another host separately; record the result for each feature.
5. Exercise layers/modifiers, lighting, hardware hotkeys and failures. For media, test each of the eight actions on macOS and Linux separately and record the OS, player, action and observed result. Macros need separate acceptance once implemented.

These steps are pending, not tests already performed. Unverified HID probes, reset and flashing require separate approval. See [release gates](../app/docs/research/core-parity.md) and the [Windows UI map](../app/docs/research/windows-ui-map.md).
