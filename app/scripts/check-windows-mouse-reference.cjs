'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { test } = require('node:test');
const vm = require('node:vm');

const [mainPath, rendererPath] = process.argv.slice(2);
if (!mainPath || !rendererPath) {
  throw new Error('Usage: node app/scripts/check-windows-mouse-reference.cjs /path/to/out/main/index.js /path/to/out/renderer-process/assets/index-401429d5.js');
}

function verifiedSource(filename, expectedHash) {
  const bytes = readFileSync(filename);
  assert.equal(createHash('sha256').update(bytes).digest('hex'), expectedHash,
    'Reference hash mismatch: review the new source before executing any extracted helpers');
  return bytes.toString('utf8');
}

function declarationRange(source, startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, `Missing start marker: ${startMarker}`);
  assert.equal(source.indexOf(startMarker, start + startMarker.length), -1,
    `Ambiguous start marker: ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, `Missing end marker: ${endMarker}`);
  return source.slice(start, end);
}

const main = verifiedSource(mainPath, '96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531');
const renderer = verifiedSource(rendererPath, '9b8adecdf2957e81d2d6e674af5b6759c138565612d8cbadd67c8049d9c2f4ed');
const mainDeclarations = [
  declarationRange(main, 'class TargetBindingState_MouseFunction {', 'class TargetBindingState_Macro {'),
  declarationRange(main, 'class KeyBindingState {', 'class TargetBindingState_SoundControl {'),
  declarationRange(main, 'class DeviceKeybindingState {', 'function getTargetBindingStateType(bindingType) {'),
  declarationRange(main, 'function getHIDValue(functionCode) {', 'const MacroStateToMacroEventSequence ='),
  declarationRange(main, 'function GetKeyAssignmentBuffer$1(', 'const RotaryEncoderHIDValues$1 ='),
].join('\n');
const rendererTarget = declarationRange(renderer, 'class TargetBindingState_MouseFunction {', 'class TargetBindingState_Macro {');
assert.equal(rendererTarget.trim(), declarationRange(main, 'class TargetBindingState_MouseFunction {', 'class TargetBindingState_Macro {').trim(),
  'Renderer and main mouse target definitions differ; trace their conversion before reusing the class');
const selectionSource = declarationRange(renderer, '  const handleMouseFunctionChange =', '  const onMultimediaOptionSelection =');
let selectedBindings;
const context = vm.createContext({
  Buffer,
  BindingType: { MouseFunction: 'MouseFunction' },
  TargetBindingState_Keystroke: class {},
  DeviceProductIdentifier: { GMMKNumpad: 'different-device' },
  KeybedComponentIdentifier: { RotaryEncoder: 'RotaryEncoder' },
  DefaultKeyBuffer$2: { 'isolated-gmmk2-test': Buffer.alloc(240, 0xa5) },
  isGMMK3Wired$1: () => false,
  getKeyBufferIndex$1: (_product, key) => { assert.equal(key, 'KeyA'); return 0; },
  reactExports: { useCallback: callback => callback },
  setSelectedMouseFunction: () => {},
  setMouseFunctiondpiShiftValue: () => {},
  setCurrentKeySelectionState: () => {},
  currentComponentSelection: 'KeyA',
  currentBindingEffect: 'MouseFunction',
  currentTargetType: 'MouseFunction',
  previewKeybindStates: [],
  onKeybindingStateUpdate: bindings => { selectedBindings = bindings; },
  onKeybindingChange: undefined,
}, { codeGeneration: { strings: false, wasm: false } });
vm.runInContext(`${mainDeclarations}
function getTargetBindingStateType(bindingType) {
  if (bindingType !== BindingType.MouseFunction) throw new Error('Unexpected binding type');
  return TargetBindingState_MouseFunction;
}
${selectionSource}
this.reference = {
  select: handleMouseFunctionChange,
  restore: DeviceKeybindingState.fromStateProperties,
  encode: GetKeyAssignmentBuffer$1,
};`, context, { timeout: 1000 });
const reference = context.reference;
const device = {
  supportedDeviceData: { productId: 'isolated-gmmk2-test' },
  deviceProfiles: [{ id: 'first' }, { id: 'second' }],
  rendererState: { currentProfileData: { id: 'second', activeLayerIndex: 2 } },
};

function reportFor(identifier) {
  reference.select(identifier);
  assert.equal(selectedBindings.length, 1);
  assert.equal(selectedBindings[0].target.functionIdentifier, identifier);
  const serialized = JSON.parse(JSON.stringify({ boundKeys: selectedBindings }));
  const restored = reference.restore(serialized);
  assert.equal(restored.boundKeys[0].target.functionIdentifier, identifier);
  return reference.encode(device, restored, 120);
}

const choices = [
  ['LeftClick', 'MOUSE_Fun_14', 1],
  ['RightClick', 'MOUSE_Fun_15', 2],
  ['MiddleClick', 'MOUSE_Fun_16', 3],
  ['Forward', 'MOUSE_Fun_17', 5],
  ['Back', 'MOUSE_Fun_18', 4],
  ['ScrollUp', 'MOUSE_Fun_19', 6],
  ['ScrollDown', 'MOUSE_Fun_20', 7],
];

for (const [identifier, legacyIdentifier, code] of choices) {
  test(`active ${identifier} survives serialization but produces mouse code zero`, () => {
    const report = reportFor(identifier);
    assert.equal(report[8], 9);
    assert.equal(report[128], 0);
  });

  test(`legacy ${legacyIdentifier} produces mouse code ${code}`, () => {
    const report = reportFor(legacyIdentifier);
    assert.equal(report[8], 9);
    assert.equal(report[128], code);
  });
}

test('the isolated encoder preserves framing, target and untouched default bytes', () => {
  const report = reportFor('LeftClick');
  assert.equal(report.length, 264);
  assert.deepEqual([...report.subarray(0, 8)], [7, 3, 2, 3, 0, 0, 0, 0]);
  for (let offset = 8; offset < 248; offset++) {
    if (offset !== 8 && offset !== 128) assert.equal(report[offset], 0xa5);
  }
  assert.ok(report.subarray(248).every(byte => byte === 0));
});

test('None also becomes type nine and code zero, not the native disabled-key encoding', () => {
  const report = reportFor('None');
  assert.deepEqual([report[8], report[128]], [9, 0]);
});
