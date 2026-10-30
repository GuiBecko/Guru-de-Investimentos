const { test, expect } = require("@playwright/test");
const { lancar } = require("./_app");

const ANO = new Date().getFullYear();
const VENC_CDB = `${ANO + 3}-03-15`;
const VENC_CDB_BR = `15/03/${ANO + 3}`;

const CDB = {
  tipo: "rf-cdb",
  instituicao: "Rico",
  emissor: "Banco Inter",
  indexador: "pos",
  taxa: "102",
  benchmark: "cdi",
  dataAplicacao: "2026-01-15",
  vencimento: VENC_CDB,
  valorAplicadoCentavos: 2000000,
  liquidez: "vencimento",
  ir: "regressiva",
};
const ITUB4 = {
  tipo: "rv-acao",
  instituicao: "Rico",
  ticker: "ITUB4",
  quantidade: 150,
  precoMedioCentavos: 3142,
  dataAplicacao: "2026-02-10",
};
const ITUB4_NOVO = { ...ITUB4, dataAplicacao: "2026-10-01" };

function criar(page, dados) {
  return page.evaluate((d) => import("./js/api.js").then(({ api }) => api.investimentos.criar(d)), dados);
}

async function atualizarAgora(page) {
  const mercado = page.getByTestId("mercado");
  await mercado.getByRole("button", { name: "Atualizar" }).click();
  await expect(mercado.getByRole("button", { name: "Atualizar", exact: true })).toBeEnabled({ timeout: 20000 });
}

async function abrir(page) {
  await page.evaluate(() => {
    window.location.hash = "#/carteira";
  });
}

test("painel mostra saudação, números, gráficos e listas", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await criar(page, ITUB4);
    await atualizarAgora(page);
    await abrir(page);
    const main = page.locator("main");

    await expect(main.getByTestId("saudacao")).toContainText(/^(Bom dia|Boa tarde|Boa noite)\./);
    await expect(main.getByTestId("data-extenso")).toContainText(/\d{4}/);
    await expect(main.getByTestId("data-extenso")).toContainText(/de (janeiro|fevereiro|março|abril|maio|junho|julho|agosto|setembro|outubro|novembro|dezembro) de/);
    await expect(main.getByRole("link", { name: "Tenho dinheiro para aportar" })).toHaveAttribute("href", "#/aporte");

    // Atual: CDB 20.000,00 + ITUB4 150 x 39,78 = 5.967,00. Aportado: 20.000,00 + 150 x 31,42 = 4.713,00.
    const patrimonio = main.getByTestId("numero-patrimonio");
    await expect(patrimonio).toContainText("Patrimônio total");
    await expect(patrimonio).toContainText("aportado R$ 24.713,00");
    await expect(patrimonio.locator(".mid")).toHaveText("R$ 25.967,00");

    const numeros = main.locator(".numeros .card b");
    await expect(numeros).toHaveText(["Patrimônio total", "No mês", "Em 12 meses", "Rendeu em 12 meses", "Total aportado"]);
    // CDI de 12 meses é nulo nas fixtures (começa em 2026-09-01).
    await expect(main.getByTestId("numero-periodo")).toContainText("CDI —");
    await main.getByRole("button", { name: "6 meses" }).click();
    await expect(main.getByRole("button", { name: "6 meses" })).toHaveAttribute("aria-pressed", "true");
    await expect(main.getByRole("button", { name: "12 meses" })).toHaveAttribute("aria-pressed", "false");
    await expect(numeros).toHaveText(["Patrimônio total", "No mês", "Em 6 meses", "Rendeu em 6 meses", "Total aportado"]);

    for (const rotulo of [
      "Evolução do patrimônio contra o total aportado",
      "Distribuição do patrimônio por classe",
      "Rentabilidade acumulada da carteira contra CDI, IPCA e BOVA11",
      "Rentabilidade mês a mês",
    ]) {
      await expect(main.locator(`svg[aria-label="${rotulo}"]`)).toHaveCount(1);
    }
    await expect(main).not.toContainText("NaN");

    const instituicao = main.getByTestId("lista-instituicoes");
    await expect(instituicao).toContainText("Rico");
    await expect(instituicao).toContainText("100,0%");

    const venc = main.getByTestId("lista-vencimentos");
    await expect(venc).toContainText(VENC_CDB_BR);
    await expect(venc.getByRole("link", { name: /CDB Banco Inter/ })).toHaveAttribute("href", /^#\/investimentos\/\d+$/);
    // As fixtures só têm a cotação de 28/10: sem a do fim de setembro não há variação no mês.
    await expect(main.getByTestId("lista-variacoes")).toContainText("Nada por aqui ainda.");
    await expect(main).toContainText("Valores brutos, antes do IR. Dados de mercado de");
  } finally {
    await app.close();
  }
});

test("carteira de um investimento só deste mês não tem histórico para comparar", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, ITUB4_NOVO);
    await atualizarAgora(page);
    await abrir(page);
    const main = page.locator("main");
    await expect(main).toContainText("Ainda não há histórico para comparar.");
    await expect(main.locator('svg[aria-label^="Rentabilidade"]')).toHaveCount(0);
    await expect(main.locator('svg[aria-label="Evolução do patrimônio contra o total aportado"]')).toHaveCount(1);
    await expect(main.getByTestId("lista-instituicoes")).toContainText("100,0%");
    await expect(main).not.toContainText("NaN");
  } finally {
    await app.close();
  }
});

test("listas sem itens mostram o texto de vazio", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, ITUB4_NOVO);
    await abrir(page);
    await expect(page.locator("main").getByTestId("lista-vencimentos")).toContainText("Nada por aqui ainda.");
  } finally {
    await app.close();
  }
});

test("trocar o período várias vezes seguidas termina no último escolhido", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await abrir(page);
    const main = page.locator("main");
    await expect(main.locator(".numeros .card b").nth(2)).toHaveText("Em 12 meses");
    const seis = main.getByRole("button", { name: "6 meses" });
    const doze = main.getByRole("button", { name: "12 meses" });
    for (let i = 0; i < 4; i++) {
      await seis.click();
      await doze.click();
    }
    await seis.click();
    await expect(main.locator(".numeros .card b").nth(2)).toHaveText("Em 6 meses");
    await expect(seis).toHaveAttribute("aria-pressed", "true");
    // A resposta antiga não volta a sobrescrever: o texto continua o do último pedido.
    await page.waitForTimeout(500);
    await expect(main.locator(".numeros .card b").nth(3)).toHaveText("Rendeu em 6 meses");
  } finally {
    await app.close();
  }
});
