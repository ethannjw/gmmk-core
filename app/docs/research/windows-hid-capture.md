# The HID logger passes isolated validation; hardware readback remains unresolved

The bounded Frida logger passes 17 Node tests, 20 synthetic native checks inside Windows CORE, and an invalid-handle check through CORE's actual imported `DeviceIoControl` address. These checks validate API interception, byte accounting, pending-operation handling, and teardown without attaching the keyboard. A separate physical capture records successful HID metadata responses, but no completed feature-report response or onboard-settings download. The native macOS/Linux application gains research tooling, not onboard readback, from this work.

Scope: stock GMMK 2 96% ANSI (`320f:504b`), CORE 2.1.23 / package 2.1.23.2, Windows 11 ARM64 guest, Frida 17.26.1, validation date 2026-11-07. See [startup readback findings](windows-readback.md) and [VM isolation and provenance](windows-vm-smoke-test.md).

The separate [approved physical startup capture](windows-hardware-readback.md) completes with a 50,000-event limit after physical reconnect: 23,111 target events, zero recorder drops, one pending read at stop, and 55 descriptor snapshots truncated to 65 bytes. It records metadata and input-queue configuration, with no observed feature-report calls. The passing no-device result below remains distinct from this bounded hardware result.

## The recorder separates observed data from uncertain output

`scripts/windows-hid-trace.cjs` accepts only handles registered from matching Windows HID paths, with an exact product-ID boundary. Unrelated handles are filtered before transfer-buffer access. Its defaults cap snapshots at 65 bytes, emitted events at 1,000, pending operations at 128, and registered handles at 32. Limits and unreadable memory remain explicit in the output.

| API family | Recorded behavior |
| --- | --- |
| `CreateFileW`, `CreateFileA`, `CloseHandle` | Register target paths after successful opens; successful close abandons pending operations and removes the handle. |
| `DeviceIoControl` | Snapshot input at entry; successful output uses the reported count, bounded by capacity and the snapshot limit. Record the control code. |
| `ReadFile`, `WriteFile` | Read output only after success with a known count. Writes snapshot input and label the count as written bytes. |
| `GetOverlappedResult`, `GetOverlappedResultEx` | Correlate by handle and overlapped pointer. Retain incomplete, timed-out, and alertable-wait operations until a later completion. |
| `HidD_GetFeature`, `HidD_GetInputReport` | Record the report-selector byte and a successful buffer-capacity snapshot, explicitly labelled `buffer-capacity`. These APIs provide no transferred-byte count to this adapter; padding may be untouched. |
| `HidD_SetFeature`, `HidD_SetOutputReport` | Snapshot the outgoing buffer without inventing a response. |

Failed and pending calls expose no completed output. Cancellation records failure. Pending operations are registered at entry so completion on another thread can arrive before the issuing API returns. Reused overlapped pointers, handle close, and capture stop explicitly abandon outstanding operations. A final summary reports pending operations and event loss even when the event limit is reached.

`scripts/windows-hid-trace.frida.js` hooks exports and actual imports of loaded `.node` modules for these 12 API names. Per-thread, per-API depth suppresses duplicate observations through nested forwarding entry points. Different API layers, such as a `HidD_*` call wrapping `DeviceIoControl`, can still describe the same request twice. These are API-layer observations, not a one-to-one USB packet trace.

## The Windows validation exercises the actual imported entry point

In this guest, CORE's native modules import `DeviceIoControl` at `0x7ffc4f9f2e70`, while the inspected `KERNELBASE.dll` export is at `0x7ffc4d8c31b0`. Hooking that export alone does not cover the observed import address. The adapter therefore hooks the runtime imports as well. The address mismatch is observed; its architectural cause is unproven, and these addresses must never be hard-coded.

| Final validation | Result |
| --- | --- |
| Host recorder/adapter tests | 17 pass; no hardware or Frida installation required. |
| Native callback fixtures inside CORE | 20 checks pass, covering all 12 API adapters, exact input bytes, length accounting, filtering, asynchronous completion, cancellation, and teardown. |
| Actual imported API smoke | `INVALID_HANDLE_VALUE`, control code `0x000b0192`, and 65 known input bytes produce false/error 6; hook input matches exactly and no output is claimed. |
| Passive observation | 20 seconds, 18 hook addresses, four native-module import inventories, no unmatched imports for the six observed names: `CloseHandle`, `CreateFileW`, `DeviceIoControl`, `GetOverlappedResult`, `ReadFile`, and `WriteFile`. Other adapters are covered by synthetic fixtures rather than genuine CORE requests. |
| Physical target events | Zero; observation summary reports zero dropped events and zero pending operations. |
| Instrumentation health | Zero hook errors, callback errors, journal drops, or journal write errors. |
| Cleanup and process health | Hooks detached; injector stopped; one-shot task removed; CORE main PID 6324 and renderer children remain running. |
| Isolation | USB sharing disabled; zero target PnP entries; no remaining Frida processes or temporary research scheduled tasks. |
| Application integrity | Installed ASAR SHA-256 is unchanged before and after validation. |

Synthetic fixture events carry `source: synthetic`; the actual imported invalid-handle check carries `source: synthetic-import-smoke`; passive observations carry `source: observe`. Fixture handles are never sent to OS device APIs. The imported smoke uses the invalid handle only. None of these checks demonstrates a successful hardware response.

## Reproduce offline tests before isolated guest validation

From `app/`, run:

```sh
node --test scripts/windows-hid-trace.test.cjs
node --check scripts/windows-hid-trace.frida.js
node --check scripts/windows-hid-trace-selftest.frida.js
node scripts/build-windows-hid-trace.cjs /tmp/new-hid-probe.js 'C:\Windows\Temp\gmmk-hid-validation'
```

The builder refuses to overwrite its output. It combines the recorder, adapter, synthetic fixtures, and a 20-second observation wrapper. The guest output directory must already exist. The wrapper writes `script-result.json` and `hid-trace.jsonl`, caps its journal at 2,500 entries, and requires passing fixtures, imported smoke, and clean hook/journal diagnostics for `passed: true`. It does not launch CORE, perform a PnP preflight, disable USB passthrough, or enforce hardware isolation itself. Its metadata declares no physical-access approval; that declaration is not an access-control mechanism.

Run the generated bundle only in the separately isolated, no-device guest through the [launch-first procedure](windows-vm-smoke-test.md). Verify the target is absent and USB sharing is disabled before launching CORE. Preserve local application data and compare the installed ASAR hash before and after. Attach-to-running CORE remains unresolved in this guest. Guest launch/controller scripts and vendor tools are temporary local research assets rather than a supported deployment workflow.

The GitHub native workflow includes the Node tests on its Linux, Apple Silicon macOS, and Intel macOS matrix. The Node tests do not execute Frida or the vendor application. Local test success does not establish that GitHub CI or either platform's hardware acceptance suite has passed.

## Physical capture needs separate consent and broader coverage

Coverage excludes completion ports, `ReadFileEx`/`WriteFileEx` completion callbacks, direct `Nt*` calls, handles opened through unhooked paths or duplicated handles, and direct polling of overlapped memory. Partial output on failed calls such as `ERROR_MORE_DATA` is discarded. Payloads longer than 65 bytes are truncated. Module-import matching covers only the listed APIs and observed modules, not all native control flow. Successful HID buffer contents and protocol semantics require physical validation.

Before physical passthrough, obtain informed approval and arrange alternate input: the keyboard may stop controlling the Mac, and CORE's traced startup dispatches the locally saved polling rate to the keyboard even without clicking Save. Preserve local profiles, but do not describe them as an onboard backup. Prepare a separately reviewed physical-capture launcher; this no-device bundle is not that launcher. Do not reset, flash firmware, send speculative queries, or apply other configuration changes without authorization.

Compare displayed settings and local state against observed requests and responses before concluding that CORE downloads onboard settings. The current static evidence supports local-profile hydration and notifications; it neither proves a complete settings download nor rules out firmware readback support. Exact UI/feature parity and shippable macOS/Linux quality remain separate acceptance requirements.

### Evidence provenance

The repository retains a [sanitized validation record](windows-hid-validation.json) with the observed checks, import coverage, cleanup results, and SHA-256 hashes of the tested sources, generated probe, and journal. It contains no vendor binaries, local usernames, or keyboard payloads. It is a historical result summary rather than a live test or complete raw capture; changing the source invalidates any assumption that this run validates the new version.

The full local evidence set is under `/private/tmp/gmmk-core-vm-BW0D0mrV`: `hid3-script-result.json`, `hid3-controller-result.json`, `hid3-final-verification.json`, and `hid3-trace.jsonl`. These temporary paths are not portable or permanent repository artifacts. The cleanup inventory tracks their removal with the disposable VM after research. No machine-setup entry is added.

- Installed ASAR SHA-256: `64e3de1cdec77fba29a9db3313ec945c89dd34c22ad5f8609ec1b125ad67c7b2`.
- Frida injector SHA-256: `39d38f9518236f6eb56edb618f94f367a0c714c8f98075dffa7485f1cf37e8a0`.
- API contracts: [DeviceIoControl](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-deviceiocontrol), [GetOverlappedResultEx](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getoverlappedresultex), [HidD_GetFeature](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/hidsdi/nf-hidsdi-hidd_getfeature), and [Frida JavaScript API](https://frida.re/docs/javascript-api/).
