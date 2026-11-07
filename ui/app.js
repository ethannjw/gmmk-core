const statusLine = document.getElementById("status-line");
const scanSummary = document.getElementById("scan-summary");
const effectSelect = document.getElementById("effect-select");
const pollingSelect = document.getElementById("polling-select");

function setStatus(text, kind = "") {
  statusLine.textContent = text;
  statusLine.className = `status ${kind}`;
}

async function bootstrap() {
  if (!window.gmmk) {
    scanSummary.textContent = "Preload failed — window.gmmk is missing.";
    setStatus(
      "Electron preload did not load. Restart with npm start from ~/myapps/gmmk-core-mac.",
      "error",
    );
    return;
  }

  try {
    await loadMeta();
    await refreshScan();
  } catch (err) {
    setStatus(err instanceof Error ? err.message : String(err), "error");
  }
}

async function refreshScan() {
  const scan = await window.gmmk.scan();
  const bridge = scan.swiftBridge ? "IOKit bridge ready" : "IOKit bridge missing (npm run build:hid)";
  const found = scan.configPath || scan.swiftScan?.ok ? "vendor interface found" : "vendor interface missing";
  scanSummary.textContent = `${scan.product ?? "No device"} — VID 0x${scan.vid.toString(16)} PID 0x${scan.pid.toString(16)} — ${found} — ${bridge}`;
}

async function loadMeta() {
  const meta = await window.gmmk.meta();
  const uiValues = [8, 0, 1, 3, 4, 5, 2, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18];
  effectSelect.innerHTML = "";
  meta.effects.forEach((label, i) => {
    const opt = document.createElement("option");
    opt.value = String(uiValues[i] ?? 0);
    opt.textContent = label;
    effectSelect.appendChild(opt);
  });

  pollingSelect.innerHTML = "";
  meta.pollingRates.forEach((hz) => {
    const opt = document.createElement("option");
    opt.value = String(hz);
    opt.textContent = `${hz} Hz`;
    pollingSelect.appendChild(opt);
  });
}

function rgbFromHex(hex) {
  const n = hex.replace("#", "");
  return [parseInt(n.slice(0, 2), 16), parseInt(n.slice(2, 4), 16), parseInt(n.slice(4, 6), 16)];
}

async function syncControlsFromState() {
  const state = await window.gmmk.state();
  document.getElementById("brightness").value = String(state.brightness);
  document.getElementById("speed").value = String(state.speed);
  document.getElementById("side-light").checked = state.sideLightOn;
  pollingSelect.value = String(state.pollingHz);
}

document.getElementById("btn-scan").addEventListener("click", () => refreshScan());

document.getElementById("btn-connect").addEventListener("click", async () => {
  const res = await window.gmmk.connect();
  if (res.ok) {
    setStatus(`Connected via ${res.info.backend} (${res.info.product})`, "ok");
    await syncControlsFromState();
  } else {
    setStatus(res.error ?? "Connect failed", "error");
  }
});

document.getElementById("btn-disconnect").addEventListener("click", async () => {
  await window.gmmk.disconnect();
  setStatus("Disconnected");
});

document.querySelectorAll("[data-profile]").forEach((btn) => {
  btn.addEventListener("click", async () => {
    const profileIndex = Number(btn.getAttribute("data-profile"));
    const layerIndex = Number(document.getElementById("layer-select").value);
    const res = await window.gmmk.switchProfile(profileIndex, layerIndex);
    setStatus(
      res.ok ? `Profile ${profileIndex + 1} layer ${layerIndex + 1}` : res.error,
      res.ok ? "ok" : "error",
    );
  });
});

document.getElementById("btn-apply-light").addEventListener("click", async () => {
  await window.gmmk.updateSettings({
    effectUiValue: Number(effectSelect.value),
    brightness: Number(document.getElementById("brightness").value),
    speed: Number(document.getElementById("speed").value),
    sideLightOn: document.getElementById("side-light").checked,
    rgb: rgbFromHex(document.getElementById("color").value),
    layerIndex: Number(document.getElementById("layer-select").value),
  });
  const res = await window.gmmk.applyLighting();
  setStatus(res.ok ? "Lighting applied" : res.error, res.ok ? "ok" : "error");
});

document.getElementById("btn-apply-polling").addEventListener("click", async () => {
  const hz = Number(pollingSelect.value);
  const res = await window.gmmk.applyPolling(hz);
  setStatus(res.ok ? `Polling ${hz} Hz` : res.error, res.ok ? "ok" : "error");
});

bootstrap();
