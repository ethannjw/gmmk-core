'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

const referencePath = process.argv[2];
if (!referencePath) {
  throw new Error('Usage: node app/scripts/check-windows-performance-reference.cjs /path/to/CORE-2.1.23/out/main/index.js');
}
const bytes = readFileSync(referencePath);
assert.equal(createHash('sha256').update(bytes).digest('hex'),
  '96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531',
  'Reference hash mismatch: review the new source before executing any extracted helpers');
const source = bytes.toString('utf8');
const vectors = JSON.parse(readFileSync(join(__dirname, '../assets/windows-preset-lighting-vectors.json'), 'utf8'));

function range(text, startMarker, endMarker) {
  const start = text.indexOf(startMarker);
  assert.notEqual(start, -1, `Missing start marker: ${startMarker}`);
  assert.equal(text.indexOf(startMarker, start + startMarker.length), -1,
    `Ambiguous start marker: ${startMarker}`);
  const end = text.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, `Missing end marker: ${endMarker}`);
  return text.slice(start, end);
}

function classSource(name) {
  const start = source.indexOf(`class ${name} extends `);
  assert.notEqual(start, -1, `Missing class: ${name}`);
  const end = source.indexOf('\nclass ', start + 1);
  assert.notEqual(end, -1, `Missing class boundary: ${name}`);
  return source.slice(start, end);
}

const standby = range(classSource('KeyboardStandbyDeviceHandler'),
  'class KeyboardStandbyDeviceHandler extends KeyboardDeviceHandler {', '  static getIdleTimeState(device) {');
const performance = range(classSource('GmmkBaseDeviceHandler'),
  '  static async setPerformance(', '  /** GMMK 3 / GMMK 3 PRO');
const concrete = classSource('GMMKDeviceHandler');
const methods = [
  range(concrete, '  static getPropertiesSleepTimeSettings(', '  /**\n   * The sleep timer'),
  range(concrete, '  static sendReportToDevice(', '  static getReportFromDevice('),
  range(concrete, '  static async setPollingRate(', '  static async setPresetLightingEffect('),
].join('\n');
const builders = [
  range(source, 'function PreparePollingRateBuffer(', 'function PrepareProfileUpdateBuffer$1('),
  range(source, 'const LEDEffectBufferValue$2 = {', 'function PrepareLEDEffectBuffer$1('),
  range(source, 'const mapPollingRateToFirmwareRate =', 'const StartProcess_Windows ='),
].join('\n');

function harness(profileIndex = 0, activeLayer = 0, options = {}) {
  const reports = [];
  const delays = [];
  const saves = [];
  const profiles = Array.from({ length: 3 }, (_, index) => ({
    id: `profile-${index}`, order: index, activeLayerIndex: activeLayer,
    presetLightingState: { effectIdentifier: 'NormallyOn', brightness: 80 },
    keyboardPerformanceState: {
      pollingRate: 125, inputLatency: 16, osMode: 0,
      standbyDelay: 5, standbyEnabled: false, winLock: false, encoderSensitivity: 6,
    },
    layers: Array.from({ length: 3 }, () => ({ pollingRate: 125, inputLatency: 16 })),
  }));
  const device = {
    supportedDeviceData: { features: { sideLightSwitch: true } },
    communicationMethod: 'USB',
    communicationIds: { USB: options.missingHandle ? {} : { write: 'stub-config-interface' } },
    deviceProfiles: profiles,
    rendererState: { instanceId: 'isolated-gmmk2', currentProfileData: profiles[profileIndex] },
    toRecord() { return { profileId: this.rendererState.currentProfileData.id }; },
  };
  const context = vm.createContext({
    Buffer,
    REPORT_ID$3: 7,
    KeyboardDeviceHandler: class {},
    PresetLightingEffectIdentifier: Object.fromEntries(vectors.effects.map(effect => [effect.identifier, effect.identifier])),
    Device: { getGloriousDevice: instanceId => options.missingDevice || instanceId !== 'isolated-gmmk2' ? null : device },
    DataStorage: {
      saveDeviceInstance: record => saves.push(['instance', JSON.parse(JSON.stringify(record))]),
      saveDeviceProfile: record => saves.push(['profile', JSON.parse(JSON.stringify(record))]),
    },
    HID: { setFeatureReport: (handle, reportId, length, data) => {
      reports.push({ handle, reportId, length, data: Buffer.from(data) });
      return reports.length === 2 && options.failedWrite !== undefined ? options.failedWrite : length;
    } },
    Logger: { logError: () => {} },
    console: { debug: () => {}, error: () => {} },
    setTimeout: (callback, delay) => { delays.push(delay); callback(); },
  }, { codeGeneration: { strings: false, wasm: false } });
  vm.runInContext(`${builders}
${standby}}
class ReferenceHandler extends KeyboardStandbyDeviceHandler { ${performance}\n${methods} }
this.save = (state, requested) => ReferenceHandler.setPerformance(state, requested);`, context, { timeout: 1000 });
  return { device, reports, delays, saves, save: requested => context.save(device.rendererState, requested) };
}

function requested(pollingRate = 500, inputLatency = 8, osMode = 1) {
  return { pollingRate, inputLatency, osMode, standbyDelay: 99, standbyEnabled: true, winLock: true, encoderSensitivity: 10 };
}

function report(prefix) {
  const data = Buffer.alloc(264);
  data.set(prefix);
  return data;
}

for (let profileIndex = 0; profileIndex < 3; profileIndex++) {
  for (let activeLayer = 0; activeLayer < 3; activeLayer++) {
    for (const [pollingRate, wireRate] of [[1000, 0], [500, 1], [250, 2], [125, 3]]) {
      for (const latency of [2, 8, 16]) {
        test(`profile ${profileIndex + 1}, active layer ${activeLayer + 1}, ${pollingRate} Hz, ${latency} ms`, async () => {
          const state = harness(profileIndex, activeLayer);
          const before = JSON.parse(JSON.stringify(state.device.deviceProfiles));
          const result = await state.save(requested(pollingRate, latency));
          assert.equal(result, state.device.rendererState);
          const expected = [report([7, 8, 0, 0, 0, 0, 0, 0, wireRate, 1, 0])];
          for (let layer = 1; layer <= 3; layer++) {
            expected.push(report([7, 7, profileIndex + 1, layer, 7, 0, 1, 0, 16, 0, latency]));
          }
          assert.deepEqual(state.reports.map(entry => entry.data), expected);
          for (const entry of state.reports) {
            assert.deepEqual([entry.handle, entry.reportId, entry.length], ['stub-config-interface', 7, 264]);
          }
          assert.deepEqual(state.delays, [550, 550, 550, 550]);
          assert.equal(state.saves.length, 6);
          before[profileIndex].layers[activeLayer] = { pollingRate, inputLatency: latency };
          before[profileIndex].keyboardPerformanceState.pollingRate = pollingRate;
          before[profileIndex].keyboardPerformanceState.inputLatency = latency;
          assert.deepEqual(state.device.deviceProfiles, before);
        });
      }
    }
  }
}

test('Windows and macOS selections produce identical reports and retain the stored OS mode', async () => {
  const windows = harness();
  const mac = harness();
  await windows.save(requested(500, 8, 0));
  await mac.save(requested(500, 8, 1));
  assert.deepEqual(mac.reports, windows.reports);
  assert.deepEqual(mac.saves, windows.saves);
  assert.equal(mac.device.rendererState.currentProfileData.keyboardPerformanceState.osMode, 0);
});

test('the reference updates local performance even when the HID write handle is missing', async () => {
  const state = harness(0, 0, { missingHandle: true });
  await state.save(requested());
  assert.equal(state.reports.length, 0);
  assert.equal(state.delays.length, 0);
  assert.equal(state.saves.length, 6);
  assert.equal(state.device.rendererState.currentProfileData.keyboardPerformanceState.pollingRate, 500);
});

for (const failedWrite of [-1, 0, 263]) {
  test(`the reference continues and saves local state after a ${failedWrite}-byte result`, async () => {
    const state = harness(0, 0, { failedWrite });
    await state.save(requested());
    assert.equal(state.reports.length, 4);
    assert.equal(state.saves.length, 6);
    assert.equal(state.device.rendererState.currentProfileData.keyboardPerformanceState.inputLatency, 8);
  });
}

test('a missing device rejects before transport or storage', async () => {
  const state = harness(0, 0, { missingDevice: true });
  await assert.rejects(state.save(requested()), /Unable to find device from instance id/);
  assert.equal(state.reports.length, 0);
  assert.equal(state.saves.length, 0);
});
