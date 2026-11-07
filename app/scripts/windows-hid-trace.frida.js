'use strict';

globalThis.createGmmkTraceAdapter = function ({ recorder, diagnostic }) {
  const listeners = [];
  const attached = new Set();
  const depths = new Map();
  const names = ['CreateFileW', 'CreateFileA', 'ReadFile', 'WriteFile', 'DeviceIoControl',
    'GetOverlappedResult', 'GetOverlappedResultEx', 'CloseHandle',
    'HidD_GetFeature', 'HidD_GetInputReport', 'HidD_SetFeature', 'HidD_SetOutputReport'];
  let observer;
  let stopped = false;
  let callbackErrors = 0;

  function count(pointer) {
    if (!pointer || pointer.isNull()) return null;
    try { return pointer.readU32(); } catch (_) { callbackErrors++; return null; }
  }
  function callbacks(name) {
    function enter(args) {
      this.name = name;
      if (name.startsWith('CreateFile')) {
        const wide = name.endsWith('W');
        this.path = '';
        for (let index = 0; index < 1024; index++) {
          const character = wide ? args[0].add(index * 2).readU16() : args[0].add(index).readU8();
          if (character === 0) return;
          this.path += String.fromCharCode(character);
        }
        this.path = null;
        return;
      }
      this.handle = args[0].toString();
      if (name === 'CloseHandle') return;
      if (name.startsWith('GetOverlappedResult')) {
        this.operation = recorder.pending(this.handle, args[1].toString());
        this.byteCount = args[2];
        return;
      }
      const options = { api: name, handle: this.handle };
      if (name === 'DeviceIoControl') {
        Object.assign(options, { code: args[1].toUInt32(), input: args[2], inputLength: args[3].toUInt32(),
          output: args[4], outputLength: args[5].toUInt32(), overlapped: args[7].isNull() ? null : args[7].toString() });
        this.byteCount = args[6];
      } else if (name === 'ReadFile' || name === 'WriteFile') {
        Object.assign(options, name === 'ReadFile' ? { output: args[1], outputLength: args[2].toUInt32() } :
          { input: args[1], inputLength: args[2].toUInt32(), byteCountKind: 'written' });
        options.overlapped = args[4].isNull() ? null : args[4].toString();
        this.byteCount = args[3];
      } else if (name.startsWith('HidD_')) {
        const length = args[2].toUInt32();
        Object.assign(options, name.startsWith('HidD_Get') ?
          { input: args[1], inputLength: Math.min(length, 1), output: args[1], outputLength: length, lengthBasis: 'buffer-capacity' } :
          { input: args[1], inputLength: length });
      }
      this.operation = recorder.begin(options);
    }
    function leave(result) {
      const success = name.startsWith('HidD_') ? (result.toUInt32() & 0xff) !== 0 : result.toInt32() !== 0;
      const error = this.lastError;
      if (name.startsWith('CreateFile')) {
        if (!result.equals(ptr(-1))) recorder.open(result.toString(), this.path);
      } else if (name === 'CloseHandle') {
        if (success) recorder.close(this.handle);
      } else if (this.operation) {
        const outcome = { success, error, bytesReturned: success ? count(this.byteCount) : null };
        if (name.startsWith('GetOverlappedResult')) recorder.complete(this.operation, outcome);
        else recorder.end(this.operation, outcome);
      }
    }
    return {
      onEnter(args) {
        this.depthKey = `${this.threadId}:${name}`;
        const depth = (depths.get(this.depthKey) ?? 0) + 1;
        depths.set(this.depthKey, depth);
        this.suppressed = depth > 1;
        if (this.suppressed) return;
        try { enter.call(this, args); } catch (_) { this.operation = null; callbackErrors++; }
      },
      onLeave(result) {
        try { if (!this.suppressed) leave.call(this, result); } catch (_) { callbackErrors++; }
        finally {
          const depth = (depths.get(this.depthKey) ?? 1) - 1;
          if (depth === 0) depths.delete(this.depthKey);
          else depths.set(this.depthKey, depth);
        }
      },
    };
  }
  function attach(name, address) {
    const identity = `${name}:${address}`;
    if (stopped || attached.has(identity)) return;
    listeners.push(Interceptor.attach(address, callbacks(name)));
    attached.add(identity);
    diagnostic({ type: 'hook', api: name, address: address.toString() });
  }
  function observeModules() {
    observer = Process.attachModuleObserver({
      onAdded(module) {
        if (stopped) return;
        const moduleName = module.name.toLowerCase();
        if (moduleName === 'kernelbase.dll' || moduleName === 'hid.dll') {
          for (const name of names) {
            try {
              const address = module.findExportByName(name);
              if (address) attach(name, address);
            } catch (error) { diagnostic({ type: 'hook-error', api: name, error: String(error) }); }
          }
        }
        if (moduleName.endsWith('.node')) {
          try {
            const imports = module.enumerateImports().filter(entry => names.includes(entry.name)).map(entry => {
              if (entry.address) {
                try { attach(entry.name, entry.address); }
                catch (error) { diagnostic({ type: 'hook-error', api: entry.name, error: String(error) }); }
              }
              return { api: entry.name, address: entry.address?.toString() ?? null,
                hooked: entry.address ? attached.has(`${entry.name}:${entry.address}`) : false };
            });
            diagnostic({ type: 'native-imports', module: module.name, imports });
          } catch (error) { diagnostic({ type: 'import-error', module: module.name, error: String(error) }); }
        }
      },
    });
    Interceptor.flush();
  }
  function stop() {
    if (stopped) return;
    stopped = true;
    if (observer) observer.detach();
    for (const listener of listeners) listener.detach();
    Interceptor.flush();
    diagnostic({ type: 'hooks-detached', count: listeners.length, callbackErrors });
  }
  return { callbacks, attach, observeModules, stop };
};
