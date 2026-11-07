# Verify the keyboard on real hardware

Use this walkthrough to verify GMMK Config on a stock GMMK 2 96% ANSI (`320f:504b`). Complete the connection checks first. **Stop before activation or Save until the keyboard owner approves a recoverable test configuration and target slot/layer.** A successful build or HID transfer does not prove that the keyboard applied or retained a setting.

## 1. Prepare a recoverable test

1. Confirm the model, stock firmware and operating system. Record the app revision and any uncommitted changes. Keep a second keyboard or another usable input method available.
2. Agree which profile slot and layer may change. The UI numbers both from 1 to 3. Record the previously active slot/layer using an independently known baseline; local slot selection does not report the active hardware slot.
3. Identify a known-good configuration and a tested way to restore it, including the target layer's entire key matrix, lighting and latency. If recovery depends on Windows CORE, confirm access to that Windows installation and its saved configuration before testing.
4. Export the local test draft and keep a separate copy of the approved baseline. **An exported draft or factory template is not a backup of unknown onboard settings.** If the baseline or recovery method is unknown, complete only section 2.
5. Close Windows CORE and other keyboard-control applications before connecting GMMK Config. Close GMMK Config before using another application for recovery.

Do not use firmware updates, factory resets, speculative read commands or the legacy Electron prototype as recovery shortcuts. Those operations need separate evidence and approval.

## 2. Check connection without changing settings

From the repository root, build the native app:

```sh
cd app
cargo test --locked --offline
cargo build --release --locked --offline
```

On a machine without cached dependencies, omit `--offline`. Complete the platform's [permission setup](../../README.md#permissions), then launch `./target/release/gmmk-config` yourself.

Startup displays a device chooser. Automatic discovery opens the supported configuration interface and attempts the passive notification listener without configuration reports. A connected card opens **My Keyboard** with local defaults if no draft exists; **Local profile · onboard values unknown** remains visible. The preview represents the draft, not downloaded settings. Check both supported window sizes. A missing/denied interface must not appear as a connected card.

1. Expect automatic detection, or a visible access error. Open **Settings** for connection details and manual Connect/Disconnect. Manual Disconnect pauses automatic connection until Connect/Retry. USB enumeration alone does not prove that the configuration interface can be opened.
2. Check **Firmware (USB)** and **Refresh info**. The value comes from cached USB release metadata; refresh sends no feature report. Record the displayed value and source, even if the value is unavailable. Do not interpret it as downloaded settings or firmware-update status.
3. Check that **Read onboard settings (unverified)** is disabled. Connecting must not populate a keymap from the keyboard or substitute defaults for an unsuccessful read. Record the separate slot/layer listener status in Settings: listening, interface unavailable or reader error. A listening message proves only that the handle opened; no notification is a valid unknown result. No enable command is sent.
4. Import the local test draft, change tabs, select a profile/layer and edit a draft binding or color. Export and reimport that draft to check local persistence. Expect no keyboard configuration change. Record any change and stop if one occurs unexpectedly.
5. Use **Disconnect**, then **Connect** and **Refresh info** again. Expect metadata and any last notification to clear on disconnect; metadata returns after reconnect, while a slot/layer observation requires a new event. Expect no automatic application of the draft. Check ordinary typing and the agreed media keys while connected; stop if opening the listener interferes with normal input.

Do not press **Save to Keyboard**, **Apply all layers in slot**, or **Activate selected slot / layer** during this section. Observing no visible change is a smoke check; a complete no-write acceptance test also needs a captured device-transaction trace.

## 3. Verify approved writes one operation at a time

Proceed only after the owner approves the baseline, target, recovery method and these write scopes. Keep the approved draft unchanged except for the test change. Wait for each operation to finish before starting another.

### Hardware slot/layer notifications

If the owner has a verified hardware shortcut for changing slot/layer, agree on one reversible change and how to restore the previous target. Use that shortcut while connected, then inspect **Last keyboard notification**. The reported slot/layer must match the approved physical change; the selected local draft, edited values and unsaved-change indicator must remain unchanged. Restore the previous slot/layer and record its notification. Do not guess shortcuts or enable reports with speculative writes. If the shortcut or notification-enable requirement is unknown, mark this check **not run** and keep the other checks separate. A notification is not full settings readback and does not confirm persistence of a Save.

### Keybinding and activation

1. Import the approved baseline and select the approved local slot/layer. Choose a nonessential ordinary key and an ordinary replacement agreed with the owner. Avoid modifiers, Fn, shortcuts and keys required for recovery in the first test.
2. In **Keybinding**, select the physical key and choose its replacement. The local draft changes immediately. Export the test draft. Expect the keyboard to keep its previous behavior before Save.
3. Click **Save to Keyboard** once. This sends the **entire selected layer's key matrix**, including other bindings from the imported draft; it is not a one-key update. It does not activate that slot/layer or send lighting/polling settings.
4. Record the completion or error text. A sent result establishes transfer completion only. If the result is uncertain or fails, follow section 4 instead of retrying.
5. Use **Settings → Activate selected slot / layer** only for the approved target. Activation changes hardware state and remains separate from selecting a local profile tile or layer.
6. In an empty plain-text document, press the changed key and the unchanged keys selected for this test. Record actual output. Check lighting for unintended changes. Close the app and repeat the key checks.
7. If power-cycle testing is approved, disconnect and reconnect the keyboard, then check its active state and key output before reopening the app. If the keyboard boots into another slot/layer, record that separately; activation after reconnect must not be mistaken for proof of startup behavior.
8. Restore the known-good configuration through the agreed recovery method. Verify the restored key output and previous active slot/layer before testing lighting.

### Lighting

1. Import the approved baseline again. Select the approved slot/layer, then stage one owner-approved visible lighting change. Record the displayed base effect, brightness, speed, color, custom colors and input latency before Save.
2. Click **Lighting → Save to Keyboard** once. It sends the displayed base effect, input latency and any supplied custom-color pages, then activates the selected slot/layer. **Both lighting subpages contribute to Save.** It sends no key matrix or polling setting.
3. If custom colors are unspecified, Save leaves custom pages untouched. An explicitly empty custom list clears them. Do not click **Clear custom lighting** unless clearing all custom pages is part of the approved test. **All** stages changes to 99 keys and 20 side LEDs; use it only when that broader scope is explicitly approved. For an approved All test, record both side strips as well as the keys; the native static preview does not verify side-LED output.
4. Record the completion or error text and the physical LEDs. Check the agreed key outputs for unintended changes. Repeat the observation with the app closed and, if approved, after a keyboard power cycle.
5. Restore and verify the baseline through the agreed recovery method. Keep before/after photos with the test record when available.

Leave **Performance Save** and **Apply all layers in slot** out of this initial smoke test. Performance Save currently writes an entire layer, and whole-slot apply writes three layers. Their sequencing still differs from the Windows reference; each needs its own approved acceptance procedure. This walkthrough does not certify them, macros, other bindings, other firmware, Linux packages or complete visual parity.

## 4. Stop and record unexpected behavior

- If a write fails or its outcome is unknown, stop. Some earlier reports may already have changed the keyboard. Do not queue another Save, activate another target or retry automatically.
- If the UI reports that the worker stopped, preserve the exact error and test draft. Restarting the app does not establish the keyboard's state. Verify the physical state and agree on recovery before another write.
- If a write remains pending, do not start another app or unplug the keyboard as an improvised cancellation. Record the pending message and elapsed time, then agree on a recovery action. Bounded-timeout and interruption behavior still require acceptance testing.
- If keys, lighting, another slot/layer or settings outside the intended scope change, mark the test failed and stop. Preserve the evidence before attempting the approved recovery.

## 5. Record what passed

Create a test record with the following fields. Use **pass**, **fail**, or **not run** for each check; never infer a pass from another platform or a successful send.

```text
Date / tester:
OS version / architecture:
Keyboard model / VID:PID / stock firmware confirmation:
USB release metadata / independent firmware reference, if available:
App version / commit / local changes:
Approved baseline / tested recovery method:
Approved target slot / layer / previously active slot and layer:
Approved test binding / lighting change / power-cycle permission:

Check / expected result / observed result / pass, fail or not run:
Connection and metadata:
Notification listener status / approved hardware slot-layer change / observed event:
Ordinary typing and media keys while listening:
Local edits and import/export without configuration changes:
Disconnect/reconnect without automatic writes:
Keybinding Save and explicit activation:
Key output with app closed:
Lighting Save and unaffected key output:
Power-cycle behavior before app launch:
Baseline restoration:
Exact messages / photos / transaction capture references:
```

For release acceptance, expand this record to every claimed key action, lighting control, slot/layer, firmware and OS combination. Use the [parity matrix and release gates](research/core-parity.md#4-verification-status-and-proposed-release-gates) for the remaining coverage. Keep incomplete checks marked **not run**.
