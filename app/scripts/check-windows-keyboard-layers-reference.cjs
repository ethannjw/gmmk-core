'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const vm = require('node:vm');

const root = process.argv[2];
assert.ok(root, 'Usage: node app/scripts/check-windows-keyboard-layers-reference.cjs /path/to/CORE-2.1.23/app');
function verified(path, expected) {
  const bytes = readFileSync(join(root, path));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), expected, 'Review changed source before executing helpers');
  return bytes.toString('utf8');
}
const renderer = verified('out/renderer-process/assets/index-401429d5.js', '9b8adecdf2957e81d2d6e674af5b6759c138565612d8cbadd67c8049d9c2f4ed');
const css = verified('out/renderer-process/assets/index-cfb93e42.css', 'cfb93e428172b9cfff343bd4f87837f83e693aeb6c5f198ff397322bd9bc84bd');
const translations = JSON.parse(verified('out/renderer-process/i18n/en.json',
  'd5f45100b2047600d95620e62726d1243d0740cf4051b5cbd84e5327a903f5bf'));
function range(source, startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, startMarker);
  assert.equal(source.indexOf(startMarker, start + startMarker.length), -1, `Ambiguous ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, endMarker);
  return source.slice(start, end);
}
function harness(success = true) {
  const calls = [];
  const element = (type, props) => ({ type, props });
  const state = vm.createContext({
    jsxRuntimeExports: { jsx: element, jsxs: element },
    classNames$1: (...values) => values.filter(Boolean).join(' '),
    TranslationsDisplayContext: 'translations',
    reactExports: { useMemo: callback => callback(), useContext: () => translations },
    Tooltip: 'tooltip', Position: { LeftBottom: 'left-bottom' },
    deviceSupportData: { productId: 'GMMKV296ANSI' },
    deviceState: { currentProfileData: { activeLayerIndex: 0 } },
    DeviceChannel: { RequestLayerChange: 'RequestLayerChange', PropertyUpdate: 'PropertyUpdate' },
    IPCService: { invoke: async (channel, payload) => {
      calls.push(['request', channel, payload.layerIndex]);
      return { success, data: success ? { selectedLayer: payload.layerIndex } : 'rejected' };
    } },
    AppEvent: { publish: (channel, data) => calls.push(['publish', channel, data.selectedLayer]) },
    Logger: { logError: value => calls.push(['error', value]) }, console: { log() {} },
  }, { codeGeneration: { strings: false, wasm: false } });
  const page = range(renderer, 'function DeviceKeybindingPage({ deviceState }) {', '\nconst ');
  vm.runInContext(`${range(renderer, 'function useTranslate() {', 'const TranslationsContext =')}
    ${range(renderer, 'const style$1e = {', 'const PREVIEW_BOX_HEIGHT =')}
    ${range(page, '  const handleLayerChange = async (layerIndex) => {', '  return /* @__PURE__ */ jsxRuntimeExports.jsxs(')}
    this.api = { render: KeyboardLayerSelector, change: handleLayerChange, labelsMode: layerLabelsMode };`, state, { timeout: 1000 });
  return { api: state.api, calls };
}

test('layer help uses the actual translation fallback and left-bottom information tooltip', () => {
  const { api } = harness();
  const tree = api.render({ value: 0, labelsMode: api.labelsMode, onChange() {} });
  const help = tree.props.children[0].props.children[1];
  assert.equal(help.type, 'tooltip');
  assert.equal(help.props.position, 'left-bottom');
  assert.equal(help.props.title, 'Keyboard Layer');
  assert.equal(help.props.content, translations.find(entry => entry.key === 'Tooltip_Keybinding_Keyboard_Layer_Description').Value);
  assert.ok(help.props.content.startsWith('Your keyboard has 3 total layers'));
});

test('layer help responds to hover and focus, uses a noninteractive 275px panel and scoped transition', () => {
  const component = range(renderer, 'const Tooltip = ({', 'const moreButtonContainer =');
  for (const handler of ['onMouseOver: () => setShowTooltip(true)', 'onMouseOut: () => setShowTooltip(false)',
    'onFocus: () => setShowTooltip(true)', 'onBlur: () => setShowTooltip(false)', 'tabIndex: 0',
    'setShowTooltips(appSettings?.tooltip == true)', 'if (!showTooltips)']) assert.ok(component.includes(handler), handler);
  const tooltip = range(css, '}._tooltipContainer_houh9_1', '._heading1_17yvo_1');
  for (const declaration of ['--icon-size: 18px;', 'width: 275px;', 'pointer-events: none;',
    'padding: calc(var(--space) - 10px);', 'border: var(--border-thin) solid var(--atomic-purple-50);',
    'background-color: var(--base-shades-80);', 'transform: translateY(-5px);',
    'left: calc(var(--tooltip-content-width) * -1 + var(--icon-size));',
    'bottom: calc(var(--tooltip-content-height) * -1);']) assert.ok(tooltip.includes(declaration), declaration);
  const heading = range(css, '._layer-selector__heading_htzxq_297 {', '._layer-selector__title_htzxq_315');
  assert.ok(heading.includes('gap: 6px;'));
  for (const declaration of ['--space: 16px;', '--border-thin: 1px;', '--border-radius-thin: 2px;',
    '--atomic-purple-50: #8477be;', '--base-shades-80: #262626;', '--base-shades-30: #d9d9d9;']) {
    assert.ok(css.includes(declaration), declaration);
  }
  const typography = range(css, '._paragraphmedium_17yvo_81 {', '._paragraphxsmallbold_17yvo_113,');
  for (const declaration of ['font-size: 14px;', 'line-height: 20px;', 'font-size: 12px;', 'line-height: 16px;']) {
    assert.ok(typography.includes(declaration), declaration);
  }
});

for (const selected of [0, 1, 2]) {
  test(`GMMK 2 numeric layer ${selected + 1} exposes three tab buttons with selection and callbacks`, () => {
    const { api } = harness();
    assert.equal(api.labelsMode, 'number');
    const changes = [];
    const tree = api.render({ value: selected, labelsMode: api.labelsMode, onChange: value => changes.push(value) });
    const group = tree.props.children[1];
    assert.equal(group.props.role, 'tablist');
    const buttons = Array.from(group.props.children);
    assert.deepEqual(buttons.map(button => [button.type, button.props.role, button.props.children, button.props['aria-selected']]),
      [0, 1, 2].map(index => ['button', 'tab', `Layer ${index + 1}`, index === selected]));
    for (const button of buttons) button.props.onClick();
    assert.deepEqual(changes, [0, 1, 2]);
    assert.equal(tree.props.children[0].props.children[0].props.children, 'Keyboard Layer');
  });
}

test('successful layer selection requests IPC then publishes returned state', async () => {
  const { api, calls } = harness();
  await api.change(2);
  assert.deepEqual(calls, [['request', 'RequestLayerChange', 2], ['publish', 'PropertyUpdate', 2]]);
});

test('failed layer selection logs rejection without publishing a changed state', async () => {
  const { api, calls } = harness(false);
  await api.change(1);
  assert.deepEqual(calls, [['request', 'RequestLayerChange', 1], ['error', 'rejected']]);
});

test('layer selector dimensions, alignment and palette resolve from its scoped CSS', () => {
  const selector = range(css, '}._layer-selector_htzxq_1', '._page_pi1os_1');
  for (const declaration of ['--color-surface-1: #131313;', '--color-border: #262626;', '--color-text: #ebebeb;',
    '--color-text-muted: #a8a8a8;', '--color-accent: #f5a43a;', '--color-accent-ink: #1a1003;',
    '--font-size-md: 13px;', '--space-2: 8px;', '--radius-pill: 999px;',
    'align-items: flex-end;', 'margin-bottom: 24px;', 'padding: 3px;', 'gap: 2px;',
    'min-width: 72px;', 'height: 30px;', 'padding: 0 18px;']) assert.ok(selector.includes(declaration), declaration);
  assert.ok(range(selector, '._layer-selector__btn_htzxq_341:hover {', '}').includes('color: var(--color-text);'));
  const active = range(selector, '._layer-selector__btn--active_htzxq_379 {', '}');
  assert.ok(active.includes('background: var(--color-accent);'));
  assert.ok(active.includes('color: var(--color-accent-ink);'));
});

test('layer option colors transition for 120ms with CSS ease and selected shadow is separate', () => {
  const selector = range(css, '}._layer-selector_htzxq_1', '._page_pi1os_1');
  assert.ok(selector.includes('--transition-fast: 120ms ease;'));
  assert.ok(selector.includes('transition: background var(--transition-fast), color var(--transition-fast);'));
  assert.ok(selector.includes('--shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.35);'));
  assert.ok(selector.includes('box-shadow: var(--shadow-sm);'));
});
