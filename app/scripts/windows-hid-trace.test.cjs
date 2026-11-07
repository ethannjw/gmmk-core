const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const { createRecorder } = require('./windows-hid-trace.cjs');

const target = '\\\\?\\HID#VID_320F&PID_504B&MI_01#fixture';
function fixture(options = {}) {
  const events = [];
  const reads = [];
  const recorder = createRecorder({
    emit: event => events.push(event),
    readBytes: (buffer, length) => {
      reads.push(length);
      if (buffer === 'unreadable') throw new Error('unreadable');
      return buffer.slice(0, length);
    },
    ...options,
  });
  recorder.open('handle', target);
  const begin = (extra = {}) => recorder.begin({ api: 'ReadFile', handle: 'handle',
    output: [7, 8, 9, 10], outputLength: 4, ...extra });
  return { recorder, begin, events, reads };
}

test('filters unrelated handles and paths before reading any memory', () => {
  const { recorder, reads } = fixture();
  for (const path of ['C:\\VID_320F&PID_504B.txt', target.replace('504B', '504B0'), target.replace('320F', '320E')]) {
    recorder.open('other', path);
    assert.equal(recorder.begin({api:'ReadFile',handle:'other',input:'unreadable',inputLength:65}), null);
  }
  assert.deepEqual(reads, []);
});

test('records only the reported number of successful output bytes', () => {
  const { recorder, begin, events } = fixture();
  recorder.end(begin(), {success:true, bytesReturned:2});
  const transfer = events.at(-1);
  assert.equal(transfer.status, 'success');
  assert.deepEqual(transfer.output.bytes, [7, 8]);
  assert.equal(transfer.bytesReturned, 2);
});

test('failed reads do not expose stale output memory', () => {
  const { recorder, begin, events, reads } = fixture();
  recorder.end(begin({output:'unreadable'}), {success:false,error:6,bytesReturned:65});
  assert.equal(events.at(-1).status, 'failed');
  assert.equal(events.at(-1).output, null);
  assert.deepEqual(reads, []);
});

test('pending reads wait for completion and correlate by handle and overlapped pointer', () => {
  const { recorder, begin, events, reads } = fixture();
  const operation = begin({overlapped:'overlap'});
  recorder.end(operation, {success:false,error:997});
  assert.deepEqual(reads, []);
  assert.equal(events.at(-1).status, 'pending');
  assert.equal(recorder.pending('other', 'overlap'), null);
  recorder.complete(recorder.pending('handle','overlap'), {success:false,error:996});
  assert.deepEqual(reads, []);
  recorder.complete(recorder.pending('handle','overlap'), {success:true,bytesReturned:3});
  assert.deepEqual(events.at(-1).output.bytes, [7,8,9]);
  assert.equal(events.at(-1).id, operation.id);
  assert.equal(recorder.pending('handle','overlap'), null);
});

test('cancellation records failure without reading output', () => {
  const { recorder, begin, events, reads } = fixture();
  recorder.end(begin({overlapped:'overlap'}), {success:false,error:997});
  recorder.complete(recorder.pending('handle','overlap'), {success:false,error:995});
  assert.equal(events.at(-1).error, 995);
  assert.equal(events.at(-1).output, null);
  assert.deepEqual(reads, []);
});

test('HidD buffer snapshots are labelled as capacity, not transferred bytes', () => {
  const { recorder, begin, events } = fixture();
  recorder.end(begin({lengthBasis:'buffer-capacity'}), {success:true});
  assert.equal(events.at(-1).lengthBasis, 'buffer-capacity');
  assert.equal(events.at(-1).bytesReturned, null);
  assert.deepEqual(events.at(-1).output.bytes, [7,8,9,10]);
});

test('overlapped immediate success without a byte count waits for completion', () => {
  const { recorder, begin, events, reads } = fixture();
  recorder.end(begin({overlapped:'overlap'}), {success:true,bytesReturned:null});
  assert.equal(events.at(-1).status, 'pending');
  assert.deepEqual(reads, []);
  recorder.complete(recorder.pending('handle','overlap'), {success:true,bytesReturned:1});
  assert.deepEqual(events.at(-1).output.bytes, [7]);
});

test('input is snapshotted before the caller mutates its buffer', () => {
  const { recorder, begin, events } = fixture();
  const input = [7,1,2];
  const operation = begin({input,inputLength:3});
  input[1] = 99;
  recorder.end(operation, {success:true,bytesReturned:0});
  assert.deepEqual(events.at(-1).input.bytes, [7,1,2]);
  assert.deepEqual(events.at(-1).output.bytes, []);
});

test('captures are capped and inconsistent lengths are explicit', () => {
  const { recorder, begin, events, reads } = fixture({maxBytes:2});
  recorder.end(begin(), {success:true,bytesReturned:99});
  assert.deepEqual(events.at(-1).output.bytes, [7,8]);
  assert.equal(events.at(-1).output.truncated, true);
  assert.equal(events.at(-1).lengthExceedsCapacity, true);
  assert.deepEqual(reads, [2]);
});

test('unreadable memory and unknown output length remain explicit', () => {
  const { recorder, begin, events } = fixture();
  recorder.end(begin({output:'unreadable'}), {success:true,bytesReturned:2});
  assert.match(events.at(-1).output.error, /unreadable/);
  recorder.end(begin(), {success:true});
  assert.equal(events.at(-1).output, null);
  assert.equal(events.at(-1).lengthUnknown, true);
});

test('closing a handle abandons pending reads and prevents stale reuse', () => {
  const { recorder, begin, events } = fixture();
  recorder.end(begin({overlapped:'overlap'}), {success:false,error:997});
  const stale = recorder.pending('handle','overlap');
  recorder.close('handle');
  assert.equal(events.find(event => event.id === stale.id && event.status === 'abandoned').reason, 'handle-closed');
  recorder.open('handle', target);
  assert.equal(recorder.pending('handle','overlap'), null);
  const count = events.length;
  recorder.complete(stale, {success:true,bytesReturned:2});
  assert.equal(events.length, count);
});

test('event and pending limits report loss rather than growing without bound', () => {
  const { recorder, begin, events } = fixture({maxEvents:4,maxPending:1});
  recorder.end(begin({overlapped:'first'}), {success:false,error:997});
  recorder.end(begin({overlapped:'second'}), {success:false,error:997});
  assert.equal(recorder.pending('handle','second'), null);
  for (let index=0;index<20;index++) recorder.end(begin(), {success:true,bytesReturned:2});
  recorder.stop();
  assert(events.length <= 5);
  assert.equal(events.at(-1).type, 'summary');
  assert(events.at(-1).dropped > 0);
  assert.equal(events.at(-1).pendingAtStop, 1);
  assert.equal(recorder.begin({api:'ReadFile',handle:'handle'}), null);
});

test('conflicting reuse of an overlapped pointer abandons the previous operation', () => {
  const { recorder, begin, events } = fixture();
  recorder.end(begin({overlapped:'same'}), {success:false,error:997});
  const first = recorder.pending('handle','same');
  const second = begin({overlapped:'same'});
  recorder.end(second, {success:false,error:997});
  assert(events.some(event => event.id === first.id && event.reason === 'overlapped-reused'));
  assert.equal(recorder.pending('handle','same').id, second.id);
});

test('write byte counts describe the input buffer, not a nonexistent response', () => {
  const { recorder, begin, events } = fixture();
  recorder.end(begin({api:'WriteFile',input:[7,1],inputLength:2,outputLength:0,byteCountKind:'written'}),
    {success:true,bytesReturned:2});
  assert.equal(events.at(-1).lengthExceedsCapacity, false);
  assert.equal(events.at(-1).byteCountKind, 'written');
  assert.equal(events.at(-1).output, null);
});

test('nested forwarding entry points produce one operation per API invocation', () => {
  const context = { globalThis: {}, Interceptor: {} };
  vm.runInNewContext(fs.readFileSync(require.resolve('./windows-hid-trace.frida.js'), 'utf8'), context);
  let begins = 0;
  let ends = 0;
  const adapter = context.globalThis.createGmmkTraceAdapter({ recorder: {
    begin: () => { begins++; return {}; }, end: () => { ends++; },
  }, diagnostic: () => {} });
  const callbacks = adapter.callbacks('ReadFile');
  const numeric = { toUInt32: () => 65, isNull: () => true };
  const args = [{toString: () => 'handle'}, {}, numeric, numeric, numeric];
  const outer = { threadId: 1, lastError: 0 };
  const inner = { threadId: 1, lastError: 0 };
  callbacks.onEnter.call(outer, args);
  callbacks.onEnter.call(inner, args);
  callbacks.onLeave.call(inner, {toInt32: () => 1});
  callbacks.onLeave.call(outer, {toInt32: () => 1});
  assert.equal(begins, 1);
  assert.equal(ends, 1);
});

test('timed and alertable waits retain pending reads until actual completion', () => {
  const { recorder, begin, events, reads } = fixture();
  recorder.end(begin({overlapped:'overlap'}), {success:false,error:997});
  for (const error of [258, 192]) {
    recorder.complete(recorder.pending('handle','overlap'), {success:false,error});
    assert(recorder.pending('handle','overlap'));
  }
  assert.deepEqual(reads, []);
  recorder.complete(recorder.pending('handle','overlap'), {success:true,bytesReturned:2});
  assert.deepEqual(events.at(-1).output.bytes, [7,8]);
});

test('completion on another thread can arrive before the issuing API returns', () => {
  const { recorder, begin, events } = fixture();
  const operation = begin({overlapped:'overlap'});
  recorder.complete(recorder.pending('handle','overlap'), {success:true,bytesReturned:2});
  recorder.end(operation, {success:false,error:997});
  const transfers = events.filter(event => event.type === 'transfer');
  assert.equal(transfers.length, 1);
  assert.equal(transfers[0].status, 'success');
  assert.deepEqual(transfers[0].output.bytes, [7,8]);
  assert.equal(recorder.pending('handle','overlap'), null);
});
