/** @typedef {{ usagePage: number, usage: number }} HidUsage */

export const GMMK2_96_US = {
  name: "GMMK 2 Full Size 96%",
  vid: 0x320f,
  pid: 0x504b,
  pidBootloader: 0xb00b,
  serialKey: "0x320F0x504B",
  reportId: 7,
  reportSize: 264,
  keyMatrixLength: 120,
  keyColumnLength: 20,
  /** @type {HidUsage} */
  setInterface: { usagePage: 0xff01, usage: 0x01 },
  /** @type {HidUsage} */
  getInterface: { usagePage: 0xff00, usage: 0xff00 },
};

export const POLLING_RATES_HZ = [1000, 500, 250, 125];

/** UI / profile effect value → firmware effect index (GmmkSeries arrLEDType) */
export const LED_EFFECT_IDS = [
  8, 0, 1, 3, 4, 5, 2, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
];

export const LED_EFFECT_LABELS = [
  "Off",
  "Glorious Mode",
  "Wave #1",
  "Wave #2",
  "Spiraling Wave",
  "Acid Mode",
  "Breathing",
  "Normally On",
  "Ripple Graff",
  "Pass Without Trace",
  "Fast Run Without Trace",
  "Matrix 2",
  "Matrix 3",
  "Rainbow",
  "Heartbeat Sensor",
  "Digit Times",
  "Kamehameha",
  "Pingpong",
  "Surmount",
];

/**
 * @param {number} size
 * @returns {Buffer}
 */
function allocReport(size) {
  const buf = Buffer.alloc(size, 0);
  buf[0] = GMMK2_96_US.reportId;
  return buf;
}

/**
 * @param {number} rateHz
 * @param {{ ep2Enable?: number, ledNoChange?: number }} opts
 */
export function buildPollingRatePacket(rateHz, opts = {}) {
  const { ep2Enable = 1, ledNoChange = 1 } = opts;
  const idx = POLLING_RATES_HZ.indexOf(rateHz);
  if (idx < 0) {
    throw new Error(`Unsupported polling rate: ${rateHz}`);
  }
  const buf = allocReport(GMMK2_96_US.reportSize);
  buf[1] = 8;
  buf[8] = idx;
  buf[9] = ep2Enable;
  buf[10] = ledNoChange;
  return buf;
}

/**
 * @param {object} params
 * @param {number} params.profileIndex 0-based
 * @param {number} params.layerIndex 0-based
 * @param {number} params.effectUiValue UI effect id (matches LED_EFFECT_IDS source index)
 * @param {number} params.brightness 0-100
 * @param {boolean} params.sideLightOn
 * @param {number} [params.inputLatencyMs]
 */
export function buildLedTypePacket({
  profileIndex,
  layerIndex,
  effectUiValue,
  brightness,
  sideLightOn,
  inputLatencyMs = 16,
}) {
  let effectIndex = LED_EFFECT_IDS.indexOf(effectUiValue);
  if (effectIndex < 0) {
    effectIndex = 0;
  }
  const buf = allocReport(GMMK2_96_US.reportSize);
  buf[1] = 7;
  buf[2] = profileIndex + 1;
  buf[3] = layerIndex + 1;
  buf[4] = effectIndex;
  buf[5] = 0;
  buf[6] = 1;
  buf[7] = sideLightOn ? 0 : 1;
  buf[8] = Math.round((brightness * 20) / 100);
  buf[10] = inputLatencyMs;
  return buf;
}

/**
 * @param {object} params
 * @param {number} params.effectUiValue
 * @param {number} params.speed 0-100
 * @param {number} params.brightness 0-100
 * @param {[number, number, number]} params.rgb
 * @param {boolean} [params.multicolor]
 */
export function buildLedEffectPacket({
  effectUiValue,
  speed,
  brightness,
  rgb,
  multicolor = false,
}) {
  let effectIndex = LED_EFFECT_IDS.indexOf(effectUiValue);
  if (effectIndex < 0) {
    effectIndex = 0;
  }
  const buf = allocReport(GMMK2_96_US.reportSize);
  buf[1] = 2;
  buf[2] = effectIndex;
  const scaledSpeed = speed >= 5 ? Math.ceil(speed / 5) : speed > 0 ? 1 : 0;
  buf[8] = scaledSpeed;
  buf[9] = Math.round((brightness * 20) / 100);
  if (multicolor) {
    buf[10] = 1;
  }
  if (effectIndex === 13) {
    buf[12] = 1;
  }
  buf[15] = rgb[0];
  buf[16] = rgb[1];
  buf[17] = rgb[2];
  return buf;
}

/**
 * @param {number} profileIndex 0-based
 * @param {number} layerIndex 0-based
 */
export function buildChangeProfilePacket(profileIndex, layerIndex) {
  const buf = allocReport(GMMK2_96_US.reportSize);
  buf[1] = 1;
  buf[2] = profileIndex + 1;
  buf[3] = layerIndex + 1;
  return buf;
}
