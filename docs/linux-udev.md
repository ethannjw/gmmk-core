# Linux udev — GMMK 2 96% US

Grant your user access to the target keyboard's HID interfaces without running the app as root. Configuration uses `0xFF01/0x01`; passive slot/layer notifications use `0xFF00/0xFF00`. The rule matches VID/PID and grants access to all HID interfaces of that target, rather than filtering by usage.

Create `/etc/udev/rules.d/99-gmmk-config.rules`:

```udev
# GMMK 2 Full Size 96% US — target HID interfaces
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="320f", ATTRS{idProduct}=="504b", MODE="0660", GROUP="plugdev", TAG+="uaccess"
```

Then reload rules and replug the keyboard:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Add your user to `plugdev` if your distribution uses that group instead of `uaccess`.

Bootloader PIDs (`0xb00b`, `0xb00c`) are intentionally omitted — this app does not configure bootloader mode.
