# Device Entry and Profile Dashboard Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task. Preserve the current checkout and all existing changes; do not commit or publish.

**Goal:** Implement the approved Windows-style device chooser and profile dashboard while keeping all hardware writes explicit.

**Architecture:** The HID worker discovers the supported device automatically. A separate screen state controls chooser/editor navigation without coupling local profile selection to hardware activation. Existing settings tabs and transport remain in place.

**Tech Stack:** Rust, egui/eframe 0.31, hidapi, existing JSON profile format.

**Spec:** The approved November 7 mockup uses an 80px rail, 25px title strip, #303030 chooser, 300×250 device card, #111 editor, 142px #262626 header, orange selected tabs, three 59px profile rows and a large keyboard preview. Entry creates explicitly local defaults only when no draft exists. Onboard values remain unknown. Navigation and reconnection preserve drafts. Profile menus support rename, import/export and confirmed local default restoration. Other settings remain functional. Vendor artwork stays outside the distributable; original vector artwork and licensed fonts replace it.

## Constraints and Review Focus

- Only stock GMMK 2 96% ANSI 320f:504b is supported. Multiple matching interfaces fail closed.
- Discovery opens handles and reads USB metadata; it sends no configuration reports.
- Stable access errors do not trigger repeated automatic opens. Explicit retry and removal/reinsertion permit retry. Manual disconnect pauses discovery.
- Unplugging never clears a local draft or labels defaults as onboard settings.
- Import/reset must not silently discard edits; local selection never activates a slot.
- Test both 1000×700 and 1440×880, including permission errors, disconnected operation and narrow menu layouts.
- Preserve the stopped temporary VM. No physical keyboard writes or privacy changes during verification.

## Tasks

- [x] Add failing entry/navigation tests, then implement screen state, local defaults and draft preservation in `src/ui/app.rs` and `src/ui/app/entry.rs`.
- [x] Add failing discovery transition tests, then implement initial discovery, hotplug, retry suppression and disconnect pause in `src/device/discovery.rs` and `src/device/mod.rs`.
- [x] Add dashboard/menu behavior tests, then implement the approved shell, profile rows, vector graphics and licensed typography in `src/ui/app/entry.rs`, `src/ui/app/view.rs` and `src/ui/appearance.rs`.
- [x] Run focused tests, full `cargo test --locked --offline`, release build and existing protocol research tests. Inspect the native app without settings writes.
- [x] Update runtime/user documentation with verified behavior and remaining parity/release gaps.

## Verification Record

171 Rust tests pass; optimized macOS build passes with two pre-existing dead-code warnings. The trace suite passes 17 tests and the isolated readback reference suite passes 6 tests. Each new entry, discovery, menu, error-visibility and key-legend regression fails before its corresponding implementation/fix and passes afterward. Native inspection confirms automatic configuration detection, chooser entry, default/minimum-size dashboard, menus, local selection preservation and manual disconnect/retry. The final rebuilt window is open for review. See [runtime verification](../../research/native-runtime-verification.md) for the executable hash and bounded observations.

Independent review finds one chooser error-visibility defect, now fixed with a regression test. UI inspection identifies navigation-key legend overflow, now fixed and tested at card and editor sizes. Fonts come from a pinned Google Fonts revision with bundled OFL texts; vendor images are not shipped. No temporary tooling is added to machine setup. The VM is confirmed stopped and retained. No physical settings writes, permission changes, commits or publication occur.

Passing simulated UI/transport tests do not establish hardware write persistence, Linux runtime support, signing or full Windows feature parity. Notification access remains denied on the current host. `rustfmt` is unavailable; no formatter is installed as part of this task.
