const { contextBridge, ipcRenderer, webFrame } = require("electron");

// O main responde `{ ok: true, value }` ou `{ ok: false, code, message }` (ver
// `tratar` em main.js), porque o Electron reescreve a mensagem de erros lançados
// em `ipcMain.handle` e perde o `code`. O `contextBridge` também só deixa passar
// a mensagem de um `Error`. Por isso a resposta atravessa a ponte como dado e o
// `Error` com `code` é montado no mundo da página, por um script que o próprio
// preload injeta e que remove a ponte bruta em seguida.
contextBridge.exposeInMainWorld("__guruBruto", {
  info: () => ipcRenderer.invoke("app:info"),
});

webFrame.executeJavaScript(`(() => {
  const bruto = window.__guruBruto;
  delete window.__guruBruto;
  const chamar = async (metodo) => {
    const resposta = await bruto[metodo]();
    if (resposta.ok) return resposta.value;
    const erro = new Error(resposta.message);
    erro.code = resposta.code;
    throw erro;
  };
  Object.defineProperty(window, "guru", {
    value: Object.freeze({ app: Object.freeze({ info: () => chamar("info") }) }),
    enumerable: true,
  });
})();`);
