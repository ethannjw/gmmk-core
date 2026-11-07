# Windows macro behavior and implementation gates

CORE 2.1.23 provides a local macro editor and a GMMK 2 upload path using commands `4`, `5` and `3`. Thirteen isolated checks reproduce its event packing and several unsafe boundary cases. **Native macro support remains unimplemented and commands `4`/`5` remain blocked.** Matching the intended controls requires an explicit policy for reference defects, a persistent macro model and physical acceptance; copying the packer unchanged would preserve truncation and a repeat-mode exception.

Scope: GMMK 2 96% ANSI, stock firmware, CORE package `2.1.23.2`. These findings describe the extracted source and selected pure helpers executed under Node.js `22.22.0`. They do not establish how the full Windows application or keyboard behaves at these boundaries.

## Editor and local storage

The active renderer's `MacroEditor` starts at line 71917. It records document keyboard/mouse events, exposes a timeline, supports reassigning/deleting recorded actions, and offers three modes: **No Repeat**, **Repeat While Holding** and **Toggle**. The fixed-delay input defaults to 30 ms and declares a 10–10000 ms range; the alternative records timing. The Save Macro button invokes its callback with the local recording. These UI affordances do not prove that every recorded mouse/key identifier has a working hardware encoding.

In the main bundle, `MacroState` (3788) stores an ID, name, numeric value, mode and schedule. Schedule entries contain actions with input names and start/end times. `MacroChannel.UpdateMacro` (73370) reconstructs the state and calls `DataStorage.saveMacro` (3965), which writes local macro storage. Saving a macro record is separate from uploading a keybinding that references it.

`saveMacro` assigns a new ID and `macros.length + 1` as the numeric value even when replacing an existing record. Deletion/replacement and references therefore need end-to-end tracing before adopting this allocation strategy. The report has only a one-byte macro value; the isolated payload test demonstrates that 256 wraps to zero. No hardware slot-capacity claim follows from this observation.

## GMMK 2 upload path

`GMMKDeviceHandler.updateAllKeyBinding` (60528) filters macro assignments in the supplied layer, resolves their macro records from local storage, and sends:

| Order | Frame | Source-derived contents | Delay |
| --- | --- | --- | --- |
| 1, if a macro is assigned | Command `4`, 264 bytes | Profile order + 1, active layer + 1; mode table starts at byte 8, two big-endian bytes per key-matrix position | 150 ms |
| 2, for each referenced stored macro | Command `5`, 264 bytes | Macro value in byte 2; packed events start at byte 8; no profile/layer selector | 150 ms each |
| Last | Command `3`, 264 bytes | Layer key matrix; a macro binding has type byte `5` and the macro value in its code byte | 150 ms |

The GMMK 2 branch does not send an activation frame after this sequence. This is direct-source control-flow evidence, not a captured USB trace. The byte tests cover the macro helpers, not the complete handler, key-position lookup or storage integration. The unaddressed command-5 header raises a cross-profile storage question; it does not prove firmware storage scope.

Relevant main-bundle definitions: `GetKeyAssignmentBuffer$1` macro branch (46625), `MacroStateToMacroEventSequence` (44790), `convertMacroEventsToBuffer` (46827), `prepareMacroDataBuffer` (46851), `PrepareMacroBuffer` (46873), and `PrepareMacroTypeBuffer` (46877).

## Encoding and reproduced boundary cases

Each schedule action expands into a press event at its start and a release event at its end. Events sort by start time; equal-time events retain their schedule order. Each event occupies three bytes: delay high bits combined with the press flag (`0x80`), delay low byte, then HID code. Delay measures the time until the next event, with a minimum of 1 ms; the final event also gets 1 ms. The first event's absolute start time is discarded.

| Case | Isolated result | Consequence for native implementation |
| --- | --- | --- |
| 82 events | All 246 event bytes fit in the copied payload region | At most 41 complete press/release actions fit without splitting an event in this encoder. This is a source-derived encoding boundary, not a measured firmware limit. |
| 84 events | Only 248 bytes are copied: event 83 loses its key code and event 84 is omitted | Reject or redesign oversized input before any write; never silently truncate. Longer hardware sequences need additional protocol evidence. |
| 32768 ms between a release and the next event | Delay sets the same high bit used for a press | Timing needs a validated representable range or a separately verified delay-extension encoding. |
| Unknown input name | HID code becomes zero | Unsupported recording inputs need explicit handling rather than silent substitution. |
| Numeric macro value 256 | Payload byte 2 becomes zero | Validate allocation and report width before upload; firmware capacity remains unknown. |
| No Repeat | Mode table stores `00 01` | Source-derived mode value only; physical execution remains unverified. |
| Toggle | Mode table stores `00 00` | Key matrix supplies the macro binding; zeros alone do not distinguish an unbound key. |
| Repeat While Holding | `writeInt16BE(65535, offset)` throws `ERR_OUT_OF_RANGE` | The table declares `65535`, but the packer uses a signed API. Intended `FF FF` encoding and actual Windows behavior require verification before enabling this mode. |

The tests intentionally assert the reference defects. A passing characterization test records what this exact source does; it is not a target behavior for the native app. Preserving intended macro functionality while rejecting unsafe input is the recommended parity policy, pending owner agreement. No exception is silently corrected in shipping code.

## Reproduce the isolated checks

From the repository root, with the separately extracted Windows reference available:

```sh
node app/scripts/check-windows-macro-reference.cjs /path/to/CORE-2.1.23/out/main/index.js
```

The script requires the main bundle SHA-256 `96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531` before executing selected helper declarations. It uses Node's Buffer implementation, mock macro storage and a numeric key-offset stub. It does not load Electron, invoke the full main entry point, load native addons or open HID. The VM is an isolation aid, not a general security sandbox; the pinned, reviewed input is required.

Expected result: **13 passing characterization checks**. A missing input or hash mismatch fails before helper execution. Vendor source stays outside the repository. The Rust suite remains a separate check and does not run these reference-dependent tests.

## Remaining implementation work

1. Agree on intended-functionality parity versus reproducing the demonstrated defects. Keep unsupported cases visible until their behavior is established.
2. Trace macro creation, replacement, deletion and binding-reference updates. Define compatible local persistence/import/export without silently changing existing profile files.
3. Implement the matching local editor and recorder with focused/unfocused, key-release, cancellation and unsupported-input tests. Reproduce mode and delay controls without claiming unverified hardware support.
4. Add byte-level golden fixtures and write-boundary tests for complete macro uploads, preflight rejection, referenced-macro resolution and partial failures. Keep operations explicit and serialized.
5. Complete owner-approved Windows/runtime comparison and hardware tests for mode behavior, input types, timing, storage scope, app-closed execution, reconnect and power-cycle persistence. Use the [hardware walkthrough](../HARDWARE_ACCEPTANCE.md) as the recovery prerequisite, not as a macro acceptance procedure.
