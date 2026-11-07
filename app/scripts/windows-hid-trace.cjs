'use strict';

function createRecorder({ emit, readBytes, maxBytes = 65, maxEvents = 1000, maxPending = 128, maxHandles = 32 }) {
  for (const limit of [maxBytes, maxEvents, maxPending, maxHandles]) {
    if (!Number.isSafeInteger(limit) || limit < 1) throw new Error('Limits must be positive integers');
  }
  const handles = new Map();
  const waiting = new Map();
  let sequence = 0;
  let emitted = 0;
  let dropped = 0;
  let stopped = false;

  function record(event) {
    if (emitted >= maxEvents) { dropped++; return; }
    emitted++;
    emit({ time: new Date().toISOString(), ...event });
  }
  function snapshot(buffer, length) {
    const count = Math.min(length, maxBytes);
    try {
      const bytes = count === 0 ? [] : Array.from(readBytes(buffer, count));
      if (bytes.length !== count) throw new Error('Short memory snapshot');
      return { bytes, requestedLength: length, truncated: count < length };
    } catch (error) {
      return { bytes: null, requestedLength: length, truncated: count < length, error: String(error) };
    }
  }
  function key(handle, overlapped) { return `${handle}|${overlapped}`; }
  function abandon(operation, reason) {
    operation.finished = true;
    record({ type: 'transfer', id: operation.id, api: operation.api, status: 'abandoned', reason });
  }
  function close(handle) {
    if (!handles.has(handle)) return;
    for (const [pendingKey, operation] of waiting) {
      if (operation.handle === handle) { abandon(operation, 'handle-closed'); waiting.delete(pendingKey); }
    }
    handles.delete(handle);
    record({ type: 'close', handle });
  }
  function open(handle, path) {
    if (stopped || typeof path !== 'string' || !/^\\\\\?\\hid#vid_320f&pid_504b(?:&|#)/i.test(path)) return;
    if (handles.has(handle)) close(handle);
    if (handles.size >= maxHandles) { dropped++; return; }
    const identity = { path, generation: ++sequence };
    handles.set(handle, identity);
    record({ type: 'open', handle, ...identity });
  }
  function begin(options) {
    if (stopped || !handles.has(options.handle)) return null;
    if (emitted >= maxEvents) { dropped++; return null; }
    const inputLength = options.inputLength ?? 0;
    const outputLength = options.outputLength ?? 0;
    if (![inputLength, outputLength].every(length => Number.isInteger(length) && length >= 0 && length <= 0xffffffff)) {
      record({ type: 'invalid-length', api: options.api });
      return null;
    }
    const operation = { ...options, id: ++sequence, identity: handles.get(options.handle),
      inputLength, outputLength, lengthBasis: options.lengthBasis ?? 'reported',
      inputSnapshot: inputLength ? snapshot(options.input, inputLength) : null, finished: false };
    if (operation.overlapped) {
      const pendingKey = key(operation.handle, operation.overlapped);
      const previous = waiting.get(pendingKey);
      if (previous) { abandon(previous, 'overlapped-reused'); waiting.delete(pendingKey); }
      if (waiting.size >= maxPending) { dropped++; abandon(operation, 'pending-limit'); return null; }
      waiting.set(pendingKey, operation);
    }
    return operation;
  }
  function end(operation, result, completing = false) {
    if (stopped || !operation || operation.finished || handles.get(operation.handle) !== operation.identity) return;
    const countKnown = Number.isInteger(result.bytesReturned) && result.bytesReturned >= 0 && result.bytesReturned <= 0xffffffff;
    const isPending = !completing && operation.overlapped &&
      ((!result.success && result.error === 997) || (result.success && !countKnown && operation.lengthBasis === 'reported'));
    if (isPending) {
      record({ type: 'transfer', id: operation.id, api: operation.api, handle: operation.handle,
        overlapped: operation.overlapped, status: 'pending', input: operation.inputSnapshot, output: null });
      return;
    }
    if (operation.overlapped && waiting.get(key(operation.handle, operation.overlapped)) === operation) {
      waiting.delete(key(operation.handle, operation.overlapped));
    }
    operation.finished = true;
    const length = operation.lengthBasis === 'buffer-capacity' ? operation.outputLength :
      (countKnown ? Math.min(result.bytesReturned, operation.outputLength) : null);
    record({ type: 'transfer', id: operation.id, api: operation.api, handle: operation.handle,
      generation: operation.identity.generation, code: operation.code ?? null,
      overlapped: operation.overlapped ?? null, status: result.success ? 'success' : 'failed',
      error: result.success ? null : result.error ?? null, input: operation.inputSnapshot,
      output: result.success && operation.outputLength > 0 && length !== null ? snapshot(operation.output, length) : null,
      outputCapacity: operation.outputLength, lengthBasis: operation.lengthBasis,
      byteCountKind: operation.byteCountKind ?? 'returned',
      bytesReturned: result.success && countKnown ? result.bytesReturned : null,
      lengthUnknown: !!result.success && operation.outputLength > 0 && length === null,
      lengthExceedsCapacity: !!result.success && countKnown && result.bytesReturned >
        (operation.byteCountKind === 'written' ? operation.inputLength : operation.outputLength),
    });
  }
  function pending(handle, overlapped) { return waiting.get(key(handle, overlapped)) ?? null; }
  function complete(operation, result) {
    if (!operation || stopped || waiting.get(key(operation.handle, operation.overlapped)) !== operation) return;
    if (!result.success && [996, 258, 192].includes(result.error)) return;
    waiting.delete(key(operation.handle, operation.overlapped));
    end(operation, result, true);
  }
  function stop() {
    if (stopped) return;
    const pendingAtStop = waiting.size;
    for (const operation of waiting.values()) abandon(operation, 'capture-stopped');
    waiting.clear();
    handles.clear();
    stopped = true;
    emit({ type: 'summary', time: new Date().toISOString(), emitted, dropped, pendingAtStop });
  }
  return { open, close, begin, end, pending, complete, stop };
}

if (typeof module !== 'undefined') module.exports = { createRecorder };
else globalThis.GmmkHidTrace = { createRecorder };
