'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { test } = require('node:test');
const vm = require('node:vm');

const referencePath = process.argv[2];
if (!referencePath) {
  throw new Error('Usage: node app/scripts/check-windows-slider-reference.cjs /path/to/CORE-2.1.23/out/renderer-process/assets/index-401429d5.js');
}
const bytes = readFileSync(referencePath);
assert.equal(createHash('sha256').update(bytes).digest('hex'),
  '9b8adecdf2957e81d2d6e674af5b6759c138565612d8cbadd67c8049d9c2f4ed',
  'Reference hash mismatch: review the new source before executing any extracted helpers');
const source = bytes.toString('utf8');

function declarationRange(startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, `Missing start marker: ${startMarker}`);
  assert.equal(source.indexOf(startMarker, start + startMarker.length), -1,
    `Ambiguous start marker: ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, `Missing end marker: ${endMarker}`);
  return source.slice(start, end);
}

const declarations = [
  declarationRange('const STEP_THRESHOLD = 1;', 'const slider$9 ='),
  declarationRange('function PresetLightingBrightnessSelector(', 'const presetLightingModeSelector ='),
].join('\n');

function harness(selector, initialValue = 43) {
  const changed = [];
  const localValues = [];
  const timers = [];
  const element = (type, props) => ({ type, props });
  const context = vm.createContext({
    reactExports: {
      useRef: current => ({ current }),
      useState: initial => [initial, value => localValues.push(value)],
      useEffect: () => {},
    },
    jsxRuntimeExports: { jsx: element, jsxs: element },
    useTranslate: () => text => text,
    classNames$1: () => '',
    style$1P: {},
    style$1O: {},
    console: { log: () => {} },
    setTimeout: callback => timers.push(callback),
  }, { codeGeneration: { strings: false, wasm: false } });
  vm.runInContext(`${declarations}
this.selectors = { brightness: PresetLightingBrightnessSelector, speed: PresetLightingRateSelector };
this.round = calculateSteppedValue;`, context, { timeout: 1000 });
  const selected = context.selectors[selector]({
    baseEffectState: { brightness: initialValue, speed: initialValue },
    onChange: value => changed.push(value),
  });
  const inputs = [];
  function collect(node) {
    if (Array.isArray(node)) { node.forEach(collect); return; }
    if (!node || typeof node !== 'object') return;
    if (node.type === 'input') inputs.push(node.props);
    collect(node.props?.children);
  }
  collect(selected.type(selected.props));
  assert.equal(inputs.length, 2);
  const range = inputs.find(input => input.type === 'range');
  const text = inputs.find(input => input.type === 'text');
  assert.ok(range && text);
  return { props: selected.props, range, text, changed, localValues, timers, round: context.round };
}

for (const [selector, minimum] of [['brightness', 0], ['speed', 1]]) {
  test(`${selector}: bounds, five-point step and unchanged imported value`, () => {
    const state = harness(selector);
    assert.deepEqual([state.props.min, state.props.max, state.props.step], [minimum, 100, 5]);
    assert.deepEqual([state.range.min, state.range.max, state.range.step], [minimum, 100, 1]);
    assert.equal(state.range.value, 43);
    assert.equal(state.text.value, '43');
    assert.deepEqual(state.changed, []);
    assert.deepEqual(state.localValues, []);
  });

  test(`${selector}: absolute five-point rounding, ties and clamping`, () => {
    const state = harness(selector);
    const cases = [[-10, minimum], [0, minimum], [1, minimum], [2.49, minimum], [2.5, 5],
      [5, 5], [7.49, 5], [7.5, 10], [43, 45], [67, 65], [97.49, 95], [97.5, 100], [120, 100]];
    for (const [input, expected] of cases) {
      assert.equal(state.round(input, 5, minimum, 100), expected, `input ${input}`);
    }
  });

  test(`${selector}: text edits stay local until blur`, () => {
    const state = harness(selector);
    state.text.onChange({ target: { value: '67' } });
    assert.deepEqual(state.localValues, [67]);
    assert.deepEqual(state.changed, []);
    state.text.onBlur({ target: { value: '67' } });
    assert.deepEqual(state.changed, [65]);
    assert.deepEqual(state.localValues, [67, 65]);
  });

  test(`${selector}: blank text resets to the minimum on blur`, () => {
    const state = harness(selector);
    state.text.onChange({ target: { value: '' } });
    assert.deepEqual(state.localValues, ['']);
    assert.deepEqual(state.changed, []);
    state.text.onBlur({ target: { value: '' } });
    assert.deepEqual(state.changed, [minimum]);
  });
}

test('range changes are throttled; mouse-up updates local state without another callback', () => {
  const state = harness('speed');
  state.range.onChange({ target: { value: '43' } });
  state.range.onChange({ target: { value: '67' } });
  assert.deepEqual(state.changed, [45]);
  assert.equal(state.timers.length, 1);
  state.range.onMouseUp({ target: { value: '67' } });
  assert.deepEqual(state.localValues, [65]);
  assert.deepEqual(state.changed, [45]);
  state.timers.shift()();
  state.range.onChange({ target: { value: '67' } });
  assert.deepEqual(state.changed, [45, 65]);
});
