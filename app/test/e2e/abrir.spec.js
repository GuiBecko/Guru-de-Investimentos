const { test, expect, _electron } = require("@playwright/test");
const { DatabaseSync } = require("node:sqlite");
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const appDir = path.resolve(__dirname, "../..");

function novoDataDir() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "guru-e2e-"));
}

function lancar(dataDir, extraEnv = {}) {
  return _electron.launch({
    args: ["."],
    cwd: appDir,
    env: { ...process.env, GURU_DATA_DIR: dataDir, ...extraEnv },
  });
}

test("abre com título e mostra a versão do esquema", async () => {
  const dataDir = novoDataDir();
  const app = await lancar(dataDir);
  try {
    const page = await app.firstWindow();
    expect(await page.title()).toBe("Guru de Investimentos");
    await expect(page.getByTestId("schema-version")).toHaveText("4");
    await expect(page.getByTestId("core-version")).not.toBeEmpty();
    expect(fs.existsSync(path.join(dataDir, "guru.db"))).toBe(true);
  } finally {
    await app.close();
  }
});

test("a interface não tem acesso ao Node", async () => {
  const app = await lancar(novoDataDir());
  try {
    const page = await app.firstWindow();
    expect(await page.evaluate(() => typeof require)).toBe("undefined");
    expect(await page.evaluate(() => typeof process)).toBe("undefined");
  } finally {
    await app.close();
  }
});

test("a ponte expõe só os canais do app e nada do Electron ou do Node", async () => {
  const app = await lancar(novoDataDir());
  try {
    const page = await app.firstWindow();
    expect(await page.evaluate(() => Object.keys(window.guru))).toEqual([
      "app",
      "investimentos",
      "carteira",
      "perfil",
      "alocacao",
      "objetivo",
      "metas",
      "mercado",
    ]);
    expect(await page.evaluate(() => Object.keys(window.guru.app))).toEqual(["info"]);
    expect(await page.evaluate(() => Object.keys(window.guru.investimentos))).toEqual([
      "catalogo",
      "listar",
      "ler",
      "criar",
      "atualizar",
      "apagar",
    ]);
    expect(await page.evaluate(() => Object.keys(window.guru.carteira))).toEqual(["resumo"]);
    expect(await page.evaluate(() => Object.keys(window.guru.perfil))).toEqual([
      "questionario",
      "registrar",
      "atual",
    ]);
    expect(await page.evaluate(() => Object.keys(window.guru.alocacao))).toEqual(["ler", "salvar"]);
    expect(await page.evaluate(() => Object.keys(window.guru.objetivo))).toEqual(["ler", "salvar"]);
    expect(await page.evaluate(() => Object.keys(window.guru.metas))).toEqual([
      "listar",
      "ler",
      "criar",
      "atualizar",
      "apagar",
    ]);
    expect(await page.evaluate(() => Object.keys(window.guru.mercado))).toEqual([
      "atualizar",
      "status",
    ]);
    expect(await page.evaluate(() => typeof window.ipcRenderer)).toBe("undefined");
    expect(await page.evaluate(() => typeof window.require)).toBe("undefined");
    expect(await page.evaluate(() => typeof window.__guruBruto)).toBe("undefined");
  } finally {
    await app.close();
  }
});

test("o code do erro chega ao renderer", async () => {
  // GURU_TESTE_ERRO_INFO só vale fora do app empacotado (ver app/main.js).
  const app = await lancar(novoDataDir(), { GURU_TESTE_ERRO_INFO: "SQLITE" });
  try {
    const page = await app.firstWindow();
    const erro = await page.evaluate(() =>
      import("./js/api.js").then(({ api }) =>
        api.app.info().then(
          () => null,
          (e) => ({ ehError: e instanceof Error, code: e.code, message: e.message }),
        ),
      ),
    );
    expect(erro).toEqual({ ehError: true, code: "SQLITE", message: expect.any(String) });
  } finally {
    await app.close();
  }
});

test("investimentos pelo api: catálogo e NAO_ENCONTRADO", async () => {
  const app = await lancar(novoDataDir());
  try {
    const page = await app.firstWindow();
    const catalogo = await page.evaluate(() =>
      import("./js/api.js").then(({ api }) => api.investimentos.catalogo()),
    );
    expect(catalogo.tipos).toHaveLength(17);
    const erro = await page.evaluate(() =>
      import("./js/api.js").then(({ api }) =>
        api.investimentos.apagar(999).then(
          () => null,
          (e) => ({ code: e.code, message: e.message }),
        ),
      ),
    );
    expect(erro).toEqual({ code: "NAO_ENCONTRADO", message: expect.any(String) });
  } finally {
    await app.close();
  }
});

test("metas e perfil pelo api: NAO_ENCONTRADO e questionário sem pontos", async () => {
  const app = await lancar(novoDataDir());
  try {
    const page = await app.firstWindow();
    const erro = await page.evaluate(() =>
      import("./js/api.js").then(({ api }) =>
        api.metas.apagar(999).then(
          () => null,
          (e) => ({ code: e.code, message: e.message }),
        ),
      ),
    );
    expect(erro).toEqual({ code: "NAO_ENCONTRADO", message: expect.any(String) });
    const q = await page.evaluate(() =>
      import("./js/api.js").then(({ api }) => api.perfil.questionario()),
    );
    expect(q.perguntas).toHaveLength(12);
    expect(typeof q.perguntas[0].opcoes[0]).toBe("string");
  } finally {
    await app.close();
  }
});

test("mercado pelo api: status das cinco fontes sem rede", async () => {
  const app = await lancar(novoDataDir());
  try {
    const page = await app.firstWindow();
    const status = await page.evaluate(() =>
      import("./js/api.js").then(({ api }) => api.mercado.status()),
    );
    expect(status.map((s) => s.rotulo)).toEqual([
      "CDI",
      "Selic",
      "IPCA",
      "Tesouro Direto",
      "Cotações da B3",
    ]);
  } finally {
    await app.close();
  }
});

test("sem GURU_DATA_DIR a pasta de dados usa o nome do produto", async () => {
  // Redireciona a pasta de configuração do sistema para não tocar nos dados reais.
  const config = novoDataDir();
  const app = await _electron.launch({
    args: ["."],
    cwd: appDir,
    env: {
      ...Object.fromEntries(Object.entries(process.env).filter(([k]) => k !== "GURU_DATA_DIR")),
      XDG_CONFIG_HOME: config,
      APPDATA: config,
    },
  });
  try {
    expect(await app.evaluate(({ app }) => app.getName())).toBe("Guru de Investimentos");
    const dados = await app.evaluate(({ app }) => app.getPath("userData"));
    expect(path.basename(dados)).toBe("Guru de Investimentos");
  } finally {
    await app.close();
  }
});

test("banco de versão mais nova mostra a tela de erro", async () => {
  const dataDir = novoDataDir();
  const db = new DatabaseSync(path.join(dataDir, "guru.db"));
  db.exec("PRAGMA user_version = 99");
  db.close();
  const app = await lancar(dataDir);
  try {
    const page = await app.firstWindow();
    await expect(page.getByText("Atualize o Guru de Investimentos")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("addon ausente mostra a tela de erro", async () => {
  const app = await lancar(novoDataDir(), {
    GURU_ADDON_PATH: path.join(os.tmpdir(), "guru-addon-que-nao-existe"),
  });
  try {
    const page = await app.firstWindow();
    await expect(page.getByText("O núcleo do app não carregou")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("segunda instância não abre outra janela", async () => {
  const dataDir = novoDataDir();
  const primeira = await lancar(dataDir);
  try {
    await primeira.firstWindow();
    const processo = primeira.process();
    expect(processo.exitCode).toBeNull();

    // Playwright não conecta a um processo que encerra sozinho; usa-se spawn direto.
    const segunda = spawn(require("electron"), ["."], {
      cwd: appDir,
      env: { ...process.env, GURU_DATA_DIR: dataDir },
      stdio: "ignore",
    });
    const saiu = await new Promise((resolve) => {
      const timer = setTimeout(() => resolve(false), 15000);
      segunda.once("exit", () => {
        clearTimeout(timer);
        resolve(true);
      });
    });
    if (!saiu) segunda.kill();
    expect(saiu).toBe(true);

    expect(primeira.windows()).toHaveLength(1);
  } finally {
    await primeira.close();
  }
});
