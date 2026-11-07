import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
try {
  const HID = require("node-hid");
  console.log("node-hid loaded, devices:", HID.devices().length);
  process.exit(0);
} catch (e) {
  console.error("FAIL", e);
  process.exit(1);
}
