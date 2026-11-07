'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { test } = require('node:test');
const vm = require('node:vm');

const referencePath = process.argv[2];
if (!referencePath) {
  throw new Error('Usage: node app/scripts/check-windows-macro-reference.cjs /path/to/CORE-2.1.23/out/main/index.js');
}
const sourceBytes = readFileSync(referencePath);
const expectedHash = '96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531';
assert.equal(createHash('sha256').update(sourceBytes).digest('hex'), expectedHash,
  'Reference hash mismatch: review the new source before executing any extracted helpers');
const source = sourceBytes.toString('utf8');

function declarationRange(startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, `Missing start marker: ${startMarker}`);
  assert.equal(source.indexOf(startMarker, start + startMarker.length), -1,
    `Ambiguous start marker: ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, `Missing end marker: ${endMarker}`);
  return source.slice(start, end);
}

const helperSource = [
  declarationRange('function getHIDValue(functionCode) {', 'const GatherMacrosToSendToDevice ='),
  declarationRange('const convertMacroEventsToBuffer =', 'const REPORT_ID$7 = 7;'),
].join('\n');
const modes = {
  NoRepeat: 'NoRepeat',
  RepeatWhileHolding: 'RepeatWhileHolding',
  Toggle: 'Toggle',
};
const macros = [];
const context = vm.createContext({
  Buffer,
  MacroMode: modes,
  BindingType: { Macro: 'Macro' },
  DataStorage: { getMacros: () => macros },
  getKeyBufferIndex$1: (_product, key) => key,
}, { codeGeneration: { strings: false, wasm: false } });
vm.runInContext(`${helperSource}\nthis.reference = {
  events: MacroStateToMacroEventSequence,
  encodeEvents: convertMacroEventsToBuffer,
  payload: prepareMacroDataBuffer,
  macro: PrepareMacroBuffer,
  modes: PrepareMacroTypeBuffer,
};`, context, { timeout: 1000 });
const reference = context.reference;

function macro(actions, value = 7) {
  return { value, schedule: { entries: [{ actions }] } };
}

function event(type, startTime, inputName = 'KeyA') {
  return { type, startTime, inputName };
}

function modeReport(mode, key = 0) {
  macros.splice(0, macros.length, { id: 'test-macro', mode });
  const device = {
    supportedDeviceData: { productId: 'isolated-offset-test' },
    rendererState: { currentProfileData: { order: 1, activeLayerIndex: 2 } },
  };
  const bindings = {
    boundKeys: [{ componentIdentifier: key, target: { bindingType: 'Macro', macroId: 'test-macro' } }],
  };
  return reference.modes(device, bindings);
}

test('schedule actions become chronologically ordered press and release events', () => {
  const result = reference.events(macro([
    { inputName: 'KeyA', startTime: 0, endTime: 20 },
    { inputName: 'KeyB', startTime: 10, endTime: 30 },
  ]));
  assert.deepEqual(Array.from(result, entry => [entry.type, entry.inputName, entry.startTime]), [
    ['press', 'KeyA', 0], ['press', 'KeyB', 10], ['release', 'KeyA', 20], ['release', 'KeyB', 30],
  ]);
});

test('equal-time events retain schedule order and encode a minimum one-millisecond delay', () => {
  const events = reference.events(macro([
    { inputName: 'KeyA', startTime: 0, endTime: 10 },
    { inputName: 'KeyB', startTime: 10, endTime: 20 },
  ]));
  assert.deepEqual(Array.from(events, entry => entry.type), ['press', 'release', 'press', 'release']);
  assert.deepEqual([...reference.encodeEvents(events).subarray(3, 6)], [0, 1, 4]);
});

test('initial absolute time is discarded and the final event receives a one-millisecond delay', () => {
  const report = reference.macro(macro([{ inputName: 'KeyA', startTime: 1000, endTime: 1010 }]));
  assert.deepEqual([...report.subarray(8, 14)], [128, 10, 4, 0, 1, 4]);
});

test('payload header contains command five and macro value but no profile or layer selector', () => {
  const report = reference.macro(macro([{ inputName: 'KeyA', startTime: 0, endTime: 10 }]));
  assert.equal(report.length, 264);
  assert.deepEqual([...report.subarray(0, 8)], [7, 5, 7, 0, 0, 0, 0, 0]);
  assert.ok(report.subarray(14).every(byte => byte === 0));
});

test('a macro value of 256 silently wraps to zero in the payload header', () => {
  const report = reference.macro(macro([{ inputName: 'KeyA', startTime: 0, endTime: 10 }], 256));
  assert.equal(report[2], 0);
});

test('82 events fit as complete three-byte entries', () => {
  const events = Array.from({ length: 82 }, (_, index) => event(index % 2 ? 'release' : 'press', index * 10));
  const encoded = reference.encodeEvents(events);
  const report = reference.payload(7, encoded);
  assert.deepEqual(report.subarray(8, 254), encoded.subarray(0, 246));
  assert.deepEqual([...report.subarray(251, 254)], [0, 1, 4]);
  assert.ok(report.subarray(254).every(byte => byte === 0));
});

test('84 events truncate after the first two bytes of event 83', () => {
  const events = Array.from({ length: 84 }, (_, index) => event(index % 2 ? 'release' : 'press', index * 10));
  const report = reference.payload(7, reference.encodeEvents(events));
  assert.deepEqual([...report.subarray(254, 257)], [128, 10, 0]);
  assert.ok(report.subarray(256).every(byte => byte === 0));
});

test('a 32767-millisecond release delay leaves the press flag clear', () => {
  const encoded = reference.encodeEvents([event('release', 0), event('press', 32767)]);
  assert.deepEqual([...encoded.subarray(0, 3)], [127, 255, 4]);
});

test('a 32768-millisecond release delay overlaps the press flag', () => {
  const encoded = reference.encodeEvents([event('release', 0), event('press', 32768)]);
  assert.deepEqual([...encoded.subarray(0, 3)], [128, 0, 4]);
});

test('an unknown input name silently maps to key code zero', () => {
  const encoded = reference.encodeEvents([event('press', 0, 'unmapped-test-key')]);
  assert.deepEqual([...encoded.subarray(0, 3)], [128, 1, 0]);
});

test('NoRepeat uses a big-endian one at the selected key offset', () => {
  const report = modeReport(modes.NoRepeat, 119);
  assert.equal(report.length, 264);
  assert.deepEqual([...report.subarray(0, 8)], [7, 4, 2, 3, 0, 0, 0, 0]);
  assert.deepEqual([...report.subarray(246, 248)], [0, 1]);
  assert.ok(report.subarray(8, 246).every(byte => byte === 0));
  assert.ok(report.subarray(248).every(byte => byte === 0));
});

test('Toggle uses zero in the mode table', () => {
  assert.ok(modeReport(modes.Toggle).subarray(8).every(byte => byte === 0));
});

test('RepeatWhileHolding throws when 65535 reaches signed writeInt16BE', () => {
  assert.throws(() => modeReport(modes.RepeatWhileHolding), { code: 'ERR_OUT_OF_RANGE' });
});
