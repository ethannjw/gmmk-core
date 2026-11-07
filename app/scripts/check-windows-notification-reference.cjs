'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { test } = require('node:test');
const vm = require('node:vm');

const referencePath = process.argv[2];
if (!referencePath) {
  throw new Error('Usage: node app/scripts/check-windows-notification-reference.cjs /path/to/CORE-2.1.23/out/main/index.js');
}
const sourceBytes = readFileSync(referencePath);
assert.equal(createHash('sha256').update(sourceBytes).digest('hex'),
  '96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531',
  'Reference hash mismatch: review the new source before executing any extracted helpers');
const source = sourceBytes.toString('utf8');
const classMarker = 'class GMMKDeviceHandler extends GmmkBaseDeviceHandler {';
const classStart = source.indexOf(classMarker);
assert.notEqual(classStart, -1);
assert.equal(source.indexOf(classMarker, classStart + classMarker.length), -1);
const handlerStart = source.indexOf('  static async handleDeviceMessage(', classStart);
const handlerEnd = source.indexOf('  /** Saved standby settings to put in a Properties frame', handlerStart);
assert.ok(handlerStart > classStart && handlerEnd > handlerStart);
const handlerSource = source.slice(handlerStart, handlerEnd);

function harness() {
  const emitted = [];
  const saved = [];
  const flags = { assigningActiveProfile: false, resettingDevice: false };
  const context = vm.createContext({
    console: { log: () => {}, warn: () => {} },
    GmmkBaseDeviceHandler: flags,
    DataStorage: {
      saveDeviceInstance: record => saved.push(['instance', record]),
      saveDeviceProfile: record => saved.push(['profile', record]),
    },
    EventManager: { emit: (...args) => emitted.push(args) },
    AppChannel: { SendToWindow: 'SendToWindow' },
    DeviceChannel: { PropertyUpdate: 'PropertyUpdate' },
  }, { codeGeneration: { strings: false, wasm: false } });
  vm.runInContext(`class ReferenceHandler { ${handlerSource} }
this.handle = (device, data) => ReferenceHandler.handleDeviceMessage(device, data, { deviceID: 'isolated-test' });`,
  context, { timeout: 1000 });
  const profiles = Array.from({ length: 3 }, (_, order) => ({
    order,
    id: `local-${order}`,
    activeLayerIndex: 0,
    presetLightingState: { effectIdentifier: 'Wave1' },
    toRecord() { return { id: this.id, activeLayerIndex: this.activeLayerIndex }; },
  }));
  const device = {
    deviceProfiles: profiles,
    rendererState: { currentProfileData: profiles[0] },
    toRecord() { return { currentProfileId: this.rendererState.currentProfileData.id }; },
  };
  return { handle: data => context.handle(device, data), device, emitted, saved, flags };
}

for (let profile = 1; profile <= 3; profile++) {
  for (let layer = 1; layer <= 3; layer++) {
    test(`four-byte notification selects stored slot ${profile}, layer ${layer}`, async () => {
      const state = harness();
      await state.handle(Buffer.from([4, 241, profile, layer]));
      assert.equal(state.device.rendererState.currentProfileData, state.device.deviceProfiles[profile - 1]);
      assert.equal(state.device.rendererState.currentProfileData.activeLayerIndex, layer - 1);
      assert.deepEqual(state.device.rendererState.currentProfileData.presetLightingState, { effectIdentifier: 'Wave1' });
      assert.equal(state.saved.length, 2);
      assert.deepEqual(state.saved[0], ['instance', { currentProfileId: `local-${profile - 1}` }]);
      assert.equal(state.saved[1][0], 'profile');
      assert.equal(state.saved[1][1], state.device.rendererState.currentProfileData);
      assert.equal(state.emitted.length, 1);
      assert.deepEqual(state.emitted[0].slice(0, 2), ['SendToWindow', 'PropertyUpdate']);
      assert.equal(state.emitted[0][2], state.device.rendererState);
    });
  }
}

test('the trailing byte never replaces stored lighting with a hardware effect', async () => {
  const state = harness();
  for (let trailing = 0; trailing <= 255; trailing++) {
    await state.handle(Buffer.from([4, 241, 2, 3, trailing]));
    assert.equal(state.device.rendererState.currentProfileData.activeLayerIndex, 2);
    assert.deepEqual(state.device.rendererState.currentProfileData.presetLightingState, { effectIdentifier: 'Wave1' });
  }
});

for (const flag of ['assigningActiveProfile', 'resettingDevice']) {
  test(`the reference ignores profile events while ${flag}`, async () => {
    const state = harness();
    state.flags[flag] = true;
    await state.handle(Buffer.from([4, 241, 2, 3]));
    assert.equal(state.device.rendererState.currentProfileData, state.device.deviceProfiles[0]);
    assert.equal(state.saved.length, 0);
    assert.equal(state.emitted.length, 0);
  });
}

test('a different report header cannot update a profile', async () => {
  const state = harness();
  await state.handle(Buffer.from([7, 241, 2, 3]));
  assert.equal(state.device.rendererState.currentProfileData, state.device.deviceProfiles[0]);
  assert.equal(state.saved.length, 0);
  assert.equal(state.emitted.length, 0);
});
