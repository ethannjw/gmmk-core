#!/usr/bin/env node
import { scanDevices, GmmkHidTransport } from "../src/hid/transport.mjs";
import { buildPollingRatePacket } from "../src/protocol/gmmk-v2-96.mjs";

const scan = scanDevices();
console.log(JSON.stringify(scan, null, 2));

if (!scan.configPath) {
  console.error("No config interface; is the keyboard connected?");
  process.exit(1);
}

const transport = new GmmkHidTransport();
try {
  const info = transport.connect();
  console.log("Opened", info);
  const packet = buildPollingRatePacket(1000);
  const sent = transport.sendFeatureReport(packet);
  console.log("Polling packet bytes sent:", sent);
} catch (err) {
  console.error("HID error:", err instanceof Error ? err.message : err);
  console.error(
    "If you see IOHIDDeviceSetReport or 'not permitted', run from the built .app and grant Input Monitoring.",
  );
  process.exit(2);
} finally {
  transport.close();
}
