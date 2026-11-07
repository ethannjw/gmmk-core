# Windows CORE parity: research and release requirements

The target is a shipping-quality implementation of Windows Glorious CORE 2.1.23's behavior and appearance for the user's GMMK 2 96% ANSI on macOS and Linux, with device-specific support separated to allow future devices. The current repository is a partial configurator with onboard download disabled; its test coverage does not establish release readiness. Research includes static analysis, an isolated mock-backend renderer preview, and a bounded Windows startup capture against the keyboard. Release acceptance still needs native visual comparison and operation-specific hardware evidence.

Research date: **2026-11-07**. Status: **research; initial device scope and visual reference confirmed; implementation and acceptance details pending**. This document does not claim feature completion, Windows runtime equivalence, or hardware certification.

## 1. Reference and existing implementations

### Windows is the reference to capture

The official download page links `GloriousCORE_2.1.23_Setup.zip`; the official changelog dates Windows CORE 2.1.23 to **2026-10-23**. This is the agreed reference for appearance and behavior on the initial target keyboard. [S1, S2]

The current reference is **Windows CORE 2.1.23**, package build **2.1.23.2**. Static research uses its extracted Electron JavaScript, CSS, fonts and images; runtime research uses the hash-matched installation in a temporary Windows VM. The older macOS CORE 2.1.3 extract remains secondary historical evidence, not the visual acceptance reference.

Local reference provenance:

- Extract root: `/private/tmp/gmmk-core-windows-2.1.23/app` (research material outside this repository).
- Version evidence: the extract's `package.json`; renderer inventory: [Windows UI map](windows-ui-map.md).
- Download ZIP SHA-256: `edfbd8d3fc778c26f24130527a2b188a694b8154ae3476c5105f4513db415aba`.
- ASAR SHA-256: `64e3de1cdec77fba29a9db3313ec945c89dd34c22ad5f8609ec1b125ad67c7b2`.
- The installer uses Inno Setup; the research extractor uses pinned `inno` 0.7.3. Existing ASAR tooling unpacks `resources/app.asar`.
- Repository source paths below are relative to the Git root, one directory above this document's `app/` subtree.

### Other projects provide partial precedents

These are findings from a bounded search, not an exhaustive inventory. None of the inspected projects establishes a fully tested, stock-firmware, Windows CORE-equivalent replacement for this repository's GMMK 2 96% ANSI target on both platforms.

| Project | Relevant evidence | Limit for this project |
| --- | --- | --- |
| `tg5ee/inglorious-core-linux` | Rust Linux configurator; author reports hardware-tested lighting and some HE controls. Other controls are explicitly experimental. | Targets **GMMK 3 HE 75% ANSI**, VID:PID `342d:e3de`. Its documented transport uses 65-byte reports, so its commands cannot be assumed applicable to GMMK 2. [S3, S4] |
| `LatroX-dev/INGLORIOUS-Advanced` | Broad configurator with lighting, bindings, macros, and its own firmware. | Uses custom QMK-based firmware. GMMK 2 variants are experimental; macOS/Linux builds are Early Alpha. Its README declares GPL-3.0-or-later. [S5] |
| `ArshansGithub/glorious-lights` | macOS application, CLI, tests, and protocol documentation. | Targets original GMMK 1 and selected mice; its keyboard transport differs from this GMMK 2 implementation. [S6] |
| `reelmesh/openrgb-gmmk2-p96-iso` | Relevant GMMK 2 96% lighting integration. | Targets **ISO with Vial firmware**, with firmware/controller modifications; it is not evidence of stock ANSI CORE parity. [S7] |

Use these as references for investigation, diagnostics, test design, and packaging. Any code reuse needs an explicit dependency/license decision. Changing the user's firmware is a separate decision and is not authorized by this research.

### REA is relevant to the investigation

The tool in the original request is **`morluto/rea` — Reverse Engineer Anything**. Its JavaScript/Electron analysis can inspect extracted directories or ASARs and identify module, IPC, and native-addon relationships. Native investigation can use a separately configured analysis engine. [S8, S9]

REA analyzes the Windows artifact as inert text. Direct source inspection traces selected UI actions through renderer services, IPC and the device handler. Static findings still require runtime and keyboard verification; an analysis result does not certify that a command reads settings or persists them.

REA 6.8.0 successfully runs through a pinned npm package runner with Node 22.23.4. A static feasibility check analyzes four selected files from the older macOS CORE 2.1.3 renderer: the keyboard home page, keyboard keybinding-management page, device-lighting selection component, and keyboard dashboard component. It produces application and semantic graphs without executing CORE or accessing the keyboard. This establishes that the analysis workflow runs locally; it does not establish Windows 2.1.23 parity or complete UI recovery.

The source selection leaves 296 inventoried files outside analysis scope, so relationships through them remain unknown. The complete evidence bundle is `/private/tmp/gmmk-core-rea-ui-20261106.evidence.json`, with summary `/private/tmp/gmmk-core-rea-ui-20261106.summary.json`. Parent evidence ID: `ev_b6f5b019f4772363c196d66451565aef6f2e869e5e35480975310f46c2958fef`. These temporary analysis outputs are local research artifacts, not versioned release inputs.

The Windows main/preload analysis produces partial evidence before the full renderer times out. A bounded analysis of **15 verbatim top-level UI/configuration/service declarations** succeeds. The derived files retain original line locations and hashes in a provenance manifest. External declarations remain unresolved, and semantic retention limits are reached: this is useful bounded evidence, not a complete application graph. See [Windows UI map](windows-ui-map.md) for the evidence IDs and files.

No global tool installation, native provider setup or agent registration is performed. The npm runner acquires its packages in the local cache. The isolated preview executes original renderer components with a replaced bootstrap and a fake Electron bridge; it does not execute the native backend. Six captured states and seven passing browser smoke checks provide a visual/navigation reference, with synthetic profiles and firmware status. See [mocked renderer observations](windows-ui-map.md#mocked-renderer-observations). These are Chromium captures on macOS, not captures from a running Windows installation, and do not verify hardware operations or native-app parity.

## 2. What the inspected original does

The Windows CORE 2.1.23 GMMK driver combines onboard configuration, local application state, and host-side behavior. These require separate parity checks. Locations in this table refer to the extracted `out/main/index.js`.

| Behavior | Evidence in the local reference | Implication |
| --- | --- | --- |
| Initializes device communication | `GMMKDeviceHandler`, line 60250 | Initialization subscribes to notifications and attempts to write stored polling state; the observed Windows startup rejects the instance lookup before sending. The native replacement deliberately retains no automatic writes on connect. |
| Stores profiles locally | `GmmkBaseDeviceHandler`, line 46911 | A populated UI is not evidence that the full settings came from the keyboard. |
| Receives hardware notifications | `GMMKDeviceHandler.handleDeviceMessage` | Profile, lighting, launch and multicolor notifications are distinct from full-profile downloads. Host launch actions require a running process. |
| Encodes bindings and macros | `GetKeyAssignmentBuffer$1`, line 46549; `updateAllKeyBinding`, line 60528 | Supports more categories than the native editor. Macro payloads precede assignment; this GMMK 2 key-update path does not clear an AltGr buffer. |
| Encodes lighting | `PrepareLEDEffectBuffer$1`, line 46491; per-key encoding, line 46522 | Speed is `100 - speed`; multicolor uses both flags. Twenty side LEDs occupy indices 120–139, beyond the 120 keybinding positions; native lighting validation supports these separate limits. |
| Exchanges feature reports | `sendReportToDevice` / `getReportFromDevice`, around lines 60405–60427 | Key assignment uses a 264-byte JavaScript staging buffer; this read helper requests 65 bytes. Native HID writes use the physical descriptor's 256-byte report size, with zero-only padding normalization. See [framing evidence](hid-report-framing.md); full hardware acceptance remains pending. |

The inspected paths do not establish a full GMMK 2 matrix-download transaction. Readback remains **unverified**, not proven impossible. Selecting a profile and interpreting a subsequent feature response as a keymap has no verified request/response contract; factory defaults cannot substitute for a failed read. The native app therefore disables onboard download rather than presenting guessed data.

## 3. Current implementation and release gaps

| Area | Current repository evidence | Required outcome before claiming parity |
| --- | --- | --- |
| Device scope | `app/src/protocol/gmmk_v2_96.rs`: GMMK 2 96% ANSI, `320f:504b` | Explicit supported-device and firmware matrix; reject unsupported interfaces. |
| Profile download | Disabled in the native UI; speculative download/default substitution is removed | A verified query/response contract before enabling download; preserve drafts on failure. |
| Write safety | Scoped Keybinding/Lighting saves; whole-layer/slot preflight; one pending write; no unrelated AltGr writes; transport failures stop without retry and report partial state | Injected-transport regressions cover activation, scoped bindings/lighting, custom pages including side LEDs, bulk operations and closed submission queues; real HID and recovery acceptance remain pending. Pinned `hidapi` already rejects incomplete sends. |
| Hardware event synchronization | Passive `0xFF00/0xFF00` listener decodes bounded slot/layer notifications; Device displays the last observation without changing local selection/settings or completing writes. Read failures stop the listener without retry. macOS uses shared HID access. | Real event delivery, disconnect/reconnect and ordinary typing/media-key acceptance on each platform; notification-enable behavior remains unverified and Connect sends no enable write. Full onboard settings remain unknown. |
| Key actions | `app/src/profile/key_matrix.rs` supports keyboard usages/modifiers, disabled keys and all eight source-mapped media actions; unknown imported entries remain opaque | Remaining target-version binding categories, faithful encoding/decoding, and host-action integration. |
| Mouse bindings | [16 isolated reference checks](windows-ui-map.md#mouse-function-encoding-evidence) reproduce active mouse identifiers becoming code zero while legacy identifiers map to distinct codes; native assignment remains unavailable | Agree reference-defect handling, verify the intended bytes, then implement and physically test clicks/scrolling on each platform. |
| Binding codecs | Thirteen tests cover keyboard categories and mapped choices, ordinary keys, all 16 modifier masks, media bytes at all 99 physical positions, unknown-code preservation, invalid input and unrelated bytes | Hardware key-output/media acceptance remains pending. |
| Lighting | Scoped Save uses source-derived framing, commits and delays; explicit empty colors clear while unspecified colors leave custom pages untouched. Preset controls hide brightness for Off, speed for Off/Normally On, and color for Off/Glorious/Acid/Rainbow without resetting drafts. Preset side-light byte follows the target capability. Per-key editor supports 99 physical keys, RGB/HEX, breathing, six physical-key groups, All (including 20 side LEDs), erase and clear. | Chromatic/Custom mode, remaining preset/custom controls, separate side-light selection/preview, profile-wide storage, changed-state comparison and physical verification. |
| Macros | Native UI explicitly disables them; [13 isolated reference checks](windows-macros.md) reproduce event packing, truncation and a hold-mode exception | Agree reference-defect handling; implement local persistence, editor/recording, validated uploads and physical execution tests. |
| Firmware operations | The native getter returns cached USB release metadata; Rust uses the corresponding enumeration field. Speculative command 138 is rejected. Updates are outside current implemented scope. | [Native getter evidence](windows-native-firmware.md); physical comparison, explicit updater scope decision and model-specific verification remain pending. |
| UI | Device chooser with automatic detection, local-default profile dashboard, rename/import/export/confirmed reset, reference typography and vector icons/99-key keyboard; four existing settings tabs retained | Remaining controls/dialogs and complete Windows visual/functional acceptance. Native chooser/dashboard inspection passes at default/minimum size; vendor images/logos are not redistributed. |
| Packaging | Rust/egui is the approved shipping architecture; root `package.json` packaging belongs to the legacy Electron prototype | Native packages verified on each agreed OS/architecture, with installation, upgrade and signing acceptance. |
| Source hosting | Owner-approved public GitHub repository `ethannjw/gmmk-core`; a test/build workflow defines Linux and both Mac architecture checks | Successful hosted CI and release packaging; workflow existence alone does not prove either. |
| Documentation | Root README, design and persistence docs distinguish implemented drafts/writes from unverified hardware behavior; the [hardware walkthrough](../HARDWARE_ACCEPTANCE.md) provides connection checks and gated write tests | Completed hardware records, fresh-machine walkthroughs and compatibility claims tied to acceptance evidence. |

The approved architecture remains Rust/egui with no automatic writes on connection. Appearance follows the Windows reference; automatic vendor synchronization is deliberately not reproduced. Original assets remain outside the repository pending redistribution decisions.

## 4. Verification status and proposed release gates

### Checks performed locally

| Check | Observed result | What it establishes |
| --- | --- | --- |
| `cargo test --locked` in `app/` | See [current verification](native-runtime-verification.md) | Synthetic transport and headless UI tests; no real hardware writes. |
| Formatting | `rustfmt` component unavailable | Formatting validation remains pending. |
| Linux container validation | Docker image access rejected by enforced organization sign-in | Linux testing remains pending; daemon version output alone does not establish a usable container runtime. |
| REA plus direct-source inspection | Main/preload partial evidence; selected renderer declarations successfully analyzed | Screen/handler evidence with explicit scope and retention limits, not runtime equivalence. |
| Preset lighting reference vectors | 152 isolated cases pass against the hash-pinned main bundle; one Rust regression consumes the same fixture | All 19 effect IDs, both color modes and four parameter samples compare complete properties/effect reports, including zero-filled tails. Native Save still hardcodes Custom; this proves the tested encoder outputs, not mode selection, complete Windows runtime behavior or hardware acceptance. |
| Preset picker reference | 23 isolated checks pass against the hash-pinned renderer, English catalog and stylesheet | The active selector hides Surmount, leaving two pages of nine; labels, initial-page selection, click/navigation behavior and CSS dimensions/colors are checked. Native tiles and pagination implement these rules without normalizing imported effect data. Fonts, full component appearance, browser/native layout comparison and hardware remain unverified. |
| Profile notification reference handler | 13 isolated checks pass against the hash-pinned main bundle | Four-byte slot/layer events, ignored trailing lighting bytes and reference assignment/reset guards; storage and event publication are stubbed. No native addon, real notification or full settings readback is exercised. |
| Inline color and numeric reference | 64 isolated checks pass against the hash-pinned renderer and stylesheet | Surface geometry, HSV/RGB conversions, pointer/keyboard axes, numeric format/order and callbacks. Malformed-input tests expose unchecked conversions; they do not establish the eventual device output. Native inline surfaces are implemented; the RGB/HEX replacement awaits an invalid-input decision. See [color evidence](windows-ui-map.md#inline-color-surface-and-numeric-input-evidence). |
| Custom-lighting selector, groups and pages | 233 isolated checks pass against the hash-pinned renderer, main bundle and CSS | Selector labels/callbacks, geometry, placement, disabled styles, breathing toggle callbacks/styles/transition declarations, eight selector memberships and 216 full four-page comparisons across slots/layers and RGB/breathing samples. Native tests consume the shared fixtures, including side LEDs and explicit empty pages. The reference Side Lights shortcut has no tagged members; the native shortcut remains disabled pending defect handling. [Scope and invocation](windows-ui-map.md#custom-lighting-group-and-report-checks). |
| Saved-color palette | 10 isolated checks pass against hash-pinned renderer, main and CSS | Defaults, loading, selection, duplicate Add, deletion/menu gating, failure-side mutations, selected-index mismatch, serialization and geometry. Services, storage and React hooks are mocked; native palette implementation awaits the defect-handling decision. [Scope and invocation](windows-ui-map.md#saved-color-palette-and-persistence). |
| Custom brightness | 39 isolated checks pass against hash-pinned renderer and main | Class defaults, slider parameters, clamping, all-key brightness updates, immutable raw RGB, records, dim/restore and individual painting. All 5,376 channel/slider-step combinations agree between renderer and main, with 49 differences from integer round-half-up scaling. Twenty color/brightness/breathing cases compare 80 complete report pages using the real RGBA conversion. Native brightness and profile migration remain unimplemented. [Evidence](windows-custom-brightness.md). |
| Passive macOS HID metadata inspection | The connected target exposes primary usages `0xFF01/0x01` and `0xFF00/0xFF00` | The OS registry contains both expected interfaces; this does not prove either can be opened or that notifications arrive. |
| Isolated macro reference helpers | 13 characterization checks pass against the hash-pinned main bundle under Node.js 22.22.0 | Selected event/mode packing behavior and boundary defects; no complete Electron handler, native addon or hardware execution. |
| Isolated mouse reference path | 16 characterization checks pass against the hash-pinned main/renderer bundles under Node.js 22.22.0 | Selection callback, JSON conversion, target reconstruction and matrix encoding with explicit stubs; no original-window or device acceptance. |
| Isolated performance reference path | 114 characterization checks pass against the hash-pinned main bundle | Four-report sequence and local-state scope across all profile/layer/rate/latency combinations with fixed lighting; ignored OS mode, missing handle and negative/short-write behavior. HID, storage and timers are stubbed; no hardware or Windows runtime acceptance. See [Performance Save evidence](windows-ui-map.md#performance-save-evidence). |
| Native firmware trace | REA static PE-import evidence plus LLVM disassembly identifies cached USB release metadata | Source-derived getter behavior, no native execution or hardware comparison; [evidence](windows-native-firmware.md). |
| Windows physical startup | A 90-second capture records 23,111 target events without recorder drops; CORE detects the keyboard | Metadata, input-queue configuration and a pending read; no settings download in observed APIs. Startup polling setter fails before sending. This is bounded research, not full readback or feature acceptance; [evidence](windows-hardware-readback.md). |
| Native macOS runtime | Release build launches, renders the keyboard, connects the configuration interface, and imports/edits/exports a local draft | Notification opening fails with an IOKit permission error in the temporary test bundle. Missing features, glyph/label defects, hardware Save/recovery and complete parity remain open; [verification and VM-retention gates](native-runtime-verification.md). |
| Repository graph coverage | Generation `2026-11-05T04:04:23Z` is stale for edited native files | Current source is authoritative; graph edges referencing removed readback code are obsolete. |
| Git/release inspection | No configured remote; native app files untracked; `.github/workflows/native.yml` defines locked tests and release builds for Ubuntu 24.04, macOS 15 ARM and macOS 15 Intel | Workflow YAML and bounded structure checks pass locally. No hosted CI, packaging or release result is established; see [native verification](../TESTING.md). |

The Windows startup and physical metadata capture do not establish full UI/runtime equivalence, onboard-state download, physical Save behavior, Linux runtime correctness, native installed-package behavior, or signing/notarization. No Save, reset, firmware-update control, or speculative hardware query is invoked; the bounded capture does not certify that all onboard settings remain unchanged.

### Proposed acceptance requirements

Every feature should have a stable ID linked to its Windows reference evidence, implementation, automated tests, applicable hardware tests, and user documentation. A compiler success or successful HID send must not be labeled a passed feature test.

| Gate | Evidence required |
| --- | --- |
| Reference capture | Approved Windows build hash, keyboard identity/firmware, screenshots, interaction recordings, sample profiles, and operation-specific USB captures. |
| Protocol correctness | Byte-exact fixtures from observed transactions; malformed/short/stale/zero replies; model and slot mismatch; report-ID/length conventions; preservation of unsupported fields. |
| Pure behavior | Binding and modifier round trips; effect ID mapping; macro encoding, timing and limits; profile validation, migrations, atomic persistence and backups. |
| Device integration | Permission denied, conflicting app, wrong device, unplug/replug, sleep/wake, timeout, partial write and cancellation behavior; hardware notifications; no automatic replay of uncertain writes. |
| UI parity | Screen inventory, matched user flows, save/discard state, profile/layer behavior, error states, keyboard navigation, scaling and accessibility. Visual exceptions must be agreed. |
| Hardware acceptance | Each claimed model/firmware/OS combination: actual key output, modifiers, macros, lighting, hardware hotkeys, persistence after power cycle, and operation with the app closed. |
| Platform packages | Fresh-machine install, first launch, HID permissions, upgrade, uninstall, data preservation, and launcher integration on the agreed macOS/Linux targets. |
| Release integrity | Reviewed source/dependency/asset provenance; approved license and branding; pinned dependencies; CI artifacts, checksums, and the agreed signing/notarization process. |
| Documentation | Fresh-machine walkthroughs; executable examples/sample profiles; link and command checks; feature/support tables derived from the acceptance ledger. |

Hardware tests need a controlled device and an agreed recovery procedure. Start with the [hardware acceptance walkthrough](../HARDWARE_ACCEPTANCE.md); connection checks precede owner-approved write tests. Exporting a local draft is not a backup of unknown onboard settings. Firmware flashing, factory resets, and unverified HID probes require separate approval.

### Proposed investigation order

1. Use the confirmed GMMK 2 96% ANSI scope and Windows CORE 2.1.23 reference; agree supported platform targets.
2. Use the hashed Windows reference and retained REA evidence; extend the screen map without executing the device backend.
3. Close write-safety gaps and add operation-level tests; then implement the mapped native screens and missing behaviors.
4. Capture controlled Windows behavior and USB transactions on approved hardware; establish which operations read, write, activate, or require a host process.
5. Complete the parity specification for the approved Rust/egui architecture, including explicit deviations and unavailable features.
6. Implement in testable feature increments; validate real hardware and platform packages before claiming release readiness.

## 5. Decisions requiring user input

These are open requirements, not exclusions from the requested goal.

| Decision | Why it changes the project |
| --- | --- |
| Public branding/assets | Matching the reference appearance is confirmed; redistribution of original materials remains a separate decision. |
| Saved-color deletion and persistence failures | Ten isolated checks show that deleting a middle swatch can highlight a different color from the editor, and failed saves can still mutate settings or editor color. Confirm whether native should keep selection/color consistent and leave editor/palette state unchanged on failure. See [palette evidence](windows-ui-map.md#saved-color-palette-and-persistence). |
| Reference defects in macros, mouse bindings and slider callbacks | Intended-functionality parity with explicit validation is recommended. Macro truncation/hold-mode failure, the active mouse-identifier mismatch and the isolated slider's dropped final-change callback need an agreed correction policy and verification rather than silent reproduction or guessed fixes. |
| Numeric color validation | The reference forwards malformed HEX strings and unchecked RGB conversions, including red 256 becoming an overlong color string. Confirm whether the replacement should reject invalid values while preserving the last valid color instead of reproducing these defects. The native numeric controls remain unchanged pending this decision. |
| Preset color-mode persistence | Windows stores `multicolorMode`; native profiles have no equivalent and Save always passes `false`. An additive field defaulting to Custom would preserve legacy report behavior; using Windows' Chromatic default would change existing saves. Confirm compatibility behavior before changing the profile format. |
| Profile-wide lighting and custom brightness migration | Native lighting is per layer and has only encoded custom RGB bytes. Windows retains raw RGB and independent custom brightness, which updates all painted keys. Proposed migration preserves legacy RGB at brightness 100, backs up the original import and asks which lighting to keep when layers conflict. Confirm before changing persistence or save scope; [details](windows-custom-brightness.md#native-model-decision-remains-open). |
| Stock firmware requirement; firmware updating and recovery scope | Determines whether QMK/Vial-based alternatives are relevant and which high-risk operations are needed. |
| Cloud/Glorious ID, RGB synchronization, background host actions and app updater scope | These introduce services, credentials and OS-specific behavior beyond onboard configuration. |
| macOS versions/architectures and Linux distributions/desktops | Determines packaging, permissions, UI automation and real-machine acceptance coverage. |
| Windows runtime comparison and Linux hardware-test access | Static UI analysis can proceed locally; hosted CI and static findings cannot substitute for physical acceptance testing. |
| Persistent REA setup or native provider selection, if needed | The static CLI already runs locally; no agent registration or native provider setup is performed. |
| Source destination and release resources | Public `ethannjw/gmmk-core` is owner-approved for source publication. Release asset/license review and signing resources remain required for distribution. |

## Sources

External statements describe the linked maintainers' documentation as inspected on 2026-11-06; they are not independent certification of those projects.

- **S1:** [Glorious software/downloads](https://www.gloriousgaming.com/pages/software).
- **S2:** [Glorious development changelog](https://www.gloriousgaming.com/pages/development-changelog).
- **S3:** [Inglorious Core Linux README](https://github.com/tg5ee/inglorious-core-linux/blob/main/README.md).
- **S4:** [Inglorious Core Linux implementation notes](https://github.com/tg5ee/inglorious-core-linux/blob/main/NOTES.md). Its hardware-limit statements and README differ; this report does not infer complete validation from either.
- **S5:** [INGLORIOUS Advanced README](https://github.com/LatroX-dev/INGLORIOUS-Advanced/blob/main/README.md).
- **S6:** [Glorious Lights](https://github.com/ArshansGithub/glorious-lights).
- **S7:** [OpenRGB GMMK2 P96 ISO](https://github.com/reelmesh/openrgb-gmmk2-p96-iso).
- **S8:** [REA](https://github.com/morluto/rea).
- **S9:** [REA JavaScript application workflows](https://github.com/morluto/rea/blob/main/docs/javascript-application-workflows.md).

Local CORE code is used as behavioral evidence. Original app bundles, extracted implementation files and native binaries remain outside this research document and are not added to the repository by this investigation.
