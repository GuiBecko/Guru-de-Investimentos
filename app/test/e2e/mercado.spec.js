const { test, expect, _electron } = require("@playwright/test");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const appDir = path.resolve(__dirname, "../..");
const FIXTURES = path.resolve(appDir, "../crates/guru-core/tests/fixtures");

function lancar(extraEnv = {}) {
  return _electron.launch({
    args: ["."],
    cwd: appDir,
    env: { ...process.env, GURU_DATA_DIR: fs.mkdtempSync(path.join(os.tmpdir(), "guru-e2e-")), ...extraEnv },
  });
}

function criar(page, dados) {
  return page.evaluate((d) => import("./js/api.js").then(({ api }) => api.investimentos.criar(d)), dados);
}

// Um investimento novo só ganha cotação na próxima atualização: clica em Atualizar e espera terminar.
async function atualizarAgora(page) {
  const mercado = page.getByTestId("mercado");
  await mercado.getByRole("button", { name: "Atualizar" }).click();
  await expect(mercado.getByRole("button", { name: "Atualizar", exact: true })).toBeEnabled({ timeout: 20000 });
}

const ITUB4 = {
  tipo: "rv-acao",
  instituicao: "Rico",
  ticker: "ITUB4",
  quantidade: 150,
  precoMedioCentavos: 3142,
  dataAplicacao: "2026-10-01",
};
const CRIPTO = {
  tipo: "cripto",
  instituicao: "Binance",
  emissor: "Bitcoin",
  liquidez: "diaria",
  dataAplicacao: "2026-10-01",
  valorAplicadoCentavos: 50_000,
};
const SEM_COTACAO = { ...ITUB4, ticker: "ZZZZ9", quantidade: 10, precoMedioCentavos: 1000 };

test("ao abrir, o rodapé do menu chega à data dos dados (o IPCA mensal fica fora do menor)", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const mercado = page.getByTestId("mercado");
    await expect(mercado).toContainText("Dados de mercado de 28/10", { timeout: 20000 });
    await expect(mercado.getByRole("button", { name: "Atualizar" })).toBeEnabled();
    await expect(mercado).not.toContainText("com falhas");
  } finally {
    await app.close();
  }
});

test("lista mostra atual e rentabilidade com sinal; ticker sem cotação mostra a marca e traço", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await expect(page.getByTestId("mercado")).toContainText("28/10", { timeout: 20000 });
    await criar(page, ITUB4);
    await criar(page, SEM_COTACAO);
    await criar(page, CRIPTO);
    await atualizarAgora(page);
    await page.getByRole("link", { name: "Investimentos", exact: true }).click();
    await expect(page.locator("main th")).toContainText(["Aplicado", "Atual", "Rentab."]);
    const linhas = page.locator("main tbody tr");
    await expect(linhas).toHaveCount(3);
    // 150 x R$ 39,78 = R$ 5.967,00 sobre R$ 4.713,00 aplicados.
    const itub = linhas.filter({ hasText: "ITUB4" });
    await expect(itub).toContainText("R$ 4.713,00");
    await expect(itub).toContainText("R$ 5.967,00");
    await expect(itub).toContainText("+26,6%");
    await expect(itub.locator("td.up")).toHaveText("+26,6%");
    const sem = linhas.filter({ hasText: "ZZZZ9" });
    await expect(sem).toContainText("R$ 100,00");
    await expect(sem).toContainText("sem cotação");
    await expect(sem.locator("td").nth(6)).toHaveText("—");
    const cripto = linhas.filter({ hasText: "Bitcoin" });
    await expect(cripto).toContainText("sem fonte de preço");
    await expect(cripto.locator("td").nth(6)).toHaveText("—");
    await expect(page.locator("main tfoot")).toContainText("R$ 6.567,00");
    await expect(page.locator("main")).toContainText("Valores brutos, antes do IR. Dados de mercado de 28/10.");
  } finally {
    await app.close();
  }
});

test("carteira mostra o Patrimônio e a nota de dados de mercado", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await expect(page.getByTestId("mercado")).toContainText("28/10", { timeout: 20000 });
    await criar(page, ITUB4);
    await atualizarAgora(page);
    await page.evaluate(() => {
      window.location.hash = "#/carteira";
    });
    const main = page.locator("main");
    await expect(main.getByText("Patrimônio")).toBeVisible();
    await expect(main.getByText("Total aplicado")).toBeVisible();
    await expect(main).toContainText("R$ 5.967,00");
    await expect(main).toContainText("Valores brutos, antes do IR. Dados de mercado de 28/10.");
    await main.getByRole("link", { name: /Ações/ }).click();
    await expect(main.locator("th")).toHaveText([
      "Investimento", "Instituição", "Rendimento", "Aplicado", "Atual", "Rentab.", "Vencimento", "Liquidez",
    ]);
    await expect(main.locator("tbody tr")).toContainText("+26,6%");
    await expect(main).toContainText("Valores brutos, antes do IR. Dados de mercado de 28/10.");
  } finally {
    await app.close();
  }
});

test("Atualizar desabilita o botão durante a atualização", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const mercado = page.getByTestId("mercado");
    await expect(mercado).toContainText("28/10", { timeout: 20000 });
    await page.evaluate(() => {
      window.__viu = false;
      new MutationObserver(() => {
        const b = [...document.querySelectorAll('[data-testid="mercado"] button')].find((x) => x.textContent === "Atualizando…");
        if (b && b.disabled) window.__viu = true;
      }).observe(document.querySelector('[data-testid="mercado"]'), { childList: true, subtree: true });
    });
    await mercado.getByRole("button", { name: "Atualizar" }).click();
    await expect.poll(() => page.evaluate(() => window.__viu)).toBe(true);
    await expect(mercado.getByRole("button", { name: "Atualizar" })).toBeEnabled();
  } finally {
    await app.close();
  }
});

test("fonte com falha mostra 'com falhas' e o diálogo lista o CDI com a mensagem", async () => {
  const pasta = fs.mkdtempSync(path.join(os.tmpdir(), "guru-fixtures-"));
  for (const nome of fs.readdirSync(FIXTURES)) {
    if (nome !== "sgs-12.json") fs.copyFileSync(path.join(FIXTURES, nome), path.join(pasta, nome));
  }
  // O 404 do SGS é "sem dados"; a falha vem da página de erro HTML com status 200.
  fs.copyFileSync(path.join(FIXTURES, "sgs-erro.html"), path.join(pasta, "sgs-12.json"));
  const app = await lancar({ GURU_MERCADO_FIXTURES: pasta });
  try {
    const page = await app.firstWindow();
    const mercado = page.getByTestId("mercado");
    const marca = mercado.getByRole("button", { name: "com falhas" });
    await expect(marca).toBeVisible({ timeout: 20000 });
    await marca.click();
    const dialogo = page.getByRole("dialog");
    await expect(dialogo).toContainText("CDI");
    await expect(dialogo.locator("li").filter({ hasText: "CDI" }).locator("span")).not.toBeEmpty();
    await dialogo.getByRole("button", { name: "Fechar" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
  } finally {
    await app.close();
    fs.rmSync(pasta, { recursive: true, force: true });
  }
});

test("salvar pelo formulário busca o valor atual sem clicar em Atualizar", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await expect(page.getByTestId("mercado")).toContainText("28/10", { timeout: 20000 });
    await page.evaluate(() => {
      window.location.hash = "#/investimentos/novo";
    });
    await expect(page.getByRole("heading", { name: "Novo investimento" })).toBeVisible();
    await page.getByLabel("Tipo").selectOption("rv-acao");
    await page.getByLabel("Ticker").fill("itub4");
    await page.getByLabel("Quantidade").fill("150");
    await page.getByLabel("Preço médio").fill("31,42");
    await page.getByLabel("Data da primeira compra").fill("2026-10-01");
    await page.getByRole("button", { name: "Salvar", exact: true }).click();
    const linha = page.locator("main tbody tr").first();
    await expect(linha).toContainText("ITUB4");
    await expect(linha).toContainText("R$ 5.967,00", { timeout: 20000 });
    await expect(linha).toContainText("+26,6%");
  } finally {
    await app.close();
  }
});
