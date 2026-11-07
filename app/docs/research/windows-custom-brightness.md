# Custom brightness preserves source colors and updates every painted key

Windows CORE 2.1.23 stores custom brightness separately from preset brightness and from the original RGB values of painted keys. Moving custom brightness updates every existing custom key, including side lighting; new paint uses the same setting. The native app has no equivalent persisted field. Adding a brush-only slider or repeatedly scaling its stored RGB bytes would produce different behavior and lose color information.

## Reference behavior

The reviewed artifact is the hash-pinned Windows package described in the [parity reference](core-parity.md#1-reference-and-existing-implementations). Source locations below are within that extracted package, which remains outside Git.

| Path | Observed behavior |
| --- | --- |
| Renderer `PerKeyLightingBrightnessSelector`, line 66399 | A `Brightness` slider with minimum 0, maximum 100 and step 5; it follows the color editor and precedes Breathing in the active custom-lighting column. |
| Renderer `PerKeyLightingState`, line 26685 | The class defaults brightness to 60 and stores a separate list of key states. This establishes the class default, not every device initialization or imported-profile value. |
| Renderer `PerKeyBrightness`, line 67761 | Finite values clamp to 0–100; fractional values survive. Non-finite and non-number inputs become zero. The helper does not perform slider-step quantization. |
| Renderer `setPerKeyBrightness`, line 68535 | Replaces preview state with `applyToKeyStates`; it updates all existing key colors' alpha while retaining unscaled RGB, identifiers and breathing flags. Saved source objects remain unchanged. |
| Renderer `onKeySelection`, line 68542 | New/replacement paint uses the chosen RGB, current custom brightness as alpha, and current Breathing brush flag. Other custom keys retain their data. |
| Renderer `KeyLightingState.toRecord`, line 26659; main state classes, lines 2586 and 2627 | Records retain raw RGBA, breathing and custom brightness. Renderer/main reconstruction preserves those values. |
| Main `GetLEDPerKeyDataBuffer$1`, line 46522 | Each color's `toArray_rgb()` supplies scaled channels. The five-byte LED entry retains static/breathing visibility 255/254 even when brightness is zero. |

Brightness zero paints black; it does not remove the override or expose the base effect. Increasing brightness restores the original color, including channels that rounded to zero at the lower brightness. Changing the setting on an empty custom state changes its saved brightness without creating keys. Reapplying the same brightness to an already normalized state produces no changed-state difference.

The helper's Eternal-specific brightness/speed nibble methods are outside this keyboard's path. They do not justify quantizing GMMK 2 brightness to ten device levels or changing the report format.

## Color conversion has byte-level rounding differences

The reference normalizes alpha by 100 and each channel by 255, multiplies the normalized values against a black background, multiplies the result by 255, and then applies JavaScript `Math.round`. The operation order matters: binary floating-point arithmetic can leave a mathematical half just below the rounding boundary.

The checker evaluates all 256 channel values at the 21 five-point slider positions, or **5,376 combinations**, against both renderer and main color classes. They agree throughout. **49 combinations** differ by one byte from integer round-half-up scaling. For example, channel 130 at 15% encodes as 19, while integer scaling would produce 20. The [fixture](../../assets/windows-custom-brightness-vectors.json) records each difference as brightness, source channel, Windows output and mathematical round-half-up output. These are characterization results, not a decision to correct the reference.

Ten RGB/brightness samples also check full report pages, including zero, fractional brightness, small channels and a rounding-boundary case. The latter prevents a future implementation from passing color-value checks while silently changing the device payload.

## Executable evidence and limits

Run from the repository root:

```sh
node app/scripts/check-windows-custom-brightness-reference.cjs /path/to/CORE-2.1.23/app
```

The checker passes **39 isolated tests**:

- Class defaults, selector parameters and eleven clamping cases.
- The active brightness callback, immutable state replacement, dim/restore behavior, records, empty states and individual paint.
- All 5,376 channel/slider-step combinations and the 49 characterized rounding differences.
- Twenty RGB/brightness/breathing cases, each comparing all four 264-byte reports: **80 complete page comparisons**, including W at LED index 42 and the 20 side LEDs at 120–139. These cases address profile 2, layer 3; other addressing combinations remain covered by the separate [custom-lighting checker](windows-ui-map.md#custom-lighting-group-and-report-checks).

The script hash-checks both bundles before evaluating only reviewed classes, callbacks and report builders. React hooks/JSX are stubbed, and native key/auxiliary lookup dependencies are supplied from the pinned target maps. Unlike the earlier custom-color report cases, the RGBA conversion and renderer/main record classes are real reference implementations, not color-conversion stubs. The test constructs the record round trip directly; it does not run IPC, React effects, application startup, storage I/O, native addons or HID. A Node VM with reviewed dependencies is not a general-purpose sandbox for arbitrary vendor code.

These tests establish source behavior for this artifact. They do not implement native brightness or establish Windows-runtime, native-window or keyboard acceptance.

## Native model decision remains open

The current native format places `LightingState` in each layer and stores only encoded RGB/visibility for each custom entry. It has no independent custom-brightness field or retained pre-scaling color. The existing `brightness` field belongs to the base effect and cannot be reused without coupling two separate Windows controls.

A faithful implementation needs a versioned model for profile-wide preset/custom lighting, raw custom RGB and independent custom brightness. The proposed compatibility policy is to preserve legacy RGB bytes as full-intensity source colors with brightness 100, retain the original import as a backup, and require a choice when legacy layers contain conflicting lighting. The reference class's 60% default must not silently dim imported native profiles. This migration policy requires user approval before changing import/export or device-save behavior; it is not implemented.
