# Windows CORE UI reference for GMMK 2 96% ANSI

The Windows CORE 2.1.23 bundle provides the screens, navigation, styling and save handlers needed to specify a matching native UI. REA successfully analyzes a bounded selection of that renderer, and a mock-only browser preview renders the original device screens for visual reference. The current Rust/egui app is a partial editor and does **not** match the reference appearance or complete functionality.

Scope: stock GMMK 2 96% ANSI, `320f:504b`, CORE package build `2.1.23.2`. Research date: 2026-11-06. Source inspection and mocked renderer observations establish a reference; native visual parity, actual Windows runtime behavior and physical behavior still need acceptance testing.

## Active screens and interactions

Locations below are in `out/renderer-process/assets/index-401429d5.js` in the extracted Windows package. The bundle also contains legacy components with similar names. The active `App` route chooses `DevicePage$1` when a device state is selected; the GMMK 2 entry in `deviceManagementSectionsMap` selects the four keyboard sections.

| Feature ID | Active component / source line | Reference behavior | Native status |
| --- | --- | --- | --- |
| UI-01 | `App`, 105115; `NavigationComponent`, 59751 | Title bar, device navigation, connected-device landing page, settings and RGB-sync routes | Native dark shell/rail; application-wide pages incomplete |
| UI-02 | `DevicePage$1`, 91280; sections, 17403; device map, 59122 | Device header, back navigation, firmware indicator, home/performance/lighting/keybinding tabs | Four tabs and device header implemented; fonts/artwork/visual acceptance pending |
| UI-03 | `DeviceHomePage$1`, 63834; `ProfileSelector`, 63173 | Profile activation/status and active-profile reset menu | Three local-slot tiles; activation is a separate explicit Device action; reset and reference tile visuals remain pending |
| UI-04 | `DeviceAnalogKeyboardPerformancePage`, 86541 | The GMMK 2 performance route uses this component despite its name: polling, input latency, Windows/macOS selector, save/reset | Polling and latency only; OS-mode hardware semantics need tracing |
| UI-05 | `DeviceLightingPage`, 68285 | Device preview, preset and custom lighting state, brightness/speed, color controls, save/cancel; service calls separate preset and per-key state | Preset controls; physical-key paint/erase, RGB/HEX, breathing, six physical-key groups, All including 20 side LEDs, and clear. Full custom controls and profile-wide storage pending. |
| UI-06 | `DeviceKeybindingPage`, 73909; `KeyboardLayerSelector`, 73868 | Keyboard selection, binding editor, three layer choices, save/reset | All 99 keys selectable; selected binding loads into editor; three layers; five keyboard categories (102 mapped choices, five disabled), modifiers/disable and eight media actions |
| UI-07 | `MacroEditor`, 71917 | Recording, editable delay, no-repeat/hold/toggle modes and named macro saving | Native implementation missing; [macro evidence and isolated checks](windows-macros.md) identify encoding boundaries and a hold-mode exception |
| UI-08 | `DeviceService`, 59132 | Explicit IPC operations for lighting, bindings, performance and resets | Partial HID worker; host integration and notification reconciliation incomplete |

Feature gating matters. The GMMK 2 descriptor at renderer line 32344 declares RGB sync and the `sideLightSwitch` capability. The latter supplies a constant to the preset report builder; it does not establish a user-facing on/off checkbox. Standby requires `hasStandbyMode`; HE calibration requires an HE product; console mode is restricted to Eternal products. Those controls must not be added to this device simply because the shared component contains them. The visible OS selector needs separate handler/hardware verification before claiming that changing it affects this keyboard.

The older `KeybindingLayers_Keyboard` collection contains four entries plus a sentinel. It is not sufficient evidence for changing the active three-choice `KeyboardLayerSelector`. Likewise, legacy `KeyboardLightingManagementPage` and `KeyboardKeybindingManagementPage` are not the selected device route's primary components.

## Profile controls and data source

`DeviceHomePage$1` renders `ProfileSelector` (63173), which loads `GetDeviceProfiles` and renders `ProfileTile$1` (63084). For GMMK 2, the IPC handler at main line 73030 reads `DataStorage.getDeviceInstance`, then `DataStorage.getDeviceProfiles`, filtering by the instance's stored profile IDs. The storage method at main line 3903 reads `DeviceProfilesData` from the configured data-store file and reconstructs profile records. The separate A5 hardware path in that IPC handler does not apply to this keyboard. This traces the displayed list to local storage; it does not prove that firmware lacks a separate settings-read capability.

The active tile shows the stored name, a plus/check activation icon and a More menu. That menu contains **Reset to default**, enabled only for the active profile. Its import/export entries are commented out, and this active component has no rename dialog. Other application routes and legacy/cloud profile components must be scoped separately rather than used as evidence for this keyboard page.

Activation calls `RequestProfileChange` (main 73055), which delegates to the device handler's `setActiveProfile`. `ProfileSelector` rejects further activation while `activatingProfileId` is set and clears it in `finally`. The native app retains separate draft selection and explicit activation to avoid configuration writes during navigation. Its shared in-flight guard blocks repeated saves/activation and connection controls until the pending write receives a result. Draft edits and export remain available, and the pending message retains the submitted target even after local navigation. Failed queue submission is reported as not queued; worker loss during an accepted request is reported as an unknown outcome, with no retry.

### Profile notifications carry slot/layer observations, not settings

The GMMK 2 96% ANSI descriptor at main line 11859 separates the configuration interface (`0xFF01/0x01`) from the notification interface (`0xFF00/0xFF00`). `GMMKDeviceHandler.init` at line 60250 initializes the notification interface. Its `handleDeviceMessage` accepts report ID `4`; event `241` consumes bytes 2 and 3 as one-based profile/layer indexes. It selects an already stored local profile, changes that profile's active-layer field and publishes the resulting local state. It does not decode a lighting effect from byte 4 or download a keymap. Events `242` and `249` are logged without reconstructing lighting state in this handler.

The native parser accepts the four-byte prefix, validates both indexes against the three-slot/three-layer target and ignores trailing bytes. Connect opens the configuration handle first, then attempts a separate passive notification handle. Ambiguous matching interfaces are rejected instead of selecting another keyboard silently; notification failure does not discard an otherwise opened configuration connection. The worker polls nonblocking reads with a 20 ms command wait and a maximum of 64 reads per iteration, coalescing valid profile events to the latest observation. Other report contents are discarded, not logged. Read failure closes the notification handle and reports an error without retry. Explicit Disconnect closes both handles.

The Device panel labels the result **Last keyboard notification**, keeping local selection, keymaps, lighting and dirty state intact. Notifications neither create a profile nor confirm a pending write. Disconnect, worker loss and reader failure clear the observation. A new submitted write also clears the previous observation; later notifications remain observations rather than write acknowledgments.

The pinned `hidapi` 2.6.7 dependency uses `macos-shared-device`, so opening the notification interface does not request exclusive seizure of its keyboard/media collections. The existing Linux rule grants access to the target's HID interfaces. Native reader and UI behavior have mocked tests; physical event delivery, ordinary typing/media behavior and platform acceptance remain unverified.

The original `init` also calls `setPollingRate`. Native Connect deliberately sends no feature-report query, polling command or notification-enable write. Firmware may therefore remain silent. An empty observation is unknown state, not proof of the active slot or of reader failure. Passive macOS registry inspection on 2026-11-06 confirms that this connected keyboard exposes both expected primary usages; no HID open or event capture is established by that inspection.

From the repository root:

```sh
node app/scripts/check-windows-notification-reference.cjs /path/to/CORE-2.1.23/out/main/index.js
```

Thirteen isolated checks pass under Node.js 22.22.0. The checker executes the reviewed, hash-pinned handler and its local-state setter, with synthetic profiles and stubbed storage, logging and renderer-event publication. It covers all nine slot/layer combinations with four-byte data, all possible trailing-byte values, another report header and the original assigning/resetting guards. It does not execute `init`, native addons, HID calls, host-launch actions or real storage. The VM is not a general security sandbox. Native strict bounds checks and draft preservation are intentional safety behavior; this does not claim complete Windows runtime equivalence.

## Trace actions to their effects

These chains combine REA's bounded syntax observations with direct inspection of the original, unsliced bundle. Splitting declarations into research files leaves cross-file symbols unresolved; REA alone does not prove the complete chains.

| Action | Renderer service and IPC | Backend evidence / implication |
| --- | --- | --- |
| Save bindings | `DeviceKeybindingPage` → `DeviceService.updateAllKeybinds` (59281) → `PropertyUpdate_UpdateKeyboardKeybindings` | `out/main/index.js:60528`, `GMMKDeviceHandler.updateAllKeyBinding`: macro data when needed, then assignment; no AltGr clearing in this path |
| Save preset lighting | `DeviceLightingPage` → `DeviceService.setPresetLighting` (59193) → `PropertyUpdate_PresetLighting` | Preset state has a separate operation from per-key colors; do not treat a generic successful write as verification of both |
| Save custom lighting | `DeviceService.setPerKeyLighting` (59204) → `PropertyUpdate_PerKeyLighting` | Per-key encoding in main around 46522 also handles side LEDs; native support is incomplete |
| Save performance | `DeviceAnalogKeyboardPerformancePage` → `DeviceService.updateKeyboardPerformance` (59382) → `PropertyUpdate_KeyboardPerformance` | Renderer preserves win-lock/encoder state rather than resetting it when saving unrelated controls |
| Read current onboard settings | No verified full-profile request/response chain identified in the inspected GMMK 2 paths | Native download stays disabled. Local profiles and notifications are separate from a full onboard backup |

### Preset effect selector

The active selector exposes **18 choices on two pages of nine**, although the GMMK 2 descriptor contains 19 effect encodings. `PresetLightingEffectSelector` at renderer line 64088 removes `Surmount` before building its option pages. The native picker follows that filtering and the English labels, including **Digital Times** and **Kamehemeha**. Protocol encoding and profile validation still accept all 19 values; hiding a tile does not remove imported data.

The selector CSS at lines 3488–3563 defines a three-column grid of 120 × 40 px tiles with 12 px gaps, 3 px corners, `#515151` borders and `#262626` backgrounds. Selected tiles use `#C5B4E3` with `#1C1C1C` text. Two 8 px page dots use gray `#909090` and active orange `#FFA40D`, with previous/next arrow buttons. Native headless checks verify this tile geometry and color selection at 1440 × 880 and 1000 × 700. The control area reserves enough width for the grid instead of squeezing its columns. This does not establish whole-screen geometry, reference font weight, tooltip, animation or native-window parity.

Import and reopening Base Effect select the page containing the current effect without changing the draft. Dot/arrow navigation does not select an effect or send device commands. Selecting a tile changes only the current layer's effect; existing brightness, speed, color and unrelated profile fields are preserved. The native per-layer storage model remains a separate parity gap.

Color preservation on effect selection is also a parity gap: the reference `presetEffect_OnChange` replaces the palette when selecting Breathing with seven colors: red, orange, yellow, green, azure (`#007fff`), blue and violet (`#8b00ff`). An isolated callback check verifies this replacement and that it does not update the shared brush immediately. The native single-RGB model cannot represent this palette. Palette persistence requires the lighting-model decision; preserving RGB here describes current native behavior, not equivalent Windows behavior.

The reference initializes its page from the unfiltered catalog. Consequently, an imported `Surmount` starts on an empty page with neither dot active; Previous or either dot returns to the visible choices without replacing the imported effect. The native picker preserves this observed behavior rather than silently normalizing the profile. All 19 imported effects have native preservation tests; a design change to this reference edge case would require a separate decision.

From the repository root:

```sh
node app/scripts/check-windows-effect-selector-reference.cjs /path/to/CORE-2.1.23/out/renderer-process/assets/index-401429d5.js
```

The checker runs **23 isolated checks** against the hash-pinned renderer, stylesheet and adjacent `i18n/en.json`. It executes only the reviewed selector, catalog, identifier and display-option declarations with stubbed React hooks/JSX, support-data lookup and icons. It verifies target catalog selection, English labels, all initial pages, filtering, click callbacks, navigation boundaries and CSS dimensions/colors. It does not run the Electron bootstrap, a browser layout engine or HID. The English catalog SHA-256 is `d5f45100b2047600d95620e62726d1243d0740cf4051b5cbd84e5327a903f5bf`; vendor files remain outside the repository.

### Inline color surface and numeric input evidence

The native preset and custom editors include an inline saturation/value area and hue strip. The preset section uses the reference heading **Effect Color**. Surface width follows the available control column; total height is 200 px, with a 164 px saturation/value area, 12 px black separator and 24 px hue strip. The saturation/value pointer is 28 px across; the hue pointer is 7 × 28 px with 3 px corners and a 1 px white border. These dimensions come from the renderer's injected react-colorful CSS and the adjacent stylesheet, not from a native screenshot comparison.

The surface uses the reference's raw RGB-channel HSV conversion, rather than egui's gamma-linearized conversion. RGB imports round HSV components; edits retain fractional HSV values and round output RGB bytes. Rendering alone preserves the original RGB bytes, including near-gray colors whose rounded HSV representation would otherwise change them. Widget state is scoped by editor, slot and layer, retains hue at black, and resynchronizes when the external RGB value changes.

Pointer edits clamp to the surface bounds. Arrow keys change saturation/value by five percentage points or hue by 18 degrees; up/down do not change hue. Focus stays on the edited control while using arrows, and Tab can leave it. Preset changes affect only the selected local layer until explicit Save. Custom changes select the brush without dirtying a profile until keys are painted. Headless tests cover these rules, gradient meshes, pointer dimensions, imported-byte preservation and controls reached by scrolling.

The hash-pinned color checker has **68 checks**: 36 surface/math checks, 14 valid numeric round trips, ten malformed-input characterizations, four numeric component/callback checks and four lighting-color initialization/handoff/palette checks. The shared [color vectors](../../assets/windows-color-surface-vectors.json) contain 14 RGB imports and 19 HSV edits. The checker executes reviewed conversion functions and isolated components with mocked React hooks and elements. React effects, settings persistence, native addons and HID do not run. Numeric callback checks exercise independent edits from one rendered state, not a browser's full event/render lifecycle. Lighting checks execute the extracted RGBA and preset-state classes and callbacks with a passthrough debounce; they establish callback behavior, not the 100 ms timing. Any attempted device-service access fails the harness.

The native brush starts with the selected base-effect RGB when entering Lighting. Editing base RGB updates the brush; entering Custom Per Key retains it, and returning to Base Effect restores the base RGB. A fresh Lighting-page entry selects Base Effect, static painting and Add. Clicking the already-selected Lighting tab preserves those controls; switching between Base and Custom does not reset Add/Erase or breathing. Navigation and color handoff preserve saved/draft profile bytes, dirty state and pending writes and send no device commands. Native tests exercise rendered mode/tab clicks and base-color edits. The reference evidence is `currentColor` initialization, `presetColor_OnChange`, the `LightingStepToggle` callback, transient `useState` defaults and conditional `DeviceLightingPage` mounting in `DevicePage$1`; a full Windows React lifecycle comparison remains pending. Native drafts are retained across page changes rather than discarded as part of this transient-control synchronization.

```sh
node app/scripts/check-windows-color-surface-reference.cjs /path/to/CORE-2.1.23/out/renderer-process/assets/index-401429d5.js
```

The active `ColorPicker` (renderer 65875) defaults to HEX and lists RGB before HEX in its format dropdown. Numeric inputs precede the inline surface; RGB fields are ordered Red, Green, Blue. The reference `InputText` defaults to text input with validation disabled. Its callbacks publish edits immediately. `rgbToHex` (65757) applies JavaScript `parseInt` without range checks, while `hexToRgb` (65764) checks string length but not the leading hash or complete hexadecimal syntax. Examples established by the isolated checker:

| Input | Reference conversion/callback result |
| --- | --- |
| Red `256`, green `34`, blue `56` | `#1002238`, an overlong color string |
| Red `-1`, green `0`, blue `0` | `#-10000` |
| Empty red, green `0`, blue `0` | `#NaN0000` |
| Red `12junk` or `12.9`, other channels zero | `#0c0000` |
| HEX callback receives `abc` | Publishes `#abc`; the numeric parser returns zero channels for this short string |
| HEX parser receives `#GG2238` | Returns `NaN`, `34`, `56` strings |

These results characterize the selected source helpers and callbacks, not the eventual device output. A native RGB/HEX replacement needs an agreed invalid-input policy rather than silently copying defects or claiming stricter validation is identical. The existing native numeric popup and custom HEX field remain in place. The reference format dropdown, saved-color palette, exact surface corner/shadow rendering, surrounding typography and native visual acceptance remain incomplete.

### Saved-color palette and persistence

The active `ColorPicker` at renderer line 65875 loads application settings with `AppService.getAppSettings` on mount. Saved colors belong to this application-wide store, separately from device profiles and onboard lighting. `AppSettingsState` starts with six colors in order: `#ff0000`, `#4f16f9`, `#15fd07`, `#e101dc`, `#ffe501`, `#ffffff`. The swatch list protects its first six positions from the Delete menu. Additional colors may duplicate existing colors; the Add callback has no deduplication.

Selecting a swatch changes the editor color and selected index without saving settings. Add appends the current color, calls `AppService.saveAppSettings`, then selects the last swatch after success. An active, deletable swatch exposes **Delete** on right-click; left-click closes its menu and selects it. The stylesheet specifies 24-pixel circular swatches, wrapping with an 8-pixel gap, an outlined active swatch and a transparent 24-pixel circular Add button.

The storage path is `AppService.saveAppSettings` (renderer 9051) → `AppChannel.SaveAppSetting` (main 72742) → `DataStorage.saveAppSettings` (main 3858). The storage method serializes the complete application settings and calls the store's save method. The IPC handler also updates Windows login-item settings from the existing `startup` preference; that host operation is outside the isolated checker and must not be introduced merely to implement a native color palette.

The inspected callbacks contain behavior requiring a parity decision:

- Deleting an active middle custom swatch changes the editor color to its previous neighbor, but successful persistence sets the highlighted index to the final swatch. With custom colors `#123456`, `#abcdef`, `#fedcba` after the defaults, deleting `#123456` changes the editor to white while highlighting `#fedcba`.
- With an injected save failure, Delete still publishes the removed palette and changes the active editor color before the unsuccessful save result arrives. It also mutates the existing settings object. Add likewise mutates that object before persistence, even when failure prevents its success-state setters.

The [saved-color checker](../../scripts/check-windows-saved-colors-reference.cjs) runs **10 isolated checks** against hash-pinned renderer, main and CSS sources. They cover default serialization, explicit mount loading, selection, duplicate Add, deletion/menu rules, injected failures, the selected-index mismatch, storage-boundary serialization and CSS geometry. Actual color/settings classes and component callbacks run with mocked React hooks, JSX, application services and storage. Effects do not run automatically; the load check explicitly invokes the mount callback. No application bootstrap, native backend, login-item change, disk persistence or HID runs. These are callback and source-contract checks, not complete React lifecycle or Windows runtime acceptance.

```sh
node app/scripts/check-windows-saved-colors-reference.cjs /path/to/CORE-2.1.23/app
```

The native palette remains unavailable. Its implementation needs a decision on whether to preserve these defects or keep the selected swatch consistent with the active color and preserve all editor state after unsuccessful persistence. This is separate from the existing invalid-RGB/HEX decision; both affect the remaining color-editor work.

### Preset controls depend on the selected effect

The native preset editor hides controls that the reference makes unavailable, preserving their stored draft values. Visible controls follow the reference's Color, Brightness, Speed order. Rendering, switching effects and navigating pages send no keyboard commands. Two headless regressions cover all 19 effect IDs and pointer-driven transitions through Off, Normally On, Acid Mode, Breathing and Wave #1; they check control visibility and order, unchanged hidden values and an empty command queue.

| Control | Hidden for |
| --- | --- |
| Brightness | LED Off (`8`) |
| Speed | LED Off (`8`), Normally On (`6`) |
| Color | LED Off (`8`), Glorious Mode (`0`), Acid Mode (`5`), Rainbow (`13`) |

Source: `DeviceLightingPage` slice lines 558–625 gates brightness and speed. Renderer lines 68076–68189 define forced/hidden color lists and the GMMK 2 96% ANSI mapping; slice lines 234–246 applies them to the color-mode state. `PresetLightingColorSelector` at renderer line 66015 returns no control when `displayComponent` is false. The source hash is the pinned renderer hash in this document's provenance section.

The native color editor represents the existing single-color draft only. Windows also provides a **Chromatic / Custom** mode selector for eligible effects; that field, its persistence and its report encoding remain unimplemented. This visibility match does not establish complete lighting parity. Slider values follow the rules below; color-picker appearance and physical behavior still require separate verification. Hidden values remain in exported native profiles and existing outgoing reports; hiding a control does not change the protocol or migrate per-layer storage.

### Preset slider value rules and callback behavior

The hash-pinned renderer defines `calculateSteppedValue` at line 66071, `SimpleSlider` at 66076, `PresetLightingBrightnessSelector` at 66183 and `PresetLightingRateSelector` at 66188. GMMK 2 96% ANSI has no brightness or rate step override. The page supplies a minimum speed of 1 for this keyboard.

| Rule | Windows reference | Native status |
| --- | --- | --- |
| Brightness | 0 through 100, rounded to multiples of 5 | Implemented for pointer and numeric edits |
| Speed | 1 through 100, rounded to multiples of 5 and then clamped to the bounds | Implemented; the first values are 1, 5, 10 |
| Half-step values | Round upward: 2.5 becomes 5, 7.5 becomes 10 | Implemented for numeric edits |
| Existing non-step values | Render unchanged until edited | Preserved, including legacy speed 0; rendering does not dirty the draft |
| Text edit | Local text changes first; blur commits a rounded value; blank input commits the minimum | Implemented; explicit Save commits the focused input before taking the lighting snapshot |

Speed uses absolute multiples of 5, not increments anchored at its minimum. Its rounded values include 1, 5, 10 and 100, rather than 1, 6, 11 and 96. A direct egui `step_by(5)` on a range starting at 1 would therefore encode the wrong rule.

The native `preset_slider` control keeps numeric input separate from the profile until blur and rounds pointer edits before changing the draft. Five headless regressions cover numeric boundaries, decimal ties, blank/invalid input, numeric prefixes/exponents, pointer positions, legacy-zero preservation and explicit Save with focused input. The profile checks compare the entire exported draft and observe the command queue, so unrelated slots/layers and the no-write-before-Save boundary are checked. Fonts, rail/handle geometry, browser/native focus differences and the reference throttle remain outside this evidence.

From the repository root:

```sh
node app/scripts/check-windows-slider-reference.cjs /path/to/CORE-2.1.23/out/renderer-process/assets/index-401429d5.js
```

Nine isolated checks execute the reviewed selectors and slider handlers with stubbed React hooks, JSX, timers and callbacks. They verify bounds, steps, tie rounding, clamping, untouched initial values, text edits and range callbacks. The harness does not run React effects or a browser, so it does not establish layout, browser input events or full component lifecycle behavior. It executes no native backend or HID code and rejects an unexpected source hash before extraction.

One check reproduces a callback mismatch: a range change to 43 emits 45 and starts a 100 ms throttle. A second change to 67 during that interval is ignored; mouse-up stores 65 locally but does not emit another callback. The emitted value remains 45 until another accepted change. This is isolated handler evidence, not a claim about every full-runtime interaction. The native implementation does not reproduce this throttle. The reference-defect policy must be agreed before treating dropped final edits as desired parity behavior.

### Preset report builders match shared Windows vectors

`app/assets/windows-preset-lighting-vectors.json` contains protocol data, not vendor implementation code. The fixture covers 19 effects and eight parameter samples: both Custom and Chromatic at minimum, low fractional brightness, intermediate and maximum values. Each sample defines literal report prefixes; zero padding expands them to 264 bytes, and the effect's wire ID fills byte 4 of the properties report and byte 2 of the effect report.

The reference checker executes the hash-pinned `LEDEffectBufferValue$2`, `PrepareLEDPropertiesBuffer$1` and `PrepareLEDEffectBuffer$1` declarations from main lines 46455–46506. Its 152 cases compare both full reports, including reserved bytes, and verify that encoding leaves input state unchanged. The Rust `preset_reports_match_shared_windows_reference_vectors` test consumes the same fixture independently and checks the native builders against all 152 cases. It runs without the vendor bundle.

From the repository root, with the locally extracted reference available:

```sh
node app/scripts/check-windows-lighting-reference.cjs /path/to/CORE-2.1.23/out/main/index.js
```

Expected under Node.js 22.22.0: 152 passing checks. Missing input and mismatched hashes fail before extracted helpers execute. The harness supplies identity-valued effect identifiers, stamps report ID `7` as the transport does, fixes the target's side-light switch to `true`, and supplies no sleep settings. It executes no Electron bootstrap, device backend or HID operations. The VM is not a general security sandbox; only the reviewed, hash-pinned source is accepted. Some combinations exercise encoder behavior that the Windows UI would normalize; passing them does not mean every mode is offered for every effect.

The reference writes Chromatic as `1` at effect-report bytes 10 and 12; Custom leaves both zero. Both modes retain RGB at bytes 15–17. Native `build_led_effect` matches these bytes, but `lighting_reports` always passes `false`, and `LightingState` has no preset color-mode field. Therefore **the native Save path supports Custom only**, despite the lower-level builder supporting Chromatic. Adding persistence/UI wiring requires an explicit legacy-file compatibility decision; this investigation does not change existing saves.

### Lighting Save uses both drafts, not the selected subpage

The active lighting page compares the base-effect and per-key drafts independently with the current profile. When both differ, one Save calls `setPresetLighting` first and `setPerKeyLighting` second, regardless of which lighting subpage is open. A native implementation that routes Save exclusively by the selected subpage would lose this behavior. The source evidence is `DeviceLightingPage` in the renderer, lines 68678–68700, also retained in `ui-slices/DeviceLightingPage.js:394`.

For this target, the backend sequences are:

| Changed state | Report commands and sizes | Delay after each successful send |
| --- | --- | --- |
| Base effect | `7` (264 bytes), `2` (264 bytes), `1` (64 bytes) | 100 ms, 100 ms, 550 ms |
| Per-key colors | Four pages of `6` (264 bytes each), then `1` (64 bytes) | 100 ms per page, 550 ms after selection/commit |

These are source-derived sequences from `out/main/index.js:60381` and `:60506`, not hardware acceptance results. Command `1` selects the target profile/layer as part of each sequence. Command `7` also includes that layer's input latency, so a preset save cannot claim to touch lighting alone.

Clearing custom colors produces an empty key-lighting list in the renderer and still sends four zero-filled pages plus selection/commit in the backend. Native Lighting Save also distinguishes `Some(empty)`, which sends the clear sequence, from `None`, which omits custom pages. The **Clear custom lighting** control stages the empty list; only explicit Save sends it.

For preset reports, `sendLightingPropertiesToDevice` reads `sideLightSwitch` from the device descriptor. The GMMK 2 96% ANSI descriptor fixes it to `true`, and `PrepareLEDPropertiesBuffer$1` writes byte 7 as `0`. Native writes use the same constant, with no preset checkbox. The legacy `LightingState.side_light_on` field remains intact in local files but does not control that outgoing byte. The reference exposes Side Lights as a custom-color selection group. Its physical behavior still requires hardware verification.

Native Lighting Save sends the base sequence and then the custom sequence when colors are supplied, regardless of the open subpage. It preflights every frame before sending any. Unlike the Windows renderer's separate changed-state comparisons, native Save resends the displayed base state and any supplied custom state. This is a scoped write implementation, not complete lighting parity.

The native profile format remains per-layer. Windows stores the preset and per-key lighting states on the profile. Migration and conflict handling require user approval before changing existing local files; no migration is implemented by this research.

## Performance Save evidence

The GMMK 2 96% ANSI handler map at main line 72343 selects `GMMKDeviceHandler`, which inherits `GmmkBaseDeviceHandler.setPerformance` (46962). The active renderer's `onSave` at 86614 reaches that inherited method through `PropertyUpdate_KeyboardPerformance` (73313). This inheritance matters: searching only the concrete handler's method declarations misses the active implementation.

For this device, the source-derived sequence is:

| Step | Report | Payload and scope | Delay after send |
| --- | --- | --- | --- |
| Polling | Command `8`, 264 bytes | Byte 8 is the polling index; bytes 9/10 are 1/0. There is no profile/layer selector in this frame. | 550 ms |
| Latency, layer 1 | Command `7`, 264 bytes | Current profile, layer 1, profile-wide preset effect/brightness, descriptor side-light constant and requested latency | 550 ms |
| Latency, layer 2 | Command `7`, 264 bytes | Same values, addressed to layer 2 | 550 ms |
| Latency, layer 3 | Command `7`, 264 bytes | Same values, addressed to layer 3 | 550 ms |

`setPollingRate` (60457), `setInputLatency` (60475), `PreparePollingRateBuffer` (46426), `PrepareInputLatencyBuffer` (46434) and `sendReportToDevice` (60405) establish these fields, order and delays. No key-matrix, effect-data, custom-color or activation report appears in this path. Standby methods return without sending for this target because its descriptor does not enable standby. The renderer carries win-lock and encoder values through, but this inherited implementation does not write them or the OS-mode selector.

The latency loop sends the requested value to **all three layers**, while its saved-state assignment updates only the active layer and shared performance state. This source-level discrepancy needs Windows/hardware comparison; it must not be silently interpreted as a selected-layer-only operation. Likewise, an unaddressed polling report does not prove how firmware scopes that setting to profiles. Local slot selection is not hardware activation in the native app.

The isolated performance checker executes the hash-pinned inherited handler, concrete transport wrapper and report builders with stubbed HID, storage and timers. Its **114 checks** cover:

- 108 combinations of three profiles, three active layers, four polling rates and three latency values. Each compares all four complete 264-byte reports, their order, recorded delays and saved state. Lighting is fixed to NormallyOn at brightness 80 with the target's side-light capability; these cases do not cover every lighting state.
- Windows and macOS requests produce identical reports and stored records; the stored OS mode remains unchanged. This proves the inspected handler ignores the selection, not that the firmware lacks OS-mode support.
- A missing write handle produces no reports but still updates local performance state. Negative, zero and short second-write results do not stop the remaining reports or local storage updates. These are reference error-handling defects, not native acceptance requirements.
- A missing device rejects before transport or storage.

Run from the Git root with the reviewed vendor bundle outside the repository:

```sh
node app/scripts/check-windows-performance-reference.cjs /path/to/CORE-2.1.23/out/main/index.js
```

The checker rejects a missing input argument or unexpected source hash before executing extracted code. It runs selected declarations rather than the application bootstrap; the native addon, physical keyboard and Windows window are not exercised. Timer stubs record requested delays without waiting. Whether confirmed reference defects should be corrected rather than reproduced remains an owner decision.

Native Performance Save still uses the documented whole-layer apply path. Replacing it accurately depends on the pending profile-wide lighting decision, because each latency frame also includes preset properties. Constructing these frames from three independently editable lighting drafts would differ from the Windows payloads. No new performance write path is enabled from this research.

## Keyboard layer selector

The active `KeyboardLayerSelector` at renderer line 73868 presents **Layer 1**, **Layer 2** and **Layer 3** for GMMK 2. `DeviceKeybindingPage` chooses numeric labels; the Base/Fn/Fn2 labels are gated to other product families. The selector exposes a tablist with three tab buttons and selected-state attributes. Nine isolated checks in `scripts/check-windows-keyboard-layers-reference.cjs` execute the hash-pinned component, translation helper and page callback with mocked React and IPC dependencies, and verify the scoped CSS declarations and tooltip event handlers. No original application bootstrap, storage or hardware executes.

The native Keybinding page uses a right-aligned shared pill group with a `#131313` background, 1 px `#262626` border, 3 px padding and 2 px gaps. Its equal-width buttons are at least 72 px wide and 30 px high, with 18 px horizontal text padding and 13 px emphasized labels. Selected buttons use `#f5a43a` with `#1a1003` text; inactive labels use `#a8a8a8`. Hover targets `#ebebeb`, including on the selected button: the reference hover selector has higher specificity than its active class. The title is right-aligned above the group with an 8 px gap. The selected shadow uses the reference's 1 px vertical offset, 2 px blur and 35% black opacity through egui's shadow renderer; equivalent browser blur rasterization is not established.

Background opacity and text color transition independently over 120 ms with CSS `ease`. Interrupted reversals remain continuous and shorten their return; remounts settle at the current layer. The shared transition helper also serves the existing 300 ms breathing control. Eight component tests cover dimensions, painted transitions, reversal, active hover, remount, keyboard selection, disabled clicks and help-panel geometry, focus and visibility. An app-level test covers all three selections at both supported window sizes without changing profile bytes or sending device commands. Existing app tests verify selected-key binding refresh and scoped Save after layer changes.

The native heading has an 18-point information control separated from its text by 6 points. Hover or keyboard focus displays the English reference explanation below the icon, with the panel's right edge aligned to the icon. The panel is 275 points wide including its 1-point purple border and 6-point padding, with a dark background and 2-point corners. Title/body text uses 14/12-point sizes and 20/16-point line heights. Opacity and a 5-point vertical offset follow the selector's inherited 120 ms CSS `ease` transition, including shortened reversals. The painted panel does not intercept pointer input or change the selected layer; leaving the page discards stale visibility on remount. The icon is drawn independently rather than copied from vendor artwork. Original glyph contours, font weight, font metrics and native pixel parity remain unverified. The app-wide tooltip preference is not implemented; this control follows the reference's enabled default.

The reference component passes the title translation key for both title and description. The pinned English catalog stores those two entries using uppercase `Value`, while the actual `useTranslate` helper reads lowercase `value`, so the component's distinct fallback strings provide the title and complete explanation. The checker executes that helper rather than normalizing the catalog or silently correcting the repeated key. Other language catalogs and the original tooltip's asynchronous settings lifecycle are outside these checks.

Layer selection remains local draft navigation in the native app. In the reference, `handleLayerChange` invokes `RequestLayerChange` and publishes the returned state only on success. The isolated checks confirm success and rejection behavior, but do not execute the backend activation. Native hardware activation remains an explicit Device action; changing that safety boundary requires a separate decision. The old local layer controls remain on Lighting and Performance while their profile-wide model is unresolved.

The native selector occupies the existing preview-header area. Full page placement, the reference page's `translate(-50px, 100px)` transform, original fonts/weight, app-wide tooltip settings and native visual comparison remain incomplete. This control-level match does not establish whole-screen or keyboard-render parity.

```sh
node app/scripts/check-windows-keyboard-layers-reference.cjs /path/to/CORE-2.1.23/app
```

## Keyboard Function picker evidence

The active mouse-binding path has a separate identifier mismatch; see [Mouse Function encoding evidence](#mouse-function-encoding-evidence) before adding mouse assignments.

The active `KeyboardFunctionBindingPanel` at renderer line 70729 builds two dropdowns from `keyCategories` (70577), `categorizedKeys` (70605) and `KeybedComponentLegendMap` (66942). Its five categories are **Alphanumeric**, **F-Row**, **Modifiers**, **Special Characters** and **Others**. Native choices retain this order, identifier-based modifier labels and legend-based labels for other categories.

| Category | Reference choices | Source-mapped HID choices | Unresolved identifiers |
| --- | --- | --- | --- |
| Alphanumeric | 36 | 36 | None |
| F-Row | 12 | 12 | None |
| Modifiers | 13 | 12 | `Fn` |
| Special Characters | 13 | 11 | `Hashtag`, `BackslashLeft` |
| Others | 33 | 31 | `Menu`, `ScreenLock` |

The 102 numeric codes come from `DeviceFunctionCodeHIDValueMap` at main line 44618 and the keystroke branch of `GetKeyAssignmentBuffer$1` at line 46549. The five unresolved identifiers have no matching entries in that map; `getHIDValue` falls back to zero. Native displays these choices disabled, with explanatory tooltips, rather than inventing codes or turning a selection into a disabled key. In particular, the separate `ContextMenu` mapping does not establish the picker identifier `Menu`. Duplicate visible legends use distinct widget IDs.

Category browsing changes only editor selection. Choosing a verified key immediately changes the selected physical key's local matrix, with no separate assignment button. **Save to Keyboard** remains the only device-write action in this flow. The reference's `onKeySelection_KeystrokeTarget` at renderer line 73671 creates a keystroke target with no added modifier, even when the same key is selected again; native follows that rule. Unsupported imported bytes remain intact until explicitly replaced. Selecting another physical key or layer refreshes the category and modifier state; category dropdowns keep independent scroll positions.

The reference's `KeystrokeModifier` enum at renderer line 25315 contains None, Shift, Ctrl, Alt and Meta. Its **Include Modifier** checkbox reveals four mutually exclusive tiles in Shift/Ctrl/Alt/Meta order; the style uses wrapping with an 8-pixel gap. Native uses these controls and order. Enabling the checkbox only reveals choices. Choosing a tile immediately replaces the draft binding's added modifiers, and turning the checkbox off clears them. This follows `onModifierSelection_KeystrokeTarget` at renderer line 73702 and the panel callbacks at lines 70794–70805. Imported multi-modifier combinations remain intact until an explicit replacement key, modifier choice or clear action. Native preserves unsupported imported bytes instead of applying a stale picker value. The reference's keystroke builder at main line 46549 encodes Ctrl/Shift/Alt/Meta as type bytes 225/226/228/232 respectively; a key without an added modifier uses type 1.

Headless tests check category order, all 107 identifiers, 102 verified encodings at all 99 physical key addresses, imported category/modifier selection, dropdown scrolling, disabled choices, all four modifier tiles, combined-modifier preservation, same-key reselection, draft isolation and export/import preservation. Selection callbacks update only the draft; an observed command queue remains empty until explicit Save. These are source and interaction checks, not physical acceptance. Full visual and interaction parity remains incomplete.

## Multimedia binding evidence

The active renderer's `MultimediaBindingPanel` at line 70490 builds its tiles from `MultimediaOptions$1`. Selection reaches `onMultimediaOptionSelection` around line 73531, creating a local `TargetBindingState_Multimedia` before Save. The native Multimedia category likewise edits only the current selected key's draft; category navigation alone does not change it. Key/layer selection decodes supported imported media entries and displays their category.

Eight media actions have source-verified type-3 matrix encodings; see the [protocol table](../../../docs/PROTOCOL.md#multimedia-entries-inside-the-key-matrix). Native choices match their reference order and labels. The None placeholder is omitted because its backend path has no distinct disabled encoding; the explicit Disable control uses type 1/code 0. All eight tiles have headless interaction coverage. OS/player execution and visual matching remain unverified.

### Binding tile styling

Media actions and modifier choices use the reference `SelectableTile` styling instead of the native navigation pills. The renderer component at line 70212 and CSS at lines 5798–5836 specify 2-pixel corners, a 1-pixel border, 12-by-5-pixel padding and a minimum 48-pixel height. The native tiles use the reference maximum width of 117 pixels and wrap with 12-pixel media gaps or 8-pixel modifier gaps.

Normal tiles use background `#262626`, border `#515151` and white text. Selected tiles use background `#c5b4e3`, border `#d9cef4` and emphasized text in `#1c1c1c`. Hover borders use `#707070` or selected `#8477be`. These values resolve from the reference CSS root variables; its SHA-256 is `cfb93e428172b9cfff343bd4f87837f83e693aeb6c5f198ff397322bd9bc84bd`.

Two headless native tests inspect the emitted tile rectangles, text colors, selection/hover borders and modifier wrapping, while checking that navigation/hover do not modify drafts or send commands. Existing interaction tests cover media assignment and modifier editing. Native keyboard focus retains a visible border. Original fonts, CSS transitions, computed hover-background behavior and full responsive/pixel comparison remain unverified; no vendor font or artwork is bundled by this change.

## Custom lighting interaction evidence

`DeviceLightingPage` slice lines 255–333 implements Add/Erase for an individual key or a quick group. Add replaces that key's existing color and breathing flag; Erase removes its override. These actions modify preview state before Save. Native controls follow this physical-key editing behavior, with six verified groups: WASD (4 keys), Arrows (4), F Keys (12), Modifiers (7), Alphas (26) and Numbers (10). Repainting does not create duplicate entries; the native editor retains explicit empty data when the final entry is erased.

Group membership comes from `GMMK296KeybedComponentLayout` at renderer line 31815 and `QuickKeyIdGroupMap` at line 27403. WASD also belongs to Alphas; Numbers contains the top-row digits rather than the numpad; Modifiers excludes Fn. Native HEX validation accepts exactly six hexadecimal digits with an optional `#`, and invalid text blocks Add without blocking Erase.

The native editor retains full-intensity RGB bytes and encodes static/breathing entries as 255/254, matching `GetLEDPerKeyDataBuffer$1` at main line 46522. Windows additionally stores unscaled colors and an independent custom-brightness value: `PerKeyBrightness` updates the alpha of every existing key, and `RGBAColor.toArray_rgb` scales channels against a black background before encoding. Native custom brightness is unavailable rather than repeatedly scaling stored bytes and losing color information. The [custom-brightness investigation](windows-custom-brightness.md) provides 39 isolated checks, complete report comparisons using the real color conversion, and the open profile-migration decision.

The **All** quick action includes all 99 physical keys and the `Sidelight` component, which the backend expands to LED indices 120–139. The native action paints or erases these 119 LED entries using the current brush and breathing flag. It preserves unrelated imported entries, other slots/layers, bindings and preset state. Repeating an unchanged action does not dirty the draft. Erasing the last entry retains an explicit empty list; erasing absent custom data leaves it unspecified. Save remains explicit. The heading follows the English catalog's **Quick Key Selection** rather than the component's fallback text, “Quick Apply.”

The reference's **Side Lights** quick action has no members: the target declares `Sidelight` without the Sidelights quick-group tag, while `onQuickKeySelect` filters by that tag. `Sidelight` inherits the All tag, explaining why All includes it. The native Side Lights button is visibly disabled with an explanation pending the reference-defect decision. All is functional; a separate side-only action and faithful side-light preview remain incomplete.

The native quick selector sits below the keyboard preview, outside the left control column's scrolling area. Its two rows follow the reference order: WASD, Arrows, F Keys, Modifiers; Alphas, Numbers, Side Lights, All. Each pill is 110 × 32 pixels with 15-pixel gaps and an 8-pixel left margin, producing a 493-pixel painted span. The 30-pixel CSS radius is clamped to 16 pixels for this height. Enabled pills use `#262626` backgrounds and `#d9d9d9` text at 12 pixels; the heading uses `#f5f5f5` at 14 pixels with a 5-pixel gap. Disabled controls use the reference's gray palette and 0.7 opacity. Keyboard focus has a native focus outline; Enter and Space activate enabled actions. The whole selector and all 99 preview keys fit at 1440 × 880 and 1000 × 700. Original fonts, weight matching and native pixel comparison remain pending; these geometry checks do not establish complete visual parity.

The native lighting map accepts indices 0–139; the keybinding matrix remains 120 entries / 240 bytes. This separates two distinct device limits rather than enlarging the keybinding report. The JSON field remains `matrix_index` for compatibility. Existing profiles retain their values; older binaries that enforce the 120-entry lighting limit reject new profiles containing side LEDs.

### Lighting mode selector

`LightingStepToggle` at renderer line 66346 supplies **Base Effect** and **Custom Per Key** to `SegmentedToggle` for GMMK 2. Both options remain enabled for this device regardless of the base effect; the shared component's GMBK-only disabling rules do not apply. Each option sends its Boolean value, including clicks on the selected option. The native mode transition ignores unchanged selections, preserving the brush and other transient controls.

The native selector uses one gray track rather than separate orange pills: `#1c1c1c` background, 1 px `#262626` border, 3 px padding and 3 px corners. Equal-width options use 14 px centered text, `#d9d9d9` when inactive and `#d9cef4` when selected. The selected background is `#313131`. The reference CSS declares 8 px vertical and 42 px horizontal option padding, normal inactive weight and semibold selected weight, with 20 px below the control. Native vertical padding follows the reference; horizontal space follows the available column, compressing below the reference padding in the narrower custom editor at the native 1000 px minimum window. Original fonts, exact font weight/line height, the full control-column dimensions and narrow-window reference equivalence remain open visual gaps.

The indicator slides for 200 ms using CSS `ease-out`, with continuous, shortened reversals when selection changes mid-animation. First render and remount start settled. Hover and repeated selection do not restart the motion. Seven component tests cover Bézier samples, timing, repeated reversals, remounts, emitted indicator geometry, keyboard selection and disabled pointer safety. An app-level test checks both selected states and reference colors at the minimum/default viewports without mutating drafts or sending commands. Enter/Space and visible focus feedback are native accessibility support; the extracted Windows options are clickable divs, so these tests do not claim matching Windows keyboard-navigation behavior. Existing color-handoff tests cover mode changes through the rendered selector.

Three isolated reference checks execute the source components with stubbed support data and verify the corresponding CSS. They establish source behavior and declarations, not browser-computed geometry or native pixel equality. Transition mathematics follow CSS Easing Level 1's `ease-out` definition and CSS Transitions Level 1's interrupted-reversal rules; the separate native tests inspect the calculated and painted results.

### Breathing switch

The active `PerKeyLightingBreathingToggle` at renderer line 66373 delegates to the shared `Toggle` at line 63254. It shows **Breathing**, an **On/Off** state label, and a checkbox-backed slider. The text labels are separate from the clickable track. Its callback changes the page's brush flag; existing key colors remain unchanged until an individual key or quick group is painted. New entries then use static/breathing visibility bytes 255/254.

The native switch uses the reference's 48 × 24 pixel track, 16-pixel handle, 22-pixel handle travel, 8-pixel label gaps and 12-pixel label text. Resting colors, orange selected handle, pale hover handle/border and blue focus outline follow the inspected CSS variables. Pointer and Space activation change only the brush. Headless tests check both resting positions, hover/focus geometry, inert label clicks, draft preservation and subsequently painted visibility bytes, without device writes. Original font-weight matching and native visual acceptance remain pending.

The track's border/background and the handle's border/background/position transition independently over 300 ms. The reference CSS omits timing and delay, so the native implementation uses CSS's default `ease` curve, `cubic-bezier(0.25, 0.1, 0.25, 1)`, with zero delay. Interrupted changes start from the current interpolated value; reversals shorten their duration using the CSS reversing-shortening factor, including repeated reversals. A hover or disabled-color change does not restart handle travel. Initial appearance and reappearance after an absent render pass show the current state immediately. Animation state is temporary UI state, requests repaint only while moving, and never edits a profile or submits a device command. Explicit disabled colors bypass egui's additional disabled tint without making an invisible control visible.

Timing follows [CSS Transitions](https://www.w3.org/TR/css-transitions-1/#reversing) and [CSS Easing](https://www.w3.org/TR/css-easing-1/#valdef-cubic-bezier-easing-function-ease). The opaque hex colors interpolate in gamma-encoded sRGB, following the [legacy-color interpolation rule](https://www.w3.org/TR/css-color-4/#interpolation-space). Deterministic headless tests sample painted positions and colors at known Bézier points, interruption/reversal boundaries, hover/disabled changes, remounts and delayed frames. These are source-and-specification-derived tests, not a browser/native-window animation comparison or pixel-level certification of the Windows executable.

### Custom lighting group and report checks

The [shared custom-lighting vectors](../../assets/windows-custom-lighting-vectors.json) record the target's eight selector IDs in order, component counts and exact LED memberships. Six physical-key groups retain their reference memberships; Side Lights is empty; All has 100 components expanding to 119 unique LEDs. The checker extracts the pinned renderer's real key identifiers, layout item defaults, quick-group map, target layout and selector list. It combines these with the pinned main bundle's key-index map, side-index list and custom-page builders.

```sh
node app/scripts/check-windows-custom-lighting-reference.cjs /path/to/CORE-2.1.23/app
```

The **236 reference checks** comprise one selector/auxiliary-map check, eight membership checks, four quick-selector component/style checks, three lighting-mode component/style checks, four breathing-toggle checks and 216 four-page comparisons: eight groups × three profiles × three layers × three color/breathing samples. The component/style checks cover the English heading and label order, selector deduplication and callbacks, grid geometry/palette/typography, placement after the preview, and global disabled styles. Lighting-mode checks cover option availability, selection callbacks, track/label colors and geometry declarations, and the 200 ms ease-out indicator. Breathing checks cover the wrapper callback, shared On/Off checkbox semantics and disabled callback gating, track/handle geometry, typography, hover/focus and disabled styles, plus the 300 ms transition declarations with implicit timing/delay. Each page comparison checks all 264 bytes, including addressing, payload and zero-filled tails. The color object's conversion is stubbed to supply the fixture RGB bytes, so these cases cover full-intensity custom colors, not per-key brightness conversion. No application bootstrap, native addon or HID runs.

Native injected-transport tests consume the same fixtures, compare all four pages, verify their 100 ms delays and assert the existing Lighting Save command order. Headless UI tests verify All's exact indices, add/erase/no-op behavior, invalid-brush gating, preserved unknown entries and explicit Save target. Quick-selector tests cover geometry, order, enabled colors, keyboard/selector/footer separation at both supported window sizes, Enter activation of every enabled group, disabled-action safety, Space activation of All in erase mode, and visibility only for an imported custom-lighting draft. Navigation and selector edits send no device commands. Profile tests cover side-color import/export, duplicate and out-of-range rejection, and the unchanged key-matrix size. Physical behavior, persistence, native visuals and platform acceptance remain unverified.

## Appearance inputs

The reference HTML declares **Work Sans** and **Space Grotesk**. Package window metadata is **1440 × 880**; the GMMK 2 layout declares **750 × 372** device-render dimensions. These are source values, not measured screenshots.

The CSS bundle declares an **80 px** navigation width and **25 px** title bar, with a renderer-specific **30 px** macOS title-bar override. It contains global gray/gold tokens and newer component-specific tokens. Colors must be resolved for the active component and state; copying one palette across all screens would miss the actual styling.

The native device header uses the active dashboard's 20-pixel white product title and 12-pixel tab labels. The selected tab text and 2-pixel underline use `--glorange-60`, resolved to `#ffa40d`; this token is scoped to the header rather than imposed on unrelated controls. Reference CSS locations are `_product-name_1bl1a_119` at line 663, `_link_1bl1a_215` at line 711, `_current-selection_1bl1a_237` at line 722 and the root palette at line 24453. A headless regression inspects emitted text sizes/colors and the selected underline across all four pages, and checks that navigation and hover preserve the draft and send no commands. The original font faces, bold weight, tab icons and exact tab spacing remain unmatched.

The isolated preview provides reference captures at **1440 × 880**, device pixel ratio **1**. Native captures at matched viewport, scale, profile, tab and selection state remain required. Compare geometry, typography, keyboard positions, hover/focus/disabled states, dialogs and save/cancel flows. The reference uses Chromium on macOS rather than Windows Electron; font rasterization and native window decoration can differ. Original fonts, logos, images and application code are not redistributed in this repository.

## Mouse Function encoding evidence

The active `MouseFunctionBindingPanel` (renderer 70289) emits enum strings such as `LeftClick`. `handleMouseFunctionChange` (73491) places the string directly into `TargetBindingState_MouseFunction.functionIdentifier`. `DeviceKeybindingPage` supplies the preview-state callback without an additional `onKeybindingChange` normalizer. `DeviceService.updateAllKeybinds` (59281) forwards keyboard state through `PropertyUpdate_UpdateKeyboardKeybindings` (main 73203).

The main-side `DeviceKeybindingState.fromStateProperties` and `KeyBindingState.fromStateProperties` reconstruct the target without renaming its identifier. `GetKeyAssignmentBuffer$1` (46549) writes binding type **9** and passes that identifier to `getHIDValue`. The map at main 44618 contains the legacy strings in this table, but not the corresponding active enum strings:

| Active selection | Legacy map identifier | Legacy code | Active selection code in isolated path |
| --- | --- | --- | --- |
| Left Click (`LeftClick`) | `MOUSE_Fun_14` | 1 | 0 |
| Right Click (`RightClick`) | `MOUSE_Fun_15` | 2 | 0 |
| Middle Click (`MiddleClick`) | `MOUSE_Fun_16` | 3 | 0 |
| Forward (`Forward`) | `MOUSE_Fun_17` | 5 | 0 |
| Back (`Back`) | `MOUSE_Fun_18` | 4 | 0 |
| Scroll Up (`ScrollUp`) | `MOUSE_Fun_19` | 6 | 0 |
| Scroll Down (`ScrollDown`) | `MOUSE_Fun_20` | 7 | 0 |

**Sixteen isolated checks reproduce this mismatch.** They execute the extracted selection callback, serialize its output, reconstruct the real main-side model classes, and run the extracted matrix builder. Renderer and main mouse-target definitions are checked for equality. The main and renderer hashes must match the pinned reference before any extracted declaration runs.

From the repository root:

```sh
node app/scripts/check-windows-mouse-reference.cjs /path/to/out/main/index.js /path/to/out/renderer-process/assets/index-401429d5.js
```

Expected result under Node.js 22.22.0: 16 passing characterization checks. The tests stub React/state callbacks, target-type dispatch, the default matrix, device-family predicates and the physical-key offset. They do not render the original UI, load Electron/native addons, execute the full device handler or access HID. The VM is not a general security sandbox; the reviewed, hash-pinned source is required.

These results establish a source-path inconsistency, not observed Windows failure or working mouse output on the keyboard. `None` also becomes type 9/code 0; it must not be silently equated with the native disabled-key encoding (type 1/code 0). The legacy codes are candidates for intended behavior, not evidence that the active UI sends them. Native mouse assignment remains unavailable pending the reference-defect policy and runtime/hardware verification. Other shared-panel mouse functions require their own device-specific trace.

## Mocked renderer observations

The preview serves the original renderer files on loopback and replaces the final React bootstrap in memory. It mounts the original navigation, translations and `DevicePage$1` with a synthetic GMMK 2 and three synthetic profiles. The original extracted files remain unchanged. A research banner replaces the title bar content.

The fake Electron bridge records IPC calls, returns only explicitly mocked read responses and rejects other operations. The HTTP server serves GET requests under the renderer directory; browser interception and the content security policy restrict network access to the local preview. Electron main, preload and native addons are not loaded. No keyboard command is sent. The firmware **Up to Date** indicator and all displayed settings are synthetic, not device observations. Macro storage is not implemented by the mock.

Seven browser smoke checks pass: each of the four tabs opens its expected route and heading; selecting A exposes binding controls and exactly three layers; custom per-key lighting exposes its picker and eight quick-selection groups; those navigation/selection interactions invoke only the expected mock read channels. These checks exercise the reference renderer, not the native application or USB protocol. No save, reset, firmware update or physical key output is tested by this sequence.

Captures are local research files in the ignored `app/.playwright-mcp/` directory, relative to the Git root:

| Capture | Observed state |
| --- | --- |
| `windows-reference-home.png` | Profile dashboard, three synthetic profile tiles and keyboard image |
| `windows-reference-performance.png` | Polling choices 125/250/500/1000 Hz, latency choices 2/8/16 ms, Windows/macOS selector |
| `windows-reference-lighting.png` | Base Effect tab, paged effect tiles, brightness and speed controls |
| `windows-reference-keybinding.png` | Unselected-key prompt, binding-category rail and three-layer selector |
| `windows-reference-keybinding-selected.png` | A selected; keyboard-function category/key controls and modifier checkbox |
| `windows-reference-lighting-per-key.png` | RGB/HEX picker, saved colors, breathing, add/erase controls, base preview and quick selections |

The per-key quick selections are WASD, Arrows, F Keys, Modifiers, Alphas, Numbers, Side Lights and All. Their presence is confirmed; their LED mappings and persistence require separate tests.

Computed shell measurements at this viewport:

| Element | Bounds `(x, y, width, height)` in CSS pixels | Computed background |
| --- | --- | --- |
| Research title bar | `(0, 0, 1440, 25)` | `rgb(28, 28, 28)` |
| Navigation rail | `(0, 25, 80, 855)` | `rgb(28, 28, 28)` |
| Device header | `(90, 35, 1340, 142)` | `rgb(38, 38, 38)` |

These measurements specify only the observed shell. They do not establish all responsive layouts, gradients, focus states, dialogs or feature behavior.

## Native verification and differences

The native app renders four tabs, an 80-point rail, a 142-point device header, profile tiles and a two-column editor at a default 1440 × 880 logical viewport. All 99 physical keys have unique visual positions and LED addresses; identifier tests cover letter/digit prefixes, punctuation and numpad Enter. The keyboard is drawn from layout data, with default egui fonts rather than copied vendor artwork/fonts.

Before a local profile is imported, every tab shows the keyboard layout beside the import instructions. The preview is read-only and explicitly labels onboard settings as unknown; connection does not populate a draft. Headless startup tests cover the 1000 × 700 minimum and 1440 × 880 default viewports, both connection states, and the transition to editing after explicit import. These checks do not establish native-window rendering or visual equivalence to CORE.

Native UI tests cover draft/export state and rendered interactions, including navigation, key selection, layer changes, polling/latency/effect controls, explicit save gating, scoped keybinding/lighting saves, staged custom-color clearing and connected/disconnected firmware refresh. Media checks cover all eight tiles, imported selection/layer state, disabled encoding, round trips and explicit save. Per-key checks cover all 99 physical-key addresses, six quick groups, HEX text entry and rejection, breathing flags, replacing versus duplicating entries, erasing and export/import round trips. The preset picker has four regressions covering tile geometry/colors at both window sizes, all 18 choices, arrow/dot navigation, all 19 imported values and reopening the selected page. An observed command queue verifies that launch, import, navigation, connection-status updates and local edits enqueue no hardware operations. Enabled refresh queues only a cached-info request; disconnected refresh queues nothing. Keybinding save queues only the selected slot/layer's matrix. Lighting Save includes both lighting drafts from either subpage, targets the selected slot/layer and discloses activation. Neither save changes the local draft or export-dirty state. This is headless egui interaction evidence, not physical-device or pixel-comparison evidence.

Native-window visual inspection remains pending: the built preview launches, but Computer Use is not approved to inspect that application. Full custom per-key editing, additional binding categories/macros, the active-profile reset action and application-wide pages are incomplete. Keybinding and Lighting saves have separate scoped paths; Performance Save still applies a whole layer, and dashboard apply writes a whole slot, as stated in the footer. Reference-equivalent lighting draft comparison/storage and performance-only writes remain required. The device OS-mode control is unavailable. Firmware displays cached USB release metadata, or unknown when unavailable, without an update-status claim; see [native getter evidence](windows-native-firmware.md).

## Retained research evidence

Paths in the following table are local research artifacts outside the repository, under `/private/tmp/gmmk-core-windows-2.1.23/`; temporary files are not durable release inputs. The ignored screenshots listed above are separate local outputs and must not be published with the source.

| Evidence | Location / identity | Coverage |
| --- | --- | --- |
| Original renderer | `app/out/renderer-process/assets/index-401429d5.js`; SHA-256 `9b8adecdf2957e81d2d6e674af5b6759c138565612d8cbadd67c8049d9c2f4ed` | Readable bundled source, including shared/legacy code |
| Original main process | `app/out/main/index.js`; SHA-256 `96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531` | Main and renderer bytes match their entries in the retained, hashed ASAR; this provenance comparison does not execute application code |
| Original styles | `app/out/renderer-process/assets/index-cfb93e42.css`; SHA-256 `cfb93e428172b9cfff343bd4f87837f83e693aeb6c5f198ff397322bd9bc84bd` | Requires active-component/cascade inspection |
| Main/preload partial REA | `rea-windows.summary.json`, `details.partial_observation`; `ev_18efeaf191c9f62b93aef45cb4bea54dc60d1ecd1fb494017729ed49f71a56f3` | Main/preload analyzed; full renderer timed out; 171 inventoried files unselected |
| Renderer declaration provenance | `ui-slices/provenance.json`; generator `extract-ui-slices.cjs` | Fifteen verbatim AST-selected declarations, original line ranges and per-slice SHA-256 |
| Successful scoped REA | `rea-ui-slices.evidence.json`; `rea-ui-slices.summary.json`; parent `ev_9419ca7e1dfa419457b771a79eef73fcb0f62edfb54179ff090096b612556651` | Fifteen declaration files plus provenance metadata; semantic caps reached; 1,684 retained unknowns |
| Isolated renderer harness | `preview-server.cjs` | Original renderer components with synthetic device/profile state and a mock-only IPC bridge; no native backend |

The scoped run uses REA **6.8.0**, Node **22.23.4**, 1,024 MiB worker heap, 60-second per-file timeout, 10,000 total/2,000 per-file semantic-node limits and 2,000 total/200 per-file unknown limits. A successful run means the bounded analysis completed, not that every relationship or behavior is known. Neither Electron main nor native addons are executed, and no keyboard command is sent during this UI investigation.
