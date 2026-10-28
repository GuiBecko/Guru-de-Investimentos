const { contextBridge, ipcRenderer } = require("electron");

// Sem lógica. A resposta é o envelope do main (ver `tratar` em main.js);
// quem desembrulha é renderer/js/api.js.
contextBridge.exposeInMainWorld("guru", {
  app: { info: () => ipcRenderer.invoke("app:info") },
});
