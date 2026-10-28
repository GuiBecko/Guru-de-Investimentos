const { test, expect, _electron } = require("@playwright/test");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

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
  vencimento: "2028-03-15",
  valorAplicadoCentavos: 2000000,
  liquidez: "vencimento",
  ir: "regressiva",
};
const SELIC = {
  tipo: "td-selic",
  instituicao: "XP Investimentos",
  indexador: "selic",
  taxa: "0.0905",
  dataAplicacao: "2025-03-01",
  vencimento: "2029-03-01",
  valorAplicadoCentavos: 1800000,
  liquidez: "d1",
  ir: "regressiva",
};
const ITUB4 = {
  tipo: "rv-acao",
  instituicao: "Rico",
  ticker: "ITUB4",
  quantidade: 150,
  precoMedioCentavos: 3142,
  dataAplicacao: "2025-05-10",
};

function criar(page, dados) {
  return page.evaluate(
    (d) => import("./js/api.js").then(({ api }) => api.investimentos.criar(d)),
    dados,
  );
}

test("carteira vazia mostra o texto e o link para lançar", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    // Sinal de app pronto (o mesmo das outras specs): se a partida for lenta, a falha diz onde parou.
    await expect(page.getByTestId("core-version")).not.toBeEmpty({ timeout: 20000 });
    await page.getByRole("link", { name: "Carteira", exact: true }).click();
    const main = page.locator("main");
    await expect(main.getByRole("heading", { name: "Carteira" })).toBeVisible();
    await expect(main).toContainText("Sua carteira está vazia.");
    await expect(main.getByRole("link", { name: "Lançar o primeiro investimento" })).toHaveAttribute(
      "href",
      "#/investimentos/novo",
    );
  } finally {
    await app.close();
  }
});

test("carteira mostra total e um card por classe, e o card abre o detalhe", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await criar(page, SELIC);
    await criar(page, ITUB4);
    await page.evaluate(() => {
      window.location.hash = "#/carteira";
    });
    const main = page.locator("main");
    await expect(main.getByText("Total aplicado")).toBeVisible();
    await expect(main).toContainText("R$ 42.713,00");
    await expect(main).toContainText("Valores aplicados. Cotações e rentabilidade chegam numa próxima versão.");

    const rf = main.getByRole("link", { name: /Renda fixa/ });
    await expect(rf).toHaveAttribute("href", "#/carteira/rf");
    await expect(rf).toContainText("R$ 38.000,00");
    await expect(rf).toContainText("2 investimentos");
    await expect(rf).toContainText("89,0%");
    const acoes = main.getByRole("link", { name: /Ações/ });
    await expect(acoes).toContainText("1 investimento");
    await expect(acoes).toContainText("11,0%");
    await expect(page.getByRole("navigation").getByRole("link", { name: "Carteira" })).toHaveClass(/on/);

    await rf.click();
    await expect(main.getByRole("heading", { name: "Renda fixa" })).toBeVisible();
    await expect(main).toContainText("R$ 38.000,00");
    await expect(page.getByRole("navigation").getByRole("link", { name: "Carteira" })).toHaveClass(/on/);
    const linhas = main.locator("tbody tr");
    await expect(linhas).toHaveCount(2);
    await expect(linhas.nth(0)).toContainText("CDB Banco Inter");
    await expect(linhas.nth(0)).toContainText("102% do CDI");
    await expect(linhas.nth(0)).toContainText("15/03/2028");
    await expect(linhas.nth(1)).toContainText("Tesouro Selic 2029");
    await expect(main).not.toContainText("undefined");
    await expect(main.locator("th")).toHaveText([
      "Investimento", "Instituição", "Rendimento", "Aplicado", "Vencimento", "Liquidez",
    ]);

    await main.getByRole("link", { name: "Carteira ›" }).click();
    await expect(main.getByText("Total aplicado")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("classe desconhecida ou vazia mostra o aviso e o link de volta", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    for (const hash of ["#/carteira/xyz", "#/carteira/fii"]) {
      await page.evaluate((h) => {
        window.location.hash = h;
      }, hash);
      const main = page.locator("main");
      await expect(main).toContainText("Nenhum investimento nesta classe.");
      await expect(main.getByRole("link", { name: "Carteira ›" })).toHaveAttribute("href", "#/carteira");
    }
  } finally {
    await app.close();
  }
});
