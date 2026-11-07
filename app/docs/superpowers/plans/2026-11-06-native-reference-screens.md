# Native Reference Screens Implementation Plan

**Goal:** Render the confirmed four-screen CORE layout in the native app while preserving explicit hardware actions and imported local drafts.

**Architecture:** Rust/egui remains the shipping UI. A shared appearance module owns colors and keyboard drawing; the app owns navigation and editing. Existing HID operations remain explicit, with their write scope displayed. Original vendor artwork and fonts remain research-only.

**Spec:** [Windows UI reference](../../research/windows-ui-map.md).

## Constraints

- GMMK 2 96% ANSI, stock firmware; three local slots and three layers.
- No write on launch, connection, navigation, key selection or local editing.
- Imported bytes remain the editing source; unknown onboard state is never presented as downloaded.
- Unsupported features remain visible as unavailable where appropriate, without pretending to implement them.
- No commits, publication, firmware operations or new dependencies in this increment.

## Testable Work

- [x] Cover all 99 physical keys with unique layout positions; fix identifier matching.
- [x] Render navigation, device header, profile dashboard and two-column editors at the reference viewport.
- [x] Preserve import/export and explicit device actions; decode the selected key into its editor; preserve selection semantics across layers/slots.
- [x] Test the rendered labels and keyboard interactions with egui pointer input; test that navigation/editing enqueue no device commands.
- [x] Build the native artifact without connecting to hardware; document remaining visual and functional differences.
- [ ] Inspect the native window and compare matched captures. Computer Use approval for the development preview is unavailable; headless tests do not replace this gate.

## Remaining Acceptance

This increment does not establish full parity. Matching fonts/artwork, section-specific hardware saves, complete bindings/macros, full per-key and side-light behavior, hardware readback, Windows comparison, Linux runtime/packages and GitHub release gates remain separate requirements. Physical verification requires an agreed recovery procedure and user-assisted observations.
