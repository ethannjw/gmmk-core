# GMMK Core Mac

**GMMK Core Mac** is a clean-room macOS controller for the **Glorious GMMK 2 Full Size 96%** (USB `0x320F:0x504B`), reverse-engineered from Glorious CORE **2.1.3** behavior on Windows and macOS. It targets the same vendor HID feature protocol (report ID `7`, usage page `0xFF01`) so lighting, profiles, and polling rate match what CORE does on PC.

The official [Glorious CORE](https://www.gloriousgaming.com/pages/software) Mac build lists support through **macOS Sonoma** only. On newer macOS releases the stock app often fails to talk to the keyboard; this project is an open, maintainable alternative.

## Your keyboard

Detected on this machine as:

- **Name:** GMMK 2 Full Size 96%
- **VID:PID:** `0x320F:0x504B` (US 96% layout)

## Quick start

```bash
cd ~/myapps/gmmk-core-mac
npm install
npm start
```

CLI probe (same HID stack as the app):

```bash
npm run probe
```

Build a signed-ish local `.app` (for proper TCC prompts):

```bash
npm run dist
```

## Permissions (required)

macOS blocks low-level HID unless the **app binary** is trusted:

1. Quit **Glorious CORE** (only one app can hold the vendor interface).
2. System Settings → **Privacy & Security** → **Input Monitoring** → enable **GMMK Core Mac** (or Electron during dev).
3. If writes still fail, also allow **Accessibility** for the same binary.

Development mode (`npm start`) uses Electron’s binary; grant permissions to **Electron** in those panes, or use `npm run dist` and grant them to **GMMK Core Mac**.

## Feature parity (v0.1)

| Glorious CORE (Windows) | GMMK Core Mac |
|-------------------------|---------------|
| Device detect GMMK 2 96% | Yes |
| 3 onboard profiles + layers | Profile switch (1–3) + layer select |
| RGB presets / brightness / speed | Yes (GmmkSeries packet layout) |
| Polling 1000/500/250/125 Hz | Yes |
| Per-key remap + macros | Not yet |
| Firmware update | Not yet |
| Glorious account / cloud profiles | No (local only) |

Protocol details: [docs/PROTOCOL.md](docs/PROTOCOL.md).

## Reverse engineering method

Analysis used the installed **Glorious CORE 2.1.3** Electron bundle (`app.asar`): `GmmkSeries` in the main process, plus `SupportDevice.DB` entries for `0x504B` / `0x505A`. No Glorious assets or native binaries are shipped in this repo.

The GitHub link you shared ([morluto/rea](https://github.com/morluto/rea)) is the **REA** reverse-engineering MCP toolkit, not keyboard software. REA is useful for deeper binary work; this repo already encodes the GMMK 2 protocol recovered from CORE’s JavaScript.

## Legal

For personal use with hardware you own. Glorious CORE, logos, and UI are trademarks of Glorious, LLC. This project is independent and MIT-licensed.

## Dependencies

- [node-hid](https://github.com/node-hid/node-hid) (HIDAPI)
- Homebrew **hidapi** may be required for Python-side experiments: `brew install hidapi`
