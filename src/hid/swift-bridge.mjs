import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const BIN_PATH = path.join(__dirname, "../../native/hid-gmmk");

export function swiftBinaryPath() {
  return BIN_PATH;
}

export function isSwiftBridgeAvailable() {
  try {
    return fs.existsSync(BIN_PATH) && fs.statSync(BIN_PATH).isFile();
  } catch {
    return false;
  }
}

/**
 * @param {object} request
 * @returns {Promise<object>}
 */
export function swiftRequest(request) {
  return new Promise((resolve, reject) => {
    if (!isSwiftBridgeAvailable()) {
      reject(new Error("Swift HID bridge not built. Run: npm run build:hid"));
      return;
    }

    const payload = JSON.stringify(request);
    const result = spawnSync(BIN_PATH, [payload], {
      encoding: "utf8",
      maxBuffer: 1024 * 1024,
    });

    const line = (result.stdout ?? "").trim().split("\n").filter(Boolean).pop();
    if (!line) {
      reject(
        new Error(
          result.stderr?.trim() ||
            `hid-gmmk failed (code ${result.status ?? "?"})`,
        ),
      );
      return;
    }

    try {
      const parsed = JSON.parse(line);
      if (result.status !== 0 && parsed.ok !== true) {
        reject(new Error(parsed.error ?? line));
        return;
      }
      resolve(parsed);
    } catch {
      reject(new Error(`Invalid bridge output: ${line}`));
    }
  });
}
