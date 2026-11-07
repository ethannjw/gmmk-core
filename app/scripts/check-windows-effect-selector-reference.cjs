'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { join, dirname } = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

const referencePath = process.argv[2];
assert.ok(referencePath, 'Usage: node app/scripts/check-windows-effect-selector-reference.cjs /path/to/CORE-2.1.23/out/renderer-process/assets/index-401429d5.js');
function readPinned(path, hash) {
  const bytes = readFileSync(path);
  assert.equal(createHash('sha256').update(bytes).digest('hex'), hash,
    'Reference hash mismatch: review the source before executing extracted helpers');
  return bytes.toString('utf8');
}
const source = readPinned(referencePath, '9b8adecdf2957e81d2d6e674af5b6759c138565612d8cbadd67c8049d9c2f4ed');
const translations = new Map(JSON.parse(readPinned(join(dirname(referencePath), '../i18n/en.json'),
  'd5f45100b2047600d95620e62726d1243d0740cf4051b5cbd84e5327a903f5bf')).map(entry => [entry.key, entry.value]));
const css = readPinned(join(dirname(referencePath), 'index-cfb93e42.css'),
  'cfb93e428172b9cfff343bd4f87837f83e693aeb6c5f198ff397322bd9bc84bd');
const effects = JSON.parse(readFileSync(join(__dirname, '../assets/windows-preset-lighting-vectors.json'), 'utf8')).effects;
const expectedLabels = ['LED Off', 'Glorious Mode', 'Wave #1', 'Wave #2', 'Spiraling Wave', 'Acid Mode',
  'Breathing', 'Normally On', 'Ripple Graff', 'Pass Without Trace', 'Fast Run Without Trace', 'Matrix 2',
  'Matrix 3', 'Rainbow', 'Heartbeat Sensor', 'Digital Times', 'Kamehemeha', 'Pingpong'];

function declarationRange(startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, `Missing start marker: ${startMarker}`);
  assert.equal(source.indexOf(startMarker, start + startMarker.length), -1, `Ambiguous start: ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, `Missing end marker: ${endMarker}`);
  return source.slice(start, end);
}

const declarations = [
  declarationRange('class DisplayOption {', 'const Channels = MessageChannels.DataChannel;'),
  declarationRange('class PresetLightingEffectData {', 'const PresetLightingEffectData_Headset_A2 ='),
  declarationRange('const PresetLightingEffectData_Keyboard_V1 =', 'const PresetLightingEffectData_GMBK ='),
  declarationRange('const PAGE_OPTION_COUNT$1 = 9;', 'function u() {'),
].join('\n');

function harness(identifier) {
  const state = [];
  const mountedEffects = [];
  const changed = [];
  let cursor = 0;
  let mounted = false;
  const element = (type, props) => ({ type, props });
  const context = vm.createContext({
    structuredClone,
    reactExports: {
      useState: initial => {
        const index = cursor++;
        if (!(index in state)) state[index] = initial;
        return [state[index], value => { state[index] = value; }];
      },
      useMemo: callback => callback(),
      useEffect: callback => { if (!mounted) mountedEffects.push(callback); },
    },
    jsxRuntimeExports: { jsx: element, jsxs: element, Fragment: 'fragment' },
    useTranslate: () => (key, fallback) => translations.get(key) ?? fallback,
    style$1R: { option: 'option', selected: 'selected', 'page-link': 'page-link', open: 'open',
      nav: 'nav', previous: 'previous', next: 'next' },
    Tooltip: () => {}, Icon: () => {}, IconType$1: { ArrowLeft: 'arrow' }, Color: { Base10: 'white' },
    DeviceCategory: { Mousepad: 'Mousepad', Keyboard: 'Keyboard' },
    classNames$1: (...values) => values.join(' '),
  }, { codeGeneration: { strings: false, wasm: false } });
  vm.runInContext(`${declarations}
function getSupportedDeviceData() { return { category: DeviceCategory.Keyboard, presetLightingEffects: PresetLightingEffectData_Keyboard_V1 }; }
this.renderSelector = PresetLightingEffectSelector;
this.catalog = PresetLightingEffectData_Keyboard_V1;`, context, { timeout: 1000 });
  function render() {
    cursor = 0;
    const tree = context.renderSelector({ productId: 'GMMKV296ANSI',
      baseEffectState: { effectIdentifier: identifier }, onChange: value => changed.push(value) });
    const nodes = [];
    function collect(node) {
      if (Array.isArray(node)) { node.forEach(collect); return; }
      if (!node || typeof node !== 'object') return;
      nodes.push(node);
      collect(node.props?.children);
    }
    collect(tree);
    return {
      options: nodes.filter(node => node.props.className?.split(' ').includes('option')),
      dots: nodes.filter(node => node.props.className?.split(' ').includes('page-link')),
      previous: nodes.find(node => node.type === 'button' && node.props.className === 'nav previous'),
      next: nodes.find(node => node.type === 'button' && node.props.className === 'nav next'),
    };
  }
  render();
  mounted = true;
  mountedEffects.forEach(callback => callback());
  return { render, changed, catalog: context.catalog };
}

for (const [index, effect] of effects.entries()) {
  test(`${effect.identifier}: initial page, visible labels and retained state`, () => {
    const state = harness(effect.identifier);
    assert.equal(state.catalog[effect.identifier].valueToSendToDevice, effect.local_id);
    const page = state.render();
    const start = Math.floor(index / 9) * 9;
    assert.deepEqual(page.options.map(option => option.props.children), expectedLabels.slice(start, start + 9));
    assert.equal(page.dots.length, 2);
    assert.equal(page.options.filter(option => option.props.className.includes('selected')).length, index < 18 ? 1 : 0);
    assert.deepEqual(state.changed, []);
    for (const option of page.options) {
      option.props.onClick({ preventDefault() {}, stopPropagation() {} });
    }
    assert.deepEqual(state.changed, effects.slice(start, Math.min(start + 9, 18)).map(item => item.identifier));
  });
}

test('arrows and page dots navigate without changing the effect', () => {
  const state = harness('LEDOFF');
  state.render().previous.props.onClick();
  assert.equal(state.render().options[0].props.children, 'LED Off');
  state.render().next.props.onClick();
  state.render().next.props.onClick();
  assert.equal(state.render().options[0].props.children, 'Pass Without Trace');
  state.render().dots[0].props.onClick();
  assert.equal(state.render().options[0].props.children, 'LED Off');
  state.render().dots[1].props.onClick();
  assert.equal(state.render().options.at(-1).props.children, 'Pingpong');
  assert.deepEqual(state.changed, []);
});

test('a retained Surmount effect has no tile and can navigate back without an effect update', () => {
  const state = harness('Surmount');
  assert.equal(state.render().options.length, 0);
  assert.equal(state.render().dots.filter(dot => dot.props.className.includes('open')).length, 0);
  state.render().previous.props.onClick();
  assert.equal(state.render().options.length, 9);
  assert.deepEqual(state.changed, []);
});

test('GMMK 2 ANSI uses the inspected keyboard preset catalog', () => {
  const descriptor = declarationRange('const GMMKV296ANSI = {', 'const GMMK296ISOKeybedRows =');
  assert.match(descriptor, /presetLightingEffects: PresetLightingEffectData_Keyboard_V1,/);
  assert.match(descriptor, /pid: "0x504B"/);
});

test('effect grid, selection and pagination use the documented CSS values', () => {
  const rules = selector => {
    const start = css.indexOf(`${selector} {`);
    assert.notEqual(start, -1, selector);
    return css.slice(start, css.indexOf('}', start));
  };
  const grid = rules('._page_z0p8u_29');
  assert.match(grid, /grid-template-columns: repeat\(3, 120px\)/);
  assert.match(grid, /grid-template-rows: repeat\(3, 40px\)/);
  assert.match(grid, /gap: 12px/);
  const option = rules('._option_z0p8u_51');
  for (const value of ['padding: 12px 16px', 'border-radius: 3px', 'border: solid 1px var(--base-shades-60)',
    'background-color: var(--base-shades-80)', 'font-size: 12px']) assert.ok(option.includes(value), value);
  const selected = rules('._selected_z0p8u_83');
  assert.match(selected, /color: var\(--base-shades-90\)/);
  assert.match(selected, /background-color: var\(--purple-40\)/);
  assert.match(rules('._pagination_z0p8u_93'), /gap: 8px/);
  const dots = css.match(/\._page-link_z0p8u_111 \{[^}]*width: 8px;[^}]*height: 8px;[^}]*\}/);
  assert.ok(dots);
  assert.match(rules('._page-link_z0p8u_111._open_z0p8u_143'), /background-color: var\(--glorange-60\)/);
  for (const token of ['--base-shades-40: #909090', '--base-shades-60: #515151', '--base-shades-80: #262626',
    '--base-shades-90: #1c1c1c', '--purple-40: #C5B4E3', '--glorange-60: #ffa40d']) assert.ok(css.includes(token), token);
});
