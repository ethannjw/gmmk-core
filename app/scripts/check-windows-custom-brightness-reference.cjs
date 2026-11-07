'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

const root = process.argv[2];
assert.ok(root, 'Usage: node app/scripts/check-windows-custom-brightness-reference.cjs /path/to/CORE-2.1.23/app');
const fixture = JSON.parse(readFileSync(join(__dirname, '../assets/windows-custom-brightness-vectors.json'), 'utf8'));
assert.equal(fixture.renderer_sha256, '9b8adecdf2957e81d2d6e674af5b6759c138565612d8cbadd67c8049d9c2f4ed');
assert.equal(fixture.main_sha256, '96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531');
function verifiedSource(relativePath, hash) {
  const bytes = readFileSync(join(root, relativePath));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), hash, 'Review changed reference source before execution');
  return bytes.toString('utf8');
}
function range(source, startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, startMarker);
  assert.equal(source.indexOf(startMarker, start + startMarker.length), -1, `Ambiguous ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, endMarker);
  return source.slice(start, end);
}
const renderer = verifiedSource('out/renderer-process/assets/index-401429d5.js', fixture.renderer_sha256);
const main = verifiedSource('out/main/index.js', fixture.main_sha256);
const customCallbacks = range(renderer, '  const setPerKeyBrightness =', '  const [previewZoneState,');
const product = 'GMMKV296ANSI';
const plain = value => JSON.parse(JSON.stringify(value));
const context = vm.createContext({
  Buffer,
  jsxRuntimeExports: { jsx: (type, props) => ({ type, props }) },
  useTranslate: () => () => {},
  SimpleSlider: 'SimpleSlider',
  reactExports: { useCallback: callback => callback },
  previewPerKeyState: null,
  setPreviewPerKeyState: state => { context.changedState = state; },
  deviceState: { currentProfileData: { perKeyLightingState: null } },
  isAdding: true,
  currentColor: '#0c2238',
  perKeyBreathing: false,
}, { codeGeneration: { strings: false, wasm: false } });
vm.runInContext([
  range(renderer, 'function toHex(color2) {', 'const BindingTypes_ButtonPress ='),
  range(renderer, 'class KeyLightingState {', 'class PresetLightingState {'),
  range(renderer, 'class PerKeyBrightness {', 'function PerZoneLightingBrightnessSelector('),
  range(renderer, 'function PerKeyLightingBrightnessSelector(', 'const header$t ='),
  range(renderer, '  const setPerKeyBrightness =', '  const [perKeyBreathing,'),
  range(customCallbacks, '  const onKeySelection =', '  const onQuickKeySelect ='),
  'this.Color = RGBAColor; this.KeyState = KeyLightingState; this.State = PerKeyLightingState;',
  'this.Brightness = PerKeyBrightness; this.renderSlider = PerKeyLightingBrightnessSelector;',
  'this.changeBrightness = setPerKeyBrightness; this.paintKey = onKeySelection;',
].join('\n'), context, { timeout: 1000 });
const mapsStart = main.indexOf('const KeyBufferIndexMaps =');
assert.notEqual(mapsStart, -1);
const mapStart = main.indexOf('  [DeviceProductIdentifier.GMMKV296ANSI]: {', mapsStart);
assert.notEqual(mapStart, -1);
const mapEnd = main.indexOf('\n  },', mapStart);
assert.notEqual(mapEnd, -1);
vm.runInContext(`this.keyIndices = ${main.slice(main.indexOf('{', mapStart), mapEnd + 4)};`, context, { timeout: 1000 });
const sideIndices = JSON.parse(range(main, 'const GMMK_V2_96_SIDE_LED = ', ';').split(' = ')[1]);
assert.deepEqual(sideIndices, fixture.led_indices.Sidelight);
assert.equal(context.keyIndices.KeyW, fixture.led_indices.KeyW[0]);
const backend = vm.createContext({
  Buffer,
  getKeyBufferIndex$1: (productId, identifier) => {
    assert.equal(productId, product);
    return context.keyIndices[identifier];
  },
  AuxLightIndexMaps: { [product]: { Sidelight: sideIndices } },
}, { codeGeneration: { strings: false, wasm: false } });
vm.runInContext([
  range(main, 'function toHex(color) {', 'var PresetLightingEffectIdentifier_Glorious ='),
  range(main, 'function PrepareLEDPerKeyBuffer$1(', 'const TargetBindingTypeByteOverride$2 ='),
  'this.encodePage = PrepareLEDPerKeyBuffer$1; this.State = PerKeyLightingState; this.Color = RGBAColor;',
].join('\n'), backend, { timeout: 1000 });

function stateWithColors(rgb, breathing = false) {
  const state = new context.State();
  state.keyLightingStates = ['KeyW', 'Sidelight'].map(identifier => {
    const key = new context.KeyState(identifier);
    key.color = context.Color.fromRGB(...rgb);
    key.breathing = breathing;
    return key;
  });
  return state;
}

test('custom lighting state class defaults to 60 percent independently of raw key colors', () => {
  const state = stateWithColors([12, 34, 56]);
  assert.equal(state.brightness, fixture.default_brightness);
  assert.deepEqual(plain(state.keyLightingStates[0].color.toArray_rgba()), [12, 34, 56, 100]);
});

test('custom slider exposes five-point steps, full bounds and its own callback', () => {
  const changes = [];
  const control = context.renderSlider({ brightness: 60, onChange: value => changes.push(value) });
  assert.equal(control.type, 'SimpleSlider');
  for (const [key, value] of Object.entries(fixture.slider)) assert.equal(control.props[key], value);
  assert.equal(control.props.value, 60);
  control.props.onChange(35);
  assert.deepEqual(changes, [35]);
});

for (const [label, input, expected] of [
  ['negative', -1, 0], ['zero', 0, 0], ['fractional', 37.5, 37.5], ['maximum', 100, 100],
  ['above maximum', 101, 100], ['NaN', NaN, 0], ['positive infinity', Infinity, 0],
  ['negative infinity', -Infinity, 0], ['numeric string', '60', 0], ['null', null, 0], ['missing', undefined, 0],
]) {
  test(`brightness clamps ${label} to ${expected}`, () => assert.equal(context.Brightness.clamp(input), expected));
}

test('brightness callback updates every placed key without mutating saved state or RGB', () => {
  const original = stateWithColors([1, 127, 255], true);
  original.keyLightingStates[1].color = context.Color.fromRGB(12, 34, 56, 5);
  const before = plain(original.toRecord());
  for (const key of original.keyLightingStates) { Object.freeze(key.color); Object.freeze(key); }
  Object.freeze(original.keyLightingStates);
  Object.freeze(original);
  context.previewPerKeyState = original;
  context.changeBrightness(25);
  const changed = context.changedState;
  assert.notEqual(changed, original);
  assert.equal(changed.brightness, 25);
  assert.deepEqual(plain(original.toRecord()), before);
  for (const [index, key] of changed.keyLightingStates.entries()) {
    assert.notEqual(key, original.keyLightingStates[index]);
    assert.notEqual(key.color, original.keyLightingStates[index].color);
    assert.deepEqual(plain(key.color.toArray_rgba()), [...before.keyLightingRecords[index].color.slice(0, 3), 25]);
    assert.equal(key.breathing, true);
  }
  assert.equal(changed.hasUpdates(original), true);
});

test('dimming to black and restoring brightness recovers the original unscaled RGB', () => {
  const original = stateWithColors([1, 127, 255]);
  const black = context.Brightness.applyToKeyStates(original, 0);
  assert.equal(black.keyLightingStates.length, 2);
  assert.deepEqual(plain(black.keyLightingStates[0].color.toArray_rgb()), [0, 0, 0]);
  const restored = context.Brightness.applyToKeyStates(black, 100);
  assert.deepEqual(plain(restored.keyLightingStates[0].color.toArray_rgb()), [1, 127, 255]);
  const repeated = context.Brightness.applyToKeyStates(restored, 100);
  assert.equal(repeated.hasUpdates(restored), false);
});

test('saved records retain raw RGB, alpha and the custom brightness setting', () => {
  const changed = context.Brightness.applyToKeyStates(stateWithColors([12, 34, 56], true), 37.5);
  const record = plain(changed.toRecord());
  assert.equal(record.brightness, 37.5);
  assert.deepEqual(record.keyLightingRecords[0].color, [12, 34, 56, 37.5]);
  const restored = context.State.fromRecord(record);
  assert.equal(restored.hasUpdates(changed), false);
  assert.deepEqual(plain(restored.keyLightingStates[0].color.toArray_rgb()), [5, 13, 21]);
  const saved = backend.State.fromRecord(record);
  assert.deepEqual(plain(saved.toRecord()), record);
  assert.deepEqual(plain(saved.keyLightingStates[0].color.toArray_rgb()), [5, 13, 21]);
});

test('brightness on an empty custom state changes the saved setting without adding keys', () => {
  const original = new context.State();
  const changed = context.Brightness.applyToKeyStates(original, 0);
  assert.equal(changed.hasUpdates(original), true);
  assert.equal(changed.keyLightingStates.length, 0);
  assert.equal(original.brightness, 60);
});

test('individual painting uses current custom brightness and preserves existing colors', () => {
  const original = context.Brightness.applyToKeyStates(stateWithColors([12, 34, 56]), 60);
  context.previewPerKeyState = original;
  context.currentColor = '#ff7f01';
  context.isAdding = true;
  context.perKeyBreathing = true;
  context.paintKey({ keyIdentifier: 'KeyW' });
  const changed = context.changedState;
  assert.equal(changed.keyLightingStates.length, 2);
  const painted = changed.keyLightingStates.find(key => key.keyIdentifier === 'KeyW');
  assert.deepEqual(plain(painted.color.toArray_rgba()), [255, 127, 1, 60]);
  assert.deepEqual(plain(painted.color.toArray_rgb()), [153, 76, 1]);
  assert.equal(painted.breathing, true);
  assert.deepEqual(plain(original.keyLightingStates[0].color.toArray_rgba()), [12, 34, 56, 60]);
  assert.deepEqual(plain(changed.keyLightingStates.find(key => key.keyIdentifier === 'Sidelight').toRecord()),
    plain(original.keyLightingStates[1].toRecord()));
});

test('all byte channels at slider steps retain characterized floating-point rounding differences', () => {
  const differences = [];
  for (let brightness = 0; brightness <= 100; brightness += 5) {
    for (let channel = 0; channel <= 255; channel++) {
      const actual = context.Color.fromRGB(channel, channel, channel, brightness).toArray_rgb()[0];
      assert.equal(backend.Color.fromRGB(channel, channel, channel, brightness).toArray_rgb()[0], actual);
      const mathematical = Math.floor((channel * brightness + 50) / 100);
      if (actual !== mathematical) differences.push([brightness, channel, actual, mathematical]);
    }
  }
  assert.deepEqual(differences, fixture.rounding_differences);
});

for (const sample of fixture.samples) {
  for (const breathing of [false, true]) {
    test(`actual RGBA encoding: ${sample.rgb}, brightness ${sample.brightness}, breathing ${breathing}`, () => {
      const state = context.Brightness.applyToKeyStates(stateWithColors(sample.rgb, breathing), sample.brightness);
      assert.deepEqual(plain(state.keyLightingStates[0].color.toArray_rgb()), sample.encoded_rgb);
      const saved = backend.State.fromRecord(plain(state.toRecord()));
      const data = Buffer.alloc(800);
      for (const index of Object.values(fixture.led_indices).flat()) {
        data.set([breathing ? 254 : 255, index, ...sample.encoded_rgb], index * 5);
      }
      for (let page = 0; page < 4; page++) {
        const report = Buffer.alloc(264);
        backend.encodePage(report, { supportedDeviceData: { productId: product } }, 1, 2, saved, page);
        const expected = Buffer.alloc(264);
        expected.set([7, 6, 2, 3, 83, page, 20]);
        expected.set(data.subarray(page * 200, (page + 1) * 200), 8);
        assert.deepEqual(report, expected);
      }
    });
  }
}
