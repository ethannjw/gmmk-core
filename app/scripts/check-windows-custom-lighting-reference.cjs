'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

const root = process.argv[2];
assert.ok(root, 'Usage: node app/scripts/check-windows-custom-lighting-reference.cjs /path/to/CORE-2.1.23/app');
const fixture = JSON.parse(readFileSync(join(__dirname, '../assets/windows-custom-lighting-vectors.json'), 'utf8'));
assert.equal(fixture.main_sha256, '96e373835ce225da34c974c9200e75e737048719ba6c6cb674c511be7c4a0531');
assert.equal(fixture.renderer_sha256, '9b8adecdf2957e81d2d6e674af5b6759c138565612d8cbadd67c8049d9c2f4ed');
function verifiedSource(relativePath, hash) {
  const bytes = readFileSync(join(root, relativePath));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), hash, 'Review changed reference source before execution');
  return bytes.toString('utf8');
}
const main = verifiedSource('out/main/index.js', fixture.main_sha256);
const renderer = verifiedSource('out/renderer-process/assets/index-401429d5.js', fixture.renderer_sha256);
const css = verifiedSource('out/renderer-process/assets/index-cfb93e42.css',
  'cfb93e428172b9cfff343bd4f87837f83e693aeb6c5f198ff397322bd9bc84bd');
const translations = new Map(JSON.parse(verifiedSource('out/renderer-process/i18n/en.json',
  'd5f45100b2047600d95620e62726d1243d0740cf4051b5cbd84e5327a903f5bf')).map(entry => [entry.key, entry.value]));
function range(source, startMarker, endMarker, includeEnd = false) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, startMarker);
  assert.equal(source.indexOf(startMarker, start + startMarker.length), -1, `Ambiguous ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, endMarker);
  return source.slice(start, end + (includeEnd ? endMarker.length : 0));
}
const context = vm.createContext({ Buffer }, { codeGeneration: { strings: false, wasm: false } });
vm.runInContext([
  range(renderer, 'var KeybedComponentIdentifier =', '})(KeybedComponentIdentifier || {});', true),
  range(renderer, 'var QuickKeyId =', 'const GMMK3100KeybedComponentLayout ='),
  range(renderer, 'const GMMK296KeybedComponentLayout =', 'const GMMK296ISOKeybedComponentLayout ='),
  range(renderer, 'const QuickKeySelectorsData_NoBadge_Keyboard =', 'const QuickKeySelectorsData_GMMKNumpad ='),
  'this.layout = GMMK296KeybedComponentLayout; this.selectors = QuickKeySelectorsData_NoBadge_Keyboard;',
].join('\n'), context, { timeout: 1000 });
const mapsStart = main.indexOf('const KeyBufferIndexMaps =');
assert.notEqual(mapsStart, -1);
const mapStart = main.indexOf('  [DeviceProductIdentifier.GMMKV296ANSI]: {', mapsStart);
assert.notEqual(mapStart, -1);
const mapEnd = main.indexOf('\n  },', mapStart);
assert.notEqual(mapEnd, -1);
vm.runInContext(`this.keyIndices = ${main.slice(main.indexOf('{', mapStart), mapEnd + 4)};`, context, { timeout: 1000 });
const sideIndices = JSON.parse(range(main, 'const GMMK_V2_96_SIDE_LED = ', ';').split(' = ')[1]);
function lightingModeReference() {
  const element = (type, props) => ({ type, props });
  const state = vm.createContext({
    reactExports: { useMemo: callback => callback() },
    jsxRuntimeExports: { jsx: element, jsxs: element },
    getSupportedDeviceData: () => ({ category: 'Keyboard' }),
    DeviceCategory: { Keyboard: 'Keyboard', Mousepad: 'Mousepad' },
    classNames$1: (...values) => values.flatMap(value => typeof value === 'object'
      ? Object.entries(value).filter(([, enabled]) => enabled).map(([name]) => name) : [value]).join(' '),
  }, { codeGeneration: { strings: false, wasm: false } });
  vm.runInContext(`${range(renderer, 'const toggle$3 =', 'function PerKeyLightingBreathingToggle(')}
    this.renderMode = LightingStepToggle; this.renderSegments = SegmentedToggle;`, state, { timeout: 1000 });
  return state;
}

test('GMMK 2 lighting modes expose both choices regardless of the selected base effect', () => {
  const state = lightingModeReference();
  for (const currentEffectIdentifier of ['LEDOFF', 'Wave1', 'AcidMode', 'Breathing']) {
    for (const stepState of [false, true]) {
      const changes = [];
      const wrapper = state.renderMode({ deviceState: { productId: 'GMMKV296ANSI' },
        currentEffectIdentifier, stepState, setStepState: value => changes.push(value) });
      assert.deepEqual(Array.from(wrapper.props.options, option => [option.label, option.value, !!option.disabled]),
        [['Base Effect', false, false], ['Custom Per Key', true, false]]);
      const tree = state.renderSegments(wrapper.props);
      assert.equal(tree.props.style['--option-count'], 2);
      assert.equal(tree.props.style['--selected-index'], Number(stepState));
      const options = tree.props.children[1];
      for (const option of options) option.props.onClick();
      assert.deepEqual(changes, [false, true]);
      assert.ok(options[Number(stepState)].props.className.includes('_selected_1e1au_49'));
    }
  }
});

test('lighting mode track and label CSS use the active segmented-selector geometry and palette', () => {
  const track = range(css, '._toggle_1e1au_1 {', '}');
  for (const declaration of ['--padding: 3px;', 'grid-template-columns: repeat(var(--option-count, 2), 1fr);',
    'background-color: var(--base-shades-90);', 'padding: var(--padding);', 'border-radius: 3px;',
    'border: 1px solid var(--base-shades-80);']) assert.ok(track.includes(declaration), declaration);
  const option = range(css, '._option_1e1au_23 {', '}');
  for (const declaration of ['padding: 8px 42px;', 'font-size: 14px;', 'white-space: nowrap;',
    'text-align: center;', 'color: var(--base-shades-30);', 'font-weight: 400;']) assert.ok(option.includes(declaration), declaration);
  const selected = range(css, '._option_1e1au_23._selected_1e1au_49 {', '}');
  assert.ok(selected.includes('font-weight: 600;'));
  assert.ok(selected.includes('color: var(--atomic-purple-30);'));
  for (const declaration of ['--base-shades-90: #1c1c1c;', '--base-shades-80: #262626;',
    '--base-shades-70: #313131;', '--base-shades-30: #d9d9d9;', '--atomic-purple-30: #d9cef4;']) {
    assert.ok(css.includes(declaration), declaration);
  }
  assert.ok(range(css, '._lighting-step-toggle_3wr88_143', '}').includes('margin-bottom: 20px;'));
});

test('lighting mode indicator slides for 200ms with ease-out inside the padded track', () => {
  const indicator = range(css, '._indicator_1e1au_73 {', '}');
  for (const declaration of ['background-color: var(--base-shades-70);', 'transition: transform 200ms ease-out;',
    'border-radius: 3px;', 'top: var(--padding);', 'left: var(--padding);',
    'height: calc(100% - var(--padding) - var(--padding));',
    'width: calc((100% - var(--padding) * (var(--option-count, 2))) / var(--option-count, 2));',
    'transform: translateX(calc(var(--selected-index, 0) * 100%));']) assert.ok(indicator.includes(declaration), declaration);
});

const product = 'GMMKV296ANSI';
context.getKeyBufferIndex$1 = (productId, identifier) => {
  assert.equal(productId, product);
  return context.keyIndices[identifier];
};
context.AuxLightIndexMaps = { [product]: { Sidelight: sideIndices } };
vm.runInContext(`${range(main, 'function PrepareLEDPerKeyBuffer$1(', 'const TargetBindingTypeByteOverride$2 =')}
this.encodePage = PrepareLEDPerKeyBuffer$1;`, context, { timeout: 1000 });

test('quick-selector renders English labels in order and emits the selected group ID', () => {
  const element = (type, props) => ({ type, props });
  context.reactExports = { useMemo: callback => callback() };
  context.jsxRuntimeExports = { jsx: element, jsxs: element, Fragment: 'fragment' };
  context.useTranslate = () => (key, fallback) => translations.get(key) ?? fallback;
  context.style$1J = { 'quick-keys': 'quick-keys', header: 'header', title: 'title', row: 'row', option: 'option' };
  vm.runInContext(`${range(renderer, 'const QuickKeyLabelMap =', '\n[\n  // GMMK 3 PRO HE')}
  this.renderQuickSelector = PerKeyLightingQuickKeySelector;`, context, { timeout: 1000 });
  const selected = [];
  const tree = context.renderQuickSelector({ productId: product,
    deviceQuickKeySelectors: [...context.selectors, ...context.selectors], onSelect: group => selected.push(group) });
  const nodes = [];
  function visit(node) {
    if (Array.isArray(node)) node.forEach(visit);
    else if (node && typeof node === 'object') { nodes.push(node); visit(node.props?.children); }
  }
  visit(tree);
  assert.equal(nodes.find(node => node.props.className === 'title').props.children, 'Quick Key Selection');
  const buttons = nodes.filter(node => node.type === 'button');
  assert.deepEqual(buttons.map(button => button.props.children), fixture.groups.map(group => group.label));
  for (const button of buttons) button.props.onClick();
  assert.deepEqual(selected, fixture.groups.map(group => group.id));
});

test('quick-selector CSS specifies four fixed-width pill columns, gaps, typography and colors', () => {
  const expected = fixture.selector;
  const selector = range(css, '._quick-keys_13h9s_1 {', '/* GMMK Eternal only. */');
  for (const [property, value] of [['columns', expected.columns], ['option-width', `${expected.button_size[0]}px`],
    ['option-height', `${expected.button_size[1]}px`], ['option-gap', `${expected.gap}px`], ['header-gap', `${expected.header_gap}px`]]) {
    assert.ok(selector.includes(`--quick-key-${property}: ${value};`));
  }
  const options = range(css, '._option_13h9s_85 {', '}');
  for (const value of ['width: var(--quick-key-option-width);', 'height: var(--quick-key-option-height);',
    'background-color: var(--base-shades-80);', 'color: var(--base-shades-30);',
    `border-radius: ${expected.border_radius}px;`, `font-size: ${expected.font_size}px;`, 'font-weight: 600;']) {
    assert.ok(options.includes(value), value);
  }
  const title = range(css, '._title_13h9s_59 {', '}');
  assert.ok(title.includes(`font-size: ${expected.header_font_size}px;`));
  assert.ok(title.includes('color: var(--base-shades-20);'));
  assert.ok(title.includes('font-weight: 700;'));
  for (const [token, color] of [['80', expected.background], ['30', expected.foreground], ['20', expected.header_foreground]]) {
    assert.ok(css.includes(`--base-shades-${token}: ${color};`));
  }
  assert.match(css, /button\s*\{\s*margin-left: 8px;/);
  assert.equal(expected.button_margin_left, 8);
  const row = range(css, '._row_13h9s_71 {', '}');
  assert.ok(row.includes('grid-template-columns: repeat(var(--quick-key-columns), var(--quick-key-option-width));'));
  assert.ok(row.includes('gap: var(--quick-key-option-gap);'));
});

test('page source puts the selector after the preview while controls span the left column', () => {
  const page = range(css, '._page_3wr88_1 {', '}');
  assert.ok(page.includes('grid-template-columns: auto 1fr;'));
  const controls = range(css, '._controls_3wr88_29 {', '}');
  assert.ok(controls.includes('grid-row: 1 / -1;'));
  const component = renderer.slice(renderer.indexOf('function DeviceLightingPage({ deviceState })'));
  assert.ok(component.indexOf('      DevicePreview,') < component.indexOf('      PerKeyLightingQuickKeySelector,'));
  assert.ok(component.indexOf('      DevicePreview,') >= 0);
});

test('global disabled-button CSS specifies subdued colors and opacity', () => {
  assert.match(css, /button\[disabled\]\s*\{\s*opacity: \.7;/);
  assert.match(css, /button\[disabled\]\s*\{\s*background-color: #707070;\s*color: #999999;/);
});

test('breathing wrapper forwards its boolean and callback to the shared toggle', () => {
  context.useTranslate = () => () => {};
  context.jsxRuntimeExports = { jsx: (type, props) => ({ type, props }) };
  context.Toggle = 'Toggle';
  vm.runInContext(`${range(renderer, 'function PerKeyLightingBreathingToggle(', 'const header$u =')}
  this.renderBreathing = PerKeyLightingBreathingToggle;`, context, { timeout: 1000 });
  for (const initial of [false, true]) {
    const events = [];
    const node = context.renderBreathing({ perKeyBreathingOn: initial, onChange: value => events.push(value) });
    assert.equal(node.type, 'Toggle');
    assert.equal(node.props.label, 'Breathing');
    assert.equal(node.props.value, initial);
    node.props.onChange(!initial);
    assert.deepEqual(events, [!initial]);
  }
});

test('shared breathing toggle exposes On/Off state, checked input and disabled callback gating', () => {
  const element = (type, props) => ({ type, props });
  Object.assign(context, {
    jsxRuntimeExports: { jsx: element, jsxs: element },
    Size: { Small: 'small', Medium: 'medium', Large: 'large' },
    TextType$1: { ParagraphSmallBold: 'small-bold' },
    Color: { Base20: 'base20', Base30: 'base30', Base50: 'base50' },
    Typography$1: 'Typography',
    styles$F: { toggleComponent: 'component', toggleContainer: 'container', toggleInput: 'input',
      toggle: 'track', handle: 'handle', selected: 'selected', disabled: 'disabled' },
    classNames$1: (...values) => values.filter(Boolean).flatMap(value => typeof value === 'object'
      ? Object.keys(value).filter(key => value[key]) : [value]).join(' '),
  });
  vm.runInContext(`${range(renderer, 'const Toggle = ({', 'const sliderContainer =')}
  this.renderToggle = Toggle;`, context, { timeout: 1000 });
  for (const initial of [false, true]) {
    for (const disabled of [false, true]) {
      const events = [];
      const tree = context.renderToggle({ label: 'Breathing', value: initial, disabled, onChange: value => events.push(value) });
      const children = tree.props.children[0].props.children;
      assert.equal(children[0].props.children, 'Breathing');
      assert.equal(children[0].props.type, 'small-bold');
      assert.equal(children[1].props.children, initial ? 'On' : 'Off');
      const input = children.find(node => node && node.type === 'input');
      assert.equal(input.props.checked, initial);
      assert.equal(input.props.id, 'toggle-breathing');
      assert.equal(input.props.type, 'checkbox');
      assert.equal(children.find(node => node && node.type === 'label').props.htmlFor, input.props.id);
      input.props.onChange({ target: { checked: !initial } });
      assert.deepEqual(events, disabled ? [] : [!initial]);
    }
  }
});

test('breathing toggle CSS specifies track, handle, hover, focus and disabled states', () => {
  const expected = fixture.breathing_toggle;
  assert.deepEqual(expected.track_size, [48, 24]);
  assert.equal(expected.handle_size, 16);
  assert.equal(expected.handle_travel, 22);
  assert.match(css, /--element-height-small: 24px;/);
  assert.match(css, /--element-height: 32px;/);
  assert.match(css, /--gap-small: 4px;/);
  assert.match(css, /--gap: 8px;/);
  assert.equal(expected.label_gap, 8);
  const track = range(css, '\n._toggle_h1dd2_1 {', '}');
  for (const rule of ['width: calc(var(--element-height-small) * 2);', 'height: var(--element-height-small);',
    'padding: var(--gap-small);', 'border: var(--border-thin) solid var(--base-shades-40);',
    'background-color: var(--base-shades-100);']) assert.ok(track.includes(rule), rule);
  const handle = range(css, '\n._handle_h1dd2_73 {', '}');
  assert.ok(handle.includes('width: calc(var(--element-height) / 2);'));
  assert.ok(handle.includes('height: calc(var(--element-height) / 2);'));
  assert.ok(handle.includes('background-color: var(--base-shades-80);'));
  assert.ok(handle.includes('border: var(--border-thin) solid var(--base-shades-30);'));
  const selected = range(css, '\n._handle_h1dd2_73._selected_h1dd2_99 {', '}');
  assert.ok(selected.includes('transform: translateX(22px);'));
  assert.ok(selected.includes('background-color: var(--glorange-60);'));
  assert.ok(range(css, '._toggle_h1dd2_1:hover {', '}').includes('background-color: var(--base-shades-70);'));
  assert.ok(range(css, '._toggle_h1dd2_1:hover > ._handle_h1dd2_73,', '}').includes('background-color: var(--glorange-20);'));
  assert.ok(range(css, '._toggleInput_h1dd2_27:focus ~ ._toggle_h1dd2_1 {', '}').includes('outline: 2px solid var(--focus-60);'));
  assert.ok(range(css, '._toggleInput_h1dd2_27._disabled_h1dd2_133 ~ ._toggle_h1dd2_1 {', '}').includes('background-color: var(--base-shades-90);'));
  assert.equal(expected.font_size, 12);
  assert.match(css, /\._paragraphsmallbold_17yvo_97,\s*\._paragraphsmall_17yvo_97\s*\{[^}]*font-size: 12px;/);
  for (const [token, value] of [['base-shades-100', expected.track_fill], ['base-shades-40', expected.track_border],
    ['base-shades-80', expected.handle_fill], ['base-shades-30', expected.handle_border],
    ['glorange-60', expected.selected], ['base-shades-70', expected.hover_fill],
    ['glorange-20', expected.hover_accent], ['focus-60', expected.focus]]) assert.ok(css.includes(`--${token}: ${value};`));
});

test('breathing track and handle use 300ms transitions with implicit CSS ease and no delay', () => {
  const expected = fixture.breathing_toggle;
  assert.equal(expected.transition_ms, 300);
  assert.equal(expected.transition_easing, 'ease');
  assert.ok(css.includes(`--transition-fast: ${expected.transition_ms}ms;`));
  for (const [selector, properties] of [
    ['\n._toggle_h1dd2_1 {', ['border-color', 'background-color']],
    ['\n._handle_h1dd2_73 {', ['border-color', 'background-color', 'transform']],
  ]) {
    const declarations = range(css, selector, '}').match(/transition:\s*([^;]+);/);
    assert.ok(declarations);
    assert.deepEqual(declarations[1].split(',').map(value => value.trim()),
      properties.map(property => `${property} var(--transition-fast)`));
  }
});

test('target selector order and auxiliary indices match the pinned reference', () => {
  assert.deepEqual(Array.from(context.selectors), fixture.groups.map(group => group.id));
  assert.deepEqual(sideIndices, fixture.side_led_indices);
  const descriptor = range(renderer, 'const GMMKV296ANSI = {', 'const GMMK296ISOKeybedRows =');
  assert.match(descriptor, /lightingQickKeySelectors: QuickKeySelectorsData_NoBadge_Keyboard/);
  const mappings = range(main, 'const AuxLightIndexMaps = {', 'const GMMK_DEFAULT_BUFFER_VARIANT_A');
  assert.match(mappings, /\[DeviceProductIdentifier.GMMKV296ANSI\]: \{\s*Sidelight: GMMK_V2_96_SIDE_LED/);
});

for (const group of fixture.groups) {
  const identifiers = Object.entries(context.layout)
    .filter(([, component]) => component.quickKeyIds.includes(group.id)).map(([identifier]) => identifier);
  test(`${group.label}: exact component membership and wire LED indices`, () => {
    assert.equal(identifiers.length, group.component_count);
    const indices = identifiers.flatMap(identifier => context.keyIndices[identifier] === undefined
      ? context.AuxLightIndexMaps[product][identifier] : [context.keyIndices[identifier]]);
    assert.deepEqual(indices.sort((left, right) => left - right), group.indices);
    assert.equal(new Set(indices).size, indices.length);
  });
  for (let profile = 0; profile < 3; profile++) {
    for (let layer = 0; layer < 3; layer++) {
      for (const sample of fixture.samples) {
        test(`${group.label}: profile ${profile}, layer ${layer}, RGB ${sample.rgb}, breathing ${sample.breathing}`, () => {
          const state = { keyLightingStates: identifiers.map(keyIdentifier => ({
            keyIdentifier, breathing: sample.breathing, color: { toArray_rgb: () => sample.rgb },
          })) };
          const data = Buffer.alloc(800);
          for (const index of group.indices) data.set([sample.breathing ? 254 : 255, index, ...sample.rgb], index * 5);
          for (let page = 0; page < 4; page++) {
            const report = Buffer.alloc(264);
            context.encodePage(report, { supportedDeviceData: { productId: product } }, profile, layer, state, page);
            const expected = Buffer.alloc(264);
            expected.set([7, 6, profile + 1, layer + 1, 83, page, 20]);
            expected.set(data.subarray(page * 200, (page + 1) * 200), 8);
            assert.deepEqual(report, expected);
          }
        });
      }
    }
  }
}
