const { app, BrowserWindow, ipcMain, shell } = require("electron");
const path = require("node:path");

if (process.env.GURU_DATA_DIR) {
  app.setPath("userData", process.env.GURU_DATA_DIR);
}

let janela = null;

function abrirNucleo() {
  let Core;
  try {
    Core = require(process.env.GURU_ADDON_PATH || "@guru/core-node").Core;
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

  let core = null;
  let codigo = null;
  try {
    core = abrirNucleo();
  } catch (e) {
    console.error("Falha ao abrir o núcleo:", e);
    codigo = e.code || "ADDON_INDISPONIVEL";
  }

  if (core) {
    ipcMain.handle("app:info", () => core.info());
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
