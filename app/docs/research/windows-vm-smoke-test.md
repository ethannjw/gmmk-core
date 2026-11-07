# CORE launches in the isolated Windows VM without a keyboard

The Windows CORE 2.1.23 reference installs and launches in a temporary Windows 11 ARM64 VM. Its renderer displays the dark application shell, Glorious branding, settings icon, and an empty device area. Runtime logs report zero enumerated devices and successful startup. This is a no-device smoke test, not proof of onboard readback, device-feature parity, or release readiness for the native application.

Test date: 2026-11-07. Physical USB passthrough remains disabled throughout the test. No physical keyboard configuration, reset, or firmware operation is performed.

This page describes the isolated test. The separately approved [physical startup capture](windows-hardware-readback.md) enables USB sharing, detects the keyboard, and records 23,111 target events without recorder drops after a physical reconnect. No settings download is observed within its bounded API coverage. Its results and environment-cleanup status must not be confused with this earlier isolation baseline.

## Installation and startup have verified provenance

| Check | Observed result |
| --- | --- |
| Installer | `GloriousCORE_2.1.23_Setup.exe`, 391,694,128 bytes |
| Installer SHA-256 inside guest | `7f1fd7910b4d70636481b5096bc4d47cd12bdaf0ea6b1db11dd0a152ee7d2a75`, matching the host file |
| Authenticode | Windows reports `Valid`; signer is Glorious LLC |
| Installation | Inno Setup reports successful installation and process exit code 0; Windows is restarted as requested by the installer |
| Installed executable | `C:\Program Files (x86)\Glorious CORE\Glorious Core.exe` |
| Installed `resources\app.asar` SHA-256 | `64e3de1cdec77fba29a9db3313ec945c89dd34c22ad5f8609ec1b125ad67c7b2`, matching the statically inspected reference |
| Runtime version | CORE's environment log reports `AppVersion:2.1.23`, `x64`, `win32`, and Windows `10.0.26300` |
| Physical target isolation | Guest PnP inspection finds zero entries matching `VID_320F` or `PID_504B`; the VM configuration has `UsbSharing=false` |

The VM uses UTM 5.0.8, Windows 11 Home Single Language ARM64 10.0.26300.9457, four virtual CPUs, and 8192 MiB RAM. CORE is an x64 process in this ARM64 guest. The disposable account is `GMMKResearch`; no Microsoft account or personal credentials are used.

The verified installer is delivered on a read-only ISO. A previous interrupted guest-agent transfer leaves an incomplete copy in guest Downloads; that copy is not executed. Installation uses the guest agent with `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-` and an installation log. The installed application starts in the interactive guest session after reboot.

## The empty dashboard corresponds to zero enumerated devices

The guest's `CORELog_2026-11-7.log` records the following sequence:

1. Probe 81 supported device definitions and enumerate zero devices.
2. Complete initialization with zero connected devices.
3. Create the renderer and load `out\renderer-process\index.html` from the installed ASAR.
4. Deliver `PreConnectedDevices` containing zero devices and complete startup.
5. Reconcile again at startup catch-up and periodically, retaining zero devices.

The guest datastore's `Devices.json` contains `{"deviceInstances":[]}`. Visual inspection shows an empty device area rather than a keyboard illustration. This establishes the disconnected launch behavior of this reference build; it does not reveal how a connected GMMK 2 reads or writes settings.

The main window can be minimized and restored using keyboard navigation. Attempts to open Settings through automated pointer clicks do not produce a visible panel. Pointer input into the VM is unreliable in this session, so Settings interaction is unverified rather than classified as a CORE defect.

## VM compatibility and hardware acceptance remain separate

Switching the virtual display from `virtio-ramfb-gl` to `virtio-ramfb`, with the stable Red Hat VirtIO GPU DOD driver installed, produces a fully rendered Windows desktop after partial gray rendering on the GL configuration. This is an observed workaround, not an isolated proof of the rendering defect's cause. Emulated VLAN networking works; the earlier Shared network configuration did not obtain an IPv4 gateway.

This smoke test does not verify:

- Physical USB passthrough, GMMK 2 enumeration, or packet capture on Windows ARM64.
- Onboard profile, lighting, binding, macro, or performance-state download.
- Device-specific screens or writes, firmware updates, reconnect behavior, or persistence.
- Native macOS/Linux UI parity, distribution packaging, or hardware acceptance.

The six isolated [readback reference checks](windows-readback.md) also pass on this date. They exercise selected JavaScript declarations with storage and HID doubles; they are not VM-to-keyboard tests.

## Capture validation remains incomplete

### Built-in USB tracing records metadata but has no verified payload capture

A no-device capture probe on 2026-11-07 reads 4096 bytes from the verified installer on the VM's virtual USB CD-ROM. The read uses `CreateFileW` with `GENERIC_READ`, `OPEN_EXISTING`, `FILE_FLAG_NO_BUFFERING`, and an aligned buffer to avoid accepting a cached file read as USB capture evidence. The probe refuses to run if the physical target is present and verifies the virtual disc identity before reading.

The trace enables `Microsoft-Windows-USB-USBXHCI`, `Microsoft-Windows-USB-UCX`, and `Microsoft-Windows-USB-USBHUB3` with keyword mask `0x101` (`Default` plus `FullDataBusTrace`) and level 5. The saved session query confirms these settings. Microsoft documents this [USB event trace configuration](https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/how-to-capture-a-usb-event-trace), but the local result does not establish usable packet-byte capture:

| Check | Observed result |
| --- | --- |
| Physical target PnP entries | 0 |
| Unbuffered read | 4096 bytes at offset 0 |
| Read-buffer SHA-256 | `69f6282d85b58b65629358d57fbb4d7c8f9328e69dfc90e2f39d19f587e2ef28` |
| Raw ETL size | 655,360 bytes |
| Decoded transfer lengths | Two events with `0x1000`; twelve each with `0x1F` and `0xD` |
| Known payload in raw ETL | Neither the first 64 bytes nor the complete 4096-byte sequence is found |
| Trace teardown | Session stopped and deleted |

The decoded transfer events contain metadata, including lengths and addresses. The independent raw-ETL search avoids relying solely on XML decoding, but does not exclude a fragmented or differently encoded representation. The cause of missing payload bytes remains unresolved. This virtual-storage test also does not prove how HID feature reports would be represented. ETW is not accepted as the keyboard payload-capture mechanism on this evidence.

### Launching CORE through Frida validates synthetic API interception

The official Frida 17.26.1 Windows ARM64 injector runs inside the guest and reports version `17.26.1`. Its expanded SHA-256 matches the host copy: `39d38f9518236f6eb56edb618f94f367a0c714c8f98075dffa7485f1cf37e8a0`. The executable transfers through a temporary loopback-only HTTP server; that server exits after transfer. No host Frida installation or machine-setup entry is created.

A disposable PowerShell control process successfully executes the injected probe and reports `Process.arch = x64`, pointer size 8. The probe attaches to `KERNELBASE.dll!DeviceIoControl`, calls that function with `INVALID_HANDLE_VALUE`, control code `0x000b0192`, input bytes `0` through `64`, a 65-byte output capacity, and no overlapped structure. The interceptor records all 65 input bytes exactly, a false return, and error 6 (`ERROR_INVALID_HANDLE`). The independent `SystemFunction` result agrees. The probe detaches its listener before writing its completion record; the controller then stops the injector and its disposable control process.

This control validates interception of a deliberately failing call. The invalid handle cannot identify the physical keyboard, and the control does not open any device handle.

Three bounded attempts against the existing CORE main process produce no verified script execution: an initial 30-second attempt, a 60-second attempt with an explicit script-result file instead of relying on redirected stdout, and a 60-second attempt under the logged-in disposable user. The last uses a one-shot interactive scheduled task, which is removed afterward. Its result does not support treating cross-session execution alone as the cause. The attachment failure remains unresolved; further identical retries are not a validated capture plan.

Launching CORE through Frida's `-f` option succeeds in this guest. With separate approval to restart only the Windows application, the controller checks that the target is absent, verifies the injector and ASAR hashes, and copies the disposable user's local datastore before stopping the original CORE process. This datastore copy is a local-application backup, not an onboard backup. An interactive, limited-privilege scheduled task runs the launch probe and is removed afterward.

The relevant invocation inside the guest is:

```powershell
frida-inject.exe -f "C:\Program Files (x86)\Glorious CORE\Glorious Core.exe" -s <probe-script.js>
```

The probe writes an explicit result file, avoiding dependence on redirected stdout buffering. Its result identifies the newly spawned CORE main process, reports `x64` and Frida `17.26.1`, and verifies the same 65-byte invalid-handle `DeviceIoControl` test inside that process. All input bytes match; both the interceptor and direct call report false/error 6. The listener is detached before the controller stops the injector. The test does not use eternalization or modify the installed application files.

| Startup-injection check | Observed result on 2026-11-07 |
| --- | --- |
| CORE main process | Probe PID 7128 matches the spawned `Glorious Core.exe` main process |
| Synthetic input | All 65 bytes match the sequence `0` through `64` |
| Call result | False, `ERROR_INVALID_HANDLE` (6), observed independently by the hook and caller |
| Hook teardown | Explicit `hooksDetached=true` completion record |
| Continued startup | Same main process remains running with GPU, utility, and renderer children after injector shutdown |
| Runtime log | Fresh initialization and renderer-load sequence completes with zero connected devices |
| Visual check | Dark CORE dashboard, branding, and settings icon render with an empty device area |
| Application integrity | ASAR hash is identical before and after the test |
| Isolation and cleanup | USB sharing false, zero physical-target PnP entries, zero Frida processes, zero temporary research scheduled tasks |

Launch-first instrumentation is therefore a verified way to run this synthetic hook inside CORE. The reason that attachment to an already-running instance stalls remains unresolved. The separate [bounded HID logger validation](windows-hid-capture.md) passes 17 Node tests, 20 native synthetic checks, and an invalid-handle smoke through CORE's actual imported API address. It also verifies teardown after 20 seconds of no-device observation. Successful physical HID responses, genuine CORE keyboard requests, and onboard-state download remain untested. The visible dashboard does not establish settings interaction or device-feature parity.

## Hardware access requires separate approval

Before attaching the physical keyboard, review the logger's coverage limits and obtain informed approval for passthrough and CORE's startup polling-rate write. Arrange another input device because passthrough can remove the keyboard from the host. A separately reviewed physical-capture launcher is still required. Do not infer firmware readback support or send speculative commands from this no-device test.

The VM and tooling are temporary and excluded from machine setup scripts. The task-local cleanup inventory tracks the VM bundle, UTM installation, ISOs, incomplete transfer, and guest logs. Remove those task-created assets after research, preserving only the agreed findings and captures. Vendor executables and assets are not added to Git.
