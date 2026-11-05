import HID from "node-hid";
import { GMMK2_96_US } from "../protocol/gmmk-v2-96.mjs";

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

  /**
   * @param {{ vid?: number, pid?: number }} [opts]
   */
  connect(opts = {}) {
    const vid = opts.vid ?? GMMK2_96_US.vid;
    const pid = opts.pid ?? GMMK2_96_US.pid;
    const iface = listMatchingInterfaces(vid, pid, GMMK2_96_US.setInterface)[0];
    if (!iface?.path) {
      throw new Error(
        `GMMK vendor interface not found (VID ${vid.toString(16)} PID ${pid.toString(16)} usage FF01:01). Is the keyboard plugged in?`,
      );
    }
    this.close();
    this.#path = iface.path;
    this.#handle = new HID.HID(iface.path);
    return {
      path: iface.path,
      product: iface.product,
      manufacturer: iface.manufacturer,
    };
  }

  close() {
    if (this.#handle) {
      try {
        this.#handle.close();
      } catch {
        /* ignore */
      }
      this.#handle = null;
      this.#path = null;
    }
  }

  get isOpen() {
    return this.#handle !== null;
  }

  get path() {
    return this.#path;
  }

  /**
   * @param {Buffer} report Buffer whose byte 0 is report ID (7)
   */
  sendFeatureReport(report) {
    if (!this.#handle) {
      throw new Error("Device not connected");
    }
    return this.#handle.sendFeatureReport(report);
  }

  /**
   * @param {number} [reportSize]
   * @returns {Buffer}
   */
  getFeatureReport(reportSize = GMMK2_96_US.reportSize) {
    if (!this.#handle) {
      throw new Error("Device not connected");
    }
    return this.#handle.getFeatureReport(GMMK2_96_US.reportId, reportSize);
  }
}

/**
 * @param {{ vid?: number, pid?: number }} [opts]
 */
export function scanDevices(opts = {}) {
  const vid = opts.vid ?? GMMK2_96_US.vid;
  const pid = opts.pid ?? GMMK2_96_US.pid;
  const all = HID.devices(vid, pid);
  const setIface = listMatchingInterfaces(vid, pid, GMMK2_96_US.setInterface)[0];
  const getIface = listMatchingInterfaces(vid, pid, GMMK2_96_US.getInterface)[0];
  return {
    vid,
    pid,
    interfaces: all.map((d) => ({
      path: d.path,
      usagePage: d.usagePage,
      usage: d.usage,
      interface: d.interface,
    })),
    configPath: setIface?.path ?? null,
    notifyPath: getIface?.path ?? null,
    product: setIface?.product ?? all[0]?.product ?? null,
  };
}
