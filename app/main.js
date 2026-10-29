const { app, BrowserWindow, ipcMain, shell } = require("electron");
const path = require("node:path");
const { pathToFileURL } = require("node:url");
const { origemPermitida } = require("./main/origem.js");
const { normalizarErro } = require("./main/erro.js");

// Variáveis de teste (GURU_*) só valem fora do app empacotado.
if (process.env.GURU_DATA_DIR && !app.isPackaged) {
  app.setPath("userData", process.env.GURU_DATA_DIR);
}

// e2e com janela escondida: sem isso o Chromium pausa os quadros da página e cada clique do Playwright espera ~2 s.
if (process.env.GURU_JANELA_OCULTA === "1" && !app.isPackaged) {
  app.commandLine.appendSwitch("disable-renderer-backgrounding");
  app.commandLine.appendSwitch("run-all-compositor-stages-before-draw");
  app.commandLine.appendSwitch("disable-frame-rate-limit");
  app.commandLine.appendSwitch("disable-background-timer-throttling");
  app.commandLine.appendSwitch("disable-backgrounding-occluded-windows");
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
  "perfil:questionario": () => nucleo().questionario(),
  "perfil:registrar": (respostas) => nucleo().registrarPerfil(respostas),
  "perfil:atual": () => nucleo().perfilAtual(),
  "alocacao:ler": () => nucleo().alocacaoAlvo(),
  "alocacao:salvar": (valores) => nucleo().salvarAlocacaoAlvo(valores),
  "objetivo:ler": () => nucleo().objetivo(),
  "objetivo:salvar": (texto) => nucleo().salvarObjetivo(texto),
  "metas:listar": () => nucleo().listarMetas(),
  "metas:ler": (id) => nucleo().meta(id),
  "metas:criar": (dados) => nucleo().criarMeta(dados),
  "metas:atualizar": (id, dados) => nucleo().atualizarMeta(id, dados),
  "metas:apagar": (id) => nucleo().apagarMeta(id),
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
  const oculta = process.env.GURU_JANELA_OCULTA === "1" && !app.isPackaged;
  janela = new BrowserWindow({
    title: "Guru de Investimentos",
    width: 1280,
    height: 800,
    minWidth: 1100,
    minHeight: 700,
    // Só os e2e (fora do app empacotado) criam a janela escondida.
    show: !oculta,
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      sandbox: true,
      nodeIntegration: false,
      // Janela escondida pausaria requestAnimationFrame e o Playwright não conseguiria clicar.
      backgroundThrottling: !oculta,
    },
  });

  if (oculta) {
    // Faz a página se comportar como visível e com foco, para os quadros e o teclado funcionarem nos e2e.
    try {
      janela.webContents.debugger.attach("1.3");
      janela.webContents.debugger.sendCommand("Emulation.setFocusEmulationEnabled", { enabled: true });
      janela.webContents.debugger.sendCommand("Page.bringToFront");
    } catch (e) {
      console.error("Falha ao emular foco na janela escondida:", e);
    }
  }

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
  // Idioma fixo: campos de data e números se comportam igual em qualquer máquina.
  app.commandLine.appendSwitch("lang", "pt-BR");
  app.whenReady().then(criarJanela);
  app.on("window-all-closed", () => app.quit());
}
