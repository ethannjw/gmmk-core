import HID from "node-hid";
import { GMMK2_96_US } from "../protocol/gmmk-v2-96.mjs";
import { isSwiftBridgeAvailable, swiftBinaryPath, swiftRequest } from "./swift-bridge.mjs";

/**
 * @param {number} vid
 * @param {number} pid
 * @param {{ usagePage: number, usage: number }} usage
 */
export function listMatchingInterfaces(vid, pid, usage) {
  return HID.devices(vid, pid).filter(
    (d) => d.usagePage === usage.usagePage && d.usage === usage.usage,
  );
}

export class GmmkHidTransport {
  /** @type {HID.HID | null} */
  #handle = null;
  /** @type {string | null} */
  #path = null;
  /** @type {"swift" | "node-hid" | null} */
  #backend = null;
  #product = null;

  get backend() {
    return this.#backend;
  }

  /**
   * @param {{ vid?: number, pid?: number, preferSwift?: boolean }} [opts]
   */
  async connect(opts = {}) {
    const vid = opts.vid ?? GMMK2_96_US.vid;
    const pid = opts.pid ?? GMMK2_96_US.pid;
    const preferSwift = opts.preferSwift ?? true;

    this.close();

    if (preferSwift && isSwiftBridgeAvailable()) {
      const res = await swiftRequest({
        op: "scan",
        vid,
        pid,
        usagePage: GMMK2_96_US.setInterface.usagePage,
        usage: GMMK2_96_US.setInterface.usage,
      });
      if (!res.ok) {
        throw new Error(res.error ?? "Swift HID scan failed");
      }
      this.#backend = "swift";
      this.#path = swiftBinaryPath();
      this.#product = res.product ?? null;
      return {
        path: this.#path,
        product: this.#product,
        backend: this.#backend,
      };
    }

    const iface = listMatchingInterfaces(vid, pid, GMMK2_96_US.setInterface)[0];
    if (!iface?.path) {
      const hint = isSwiftBridgeAvailable()
        ? ""
        : " Run `npm run build:hid` for the macOS IOKit bridge.";
      throw new Error(
        `GMMK vendor interface not found (VID ${vid.toString(16)} PID ${pid.toString(16)} usage FF01:01). Is the keyboard plugged in?${hint}`,
      );
    }
    this.#path = iface.path;
    try {
      this.#handle = new HID.HID(iface.path);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      throw new Error(
        `${message}. Quit Glorious CORE and allow Input Monitoring for Electron (dev) or GMMK Core Mac (packaged app).`,
      );
    }
    this.#backend = "node-hid";
    this.#product = iface.product ?? null;
    return {
      path: iface.path,
      product: this.#product,
      backend: this.#backend,
    };
  }

  close() {
    if (this.#handle) {
      try {
        this.#handle.close();
      } catch {
        /* ignore */
      }
    }
    this.#handle = null;
    this.#path = null;
    this.#backend = null;
    this.#product = null;
  }

  get isOpen() {
    return this.#backend !== null;
  }

  get path() {
    return this.#path;
  }

  /**
   * @param {Buffer} report Buffer whose byte 0 is report ID (7)
   */
  async sendFeatureReport(report) {
    if (!this.isOpen) {
      throw new Error("Device not connected");
    }
    if (this.#backend === "swift") {
      const res = await swiftRequest({
        op: "setFeature",
        data: [...report],
        reportSize: GMMK2_96_US.reportSize,
      });
      if (!res.ok) {
        throw new Error(res.error ?? "setFeature failed");
      }
      return res.sent ?? report.length;
    }
    if (!this.#handle) {
      throw new Error("Device not connected");
    }
    try {
      return this.#handle.sendFeatureReport(report);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      throw new Error(
        `${message}. Try: npm run build:hid, quit Glorious CORE, grant Input Monitoring.`,
      );
    }
  }

  /**
   * @param {number} [reportSize]
   * @returns {Buffer}
   */
  getFeatureReport(reportSize = GMMK2_96_US.reportSize) {
    if (!this.#handle) {
      throw new Error("Get feature report requires node-hid backend");
    }
    return this.#handle.getFeatureReport(GMMK2_96_US.reportId, reportSize);
  }
}

/**
 * @param {{ vid?: number, pid?: number }} [opts]
 */
export async function scanDevices(opts = {}) {
  const vid = opts.vid ?? GMMK2_96_US.vid;
  const pid = opts.pid ?? GMMK2_96_US.pid;
  const all = HID.devices(vid, pid);
  const setIface = listMatchingInterfaces(vid, pid, GMMK2_96_US.setInterface)[0];
  const getIface = listMatchingInterfaces(vid, pid, GMMK2_96_US.getInterface)[0];

  let swiftScan = null;
  if (isSwiftBridgeAvailable()) {
    try {
      swiftScan = await swiftRequest({
        op: "scan",
        vid,
        pid,
        usagePage: GMMK2_96_US.setInterface.usagePage,
        usage: GMMK2_96_US.setInterface.usage,
      });
    } catch {
      swiftScan = null;
    }
  }

  return {
    vid,
    pid,
    swiftBridge: isSwiftBridgeAvailable(),
    swiftScan,
    interfaces: all.map((d) => ({
      path: d.path,
      usagePage: d.usagePage,
      usage: d.usage,
      interface: d.interface,
    })),
    configPath: setIface?.path ?? null,
    notifyPath: getIface?.path ?? null,
    product: swiftScan?.product ?? setIface?.product ?? all[0]?.product ?? null,
  };
}
