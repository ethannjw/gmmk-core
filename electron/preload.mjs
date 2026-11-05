import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("gmmk", {
  scan: () => ipcRenderer.invoke("gmmk:scan"),
  connect: () => ipcRenderer.invoke("gmmk:connect"),
  disconnect: () => ipcRenderer.invoke("gmmk:disconnect"),
  state: () => ipcRenderer.invoke("gmmk:state"),
  meta: () => ipcRenderer.invoke("gmmk:meta"),
  updateSettings: (patch) => ipcRenderer.invoke("gmmk:update-settings", patch),
  applyLighting: () => ipcRenderer.invoke("gmmk:apply-lighting"),
  applyPolling: (hz) => ipcRenderer.invoke("gmmk:apply-polling", hz),
  switchProfile: (profileIndex, layerIndex) =>
    ipcRenderer.invoke("gmmk:switch-profile", profileIndex, layerIndex),
});
