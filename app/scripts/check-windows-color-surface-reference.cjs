'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { join, dirname } = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

const referencePath = process.argv[2];
assert.ok(referencePath, 'Usage: node app/scripts/check-windows-color-surface-reference.cjs /path/to/CORE-2.1.23/out/renderer-process/assets/index-401429d5.js');
const vectors = JSON.parse(readFileSync(join(__dirname, '../assets/windows-color-surface-vectors.json'), 'utf8'));
const bytes = readFileSync(referencePath);
assert.equal(createHash('sha256').update(bytes).digest('hex'), vectors.renderer_sha256,
  'Reference hash mismatch: review source before executing extracted helpers');
assert.equal(vectors.renderer_sha256, '9b8adecdf2957e81d2d6e674af5b6759c138565612d8cbadd67c8049d9c2f4ed');
const source = bytes.toString('utf8');
function lightingHandoff() {
  const calls = [];
  const state = vm.createContext({
    reactExports: { useCallback: callback => callback, useState: value => [value, next => calls.push(['color', next])] },
    debounce: callback => callback,
    PresetLightingEffectIdentifier: { GloriousMode: 'GloriousMode', Breathing: 'Breathing', Rave: 'Rave' },
    DeviceCategory: { Keyboard: 'Keyboard', Mousepad: 'Mousepad' },
    deviceSupportData: { category: 'Keyboard' }, deviceState: {},
    isHeadset: false, useLivePreview: false,
    setPreviewBaseEffectState: value => calls.push(['preset', value]),
    setStepState: value => calls.push(['step', value]),
    setShowBasePreview: value => calls.push(['preview', value]),
    DeviceService: new Proxy({}, { get() { throw new Error('Unexpected device operation'); } }),
    console: { log() {} },
  }, { codeGeneration: { strings: false, wasm: false } });
  vm.runInContext(`${range('function toHex(color2) {', 'const BindingTypes_ButtonPress =')}
    ${range('class PresetLightingState {', 'class ZoneLightingState {')}
    const previewBaseEffectState = new PresetLightingState();
    previewBaseEffectState.colors = [RGBAColor.fromHex('#123456')];
    ${range('  const presetEffect_OnChange = reactExports.useCallback(', '  const isFirstRun =')}
    const isFirstRun = { current: true };
    ${range('  const presetColor_OnChange = reactExports.useCallback(', '  const presetBrightness_OnChange =')}
    const toggle = { ${range('setStepState: (value2) => {', '\n          }\n        )')} };
    this.api = { currentColor, presetEffect_OnChange, presetColor_OnChange, toggle, previewBaseEffectState };
  `, state, { timeout: 1000 });
  return { api: state.api, calls };
}

test('lighting brush initializes from the first base color; custom entry retains it', () => {
  const { api, calls } = lightingHandoff();
  assert.equal(api.currentColor.toLowerCase(), '#123456');
  api.toggle.setStepState(true);
  assert.deepEqual(calls, [['step', true]]);
});

test('returning to base restores its first color and enables base preview', () => {
  const { api, calls } = lightingHandoff();
  api.toggle.setStepState(false);
  assert.deepEqual(calls.map(([name, value]) => [name, typeof value === 'string' ? value.toLowerCase() : value]),
    [['step', false], ['preview', true], ['color', '#123456']]);
});

test('base RGB callback updates the shared color and preserves non-color settings', async () => {
  const { api, calls } = lightingHandoff();
  const before = api.previewBaseEffectState.toRecord();
  await api.presetColor_OnChange('#abcdef', 0);
  assert.equal(calls[0][0], 'preset');
  assert.equal(calls[0][1].colors[0].toHex().toLowerCase(), '#abcdef');
  assert.deepEqual({ ...calls[0][1].toRecord(), colors: before.colors }, { ...before });
  assert.deepEqual(calls[1], ['color', '#abcdef']);
  assert.equal(calls.length, 2);
});

test('selecting Breathing replaces the base palette with seven colors without updating the shared brush', async () => {
  const { api, calls } = lightingHandoff();
  await api.presetEffect_OnChange('Breathing');
  assert.equal(calls.length, 1);
  assert.equal(calls[0][0], 'preset');
  assert.deepEqual(Array.from(calls[0][1].colors, color => color.toHex().toLowerCase()),
    ['#ff0000', '#ffa500', '#ffff00', '#00ff00', '#007fff', '#0000ff', '#8b00ff']);
  assert.equal(api.currentColor.toLowerCase(), '#123456');
});

function range(startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, startMarker);
  assert.equal(source.indexOf(startMarker, start + startMarker.length), -1, `Ambiguous ${startMarker}`);
  const end = source.indexOf(endMarker, start);
  assert.notEqual(end, -1, endMarker);
  return source.slice(start, end);
}
const context = vm.createContext({
  React: { memo: callback => callback, createElement: (type, props, ...children) => ({ type, props, children }) },
  g: classes => classes.filter(Boolean).join(' '),
  s: (value, minimum = 0, maximum = 1) => Math.max(minimum, Math.min(maximum, value)),
  m: 'interactive', p: 'pointer',
}, { codeGeneration: { strings: false, wasm: false } });
vm.runInContext(`const ${range('b = function(e, r2, t2) {', ', S = React.memo')};
const ${range('S = React.memo(function(r2) {', ', F = function(e, r2) {')};
this.reference = { fromRgb: L, toRgb: I, hue: S, saturation: T };`, context, { timeout: 1000 });
const reference = context.reference;

for (const entry of vectors.rgb_imports) {
  test(`RGB ${entry.rgb}: reference's rounded HSV state`, () => {
    const result = reference.fromRgb({ r: entry.rgb[0], g: entry.rgb[1], b: entry.rgb[2], a: 1 });
    assert.deepEqual([result.h, result.s, result.v], entry.hsv);
  });
}
for (const entry of vectors.hsv_edits) {
  test(`HSV ${entry.hsv}: encoded color channels`, () => {
    const result = reference.toRgb({ h: entry.hsv[0], s: entry.hsv[1], v: entry.hsv[2], a: 1 });
    assert.deepEqual([result.r, result.g, result.b], entry.rgb);
  });
}

test('hue pointer and 5% keyboard movements preserve reference bounds', () => {
  for (const hue of [0, 18, 120, 342, 360]) {
    const changes = [];
    const tree = reference.hue({ hue, onChange: value => changes.push(value) });
    const controls = tree.children[0].props;
    assert.equal(controls['aria-label'], 'Hue');
    for (const left of [0, 0.25, 0.5, 0.75, 1]) controls.onMove({ left });
    for (const left of [-0.05, 0.05]) controls.onKey({ left, top: 0 });
    assert.deepEqual(changes.map(value => value.h), [0, 90, 180, 270, 360, Math.max(0, hue - 18), Math.min(360, hue + 18)]);
  }
});

test('saturation and value pointer axes and keyboard steps match the reference', () => {
  const changes = [];
  const tree = reference.saturation({ hsva: { h: 120, s: 50, v: 50, a: 1 }, onChange: value => changes.push(value) });
  const controls = tree.children[0].props;
  assert.equal(controls['aria-label'], 'Color');
  for (const [left, top] of [[0, 0], [1, 0], [0, 1], [1, 1], [0.5, 0.5]]) controls.onMove({ left, top });
  for (const [left, top] of [[0.05, 0], [-0.05, 0], [0, 0.05], [0, -0.05]]) controls.onKey({ left, top });
  assert.deepEqual(changes.map(value => [value.s, value.v]), [[0, 100], [100, 100], [0, 0], [100, 0], [50, 50],
    [55, 50], [45, 50], [50, 45], [50, 55]]);
});

test('reference surface has 200px total height, a 24px hue bar and a 12px black separator', () => {
  const cssBytes = readFileSync(join(dirname(referencePath), 'index-cfb93e42.css'));
  assert.equal(createHash('sha256').update(cssBytes).digest('hex'), 'cfb93e428172b9cfff343bd4f87837f83e693aeb6c5f198ff397322bd9bc84bd');
  const css = cssBytes.toString('utf8');
  assert.ok(source.includes('width:200px;height:200px;'));
  assert.ok(source.includes('border-bottom:12px solid #000;'));
  assert.ok(source.includes('.react-colorful__hue{position:relative;height:24px}'));
  assert.match(css, /\._reactColorful_1wasp_1 \{\s*width: 100%;/);
  assert.match(css, /\.react-colorful__hue-pointer \{\s*width: 7px;\s*border-radius: 3px;\s*border-width: 1px;/);
});

const numericContext = vm.createContext({}, { codeGeneration: { strings: false, wasm: false } });
vm.runInContext(`${range('const rgbToHex = (r2, g2, b2) => {', 'const relativeLuminanceRed =')}
this.numeric = { rgbToHex, hexToRgb };`, numericContext, { timeout: 1000 });
const numeric = numericContext.numeric;

for (const { rgb } of vectors.rgb_imports) {
  test(`numeric RGB ${rgb} round trips through six-digit HEX`, () => {
    const hex = `#${rgb.map(channel => channel.toString(16).padStart(2, '0')).join('')}`;
    assert.equal(numeric.rgbToHex(...rgb), hex);
    const result = numeric.hexToRgb(hex);
    assert.deepEqual([result.r, result.g, result.b], rgb.map(String));
  });
}

for (const [input, expected] of [
  ['-1', '#-10000'], ['256', '#1000000'], ['', '#NaN0000'],
  ['12junk', '#0c0000'], ['12.9', '#0c0000'], ['1e2', '#010000'],
]) {
  test(`characterize unchecked reference red input ${JSON.stringify(input)}`, () => {
    assert.equal(numeric.rgbToHex(input, '0', '0'), expected);
  });
}

for (const [input, expected] of [
  ['#abc', ['0', '0', '0']], ['#GG2238', ['NaN', '34', '56']],
  ['x0c2238', ['12', '34', '56']], ['#12345678', ['0', '0', '0']],
]) {
  test(`characterize unchecked reference HEX input ${JSON.stringify(input)}`, () => {
    const result = numeric.hexToRgb(input);
    assert.deepEqual([result.r, result.g, result.b], expected);
  });
}

function picker(format, colorHex = '#0c2238') {
  let stateIndex = 0;
  const changes = [];
  const setters = [];
  const jsx = (type, props) => ({ type, props });
  const context = vm.createContext({
    reactExports: {
      useState: initial => {
        const index = stateIndex++;
        return [index === 0 && format !== undefined ? format : initial,
          value => setters.push({ index, value })];
      },
      useMemo: callback => callback(),
      useEffect: () => {},
    },
    jsxRuntimeExports: { jsx, jsxs: jsx, Fragment: 'Fragment' },
    AppSettingsState: class { constructor() { this.colorSwatches = []; } },
    hexToRgb: numeric.hexToRgb, rgbToHex: numeric.rgbToHex,
    console: { log: () => {} }, classNames$1: () => '', styles$x: {},
    Dropdown: 'Dropdown', InputText: 'InputText', Z: 'ColorSurface',
    Typography$1: 'Typography', TextType$1: { ParagraphSmallBold: 'smallBold' },
    Tooltip: 'Tooltip', Swatch: 'Swatch', Icon: 'Icon',
    IconType$1: { Plus: 'plus' }, IconSize: { Small: 'small' }, Color: { Base50: 'base50' },
  }, { codeGeneration: { strings: false, wasm: false } });
  vm.runInContext(`${range('const ColorPicker = ({ colorHex, onChange, onChangeSwatches, className }) => {', 'const header$x =')}
this.renderPicker = ColorPicker;`, context, { timeout: 1000 });
  const tree = context.renderPicker({ colorHex, onChange: value => changes.push(value), onChangeSwatches: () => {} });
  const nodes = [];
  function visit(node) {
    if (Array.isArray(node)) node.forEach(visit);
    else if (node && typeof node === 'object') { nodes.push(node); visit(node.props?.children); }
  }
  visit(tree);
  return { nodes, changes, setters };
}

test('numeric picker defaults to HEX with RGB then HEX options before the inline surface', () => {
  const { nodes, changes, setters } = picker();
  const dropdown = nodes.find(node => node.type === 'Dropdown');
  const inputs = nodes.filter(node => node.type === 'InputText');
  assert.equal(dropdown.props.selectedValue, 'HEX');
  assert.deepEqual(Array.from(dropdown.props.options), ['RGB', 'HEX']);
  assert.equal(dropdown.props.label, 'Color format');
  assert.equal(dropdown.props.labelHidden, true);
  assert.equal(inputs.length, 1);
  assert.equal(inputs[0].props.label, 'HEX');
  assert.equal(inputs[0].props.value, '#0c2238');
  assert.equal(inputs[0].props.centred, true);
  assert.ok(nodes.indexOf(inputs[0]) < nodes.findIndex(node => node.type === 'ColorSurface'));
  dropdown.props.onChange('RGB');
  assert.deepEqual(setters, [{ index: 0, value: 'RGB' }]);
  assert.deepEqual(changes, []);
});

test('HEX callback prefixes a hash but forwards incomplete and invalid text', () => {
  const { nodes, changes } = picker();
  const input = nodes.find(node => node.type === 'InputText');
  for (const value of ['0c2238', '#0c2238', '', 'abc', 'GG2238']) input.props.onChange(value);
  assert.deepEqual(changes, ['#0c2238', '#0c2238', '', '#abc', '#GG2238']);
});

test('RGB callback order is Red Green Blue and forwards parsed channel edits immediately', () => {
  const { nodes, changes } = picker('RGB');
  const inputs = nodes.filter(node => node.type === 'InputText');
  assert.deepEqual(inputs.map(node => node.props.label), ['Red', 'Green', 'Blue']);
  assert.deepEqual(inputs.map(node => node.props.value), ['12', '34', '56']);
  for (const input of inputs) {
    assert.equal(input.props.labelHidden, true);
    assert.equal(input.props.centred, true);
    assert.equal(input.props.pattern, undefined);
    input.props.onChange('255');
  }
  assert.deepEqual(changes, ['#ff2238', '#0cff38', '#0c22ff']);
});

test('RGB callback permits out-of-range values; text fields default to disabled validation', () => {
  const { nodes, changes } = picker('RGB');
  const input = nodes.find(node => node.type === 'InputText');
  input.props.onChange('256');
  assert.deepEqual(changes, ['#1002238']);
  const inputSource = range('const InputText = ({', 'const rgbToHex = (r2, g2, b2) => {');
  assert.match(inputSource, /type = "text"/);
  assert.match(inputSource, /validationDisabled = true/);
});
