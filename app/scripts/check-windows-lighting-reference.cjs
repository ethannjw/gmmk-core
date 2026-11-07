'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

const referencePath = process.argv[2];
if (!referencePath) {
  throw new Error('Usage: node app/scripts/check-windows-lighting-reference.cjs /path/to/CORE-2.1.23/out/main/index.js');
}
const sourceBytes = readFileSync(referencePath);
const expectedHash = '96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531';
assert.equal(createHash('sha256').update(sourceBytes).digest('hex'), expectedHash,
  'Reference hash mismatch: review the new source before executing any extracted helpers');
const source = sourceBytes.toString('utf8');
const vectors = JSON.parse(readFileSync(join(__dirname, '../assets/windows-preset-lighting-vectors.json'), 'utf8'));
assert.equal(vectors.main_sha256, expectedHash);
assert.equal(vectors.effects.length, 19);
assert.equal(vectors.samples.length, 8);
assert.equal(vectors.report_size, 264);

function declarationRange(startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, `Missing start marker: ${startMarker}`);
  assert.equal(source.indexOf(startMarker, start + startMarker.length), -1,
    `Ambiguous start marker: ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, `Missing end marker: ${endMarker}`);
  return source.slice(start, end);
}

const context = vm.createContext({
  PresetLightingEffectIdentifier: Object.fromEntries(vectors.effects.map(effect => [effect.identifier, effect.identifier])),
}, { codeGeneration: { strings: false, wasm: false } });
vm.runInContext(`${declarationRange('const LEDEffectBufferValue$2 = {', 'function PrepareLEDPerKeyBuffer$1(')}
this.reference = { indices: LEDEffectBufferValue$2, properties: PrepareLEDPropertiesBuffer$1, effect: PrepareLEDEffectBuffer$1 };`,
context, { timeout: 1000 });
const reference = context.reference;

function expectedReport(prefix, offset, wireId) {
  const report = Buffer.alloc(vectors.report_size);
  report.set(prefix);
  report[offset] = wireId;
  return report;
}

for (const effect of vectors.effects) {
  assert.equal(reference.indices[effect.identifier], effect.wire_id, effect.identifier);
  for (const sample of vectors.samples) {
    test(`${effect.identifier}: ${sample.name} matches both complete reference reports`, () => {
      const state = {
        effectIdentifier: effect.identifier,
        speed: sample.speed,
        brightness: sample.brightness,
        colors: [{ r: sample.rgb[0], g: sample.rgb[1], b: sample.rgb[2] }],
        multicolorMode: sample.multicolor,
      };
      const before = JSON.stringify(state);
      const propertiesReport = Buffer.alloc(vectors.report_size);
      const effectReport = Buffer.alloc(vectors.report_size);
      propertiesReport[0] = 7;
      effectReport[0] = 7;
      reference.properties(propertiesReport, sample.profile, sample.layer, state, true, sample.latency, undefined);
      reference.effect(effectReport, state);
      assert.deepEqual(propertiesReport, expectedReport(sample.properties_prefix, 4, effect.wire_id));
      assert.deepEqual(effectReport, expectedReport(sample.effect_prefix, 2, effect.wire_id));
      assert.equal(JSON.stringify(state), before);
    });
  }
}
