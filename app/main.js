const { app, BrowserWindow, ipcMain, shell } = require("electron");
const path = require("node:path");
const { pathToFileURL } = require("node:url");
const { origemPermitida } = require("./main/origem.js");
const { normalizarErro } = require("./main/erro.js");

// Variáveis de teste (GURU_*) só valem fora do app empacotado.
if (process.env.GURU_DATA_DIR && !app.isPackaged) {
  app.setPath("userData", process.env.GURU_DATA_DIR);
}

let janela = null;
let core = null;

const URL_ESPERADA = pathToFileURL(path.join(__dirname, "renderer", "index.html")).href;

// Embrulha um handler IPC: devolve `{ ok: true, value }` ou
// `{ ok: false, code, message }`, para o `code` atravessar a ponte (ver preload.js).
function tratar(handler) {
  return async (event, ...args) => {
    const url = event && event.senderFrame ? event.senderFrame.url : "";
    if (!origemPermitida(url, URL_ESPERADA)) {
      return { ok: false, code: "ORIGEM_RECUSADA", message: "Chamada recusada: origem desconhecida." };
    }
    try {
      return { ok: true, value: await handler(...args) };
    } catch (e) {
      const { code, message, original } = normalizarErro(e);
      if (original) console.error("Erro inesperado no processo principal:", e);
      return { ok: false, code, message };
    }
  };
}

ipcMain.handle(
  "app:info",
  tratar(() => {
    // Só para teste (e2e): força um erro com esse code. Nunca vale no app empacotado.
    const forcado = process.env.GURU_TESTE_ERRO_INFO;
    if (forcado && !app.isPackaged) {
      const erro = new Error("Erro forçado pelo teste.");
      erro.code = forcado;
      throw erro;
    }
    return nucleo().info();
  }),
);

function nucleo() {
  if (!core) {
    const erro = new Error("O núcleo não está disponível.");
    erro.code = "ADDON_INDISPONIVEL";
    throw erro;
  }
  return core;
}

const canais = {
  "investimentos:catalogo": () => nucleo().catalogo(),
  "investimentos:listar": () => nucleo().listarInvestimentos(),
  "investimentos:ler": (id) => nucleo().investimento(id),
  "investimentos:criar": (dados) => nucleo().criarInvestimento(dados),
  "investimentos:atualizar": (id, dados) => nucleo().atualizarInvestimento(id, dados),
  "investimentos:apagar": (id) => nucleo().apagarInvestimento(id),
  "carteira:resumo": () => nucleo().resumoCarteira(),
};
for (const [canal, handler] of Object.entries(canais)) {
  ipcMain.handle(canal, tratar(handler));
}

function abrirNucleo() {
  let Core;
  try {
    Core = require((!app.isPackaged && process.env.GURU_ADDON_PATH) || "@guru/core-node").Core;
  } catch (e) {
    const erro = new Error(e.message, { cause: e });
    erro.code = "ADDON_INDISPONIVEL";
    throw erro;
  }
  return Core.open(app.getPath("userData"));
}

function criarJanela() {
  janela = new BrowserWindow({
    title: "Guru de Investimentos",
    width: 1280,
    height: 800,
    minWidth: 1100,
    minHeight: 700,
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      sandbox: true,
      nodeIntegration: false,
    },
  });

  janela.webContents.on("will-navigate", (evento, url) => {
    evento.preventDefault();
    if (url.startsWith("https:")) shell.openExternal(url);
  });
  janela.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith("https:")) shell.openExternal(url);
    return { action: "deny" };
  });

  let codigo = null;
  try {
    core = abrirNucleo();
  } catch (e) {
    console.error("Falha ao abrir o núcleo:", e);
    codigo = e.code || "ADDON_INDISPONIVEL";
  }

  if (core) {
    janela.loadFile(path.join(__dirname, "renderer", "index.html"));
  } else {
    janela.loadFile(path.join(__dirname, "renderer", "erro.html"), {
      query: { code: codigo },
    });
  }
}

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on("second-instance", () => {
    if (!janela) return;
    if (janela.isMinimized()) janela.restore();
    janela.focus();
  });
  app.whenReady().then(criarJanela);
  app.on("window-all-closed", () => app.quit());
}
