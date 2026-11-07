# Native verification

The native app has automated profile, protocol, write-boundary and headless UI tests. These tests do not certify keyboard behavior, visual parity or release readiness. Hardware checks follow the separate [acceptance walkthrough](HARDWARE_ACCEPTANCE.md).

## Local commands

Run from `app/` with Rust and the platform's native build prerequisites installed:

```sh
cargo test --locked
cargo build --release --locked
```

Add `--offline` only when dependencies are already cached. Current counts and native observations are recorded in the [runtime verification](research/native-runtime-verification.md). `rustfmt` is unavailable on the verification machine, so formatting validation remains pending.

The tests use synthetic reports and headless egui frames. They cover validation, report bytes, selected write scopes, error handling, notification parsing, draft preservation and UI interactions without a connected keyboard. Test names and individual results appear in `cargo test` output.

The [native runtime check](research/native-runtime-verification.md) covers automatic detection, the real device chooser, profile dashboard/navigation, menus and manual disconnect/retry on macOS. The [framing check](research/hid-report-framing.md) verifies descriptor-matched 256-byte native reports and records a successful lighting transfer reported by the rebuilt app during owner interaction. Earlier local import/edit/export checks are separately identified. Notification opening still fails with an IOKit permission error. Physical effects, persistence, recovery and other Save categories remain unverified. The Windows reference VM remains stopped and retained until evidence and recovery gates are satisfied.

## GitHub Actions

The [native workflow](../../.github/workflows/native.yml) runs tests and an optimized build on changes to `app/` or the workflow, and supports manual dispatch. It pins Rust 1.99.1 and the checkout action's commit, grants read-only repository permission, and does not persist checkout credentials.

| Runner | Validation target |
| --- | --- |
| `ubuntu-24.04` | Linux x86_64 |
| `macos-15` | macOS ARM64 |
| `macos-15-intel` | macOS x86_64 |

This matrix defines intended CI checks, not minimum supported operating systems. The public source repository is `ethannjw/gmmk-core`; hosted results appear in its Actions tab. A configured workflow alone does not establish a passing hosted run. The workflow does not package, sign, upload or publish the application.

Linux CI installs `pkg-config`, `libudev-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`, `libxi-dev`, `libxrandr-dev`, `libxcursor-dev` and `libgl1-mesa-dev`. The manifest enables both X11 and Wayland. A locked Linux-target dependency inspection confirms both winit features; this is not evidence of a successful Linux compilation or GUI session.

## Windows reference checks

The scripts in `app/scripts/check-windows-*-reference.cjs` compare isolated, reviewed functions from a hash-pinned Windows CORE 2.1.23 bundle. The bundle stays outside the repository. The scripts reject unexpected source hashes and use stubbed dependencies instead of the original application bootstrap or native HID backend. They are research tools for this reviewed artifact, not a sandbox for arbitrary programs.

| Check | Evidence and invocation |
| --- | --- |
| Preset report builders | [Lighting reference vectors](research/windows-ui-map.md#preset-report-builders-match-shared-windows-vectors) |
| Macro encoding boundaries | [Macro reference checks](research/windows-macros.md) |
| Mouse identifier mismatch | [Mouse-function encoding](research/windows-ui-map.md#mouse-function-encoding-evidence) |
| Slot and layer notifications | [Notification handler](research/windows-ui-map.md#profile-controls-and-data-source) |
| Keyboard-layer labels, selection callbacks, help text and scoped styling | [9 keyboard-layer reference checks](research/windows-ui-map.md#keyboard-layer-selector) |
| Preset slider rules and callbacks | [Slider reference checks](research/windows-ui-map.md#preset-slider-value-rules-and-callback-behavior) |
| Preset picker, labels and CSS | [23 effect-selector reference checks](research/windows-ui-map.md#preset-effect-selector) |
| Color surface, conversions, numeric input and lighting color handoff | [68 color reference checks](research/windows-ui-map.md#inline-color-surface-and-numeric-input-evidence) |
| Saved-color defaults, persistence callbacks, deletion and failure behavior | [10 saved-color reference checks](research/windows-ui-map.md#saved-color-palette-and-persistence) |
| Lighting-mode selector, quick groups, breathing controls, All and side-LED report bytes | [236 custom-lighting reference checks](research/windows-ui-map.md#custom-lighting-group-and-report-checks) |
| Custom brightness, raw-color preservation, rounding and scaled report bytes | [39 custom-brightness reference checks](research/windows-custom-brightness.md) |
| Performance Save and ignored OS mode | [114 performance reference checks](research/windows-ui-map.md#performance-save-evidence) |

CI does not download or execute the vendor bundle. Rust tests consume the shared lighting fixtures directly. REA findings and source comparisons specify behavior but do not establish runtime equivalence.

## Release acceptance

Release approval also needs native visual comparison, Windows reference-runtime checks, physical keyboard behavior and recovery tests, Linux runtime acceptance, and clean-machine installation checks. No onboard settings-download contract is verified. A passing automated suite cannot replace any of these checks. See the [parity roadmap](research/core-parity.md) for open decisions and release gates.
