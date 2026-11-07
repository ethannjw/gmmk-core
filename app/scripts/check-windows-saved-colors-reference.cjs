'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

const root = process.argv[2];
assert.ok(root, 'Usage: node app/scripts/check-windows-saved-colors-reference.cjs /path/to/CORE-2.1.23/app');
function verifiedSource(file, hash) {
  const bytes = readFileSync(join(root, file));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), hash, 'Review changed reference before execution');
  return bytes.toString('utf8');
}
const renderer = verifiedSource('out/renderer-process/assets/index-401429d5.js',
  '9b8adecdf2957e81d2d6e674af5b6759c138565612d8cbadd67c8049d9c2f4ed');
const main = verifiedSource('out/main/index.js', '96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531');
const css = verifiedSource('out/renderer-process/assets/index-cfb93e42.css',
  'cfb93e428172b9cfff343bd4f87837f83e693aeb6c5f198ff397322bd9bc84bd');
function range(source, startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, startMarker);
  assert.equal(source.indexOf(startMarker, start + startMarker.length), -1, `Ambiguous ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, endMarker);
  return source.slice(start, end);
}
const defaults = ['#ff0000', '#4f16f9', '#15fd07', '#e101dc', '#ffe501', '#ffffff'];
const custom = [...defaults, '#123456', '#abcdef', '#fedcba'];
const plain = value => JSON.parse(JSON.stringify(value));

function harness({ colors = defaults, loadedColors = colors, color = '#ff0000', success = true } = {}) {
  const effects = [];
  const states = [];
  const setters = [];
  const changes = [];
  const swatchChanges = [];
  const saves = [];
  const errors = [];
  let stateIndex = 0;
  let loads = 0;
  const element = (type, props, key) => ({ type, props, key });
  const context = vm.createContext({
    console: { log: () => {} },
    jsxRuntimeExports: { jsx: element, jsxs: element, Fragment: 'Fragment' },
    classNames$1: (...values) => values.filter(Boolean).flatMap(value => typeof value === 'object'
      ? Object.keys(value).filter(key => value[key]) : [value]).join(' '),
    styles$x: { swatch: 'swatch', active: 'active' },
    Dropdown: 'Dropdown', InputText: 'InputText', Z: 'ColorSurface', Typography$1: 'Typography',
    TextType$1: { ParagraphSmallBold: 'smallBold' }, Tooltip: 'Tooltip', Icon: 'Icon',
    IconType$1: { Plus: 'plus' }, IconSize: { Small: 'small' }, Color: { Base50: 'base50' },
    Logger: { logError: error => errors.push(error) },
  }, { codeGeneration: { strings: false, wasm: false } });
  vm.runInContext([
    range(renderer, 'function toHex(color2) {', 'const BindingTypes_ButtonPress ='),
    range(renderer, 'const rgbToHex = (r2, g2, b2) => {', 'const ColorPicker ='),
    'this.RGBAColor = RGBAColor; this.AppSettingsState = AppSettingsState; this.renderSwatch = Swatch;',
  ].join('\n'), context, { timeout: 1000 });
  const settings = new context.AppSettingsState();
  settings.colorSwatches = colors.map(value => context.RGBAColor.fromHex(value));
  settings.language = 'de';
  settings.startup = true;
  context.reactExports = {
    useState: initial => {
      const index = stateIndex++;
      if (!(index in states)) states[index] = index === 2 ? settings : initial;
      return [states[index], value => { states[index] = value; setters.push({ index, value }); }];
    },
    useMemo: callback => callback(),
    useEffect: callback => effects.push(callback),
  };
  context.AppService = {
    getAppSettings: async () => {
      loads++;
      return plain({ ...settings, colorSwatches: loadedColors.map(value => context.RGBAColor.fromHex(value)) });
    },
    saveAppSettings: async value => {
      saves.push(plain(value.toRecord()));
      return success ? { success: true, data: plain(value) } : { success: false, data: 'simulated storage failure' };
    },
  };
  vm.runInContext(`${range(renderer, 'const ColorPicker = ({ colorHex, onChange, onChangeSwatches, className }) => {', 'const header$x =')}
  this.renderPicker = ColorPicker;`, context, { timeout: 1000 });
  const nodes = [];
  function visit(node) {
    if (Array.isArray(node)) node.forEach(visit);
    else if (node && typeof node === 'object') { nodes.push(node); visit(node.props?.children); }
  }
  visit(context.renderPicker({ colorHex: color, onChange: value => changes.push(value),
    onChangeSwatches: value => swatchChanges.push(Array.from(value, entry => entry.toHex?.() ?? plain(entry))) }));
  return { context, settings, states, setters, changes, swatchChanges, saves, errors, effects,
    get loads() { return loads; },
    swatches: nodes.filter(node => node.type === context.renderSwatch),
    add: nodes.find(node => node.type === 'button').props.onClick,
  };
}

test('six default colors round trip through application-settings records', () => {
  const { context } = harness();
  const original = new context.AppSettingsState();
  assert.deepEqual(Array.from(original.colorSwatches, value => value.toHex()), defaults);
  const restored = context.AppSettingsState.fromRecord(original.toRecord());
  assert.deepEqual(plain(restored.toRecord()), plain(original.toRecord()));
});

test('picker loads application settings on mount and locks the first six swatches', async () => {
  const run = harness({ loadedColors: custom });
  assert.deepEqual(Array.from(run.states[2].colorSwatches, value => value.toHex()), defaults);
  run.effects.at(-1)();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(run.loads, 1);
  assert.deepEqual(Array.from(run.states[2].colorSwatches, value => value.toHex()), custom);
  assert.equal(run.states[2].startup, true);
  assert.equal(run.states[2].language, 'de');
  assert.deepEqual(run.saves, []);
  assert.deepEqual(harness({ colors: custom }).swatches.map(node => node.props.canDelete),
    [false, false, false, false, false, false, true, true, true]);
});

test('selecting a swatch changes color and selection without saving settings', () => {
  const run = harness({ colors: custom });
  for (const swatch of run.swatches) swatch.props.onSelect();
  assert.deepEqual(run.changes, custom);
  assert.equal(run.states[3], custom.length - 1);
  assert.deepEqual(run.saves, []);
});

test('adding saves a duplicate color and preserves other application settings', async () => {
  const run = harness();
  await run.add();
  assert.deepEqual(run.saves[0].colorSwatches, [...defaults, '#ff0000']);
  assert.equal(run.saves[0].startup, true);
  assert.equal(run.saves[0].language, 'de');
  assert.equal(run.states[3], 6);
  assert.deepEqual(Array.from(run.states[2].colorSwatches, value => value.toHex()), [...defaults, '#ff0000']);
  assert.deepEqual(run.changes, []);
});

test('failed add logs an error after mutating the existing settings object', async () => {
  const run = harness({ color: '#123456', success: false });
  await run.add();
  assert.deepEqual(run.errors, ['simulated storage failure']);
  assert.deepEqual(run.setters, []);
  assert.deepEqual(run.swatchChanges, []);
  assert.deepEqual(Array.from(run.settings.colorSwatches, value => value.toHex()), [...defaults, '#123456']);
});

test('deleting the active middle custom color selects a neighbor color but highlights the last swatch', async () => {
  const run = harness({ colors: custom, color: '#123456' });
  await run.swatches[6].props.onDelete();
  assert.deepEqual(run.changes, ['#ffffff']);
  assert.deepEqual(run.saves[0].colorSwatches, [...defaults, '#abcdef', '#fedcba']);
  assert.equal(run.states[3], 7);
  assert.equal(run.states[2].colorSwatches[run.states[3]].toHex(), '#fedcba');
  assert.notEqual(run.changes.at(-1), run.states[2].colorSwatches[run.states[3]].toHex());
});

test('failed delete changes the active color and publishes removal before storage succeeds', async () => {
  const run = harness({ colors: custom, color: '#123456', success: false });
  await run.swatches[6].props.onDelete();
  assert.deepEqual(run.changes, ['#ffffff']);
  assert.deepEqual(run.swatchChanges, [[...defaults, '#abcdef', '#fedcba']]);
  assert.deepEqual(run.errors, ['simulated storage failure']);
  assert.deepEqual(run.setters, []);
  assert.deepEqual(Array.from(run.settings.colorSwatches, value => value.toHex()), [...defaults, '#abcdef', '#fedcba']);
});

test('right-click opens Delete only for an active deletable swatch; left-click closes it and selects', () => {
  const { context } = harness();
  for (const active of [false, true]) {
    for (const deletable of [false, true]) {
      let open = false;
      let selections = 0;
      let deletions = 0;
      context.reactExports.useState = () => [open, value => { open = value; }];
      const props = { isActive: active, canDelete: deletable, color: '#123456',
        onSelect: () => selections++, onDelete: () => deletions++ };
      context.renderSwatch(props).props.onContextMenu();
      assert.equal(open, active && deletable);
      const tree = context.renderSwatch(props);
      assert.equal(Boolean(tree.props.children), active && deletable);
      if (tree.props.children) tree.props.children.props.onClick();
      assert.equal(deletions, Number(active && deletable));
      tree.props.onClick();
      assert.equal(open, false);
      assert.equal(selections, 1);
    }
  }
});

test('settings save boundary serializes swatches and requests persistence', () => {
  const run = harness({ colors: custom });
  let saves = 0;
  const data = { save: () => saves++ };
  const context = vm.createContext({ data }, { codeGeneration: { strings: false, wasm: false } });
  const method = range(main, '  static saveAppSettings(appSettings) {', '  static #getDevicesData()');
  vm.runInContext(`class DataStorage { static #getAppData() { return data; } ${method} }
  this.save = value => DataStorage.saveAppSettings(value);`, context, { timeout: 1000 });
  assert.equal(context.save(run.settings), run.settings);
  assert.equal(saves, 1);
  assert.deepEqual(plain(data.settings), plain(run.settings.toRecord()));
});

test('saved-color CSS specifies wrapping 24px circles, an active outline and a 24px add button', () => {
  assert.match(css, /--element-height-small: 24px;/);
  assert.match(css, /--gap: 8px;/);
  const swatch = range(css, '\n._swatch_1wasp_89 {', '}');
  for (const rule of ['width: var(--element-height-small);', 'height: var(--element-height-small);',
    'border-radius: var(--element-height-small);']) assert.ok(swatch.includes(rule), rule);
  const group = range(css, '\n._swatches_1wasp_105 {', '}');
  assert.ok(group.includes('gap: var(--gap);'));
  assert.ok(group.includes('flex-wrap: wrap;'));
  assert.ok(range(css, '\n._swatch_1wasp_89._active_1wasp_165 {', '}')
    .includes('outline: var(--border-radius-thin) solid var(--base-shades-20);'));
  const add = range(css, '\n._addSwatchButton_1wasp_187 {', '}');
  assert.ok(add.includes('--size: 24px;'));
  assert.ok(add.includes('border-radius: 50%;'));
  assert.ok(add.includes('background: transparent;'));
});
