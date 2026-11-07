# Keep the Windows VM: native release acceptance is incomplete

The native macOS application opens with an automatically detected keyboard card and the approved profile-dashboard flow. Configuration access verifies the keyboard's HID descriptor; the rebuilt app reports a successful lighting transfer, and the owner confirms the physical lighting changes correctly. The replacement remains development-stage: notification access fails on this host, several binding categories are absent, and remaining physical behaviors, persistence/recovery, complete Windows UI equivalence, Linux runtime behavior and distribution remain unverified. Retain the stopped Windows VM and reference evidence until those dependencies are resolved.

Verification date: 2026-11-07. Host: macOS 26.6.2 (25G83), ARM64. Target: stock GMMK 2 96% ANSI, `320f:504b`. Agent-driven hardware access covers automatic/manual connection, descriptor inspection, cached USB metadata and passive notification-open attempts. A lighting/latency/activation Save succeeds during the owner's interaction after rebuilding; the owner confirms its visible lighting effect, while exact pre-Save controls and latency behavior remain unobserved. The agent invokes no Save, activation, reset, firmware update or permission-setting change. See [framing diagnosis](hid-report-framing.md).

## The current build passes bounded native checks

| Check | Result | Scope |
| --- | --- | --- |
| `cargo test --locked --offline` | Pass: 177 tests, zero failures | Synthetic transport, profile/protocol logic, discovery transitions and headless UI; not hardware acceptance |
| All 13 `check-windows-*-reference.cjs` scripts | Pass: 708 tests, zero failures | Hash-pinned, isolated reference behavior; native Windows HID is not exercised |
| `cargo build --release --locked --offline` | Pass, two existing warnings | macOS ARM64 optimized binary only |
| Fresh binary launch | Pass | A temporary `.app` wrapper makes the unchanged executable addressable by native UI inspection |
| Chooser and dashboard | Pass: startup detects a real card, clicking its image enters local defaults, profile 2 selection survives Devices/back navigation | Default 1440×880 and minimum 1000×700 content sizes; original vector keyboard, not copied vendor artwork |
| Profile menu and rename dialog | Pass: menu opens, Rename opens, Cancel preserves the profile | Save-name and confirmed slot-local default restoration also have headless interaction tests; no physical reset |
| Existing settings navigation | Earlier baseline check passes home, lighting, keybinding and performance navigation | All automated settings regressions pass on the rebuilt shell; full functional parity remains open |
| Configuration connection | Pass | UI reports `Connected: GMMK 2 Full Size 96%` and cached USB release `0016` |
| Notification interface | Fail in this test environment | `hid_open_path` reports `IOHIDDeviceOpen` error `0xE00002E2`, `(iokit/common) not permitted`; configuration connection remains open |
| Disconnect/retry | Pass | Settings Disconnect removes the card and pauses connection; explicit Retry restores it. Physical unplug/replug is simulated in policy tests, not repeated live |
| Local import | Earlier baseline pass; current automated regressions pass | Repository factory template imports; this template is not a backup of onboard settings |
| Local lighting edit/export | Earlier baseline pass; current automated regressions pass | Select Normally On in profile 1/layer 1, export to a task-local JSON file; normalized comparison finds only effect ID `0` changing to `6` |
| Keybinding selection | Pass | Clicking A opens keyboard/multimedia controls; macro/mouse/launch controls remain disabled |
| Export reimport | Not run in the live UI | Automated serialization round-trip tests pass; no live reimport result is claimed |
| Lighting Save transport and visible effect | App reports success; owner confirms the physical lighting changes correctly | Lighting, latency and activation transfer for profile 1/layer 1; latency behavior, other effects and persistence remain unverified |
| Other hardware Save categories, persistence and recovery | Not verified | Requires an owner-approved target and known-good recovery baseline |
| Linux runtime, installer/signing | Not established | Source remote is the public `ethannjw/gmmk-core` repository; consult Actions for hosted build/test results. Clean-machine package and physical Linux acceptance remain pending |

The current release executable and verification-wrapper copy both have SHA-256 `77d80896e8b9a12b19becfb06fe320de9768c3598cdb2bc39af2aaee32f0601c`. The earlier import/edit/export baseline executable has SHA-256 `847dd515e233fb7094b765af8bfd677662d4e87414a3e53230a35cfde6679921`. The wrapper uses `local.gmmk.research-verification`, stays under the task's temporary directory, and is not a signed/notarized distribution. Its permission result must not be generalized to a differently packaged or permissioned executable. The existing native demo remains untouched.

The importer accepts camelCase aliases and the exporter writes snake_case field names. The earlier export comparison normalizes those documented aliases before checking values; it does not ignore any changed profile values. All key matrices and other layer settings match that imported template after normalization. This is a local-file comparison, not an onboard-state comparison. The user's later draft is exported before the transport-fix restart; it remains a local draft, not an onboard backup.

## The current UI and permission behavior need more work

The chooser and dashboard use the approved dark rail, device card, header, profile rows and orange local-selection marker. Work Sans and Space Grotesk have bundled license/provenance records; the vector keyboard displays arrow legends and compact navigation-key labels without the earlier wrapping sidebar. Headless tests verify all key legends fit at card/editor sizes. This is bounded screen acceptance, not pixel-identical whole-app parity. The UI still marks macro/mouse/launch actions as unimplemented, OS mode as unverified and onboard reading as disabled.

Discovery tests cover initial absence/presence, removal/reinsertion, changed paths, stable open failures, ambiguous/error scans and explicit retry after manual disconnect. Navigation tests verify that card entry creates only local defaults and that back/disconnect/reconnect preserve drafts without device commands. Menu tests verify rename and confirmed, slot-scoped local restoration. Failed import/export remains visible on the chooser and preserves the draft. Independent review identified that error-visibility case; its regression test fails before the fix and passes afterward.

Read-only System Settings inspection shows `Glorious Core.app` enabled under Input Monitoring but no `GMMK Verification.app` entry. This is a permission discrepancy to test for the verification wrapper, not a completed root-cause diagnosis. Granting that wrapper access and reconnecting requires owner confirmation. No privacy permission is changed during this check. Successful configuration access and lighting Save do not establish notification access or full onboard readback.

## VM removal depends on evidence and recovery, not a passing build

| Dependency | Current evidence | Required before removing the reference environment |
| --- | --- | --- |
| Original application and source provenance | Hash-pinned extract, installation, REA outputs and isolated reference checks exist locally | Preserve an agreed reproducible reference/evidence set outside temporary storage without adding vendor binaries to Git |
| Startup behavior | [Physical capture](windows-hardware-readback.md): 23,111 target events, no recorder drops, no observed settings download | Keep coverage limits explicit; establish or explicitly accept the readback support boundary |
| Original settings UI | Static/mocked renderer evidence and a real connected dashboard | Capture and exercise the original keyboard settings screens; current VM pointer navigation has not established this |
| Operation-specific protocol | Static packet builders and synthetic tests | Capture approved lighting, binding, performance, macro and slot/layer operations needed for parity |
| Native hardware acceptance | Configuration connection succeeds; notification opening fails | Resolve notification access and verify received events, approved writes, persistence, failure recovery and unaffected settings |
| Restore capability | VM snapshot and local CORE datastore exist | Obtain a known-good keyboard configuration and tested restoration method; a VM snapshot is not an onboard backup |
| Shipping quality | Local macOS tests/build pass | Complete native feature/visual acceptance, Linux tests, hosted CI, packaging and clean-machine verification |

The VM is retained but shut down to release its guest RAM allocation. Its disk, snapshot, CORE installation, local profiles and raw capture artifacts remain available. Nothing is added to machine setup scripts. Temporary source/extract paths alone are not a durable evidence archive. Follow the [hardware acceptance procedure](../HARDWARE_ACCEPTANCE.md) before any write test.
