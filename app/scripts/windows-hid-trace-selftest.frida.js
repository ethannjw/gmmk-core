'use strict';

globalThis.runGmmkTraceSelfTest = function (emit) {
  const events = [];
  const retained = [];
  let callbackErrors = 0;
  const recorder = GmmkHidTrace.createRecorder({
    emit: event => { events.push(event); emit({ source: 'synthetic', ...event }); },
    readBytes: (pointer, length) => new Uint8Array(pointer.readByteArray(length)),
  });
  const adapter = createGmmkTraceAdapter({ recorder, diagnostic: event => {
    if (event.type === 'hooks-detached') callbackErrors += event.callbackErrors;
    emit({ source: 'synthetic', ...event });
  }});
  const handle = ptr(0x77770001);
  const otherHandle = ptr(0x77770002);
  const output = Memory.alloc(65);
  const input = Memory.alloc(65);
  const count = Memory.alloc(4);
  const overlap = Memory.alloc(32);
  input.writeByteArray(Array.from({ length: 65 }, (_, index) => index));
  const checks = [];
  function check(condition, name) {
    if (!condition) throw new Error(`Self-test failed: ${name}`);
    checks.push(name);
  }
  function native(name, returnType, types, implementation) {
    const address = new NativeCallback(implementation, returnType, types);
    retained.push(address);
    adapter.attach(name, address);
    Interceptor.flush();
    return new SystemFunction(address, returnType, types);
  }
  function transfers() { return events.filter(event => event.type === 'transfer'); }
  try {
    const open = native('CreateFileW', 'pointer', ['pointer','uint','uint','pointer','uint','uint','pointer'], function () { return handle; });
    const path = Memory.allocUtf16String('\\\\?\\HID#VID_320F&PID_504B&MI_01#SYNTHETIC');
    open(path, 0, 0, ptr(0), 3, 0, ptr(0));
    check(events.some(event => event.type === 'open'), 'target-handle-registration');

    const ioctlTypes = ['pointer','uint','pointer','uint','pointer','uint','pointer','pointer'];
    let earlyCompletion;
    const ioctl = native('DeviceIoControl', 'int', ioctlTypes, function (device, code, source, sourceLength, destination, capacity, transferred) {
      if (code === 1) { destination.writeByteArray([7,11,22,33]); transferred.writeU32(4); return 1; }
      if (code === 4) { earlyCompletion(); this.lastError = 997; return 0; }
      this.lastError = code === 2 ? 997 : 6;
      return 0;
    });
    ioctl(handle, 1, input, 65, output, 65, count, ptr(0));
    check(JSON.stringify(transfers().at(-1).input.bytes) === JSON.stringify(Array.from({length:65}, (_, index) => index)), 'ioctl-input-exact-65');
    check(JSON.stringify(transfers().at(-1).output.bytes) === '[7,11,22,33]', 'ioctl-output-count');
    const failed = ioctl(handle, 3, input, 65, output, 65, count, ptr(0));
    check(failed.lastError === 6 && transfers().at(-1).error === 6 && transfers().at(-1).output === null, 'failed-output-and-last-error');
    const beforeUnrelated = events.length;
    ioctl(otherHandle, 3, ptr(0), 65, ptr(0), 65, count, ptr(0));
    check(events.length === beforeUnrelated, 'unrelated-handle-filter');

    ioctl(handle, 2, input, 65, output, 65, count, overlap);
    check(transfers().at(-1).status === 'pending' && transfers().at(-1).output === null, 'pending-output-not-read');
    const completion = native('GetOverlappedResult', 'int', ['pointer','pointer','pointer','int'], function (device, operation, transferred, wait) {
      if (!wait) { this.lastError = 996; return 0; }
      output.writeByteArray([7,44,55]); transferred.writeU32(3); return 1;
    });
    completion(handle, overlap, count, 0);
    check(recorder.pending(handle.toString(), overlap.toString()) !== null, 'incomplete-retains-pending');
    completion(handle, overlap, count, 1);
    check(JSON.stringify(transfers().at(-1).output.bytes) === '[7,44,55]', 'overlapped-completion-output');
    earlyCompletion = () => completion(handle, overlap, count, 1);
    ioctl(handle, 4, input, 65, output, 65, count, overlap);
    check(transfers().at(-1).status === 'success' && recorder.pending(handle.toString(), overlap.toString()) === null,
      'completion-before-issuer-return');

    const read = native('ReadFile', 'int', ['pointer','pointer','uint','pointer','pointer'], function (device, destination, capacity, transferred, operation) {
      if (!operation.isNull()) { this.lastError = 997; return 0; }
      destination.writeByteArray([7,66]); transferred.writeU32(2); return 1;
    });
    read(handle, output, 65, count, ptr(0));
    check(JSON.stringify(transfers().at(-1).output.bytes) === '[7,66]', 'readfile-output-count');
    read(handle, output, 65, count, overlap);
    const completionEx = native('GetOverlappedResultEx', 'int', ['pointer','pointer','pointer','uint','int'], function (device, operation, transferred, timeout) {
      this.lastError = timeout === 2 ? 258 : timeout === 3 ? 192 : 995;
      return 0;
    });
    completionEx(handle, overlap, count, 2, 0);
    check(recorder.pending(handle.toString(), overlap.toString()) !== null, 'timeout-retains-pending');
    completionEx(handle, overlap, count, 3, 1);
    check(recorder.pending(handle.toString(), overlap.toString()) !== null, 'alertable-wait-retains-pending');
    completionEx(handle, overlap, count, 1, 0);
    check(transfers().at(-1).error === 995 && transfers().at(-1).output === null, 'cancelled-read-no-output');

    const write = native('WriteFile', 'int', ['pointer','pointer','uint','pointer','pointer'], function (device, source, length, transferred) { transferred.writeU32(length); return 1; });
    write(handle, input, 65, count, ptr(0));
    check(transfers().at(-1).input.bytes.length === 65 && transfers().at(-1).output === null && !transfers().at(-1).lengthExceedsCapacity, 'writefile-input-only');

    for (const name of ['HidD_GetFeature','HidD_GetInputReport','HidD_SetFeature','HidD_SetOutputReport']) {
      const operation = native(name, 'uchar', ['pointer','pointer','uint'], function (device, buffer) {
        if (name.startsWith('HidD_Get')) buffer.writeByteArray([7,77,88]);
        return 1;
      });
      output.writeByteArray(new Uint8Array(65).fill(0)); output.writeU8(7);
      operation(handle, output, 65);
      const event = transfers().at(-1);
      check(name.startsWith('HidD_Get') ? event.input.bytes[0] === 7 && event.output.bytes[1] === 77 &&
        event.lengthBasis === 'buffer-capacity' && event.bytesReturned === null : event.input.bytes.length === 65 && event.output === null, name);
    }
    read(handle, output, 65, count, overlap);
    const close = native('CloseHandle', 'int', ['pointer'], function () { return 1; });
    close(handle);
    check(recorder.pending(handle.toString(), overlap.toString()) === null && transfers().at(-1).reason === 'handle-closed', 'close-abandons-pending');
    const openAnsi = native('CreateFileA', 'pointer', ['pointer','uint','uint','pointer','uint','uint','pointer'], function () { return handle; });
    openAnsi(Memory.allocUtf8String('\\\\?\\HID#VID_320F&PID_504B#SYNTHETIC'), 0, 0, ptr(0), 3, 0, ptr(0));
    check(events.at(-1).type === 'open', 'ansi-open');
    close(handle);
    return { passed: true, checks };
  } finally {
    adapter.stop();
    recorder.stop();
    if (callbackErrors !== 0) throw new Error(`Synthetic callback errors: ${callbackErrors}`);
  }
};

globalThis.testGmmkImportedApi = function (address, emit) {
  const events = [];
  const recorder = GmmkHidTrace.createRecorder({
    emit: event => { events.push(event); emit({source:'synthetic-import-smoke', ...event}); },
    readBytes: (pointer, length) => new Uint8Array(pointer.readByteArray(length)),
  });
  const adapter = createGmmkTraceAdapter({recorder, diagnostic: event => emit({source:'synthetic-import-smoke', ...event})});
  try {
    const invalid = ptr(-1);
    recorder.open(invalid.toString(), '\\\\?\\HID#VID_320F&PID_504B#SYNTHETIC-INVALID');
    adapter.attach('DeviceIoControl', ptr(address));
    Interceptor.flush();
    const input = Memory.alloc(65);
    const output = Memory.alloc(65);
    const count = Memory.alloc(4);
    const expected = Array.from({length:65}, (_, index) => index);
    input.writeByteArray(expected);
    output.writeByteArray(new Uint8Array(65).fill(0xa5));
    count.writeU32(0);
    const invoke = new SystemFunction(ptr(address), 'int', ['pointer','uint','pointer','uint','pointer','uint','pointer','pointer']);
    const result = invoke(invalid, 0x000b0192, input, 65, output, 65, count, ptr(0));
    const transfer = events.find(event => event.type === 'transfer');
    return {address, passed: result.value === 0 && result.lastError === 6 && transfer?.error === 6 &&
      transfer.output === null && JSON.stringify(transfer.input.bytes) === JSON.stringify(expected)};
  } finally {
    adapter.stop();
    recorder.stop();
  }
};
