# Release idle resource use (measured)

## Method

1. Build release binary: `cd app && cargo build --release`
2. Launch binary in background (no keyboard connected, no profile imported).
3. Wait **8 seconds** for window/init to settle.
4. Sample once with macOS `ps`:

```bash
BIN=app/target/release/gmmk-config
"$BIN" & PID=$!
sleep 8
ps -p "$PID" -o pid,rss,vsz,%cpu,command
kill "$PID"
```

`RSS` is resident set size in **kilobytes** (macOS `ps` convention). `%CPU` is instantaneous CPU use at sample time.

## Results (this repo, local machine)

| Platform | Date | Binary size | RSS | VSZ | %CPU (idle) |
|----------|------|-------------|-----|-----|-------------|
| macOS **aarch64** (Apple Silicon), darwin 25.x | 2026-11-05 | **4.0 MiB** stripped | **~111 MiB** (113552 KB) | 435649440 KB | **0.0** |

Notes:

- VSZ is not physical RAM; egui/winit map large virtual arenas.
- No background thread work when idle (HID worker blocked on channel only).
- Linux release build was not executed in this session; re-run the same `ps` pattern on your distro after `cargo build --release`.

Compare for context: Electron + Chromium idle for the legacy prototype is typically **150–400+ MiB RSS** on the same class of machine (not re-measured in this session).
