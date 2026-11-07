# GMMK 2 startup restores local profiles and subscribes to events

The inspected Windows CORE 2.1.23 reference loads GMMK 2 profiles from local storage, substitutes defaults when absent, subscribes to keyboard notifications, and dispatches a polling-rate write during initialization. Its GMMK handler also exposes a raw feature-report reader, but the traced startup path does not use it to reconstruct onboard settings. This establishes the inspected software's behavior; it does not establish that the keyboard firmware lacks a download command. Full onboard readback remains unresolved.

Scope: stock GMMK 2 96% ANSI, `320f:504b`, CORE package 2.1.23.2. Investigation date: 2026-11-07. The static and isolated checks below do not access hardware. A separately approved [physical startup test](windows-hardware-readback.md) detects the keyboard and records an instance-registration error before the polling setter sends its report. Its 90-second, 23,111-event capture has zero recorder drops and shows metadata, queue configuration, and one pending notification read; no settings download appears in the observed APIs. Bounded API coverage and pending I/O prevent a claim that all possible read paths are excluded.

## Startup has three distinct sources of state

| Source | Inspected behavior | Evidence in `out/main/index.js` |
| --- | --- | --- |
| Local records and defaults | `Device.initializeDevice` applies a saved instance or a new record before handler initialization. `GloriousDevice.applyRecordData` retrieves stored profile IDs and creates missing profiles using handler defaults. | 71990, 4077 |
| Keyboard notifications | `GMMKDeviceHandler.init` registers the callback using `getData` usage page/usage. The USB descriptor specifies `ff00:ff00`; the configuration interface is `ff01:0001`. A profile-change event selects an existing local profile and layer. | 60251, 4888, 11859, 60303 |
| Explicit raw reads | `getReportFromDevice` calls `GetFeatureReport` on the current write-interface handle, report ID 7, length 65. It returns raw bytes after a delay; it does not decode profiles, lighting, macros, or bindings. | 60427, 4926 |

The generic `GloriousDevice.getDeviceReport` returns an empty object, and `applyDeviceReportData` has an empty body. Their comments describe a hardware-state abstraction that these methods do not implement. The profile-list IPC path for this keyboard reads `DataStorage`, rather than invoking a hardware-profile downloader (main bundle 73030 and 3903). Other device branches must not be treated as this keyboard's protocol.

The GMMK handler logs lighting-change and multicolor-change notifications without reconstructing those settings. Profile/layer notifications use one-based indices to select locally stored state. They establish active selection, not the contents of the selected onboard profile. Existing notification reference tests cover this separately.

Initialization calls `setPollingRate` with the local profile's saved rate. That method prepares a Properties buffer and calls `sendReportToDevice` (60457). The call in `init` is not awaited. A Windows launch must therefore be treated as potentially configuration-changing, even before Save is clicked. The subsequent standby helper returns without writing for this target because its descriptor does not advertise `hasStandbyMode`.

## REA provides bounded static evidence, not complete runtime coverage

REA 6.8.0, running with the existing Node 22.23.4 runtime, analyzes eight verbatim class slices: `DeviceProfilesData`, `DataStorage`, `GloriousDevice`, `HID`, `KeyboardStandbyDeviceHandler`, `GmmkBaseDeviceHandler`, `GMMKDeviceHandler`, and `Device`. Source extraction records original line positions and hashes. No Electron bootstrap or native module executes.

- REA parses all eight selected JavaScript files, with zero parse failures and no reported truncation. It visits 9,843 AST nodes; the unselected ninth file is the provenance JSON.
- Parent evidence ID: `ev_9e90915dfebc22c67e92f1e5c0f654bd3967647584ae28b916750d88b53e5834`.
- Application graph coverage is **partial**; semantic coverage is **unknown**. The call-flow family retains two relations and reports 662 unknown IDs. These graphs do not establish absence of other reads.
- A supplementary full-bundle syntactic member-call census finds `getReportFromDevice` callers in other device handlers, but none in the selected GMMK 2 startup/base-handler declarations. Dynamic dispatch, native internals, and firmware commands remain outside that negative finding.
- Local evidence: `/private/tmp/gmmk-readback-rea-JPJtzs.evidence.json`; extraction manifest: `/private/tmp/gmmk-readback-rea-JPJtzs/provenance.json`. Temporary files are research outputs, not distributable release artifacts.

The main bundle SHA-256 is `96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531`. Its bytes are also checked against the matching entry inside the original ASAR, whose SHA-256 is `64e3de1cdec77fba29a9db3313ec945c89dd34c22ad5f8609ec1b125ad67c7b2`. See [reference provenance](core-parity.md).

## Native static inspection identifies the raw read transport

No native deep-analysis provider is configured in REA's current doctor result. Existing LLVM disassembly supplies the following address trace without loading the module. Addresses apply only to `hiddevice-6b262a24.node`, SHA-256 `6b262a242637c5dd157a092106884a24430c44c1236e2e9a888b5728fcec5057`, image base `0x180000000`. Nearest-export labels in the disassembler are not reliable function names.

| Export | Registration string / callback | Implementation entry |
| --- | --- | --- |
| `GetFeatureReport` | `0x180283070` / `0x1800085e4` | `0x180022620` |
| `SetupDeviceDataCallback` | `0x1802830a8` / `0x180005943` | `0x18001e070` |
| `GetEPTempData` | `0x1802830d0` / `0x1800066db` | `0x180023980` |
| `ExecuteHIDRead` | `0x180283110` / `0x1800082d8` | `0x180023010` |

`GetFeatureReport` sets the first buffer byte from the report argument at `0x1800229ff`, then calls `0x180001c62` at `0x180022a8a`. That thunk reaches `0x180078710`, which passes control code `0x000b0192` to `0x18007e180`. The latter calls the PE import slot `0x18033e060`, identified from the import table as `KERNEL32.dll!DeviceIoControl`. This identifies a generic read transport, not a keyboard-specific settings decoder.

The notification setup and synchronous-read implementation entries are located but their full native control flow is not established here. In particular, this analysis does not exclude additional native initialization traffic. [Firmware metadata research](windows-native-firmware.md) separately traces `GetFWVersion` to cached USB metadata.

## Six isolated checks reproduce the JavaScript distinction

From the Git root:

```sh
node app/scripts/check-windows-readback-reference.cjs /path/to/CORE-2.1.23/app
```

The checker verifies the main-bundle hash before evaluating selected declarations. Six checks cover stored-profile hydration, default fallback, callback registration and polling dispatch, raw report return, the single-zero-byte empty-read sentinel, and the generic report placeholders.

Storage, HID, timers, superclass dependencies and the polling setter are stubbed. The initializer test uses a USB-only fixture with the target's descriptor values; it does not exercise the complete device lifecycle, bootloader connection entry, actual polling packet, real storage, native callback thread, or physical hardware. The JavaScript VM is an isolation aid, not a general security sandbox. These are reference-behavior checks, not hardware acceptance tests.

## Runtime capture resolves the remaining uncertainty

A Windows VM or Windows machine can test what CORE actually exchanges and whether displayed state changes when local records differ from onboard state. Before proceeding, obtain approval for USB passthrough and CORE's startup writes, and arrange another input device in case passthrough removes the keyboard from the host. Preserve the Windows local profile data and VM snapshot; neither constitutes a full onboard backup.

Capture traffic before CORE starts and retain the raw transfers, interface identities, timestamps, app version and screenshots. Compare a normal stored-profile launch with a separate clean local-data environment, without clicking Save, reset or firmware update. Record the startup polling write as a mutation, not a read. Correlate any apparent settings download with request and response bytes and with the value shown by the UI. Do not infer a read protocol from a feature-report response alone or silently add speculative queries to native Connect.

A configured native analysis engine can deepen the static trace first. A separate [isolated Windows VM smoke test](windows-vm-smoke-test.md) verifies installation and no-device startup with the same ASAR hash. Its ETW validation records USB transfer metadata without verified payload bytes. The [bounded Frida HID logger](windows-hid-capture.md) passes isolated recorder tests, native synthetic checks inside CORE, and an invalid-handle smoke through CORE's actual imported API address. Launch-first instrumentation works; attachment to an already-running instance remains unresolved. The approved physical capture completes after physical reconnect, with zero recorder drops and no observed feature-report calls. Successful feature-report payload capture and full onboard readback remain unverified; see the [physical findings and coverage limits](windows-hardware-readback.md).
