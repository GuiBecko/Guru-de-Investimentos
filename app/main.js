const { app, BrowserWindow, dialog, ipcMain, net, shell } = require("electron");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { pathToFileURL } = require("node:url");
const { origemPermitida } = require("./main/origem.js");
const { normalizarErro } = require("./main/erro.js");
const { executarSmoke } = require("./main/smoke.js");
const { montarAtualizacao } = require("./main/atualizacao-electron.js");

// Smoke do pacote: uma opção, não uma variável, porque vale também no app empacotado. Pasta
// temporária, cofre em memória, nenhuma janela e nenhuma rede; o processo só imprime e sai.
const SMOKE = process.argv.includes("--guru-smoke");
const PRAZO_SMOKE_MS = 30000;
if (SMOKE) {
  const pasta = fs.mkdtempSync(path.join(os.tmpdir(), "guru-smoke-"));
  app.setPath("userData", pasta);
  app.dock?.hide();
  const sair = (codigo) => {
    try {
      fs.rmSync(pasta, { recursive: true, force: true });
    } catch (e) {
      console.error("Não foi possível apagar a pasta do smoke:", e.message);
    }
    app.exit(codigo);
  };
  setTimeout(() => {
    console.log(`GURU_SMOKE_FALHOU PRAZO: o smoke passou de ${PRAZO_SMOKE_MS / 1000} s.`);
    sair(2);
  }, PRAZO_SMOKE_MS);
  app
    .whenReady()
    .then(() =>
      executarSmoke({
        pasta,
        versaoApp: app.getVersion(),
        empacotado: app.isPackaged,
        abrirCore: (dir, opcoes) => require("@guru/core-node").Core.open(dir, opcoes),
        existe: (caminho) => fs.existsSync(caminho),
        listar: (dir) => fs.readdirSync(dir),
        dirApp: app.getAppPath(),
        dirRecursos: process.resourcesPath,
        plataforma: process.platform,
        arquitetura: process.arch,
        log: (linha) => console.log(linha),
      }),
    )
    .then(sair, (e) => {
      console.log(`GURU_SMOKE_FALHOU ADDON_INDISPONIVEL: ${e && e.message}`);
      sair(1);
    });
}

// Variáveis de teste (GURU_*) só valem fora do app empacotado.
if (process.env.GURU_DATA_DIR && !app.isPackaged && !SMOKE) {
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
// Criada depois do whenReady e nunca no smoke (que não chama a rede).
let atualizacao = null;

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
    return { ...nucleo().info(), versaoApp: app.getVersion() };
  }),
);

// Texto gerado por scripts/avisos-terceiros.mjs e empacotado junto do app. Só é lido no clique.
ipcMain.handle(
  "app:avisosTerceiros",
  tratar(async () => {
    try {
      return await fs.promises.readFile(path.join(__dirname, "avisos-terceiros.txt"), "utf8");
    } catch (e) {
      // O code do fs (ENOENT…) passaria pela ponte com o caminho na mensagem.
      console.error("Falha ao ler os avisos de terceiros:", e);
      const erro = new Error("Não foi possível ler os avisos de terceiros.");
      erro.code = "PACOTE_INCOMPLETO";
      throw erro;
    }
  }),
);

function atualizador() {
  if (!atualizacao) {
    const erro = new Error("A procura de atualizações só funciona no app instalado.");
    erro.code = "ATUALIZACAO_INDISPONIVEL";
    throw erro;
  }
  return atualizacao;
}

function nucleo() {
  if (!core) {
    const erro = new Error("O núcleo não está disponível.");
    erro.code = "ADDON_INDISPONIVEL";
    throw erro;
  }
  return core;
}

const LIMITE_ARQUIVO = 20 * 1024 * 1024;

// Cada fonte de importação: filtro do diálogo, variável do caminho de teste, mensagem de tamanho e
// a função do núcleo que compara o arquivo.
const FONTES = {
  b3: {
    filtro: { name: "Planilha do Excel", extensions: ["xlsx"] },
    variavelTeste: "GURU_TESTE_ARQUIVO_B3",
    mensagemGrande: "O arquivo é grande demais para uma planilha de posição.",
    comparar: (bytes, nome) => nucleo().compararImportacao(bytes, nome),
  },
  btg: {
    filtro: { name: "Extrato em PDF", extensions: ["pdf"] },
    variavelTeste: "GURU_TESTE_ARQUIVO_BTG",
    mensagemGrande: "O arquivo é grande demais para um extrato da BTG.",
    comparar: (bytes, nome) => nucleo().compararImportacaoBtg(bytes, nome),
  },
};

// Escolhe o arquivo da fonte (diálogo, ou o caminho de teste fora do app empacotado), confere o
// tamanho ANTES de ler e devolve a comparação ou `{ cancelado: true }`. O núcleo recebe só o nome
// do arquivo, nunca o caminho.
async function escolherArquivo(fonte) {
  const cfg = typeof fonte === "string" && Object.hasOwn(FONTES, fonte) ? FONTES[fonte] : null;
  if (!cfg) {
    const erro = new Error("Fonte de importação desconhecida.");
    erro.code = "FONTE_DESCONHECIDA";
    throw erro;
  }
  let arquivo;
  const teste = !app.isPackaged ? process.env[cfg.variavelTeste] : undefined;
  if (teste) {
    if (teste === "cancelar") return { cancelado: true };
    arquivo = teste;
  } else {
    const r = await dialog.showOpenDialog(janela, {
      properties: ["openFile"],
      filters: [cfg.filtro],
    });
    if (r.canceled || r.filePaths.length === 0) return { cancelado: true };
    arquivo = r.filePaths[0];
  }
  // Erros do sistema de arquivos (ENOENT, EACCES...) trazem o caminho na mensagem: não passam.
  const ilegivel = () => {
    const erro = new Error("Não foi possível ler o arquivo.");
    erro.code = "ARQUIVO_INVALIDO";
    return erro;
  };
  const grande = () => {
    const erro = new Error(cfg.mensagemGrande);
    erro.code = "ARQUIVO_INVALIDO";
    return erro;
  };
  let bytes;
  try {
    const info = await fs.promises.stat(arquivo);
    if (!info.isFile()) throw ilegivel();
    if (info.size > LIMITE_ARQUIVO) throw grande();
    bytes = await fs.promises.readFile(arquivo);
  } catch (e) {
    throw e && e.code === "ARQUIVO_INVALIDO" ? e : ilegivel();
  }
  if (bytes.length > LIMITE_ARQUIVO) throw grande();
  return cfg.comparar(bytes, path.basename(arquivo));
}

const canais = {
  "investimentos:catalogo": () => nucleo().catalogo(),
  "investimentos:listar": () => nucleo().listarInvestimentos(),
  "investimentos:ler": (id) => nucleo().investimento(id),
  "investimentos:criar": (dados) => nucleo().criarInvestimento(dados),
  "investimentos:atualizar": (id, dados) => nucleo().atualizarInvestimento(id, dados),
  "investimentos:apagar": (id) => nucleo().apagarInvestimento(id),
  "carteira:resumo": () => nucleo().resumoCarteira(),
  "carteira:painel": (meses, classe) => nucleo().painel(meses, classe),
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
  "aporte:dividir": (valorCentavos) => nucleo().proximoAporte(valorCentavos),
  "mercado:atualizar": () => nucleo().atualizarMercado(),
  "mercado:status": () => nucleo().statusMercado(),
  "importacao:escolher": (fonte) => escolherArquivo(fonte),
  "importacao:aplicar": (token, decisoes) => nucleo().aplicarImportacao(token, decisoes),
  "importacao:ultima": () => nucleo().ultimaImportacao(),
  "ia:config": () => nucleo().configIa(),
  "ia:salvarConfig": (dados) => nucleo().salvarConfigIa(dados),
  "ia:salvarChave": (chave) => nucleo().salvarChaveIa(chave),
  "ia:removerChave": () => nucleo().removerChaveIa(),
  "ia:contexto": () => nucleo().contextoIa(),
  "ia:salvarParte": (parte, ligada, textoEditado) => nucleo().salvarParteContexto(parte, ligada, textoEditado ?? null),
  "ia:conversas": () => nucleo().conversas(),
  "ia:conversa": (id) => nucleo().conversa(id),
  "ia:apagarConversa": (id) => nucleo().apagarConversa(id),
  "ia:perguntar": (conversaId, pergunta, foco) => nucleo().perguntarIa(conversaId ?? null, pergunta, foco ?? null),
  "ia:progresso": (pedido) => nucleo().iaProgresso(pedido),
  "ia:cancelar": (pedido) => nucleo().cancelarIa(pedido),
  "ia:painel": () => nucleo().painelIa(),
  "ia:gerarInsights": () => nucleo().gerarInsightsIa(),
  "ia:buscarNoticias": () => nucleo().buscarNoticiasIa(),
  "atualizacao:estado": () => atualizador().estado(),
  "atualizacao:procurar": () => atualizador().procurar(),
  "atualizacao:salvarConfig": (dados) => atualizador().salvarConfig(dados),
  "atualizacao:baixar": () => atualizador().baixar(),
  "atualizacao:reiniciar": () => atualizador().reiniciar(),
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
  // Modo de teste do mercado: o atualizador lê arquivos da pasta em vez de baixar, com data fixa
  // opcional. Só fora do app empacotado; empacotado, o núcleo sempre usa a rede e a data real.
  const opcoes = {};
  if (!app.isPackaged && process.env.GURU_MERCADO_FIXTURES) {
    opcoes.pastaFixtures = process.env.GURU_MERCADO_FIXTURES;
    if (process.env.GURU_MERCADO_HOJE) opcoes.hoje = process.env.GURU_MERCADO_HOJE;
  }
  // Assistente de IA, também só fora do app empacotado: respostas gravadas no lugar da rede e
  // cofre em memória. Empacotado, o cofre é o do sistema e o cliente é sempre o HTTP.
  if (!app.isPackaged && process.env.GURU_IA_FIXTURES) opcoes.pastaIa = process.env.GURU_IA_FIXTURES;
  if (!app.isPackaged && process.env.GURU_IA_COFRE === "memoria") opcoes.cofreMemoria = true;
  return Core.open(app.getPath("userData"), opcoes);
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

  // A procura de abertura só começa com a página carregada, para não atrasar a abertura.
  // Na tela de erro (núcleo indisponível) ninguém mostraria o resultado: não procura.
  janela.webContents.once("did-finish-load", () => {
    if (atualizacao && core) atualizacao.aoAbrir();
  });

  if (core) {
    janela.loadFile(path.join(__dirname, "renderer", "index.html"));
  } else {
    janela.loadFile(path.join(__dirname, "renderer", "erro.html"), {
      query: { code: codigo },
    });
  }
}

if (SMOKE) {
  // Tratado no topo: sem trava de instância única, sem janela.
} else if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on("second-instance", () => {
    if (!janela) return;
    if (janela.isMinimized()) janela.restore();
    janela.focus();
  });
  // Idioma fixo: campos de data e números se comportam igual em qualquer máquina.
  app.commandLine.appendSwitch("lang", "pt-BR");
  app.whenReady().then(() => {
    try {
      atualizacao = montarAtualizacao({ app, net });
    } catch (e) {
      // Só o feed de teste malformado chega aqui; o app abre sem procurar atualizações.
      console.error("Falha ao preparar a atualização:", e);
    }
    criarJanela();
  });
  app.on("window-all-closed", () => app.quit());
}
