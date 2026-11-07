#!/usr/bin/env node
/**
 * Best-effort onboard read probe (macOS). Requires Input Monitoring for hid-gmmk.
 * Does not send undocumented writes — only profile select (subcmd 1) + GetFeatureReport.
 */
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const HID = path.join(root, "native/hid-gmmk");

function call(obj) {
  const out = execSync(`${HID} '${JSON.stringify(obj)}'`, { encoding: "utf8" });
  return JSON.parse(out.trim());
}

const base = {
  vid: 0x320f,
  pid: 0x504b,
  usagePage: 0xff01,
  usage: 0x01,
  reportSize: 264,
};

for (let profile = 1; profile <= 3; profile++) {
  for (let layer = 1; layer <= 3; layer++) {
    const pkt = Array(264).fill(0);
    pkt[0] = 7;
    pkt[1] = 1;
    pkt[2] = profile;
    pkt[3] = layer;
    const res = call({ ...base, op: "setThenGetFeature", data: pkt });
    const b = res.data ?? [];
    const nz = b.slice(8, 248).filter((x) => x !== 0).length;
    console.log(
      `P${profile} L${layer}: ok=${res.ok} sub=${b[1]} nonzero_payload=${nz} err=${res.error ?? ""}`,
    );
  }
}
