const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("guru", {
  app: { info: () => ipcRenderer.invoke("app:info") },
});
