'use strict';

const fs = require('node:fs');
const path = require('node:path');
const [output, guestDirectory] = process.argv.slice(2);
if (!output || !guestDirectory || !/^[A-Za-z]:\\/.test(guestDirectory)) {
  console.error('Usage: node build-windows-hid-trace.cjs <output.js> <absolute-Windows-guest-output-directory>');
  process.exit(1);
}
const sources = ['windows-hid-trace.cjs', 'windows-hid-trace.frida.js', 'windows-hid-trace-selftest.frida.js']
  .map(file => fs.readFileSync(path.join(__dirname, file), 'utf8'));
const wrapper = `
setImmediate(() => {
  const directory = ${JSON.stringify(guestDirectory)};
  const resultPath = directory + '\\\\script-result.json';
  const report = { stage: 'logger-started', processId: Process.id, arch: Process.arch,
    frida: Frida.version, passed: false, durationMs: 20000, hookErrors: 0, callbackErrors: 0,
    unmatchedImports: [], journalDropped: 0, journalError: null };
  File.writeAllText(resultPath, JSON.stringify(report));
  const journal = new File(directory + '\\\\hid-trace.jsonl', 'w');
  let journalCount = 0;
  let adapter;
  let recorder;
  let importedIoctl;
  function emit(event) {
    if (journalCount >= 2500) { report.journalDropped++; return; }
    if (report.journalError) return;
    try { journal.write(JSON.stringify(event) + '\\n'); journal.flush(); journalCount++; }
    catch (error) { report.journalError = String(error); }
  }
  function finish() {
    if (importedIoctl) {
      try { report.importedApiSmoke = testGmmkImportedApi(importedIoctl, emit); }
      catch (error) { report.error = String(error); }
    }
    if (adapter) adapter.stop();
    if (recorder) recorder.stop();
    report.hooksDetached = true;
    report.stage = 'probe-finished';
    report.passed = report.selfTest?.passed === true && report.hookErrors === 0 &&
      report.callbackErrors === 0 && report.unmatchedImports.length === 0 &&
      report.journalDropped === 0 && report.journalError === null &&
      report.importedApiSmoke?.passed === true && !report.error;
    journal.close();
    File.writeAllText(resultPath, JSON.stringify(report));
  }
  try {
    emit({type:'metadata', processId:Process.id, arch:Process.arch, frida:Frida.version,
      physicalAccessApproved:false, scope:'no-device validation', maxBytes:65, maxEvents:1000});
    report.selfTest = runGmmkTraceSelfTest(emit);
    recorder = GmmkHidTrace.createRecorder({
      emit: event => emit({source:'observe', ...event}),
      readBytes: (pointer, length) => new Uint8Array(pointer.readByteArray(length)),
    });
    adapter = createGmmkTraceAdapter({ recorder, diagnostic: event => {
      if (event.type === 'hook-error' || event.type === 'import-error') report.hookErrors++;
      if (event.type === 'hooks-detached') report.callbackErrors += event.callbackErrors;
      if (event.type === 'native-imports') {
        report.unmatchedImports.push(...event.imports.filter(entry => !entry.hooked));
        importedIoctl = event.imports.find(entry => entry.api === 'DeviceIoControl' && entry.hooked)?.address ?? importedIoctl;
      }
      emit({source:'observe', ...event});
    }});
    adapter.observeModules();
    report.stage = 'observing-no-device-startup';
    File.writeAllText(resultPath, JSON.stringify(report));
    setTimeout(finish, report.durationMs);
  } catch (error) {
    report.error = String(error);
    report.stack = error.stack;
    finish();
  }
});
`;
fs.writeFileSync(output, sources.join('\n') + '\n' + wrapper, {flag:'wx'});
console.log(`Built no-device validation bundle: ${output}`);
