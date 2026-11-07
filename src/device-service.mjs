import {
  GMMK2_96_US,
  buildChangeProfilePacket,
  buildLedEffectPacket,
  buildLedTypePacket,
  buildPollingRatePacket,
  LED_EFFECT_LABELS,
  POLLING_RATES_HZ,
} from "./protocol/gmmk-v2-96.mjs";
import { GmmkHidTransport, scanDevices } from "./hid/transport.mjs";

export class GmmkDeviceService {
  #transport = new GmmkHidTransport();
  #state = {
    connected: false,
    backend: null,
    profileIndex: 0,
    layerIndex: 0,
    pollingHz: 1000,
    effectUiValue: 0,
    brightness: 100,
    speed: 50,
    rgb: [255, 0, 0],
    sideLightOn: true,
  };

  async scan() {
    return scanDevices();
  }

  async connect() {
    const info = await this.#transport.connect();
    this.#state.connected = true;
    this.#state.backend = info.backend;
    return info;
  }

  disconnect() {
    this.#transport.close();
    this.#state.connected = false;
    this.#state.backend = null;
  }

  getState() {
    return { ...this.#state, device: GMMK2_96_US };
  }

  getEffectLabels() {
    return LED_EFFECT_LABELS;
  }

  getPollingRates() {
    return POLLING_RATES_HZ;
  }

  async applyPollingRate(hz) {
    const packet = buildPollingRatePacket(hz);
    await this.#transport.sendFeatureReport(packet);
    this.#state.pollingHz = hz;
    await sleep(100);
  }

  async applyLighting() {
    const typePacket = buildLedTypePacket({
      profileIndex: this.#state.profileIndex,
      layerIndex: this.#state.layerIndex,
      effectUiValue: this.#state.effectUiValue,
      brightness: this.#state.brightness,
      sideLightOn: this.#state.sideLightOn,
    });
    await this.#transport.sendFeatureReport(typePacket);
    await sleep(100);

    const effectPacket = buildLedEffectPacket({
      effectUiValue: this.#state.effectUiValue,
      speed: this.#state.speed,
      brightness: this.#state.brightness,
      rgb: this.#state.rgb,
    });
    await this.#transport.sendFeatureReport(effectPacket);
    await sleep(100);
  }

  async switchProfile(profileIndex, layerIndex = 0) {
    const packet = buildChangeProfilePacket(profileIndex, layerIndex);
    await this.#transport.sendFeatureReport(packet);
    this.#state.profileIndex = profileIndex;
    this.#state.layerIndex = layerIndex;
    await sleep(100);
    await this.applyLighting();
  }

  /**
   * @param {Partial<typeof this.#state>} patch
   */
  updateSettings(patch) {
    Object.assign(this.#state, patch);
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
