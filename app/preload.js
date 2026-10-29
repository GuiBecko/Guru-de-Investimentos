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
  perfil: {
    questionario: () => ipcRenderer.invoke("perfil:questionario"),
    registrar: (respostas) => ipcRenderer.invoke("perfil:registrar", respostas),
    atual: () => ipcRenderer.invoke("perfil:atual"),
  },
  alocacao: {
    ler: () => ipcRenderer.invoke("alocacao:ler"),
    salvar: (valores) => ipcRenderer.invoke("alocacao:salvar", valores),
  },
  objetivo: {
    ler: () => ipcRenderer.invoke("objetivo:ler"),
    salvar: (texto) => ipcRenderer.invoke("objetivo:salvar", texto),
  },
  metas: {
    listar: () => ipcRenderer.invoke("metas:listar"),
    ler: (id) => ipcRenderer.invoke("metas:ler", id),
    criar: (dados) => ipcRenderer.invoke("metas:criar", dados),
    atualizar: (id, dados) => ipcRenderer.invoke("metas:atualizar", id, dados),
    apagar: (id) => ipcRenderer.invoke("metas:apagar", id),
  },
});
