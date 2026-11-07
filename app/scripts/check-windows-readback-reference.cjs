'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

const root = process.argv[2];
assert.ok(root, 'Usage: node app/scripts/check-windows-readback-reference.cjs /path/to/CORE-2.1.23/app');
const source = readFileSync(join(root, 'out/main/index.js'));
assert.equal(createHash('sha256').update(source).digest('hex'),
  '96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531',
  'Review changed source before executing selected declarations');
const lines = source.toString('utf8').split('\n');
const declaration = (start, end) => lines.slice(start - 1, end).join('\n');

function harness(report = [7, 19, 37]) {
  const calls = [];
  const profiles = new Map();
  const state = vm.createContext({
    Buffer,
    console: { log() {}, debug() {}, error() {}, warn() {} },
    Logger: { logToCommandConsole() {}, logError: message => calls.push(['error', message]) },
    KeyboardDeviceHandler: class {},
    DeviceProfileState: class {},
    DataStorage: { findDeviceProfile: id => profiles.get(id) },
    generateId: () => 'generated-profile',
    REPORT_ID$3: 7,
    HidLib: {
      SetupDeviceDataCallback: (...args) => calls.push(['subscribe', ...args.slice(0, 4), typeof args[4]]),
      GetFeatureReport: (...args) => { calls.push(['read', ...args]); return report; },
    },
    setTimeout: (callback, delay) => { calls.push(['delay', delay]); callback(); },
    pollingSpy: (...args) => calls.push(['polling', args[1]]),
  }, { codeGeneration: { strings: false, wasm: false } });
  vm.runInContext(`${declaration(3999, 4098)}
    ${declaration(4883, 4958)}
    ${declaration(38513, 38576)}
    class GmmkBaseDeviceHandler extends KeyboardStandbyDeviceHandler {}
    ${declaration(60250, 60607)}
    GMMKDeviceHandler.setPollingRate = pollingSpy;
    this.api = { GloriousDevice, GMMKDeviceHandler };`, state, { timeout: 1000 });
  const device = Object.create(state.api.GloriousDevice.prototype);
  Object.assign(device, {
    deviceProfiles: [],
    rendererState: { instanceId: 'test-instance' },
    supportedDeviceData: { minimumProfilesCount: 1, features: {}, connectionMethods: {
      USB: { vid: '0x320F', pid: '0x504B', getData: { usagePage: '0xff00', usage: '0xff00' } },
    } },
    communicationMethod: 'USB', communicationIds: { USB: { write: 'test-handle' } },
  });
  return { ...state.api, device, calls, profiles };
}

function record(ids = []) {
  return { iterableProfileIds: ids, currentDeviceProfileId: '', instanceId: '', productId: 'GMMKV296ANSI' };
}

test('record hydration selects the locally stored profile without querying HID', () => {
  const { device, profiles, calls } = harness();
  const stored = { id: 'saved-profile', keyboardPerformanceState: { pollingRate: 500 } };
  profiles.set(stored.id, stored);
  device.applyRecordData(record([stored.id]), { applyProfileDefaults() { assert.fail('unexpected defaults'); } });
  assert.equal(device.rendererState.currentProfileData, stored);
  assert.deepEqual(calls, []);
});

test('missing local profiles are replaced with handler defaults, not a device download', () => {
  const { device, calls } = harness();
  device.applyRecordData(record(), { applyProfileDefaults(product, profile) {
    assert.equal(product, 'GMMKV296ANSI');
    profile.keyboardPerformanceState = { pollingRate: 1000 };
  } });
  assert.equal(device.rendererState.currentProfileData.keyboardPerformanceState.pollingRate, 1000);
  assert.equal(device.deviceProfiles.length, 1);
  assert.deepEqual(calls, []);
});

test('GMMK init registers the notification callback and dispatches saved polling rate', async () => {
  const { device, GMMKDeviceHandler, calls } = harness();
  device.rendererState.currentProfileData = { keyboardPerformanceState: { pollingRate: 500 } };
  await GMMKDeviceHandler.init(device, []);
  assert.deepEqual(calls, [['subscribe', 0xff00, 0xff00, 0x320f, 0x504b, 'function'], ['polling', 500]]);
});

test('explicit read helper returns raw report 7 bytes from the write interface', async () => {
  const { device, GMMKDeviceHandler, calls } = harness();
  const bytes = await GMMKDeviceHandler.getReportFromDevice(device);
  assert.deepEqual([...bytes], [7, 19, 37]);
  assert.deepEqual(calls, [['read', 'test-handle', 7, 65], ['delay', 150]]);
  assert.equal(device.rendererState.currentProfileData, undefined);
});

test('an empty native read becomes the original single-zero-byte sentinel', async () => {
  const { device, GMMKDeviceHandler } = harness([]);
  assert.deepEqual([...await GMMKDeviceHandler.getReportFromDevice(device)], [0]);
});

test('generic device-report methods are placeholders in this reference', () => {
  const { device, calls } = harness();
  assert.deepEqual(Object.keys(device.getDeviceReport()), []);
  assert.equal(device.applyDeviceReportData({ pollingRate: 125 }), undefined);
  assert.equal(device.rendererState.currentProfileData, undefined);
  assert.deepEqual(calls, []);
});
