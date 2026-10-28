const { contextBridge, ipcRenderer } = require("electron");

// Sem lógica. A resposta é o envelope do main (ver `tratar` em main.js);
// quem desembrulha é renderer/js/api.js.
contextBridge.exposeInMainWorld("guru", {
  app: { info: () => ipcRenderer.invoke("app:info") },
  investimentos: {
    catalogo: () => ipcRenderer.invoke("investimentos:catalogo"),
    listar: () => ipcRenderer.invoke("investimentos:listar"),
    ler: (id) => ipcRenderer.invoke("investimentos:ler", id),
    criar: (dados) => ipcRenderer.invoke("investimentos:criar", dados),
    atualizar: (id, dados) => ipcRenderer.invoke("investimentos:atualizar", id, dados),
    apagar: (id) => ipcRenderer.invoke("investimentos:apagar", id),
  },
  carteira: { resumo: () => ipcRenderer.invoke("carteira:resumo") },
});
