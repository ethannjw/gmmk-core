import { app, BrowserWindow, ipcMain } from "electron";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { GmmkDeviceService } from "../src/device-service.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const service = new GmmkDeviceService();

/** @type {BrowserWindow | null} */
let mainWindow = null;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1100,
    height: 720,
    minWidth: 900,
    minHeight: 600,
    title: "GMMK Core Mac",
    webPreferences: {
      preload: path.join(__dirname, "preload.mjs"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  mainWindow.loadFile(path.join(__dirname, "../ui/index.html"));
}

app.whenReady().then(() => {
  createWindow();
  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on("window-all-closed", () => {
  service.disconnect();
  if (process.platform !== "darwin") {
    app.quit();
  }
});

ipcMain.handle("gmmk:scan", () => service.scan());

ipcMain.handle("gmmk:connect", () => {
  try {
    return { ok: true, info: service.connect(), state: service.getState() };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
});

ipcMain.handle("gmmk:disconnect", () => {
  service.disconnect();
  return { ok: true };
});

ipcMain.handle("gmmk:state", () => service.getState());

ipcMain.handle("gmmk:meta", () => ({
  effects: service.getEffectLabels(),
  pollingRates: service.getPollingRates(),
}));

ipcMain.handle("gmmk:update-settings", (_evt, patch) => {
  service.updateSettings(patch);
  return service.getState();
});

ipcMain.handle("gmmk:apply-lighting", async () => {
  try {
    await service.applyLighting();
    return { ok: true, state: service.getState() };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
});

ipcMain.handle("gmmk:apply-polling", async (_evt, hz) => {
  try {
    await service.applyPollingRate(hz);
    return { ok: true, state: service.getState() };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
});

ipcMain.handle("gmmk:switch-profile", async (_evt, profileIndex, layerIndex = 0) => {
  try {
    await service.switchProfile(profileIndex, layerIndex);
    return { ok: true, state: service.getState() };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
});
