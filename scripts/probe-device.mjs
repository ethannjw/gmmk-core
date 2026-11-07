#!/usr/bin/env node
import { scanDevices } from "../src/hid/transport.mjs";
import { buildPollingRatePacket } from "../src/protocol/gmmk-v2-96.mjs";
import { GmmkDeviceService } from "../src/device-service.mjs";

const scan = await scanDevices();
console.log(JSON.stringify(scan, null, 2));

if (!scan.configPath && !scan.swiftScan?.ok) {
  console.error("No config interface; is the keyboard connected?");
  process.exit(1);
}

const service = new GmmkDeviceService();
try {
  const info = await service.connect();
  console.log("Connected", info);
  await service.applyPollingRate(1000);
  console.log("Polling packet OK");
  const packet = buildPollingRatePacket(1000);
  console.log("Sample packet head:", [...packet.slice(0, 12)]);
} catch (err) {
  console.error("HID error:", err instanceof Error ? err.message : err);
  console.error(
    "Grant Input Monitoring to native/hid-gmmk and Electron, quit Glorious CORE, then retry.",
  );
  process.exit(2);
} finally {
  service.disconnect();
}
