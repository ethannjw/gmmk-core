# Physical startup records metadata and a pending read, not a settings download

The approved 90-second Windows CORE 2.1.23 startup capture detects the stock GMMK 2 96% ANSI and displays its keyboard card. It records 23,111 target events with zero dropped events: HID metadata queries, input-queue configuration, and one pending notification read. No feature-report request, completed notification payload, or settings download appears in the observed APIs. Together with the source checks, this supports local-profile restoration rather than onboard-profile reconstruction in this startup path. Firmware readback capability remains unknown.

Test date: 2026-11-07. Scope: `320f:504b`, CORE package 2.1.23.2, temporary Windows 11 ARM64 VM, x64 CORE process, Frida 17.26.1. The user explicitly approves passthrough and the disclosed startup polling-rate write. No Save, reset, firmware-update control, or speculative feature-report query is invoked. This API-level capture is bounded; it cannot certify that every onboard setting remains unchanged.

## The runtime error occurs before the polling packet is prepared

CORE logs one enumerated GMMK V2 96 ANSI device, completes initialization, and renders its keyboard card and sidebar entry. The stderr report independently contains this rejection twice:

```text
Error: Unable to find device from instance id
    at GMMKDeviceHandler.setPollingRate (.../out/main/index.js:60460:13)
    at GMMKDeviceHandler.init (.../out/main/index.js:60260:25)
    at Device.initializeDevice (.../out/main/index.js:72018:33)
```

The hash-verified main bundle explains this execution path:

1. `Device.initializeDevice` hydrates local records and adds the device to `HID.knownDevices`, then awaits the handler at line 72018.
2. `GMMKDeviceHandler.init` calls `setPollingRate` without awaiting it at line 60260.
3. `setPollingRate` looks up the instance through `Device.getGloriousDevice` and throws when absent at line 60460, before preparing or sending the report.
4. `getGloriousDevice` searches `Device.gloriousDevices` at line 72235. Initialization appends to that separate registry only after the handler and firmware-version work finish.

This ordering accounts for the observed rejection while allowing initialization to continue to a Connected state. It qualifies the earlier static finding: startup dispatches the saved polling rate, but the observed invocation fails before sending that polling report. It does not establish that all startup paths, subsequent invocations, or other CORE versions behave identically. The installed vendor files are unchanged; this investigation does not patch CORE to bypass its behavior.

## The bounded capture covers startup without event-limit losses

| Measurement | Observed result |
| --- | --- |
| Target PnP entries during attachment | 11, all reporting OK |
| Capture duration | 90 seconds |
| Recorded target operations | 7,598 opens, 7,916 `DeviceIoControl` calls, 7,595 closes, one pending read and its abandonment at capture stop |
| First target event / final summary | 13:17:16.352 / 13:18:44.489 UTC |
| Recorder limit and losses | 50,000-event cap; 23,111 emitted target events; zero dropped events |
| Complete journal | 23,136 JSONL entries including metadata, hooks, native-import inventories, and summary |
| Hook health | Zero hook errors, callback errors, or unmatched imports for the observed API names |
| Journal health | Zero journal drops, write errors, or memory-snapshot errors |
| Snapshot limits | 65 bytes; 55 collection-descriptor outputs are truncated, explicitly marked |
| Pending I/O | One `ReadFile` on interface `MI_01&Col01`; no observed completion; marked abandoned when recording stops |
| Local datastore | The same three files and SHA-256 hashes before and after startup; this is not an onboard-state comparison |
| Teardown | Hooks detached; injector and temporary task removed; keyboard detached |
| Fresh post-detachment check | Zero target PnP entries, CORE processes, Frida processes, or temporary research tasks |

The control-code inventory, decoded against Microsoft's `hidclass.h`, is:

| Control code | API meaning | Count and result |
| --- | --- | --- |
| `0x000b01a8` | `IOCTL_HID_GET_COLLECTION_INFORMATION` | 7,702 successful, 12-byte metadata responses |
| `0x000b0193` | `IOCTL_HID_GET_COLLECTION_DESCRIPTOR` | 55 successful; reported lengths 268, 372, 476, 684, or 700 bytes; only first 65 retained |
| `0x000b01c2` | `IOCTL_HID_GET_SERIALNUMBER_STRING` | 52 failed, error 87; no successful output claimed |
| `0x000b01ba` | `IOCTL_HID_GET_MANUFACTURER_STRING` | 52 successful, 16-byte strings |
| `0x000b01be` | `IOCTL_HID_GET_PRODUCT_STRING` | 52 successful, 42-byte strings |
| `0x000b01a4` | `IOCTL_SET_NUM_DEVICE_INPUT_BUFFERS` | Three successful; input `40 00 00 00` sets the Windows input-report queue to 64 buffers |

The queue-setting operation configures HID class-driver buffering, not a keyboard profile or polling-rate packet. No `WriteFile`, `HidD_GetFeature`, `HidD_SetFeature`, `HidD_GetInputReport`, `HidD_SetOutputReport`, or corresponding feature/input/output-report IOCTL appears among the recorded target transfers. Counts describe API events, not unique USB packets.

Zero recorder drops does not imply complete transport coverage. This logger does not cover completion ports, `ReadFileEx`/`WriteFileEx`, direct `Nt*` calls, untracked/duplicated handles, direct polling of overlapped structures, or other processes. The pending read may complete through an unobserved path; it is not evidence of an empty successful read. No profile switching or clean-local-store comparison is performed. Detailed settings-page navigation remains unverified; prior card/sidebar clicks produce no observed page transition.

## Reattachment succeeds after physical reconnect

The retained attempt identifiers distinguish evidence quality: `hardware2` reaches the 1,000-event cap within 100 ms and drops 22,110 events; `hardware3` never attaches because UTM reports `LIBUSB_ERROR_ACCESS`; `hardware4`, after the user's physical reconnect, completes the larger capture described here. The reconnect permits this attempt; the cause of the earlier access denial remains unproven.

In this environment `utmctl` can print an Apple-event error while returning shell exit status zero. The host wrapper checks error text and the guest controller independently requires target presence before starting CORE. The completed run reports 11 healthy target PnP entries, zero remaining CORE processes, a stopped injector, and a removed one-shot task. The independent post-detachment check also reports zero target entries, CORE processes, Frida processes, and research tasks.

## Evidence remains separate from release acceptance

The ASAR SHA-256 before and after the attached run is `64e3de1cdec77fba29a9db3313ec945c89dd34c22ad5f8609ec1b125ad67c7b2`; the main-bundle hash remains `96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531`. The source and six isolated reference checks are described in [Windows readback research](windows-readback.md). The [no-device logger validation](windows-hid-capture.md) remains a separate passing result, not evidence that this hardware capture is complete.

The sanitized aggregate record is [windows-hardware-validation.json](windows-hardware-validation.json). Raw temporary evidence hashes identify the inputs used for this report; raw files are not release assets:

| Artifact | SHA-256 |
| --- | --- |
| `hardware2-probe.js` | `596324bbe31478501886bb0698155adfea9efcd22acaf5a572073fa16888a81e` |
| `hardware2-result.json` | `316e46a89f289a03d32d237b4cd26bf3355d6e4248601bb919c89890a950a2ce` |
| `hardware2-hid-trace.jsonl` | `c29c294b0c18aa1de5de1c30c964d9de2700b5ea169f711e415771f81aabf890` |
| `hardware2-core-log-tail.txt` | `d694621900f8d6d5a63a1416a4ff98f43fa4eed82a49cd54da7a36e89565750e` |
| `hardware3-result.json` | `0db1fbf3c428b9643b6a12db8a4e3f77afffb8cce4fd271f4685a798c5dd512d` |
| `hardware4-probe.js` | `e211a0793c5c6de7a3718d788447c0c80034c31c6ea5af078a39ff6e454f4d08` |
| `hardware4-result.json` | `e638dfe0e69f89eed94cf1af98e106596c63cfd0db4b2ae672e2e7e0a333d658` |
| `hardware4-hid-trace.jsonl` | `62bb98a483ff7854824ac06bf17224d7fec134c99fa1519bdd9cd6320c574312` |
| `hardware4-core-log-tail.txt` | `7f138c73fdb6a692a4fb19dbd8ffaa975c93046dba519d9029ba10429262a30d` |
| `hardware4-final-verification.json` | `2c46f3cc58a4a2123e279a64bb76cd98f5c21e5ccc1e51aa81346c7b4315d4d4` |

The task-local cleanup inventory includes the temporary VM, snapshot, tools, controllers, and raw logs. The VM is retained, shut down, pending the [native acceptance and evidence-preservation gates](native-runtime-verification.md). None is added to machine setup scripts. No successful onboard-settings download, firmware-readback impossibility, native feature parity, or shippable release is established by this result.

### External API references

- [Microsoft collection-information API](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/hidclass/ni-hidclass-ioctl_hid_get_collection_information)
- [Microsoft `hidclass.h`](https://github.com/microsoft/win32metadata/blob/main/generation/WinSDK/RecompiledIdlHeaders/shared/hidclass.h)
- [Microsoft input-queue buffer configuration](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/hidclass/ni-hidclass-ioctl_set_num_device_input_buffers)
- [UTM USB-sharing limitations](https://docs.getutm.app/guest-support/sharing/usb/)
