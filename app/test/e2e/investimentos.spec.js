const { test, expect, _electron } = require("@playwright/test");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

// Datas futuras derivadas do ano atual, para os testes não vencerem com o tempo.
const ANO = new Date().getFullYear();
const VENC_CDB = `${ANO + 3}-03-15`;
const VENC_CDB_BR = `15/03/${ANO + 3}`;
const VENC_SELIC = `${ANO + 4}-03-01`;
const PRAZO = `${ANO + 4}-12`;

const appDir = path.resolve(__dirname, "../..");

function lancar() {
  return _electron.launch({
    args: ["."],
    cwd: appDir,
    env: { ...process.env, GURU_DATA_DIR: fs.mkdtempSync(path.join(os.tmpdir(), "guru-e2e-")) },
  });
}

const CDB = {
  tipo: "rf-cdb",
  instituicao: "Banco Inter",
  emissor: "Banco Inter",
  indexador: "pos",
  taxa: "102",
  benchmark: "cdi",
  dataAplicacao: "2025-03-15",
  vencimento: VENC_CDB,
  valorAplicadoCentavos: 2000000,
  liquidez: "vencimento",
  ir: "regressiva",
};

function criar(page, dados) {
  return page.evaluate(
    (d) => import("./js/api.js").then(({ api }) => api.investimentos.criar(d)),
    dados,
  );
}

async function abrirLista(page) {
  await page.getByRole("link", { name: "Investimentos", exact: true }).click();
  await expect(page.locator("main").getByRole("heading", { name: "Investimentos" })).toBeVisible();
}

test("lista vazia mostra a mensagem e o botão de novo", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrirLista(page);
    await expect(page.locator("main")).toContainText("Nenhum investimento ainda.");
    await expect(page.getByRole("link", { name: "Novo investimento" })).toHaveAttribute(
      "href",
      "#/investimentos/novo",
    );
  } finally {
    await app.close();
  }
});

test("a linha mostra os dados do CDB e o total", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const { id, erros } = await criar(page, CDB);
    expect(erros).toEqual([]);
    await abrirLista(page);
    const linha = page.locator("main tbody tr").first();
    await expect(linha).toContainText("CDB Banco Inter");
    await expect(linha).toContainText("102% do CDI");
    await expect(linha).toContainText("R$ 20.000,00");
    await expect(linha).toContainText(VENC_CDB_BR);
    await expect(linha.getByRole("link", { name: "Editar" })).toHaveAttribute("href", `#/investimentos/${id}`);
    await expect(page.locator("main")).toContainText("Total aplicado");
  } finally {
    await app.close();
  }
});

test("apagar pede confirmação e esvazia a lista", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await abrirLista(page);
    await page.getByRole("button", { name: "Apagar" }).click();
    const dialogo = page.getByRole("dialog");
    await expect(dialogo).toContainText("Apagar CDB Banco Inter?");
    await dialogo.getByRole("button", { name: "Apagar" }).click();
    await expect(page.locator("main")).toContainText("Nenhum investimento ainda.");
  } finally {
    await app.close();
  }
});

test("cancelar mantém a linha", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await abrirLista(page);
    await page.getByRole("button", { name: "Apagar" }).click();
    await page.getByRole("dialog").getByRole("button", { name: "Cancelar" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.locator("main tbody tr")).toHaveCount(1);
  } finally {
    await app.close();
  }
});

test("apagar um investimento que já não existe mostra a mensagem e redesenha", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const { id } = await criar(page, CDB);
    await abrirLista(page);
    await page.getByRole("button", { name: "Apagar" }).click();
    await page.evaluate((i) => import("./js/api.js").then(({ api }) => api.investimentos.apagar(i)), id);
    await page.getByRole("dialog").getByRole("button", { name: "Apagar" }).click();
    await expect(page.locator("main [role=alert]")).toBeVisible();
    await expect(page.locator("main")).toContainText("Nenhum investimento ainda.");
  } finally {
    await app.close();
  }
});

test("uma lista que chega atrasada não sobrescreve a tela seguinte", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await page.evaluate(() => {
      // hashchange síncrono: a lista sai a buscar e a tela seguinte abre antes da resposta.
      window.location.hash = "#/investimentos";
      window.dispatchEvent(new Event("hashchange"));
      window.location.hash = "#/assistente";
      window.dispatchEvent(new Event("hashchange"));
    });
    const conteudo = page.locator("main");
    await expect(conteudo.getByRole("heading", { name: "Assistente" })).toBeVisible();
    await page.waitForTimeout(300);
    await expect(conteudo.getByRole("heading", { name: "Assistente" })).toBeVisible();
    await expect(conteudo.getByRole("heading", { name: "Investimentos" })).toHaveCount(0);
  } finally {
    await app.close();
  }
});
