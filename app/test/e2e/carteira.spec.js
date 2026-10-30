const { test, expect } = require("@playwright/test");
const { lancar } = require("./_app");
const path = require("node:path");

// Datas futuras derivadas do ano atual, para os testes não vencerem com o tempo.
const ANO = new Date().getFullYear();
const VENC_CDB = `${ANO + 3}-03-15`;
const VENC_CDB_BR = `15/03/${ANO + 3}`;
const VENC_SELIC = `${ANO + 4}-03-01`;
const PRAZO = `${ANO + 4}-12`;


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
const SELIC = {
  tipo: "td-selic",
  instituicao: "XP Investimentos",
  indexador: "selic",
  taxa: "0.0905",
  dataAplicacao: "2025-03-01",
  vencimento: VENC_SELIC,
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
    await expect(main.locator(".numeros").getByText("Total aportado", { exact: true })).toBeVisible();
    await expect(main).toContainText("R$ 42.713,00");
    await expect(main).toContainText("Valores brutos, antes do IR. Dados de mercado de");

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
    await expect(linhas.nth(0)).toContainText(VENC_CDB_BR);
    await expect(linhas.nth(1)).toContainText(`Tesouro Selic ${ANO + 4}`);
    await expect(main).not.toContainText("undefined");
    await expect(main.locator("th")).toHaveText([
      "Investimento", "Instituição", "Rendimento", "Aplicado", "Atual", "Rentab.", "Vencimento", "Liquidez",
    ]);

    await main.getByRole("link", { name: "Carteira ›" }).click();
    await expect(main.locator(".numeros").getByText("Total aportado", { exact: true })).toBeVisible();
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

test("com alocação-alvo cada card mostra o alvo e a diferença em pontos", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await criar(page, SELIC);
    await criar(page, ITUB4);
    await page.evaluate(() =>
      import("./js/api.js").then(({ api }) =>
        api.alocacao.salvar([
          { classe: "rf", percentual: 45 },
          { classe: "acoes", percentual: 30 },
          { classe: "fii", percentual: 15 },
          { classe: "ext", percentual: 10 },
          { classe: "outros", percentual: 0 },
        ]),
      ),
    );
    await page.evaluate(() => {
      window.location.hash = "#/carteira";
    });
    const main = page.locator("main");
    const rf = main.getByRole("link", { name: /Renda fixa/ });
    await expect(rf).toContainText("alvo 45%");
    await expect(rf).toContainText("+44,0 p.p.");
    const acoes = main.getByRole("link", { name: /Ações/ });
    await expect(acoes).toContainText("alvo 30%");
    await expect(acoes).toContainText("\u221219,0 p.p.");
    // Classe com alvo e sem investimento aparece zerada.
    // Sem investimento, o card não é link.
    await expect(main.getByRole("link", { name: /FIIs/ })).toHaveCount(0);
    const fii = main.locator("div.card", { hasText: "FIIs" });
    await expect(fii).toContainText("R$ 0,00");
    await expect(fii).toContainText("alvo 15%");
    await expect(fii).toContainText("\u221215,0 p.p.");
    // Alvo 0 e sem investimento não aparece.
    await expect(main.getByRole("link", { name: /Outros/ })).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("diferença menor que meio ponto aparece como no alvo", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await criar(page, SELIC);
    await criar(page, ITUB4);
    await page.evaluate(() =>
      import("./js/api.js").then(({ api }) =>
        api.alocacao.salvar([
          { classe: "rf", percentual: 89 },
          { classe: "acoes", percentual: 11 },
          { classe: "fii", percentual: 0 },
          { classe: "ext", percentual: 0 },
          { classe: "outros", percentual: 0 },
        ]),
      ),
    );
    await page.evaluate(() => {
      window.location.hash = "#/carteira";
    });
    const rf = page.locator("main").getByRole("link", { name: /Renda fixa/ });
    await expect(rf).toContainText("alvo 89%");
    await expect(rf).toContainText("no alvo");
  } finally {
    await app.close();
  }
});

test("sem alocação-alvo os cards não mostram alvo", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await page.evaluate(() => {
      window.location.hash = "#/carteira";
    });
    const rf = page.locator("main").getByRole("link", { name: /Renda fixa/ });
    await expect(rf).toContainText("100,0%");
    await expect(rf).not.toContainText("alvo");
  } finally {
    await app.close();
  }
});

test("só com alvo e sem investimentos a carteira continua vazia", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await page.evaluate(() =>
      import("./js/api.js").then(({ api }) =>
        api.alocacao.salvar([
          { classe: "rf", percentual: 100 },
          { classe: "acoes", percentual: 0 },
          { classe: "fii", percentual: 0 },
          { classe: "ext", percentual: 0 },
          { classe: "outros", percentual: 0 },
        ]),
      ),
    );
    await page.evaluate(() => {
      window.location.hash = "#/carteira";
    });
    const main = page.locator("main");
    await expect(main).toContainText("Sua carteira está vazia.");
    await expect(main.getByRole("link", { name: "Lançar o primeiro investimento" })).toBeVisible();
  } finally {
    await app.close();
  }
});

// Aplicados antes de um fim de mês do período das fixtures, para a classe ter retorno em outubro.
const CDB_COM_HISTORICO = { ...CDB, dataAplicacao: "2026-08-31" };
const ITUB4_COM_HISTORICO = { ...ITUB4, dataAplicacao: "2026-08-03" };

test("detalhe da classe sem histórico mostra o aviso no lugar do seletor e do gráfico", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, { ...CDB, dataAplicacao: "2026-10-20" });
    await page.evaluate(() => {
      window.location.hash = "#/carteira/rf";
    });
    const main = page.locator("main");
    await expect(main.getByRole("heading", { name: "Renda fixa" })).toBeVisible();
    await expect(main).toContainText("Ainda não há histórico para comparar.");
    await expect(main.getByText("Em 12 meses")).toBeVisible();
    await expect(main.getByRole("group", { name: "Benchmark" })).toHaveCount(0);
    await expect(main.getByRole("img", { name: /Rentabilidade acumulada da classe/ })).toHaveCount(0);
    await expect(main.locator("tbody tr")).toHaveCount(1);
    await expect(main).not.toContainText("NaN");
  } finally {
    await app.close();
  }
});

test("detalhe da classe: cabeçalho, benchmark padrão e troca sem refazer a chamada", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB_COM_HISTORICO);
    await criar(page, SELIC);
    await criar(page, ITUB4_COM_HISTORICO);
    const main = page.locator("main");
    const grupo = main.getByRole("group", { name: "Benchmark" });
    const chamadas = () => page.evaluate(() => window.__chamadasPainel ?? 0);
    const abrir = async (hash) => {
      await page.evaluate((h) => {
        window.location.hash = h;
      }, hash);
    };

    await abrir("#/carteira/acoes");
    await expect(grupo.getByRole("button", { name: "BOVA11" })).toHaveAttribute("aria-pressed", "true");
    await expect(main.getByText("Sua ações")).toBeVisible();

    await abrir("#/carteira/rf");
    await expect(main.getByRole("heading", { name: "Renda fixa" })).toBeVisible();
    await expect(main.getByText("Em 12 meses")).toBeVisible();
    await expect(main.getByText("Valor", { exact: true })).toBeVisible();
    await expect(grupo.getByRole("button")).toHaveText(["CDI", "Selic", "IPCA", "BOVA11", "XFIX11"]);
    await expect(grupo.getByRole("button", { name: "CDI" })).toHaveAttribute("aria-pressed", "true");
    await expect(main.getByText("Sua renda fixa")).toBeVisible();
    await expect(main.getByRole("img", { name: "Rentabilidade acumulada da classe contra o benchmark" })).toBeVisible();
    await expect(main.locator("tbody tr")).toHaveCount(2);

    // Conta as chamadas a partir daqui: trocar de benchmark não pode chamar a API de novo.
    await page.evaluate(async () => {
      const { api } = await import("./js/api.js");
      const original = api.carteira.painel;
      window.__chamadasPainel = 0;
      api.carteira.painel = (...args) => {
        window.__chamadasPainel += 1;
        return original(...args);
      };
    });
    await grupo.getByRole("button", { name: "IPCA" }).click();
    await expect(grupo.getByRole("button", { name: "IPCA" })).toHaveAttribute("aria-pressed", "true");
    await expect(grupo.getByRole("button", { name: "CDI" })).toHaveAttribute("aria-pressed", "false");
    await expect(main.locator(".legenda")).toContainText("IPCA");
    await expect(main.locator(".legenda")).not.toContainText("CDI");
    // IPCA tem dados no período: a linha do benchmark é desenhada (classe + benchmark = 2 séries).
    await expect(main).not.toContainText("sem dados no período.");
    await expect(main.locator("svg.chart path.serie")).toHaveCount(2);

    await grupo.getByRole("button", { name: "BOVA11" }).click();
    await expect(grupo.getByRole("button", { name: "BOVA11" })).toBeEnabled();
    await expect(main).toContainText("BOVA11 (Ibovespa): sem dados no período.");
    await expect(main.locator("svg.chart path.serie")).toHaveCount(1);
    await grupo.getByRole("button", { name: "CDI" }).click();
    await expect(grupo.getByRole("button", { name: "CDI" })).toHaveAttribute("aria-pressed", "true");
    expect(await chamadas()).toBe(0);
  } finally {
    await app.close();
  }
});
