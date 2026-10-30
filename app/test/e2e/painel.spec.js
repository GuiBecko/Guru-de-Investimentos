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
// Aplicado no dia dos dados de mercado: não ficou aplicado nenhum dia, então não há retorno.
const ITUB4_HOJE = { ...ITUB4, dataAplicacao: "2026-10-28" };

// O CDB de 15/01/2026 é posterior a 31/10/2025, o primeiro ponto de 12 meses: o período começa nele.
const DOZE = "Desde 01/26";

// Tamanho na tela do texto dos eixos: fonte do CSS vezes a escala do viewBox.
function menorFonteDosEixos(page) {
  return page.evaluate(() => {
    const tamanhos = [...document.querySelectorAll("main svg.chart text.eixo")].map(
      (t) => parseFloat(getComputedStyle(t).fontSize) * t.ownerSVGElement.getScreenCTM().a,
    );
    return tamanhos.length ? Math.min(...tamanhos) : 0;
  });
}

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
    await expect(numeros).toHaveText(["Patrimônio total", "No mês", DOZE, "Rendeu desde 01/26", "Total aportado"]);
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
      await expect(main.locator(`svg[aria-label^="${rotulo}"]`)).toHaveCount(1);
    }
    // O rótulo acessível termina com o último ponto de cada série visível.
    await expect(main.locator('svg[aria-label^="Rentabilidade acumulada da carteira"]')).toHaveAttribute(
      "aria-label",
      /\. Último ponto: Carteira [+−-]?\d+,\d+%(, IPCA [+−-]?\d+,\d+%)?$/,
    );
    await expect(main.locator('svg[aria-label^="Evolução do patrimônio"]')).toHaveAttribute(
      "aria-label",
      /Último ponto: Patrimônio R\$ 25\.967, Total aportado R\$ 24\.713$/,
    );
    await expect(main.locator('svg[aria-label^="Rentabilidade mês a mês"]')).toHaveAttribute("aria-label", /\. Último mês: [+−-]?\d+,\d+%$/);
    // Minigráficos são decorativos; BOVA11 tem cor própria (não o verde da carteira).
    await expect(main.locator(".classes svg.mini").first()).toHaveAttribute("aria-hidden", "true");
    await expect(main.locator(".classes svg.mini[role]")).toHaveCount(0);
    await expect(main.locator(".legenda li").filter({ hasText: "BOVA11" }).locator(".marca.s-c4")).toHaveCount(1);
    // Na janela normal (1280 × 800): os cinco números numa linha e os gráficos em duas colunas.
    await expect(main.locator(".numeros")).not.toHaveClass(/apertado/);
    for (const linha of [".graficos.g1", ".graficos.g2"]) {
      await expect.poll(() => main.locator(linha).evaluate((g) => getComputedStyle(g).gridTemplateColumns.split(" ").length)).toBe(2);
    }
    // Texto dos eixos com pelo menos 10 px na tela, também na meia coluna.
    await expect.poll(() => menorFonteDosEixos(page)).toBeGreaterThanOrEqual(10);
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

test("carteira aplicada no dia dos dados não tem histórico para comparar", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, ITUB4_HOJE);
    await atualizarAgora(page);
    await abrir(page);
    const main = page.locator("main");
    await expect(main).toContainText("Ainda não há histórico para comparar.");
    // Sem histórico, o período não tem valor; começa no aporte.
    await expect(main.getByTestId("numero-periodo").locator("b")).toHaveText("Desde 10/26");
    await expect(main.getByTestId("numero-periodo").locator(".mid")).toHaveText("—");
    await expect(main.locator('svg[aria-label^="Rentabilidade"]')).toHaveCount(0);
    await expect(main.locator('svg[aria-label^="Evolução do patrimônio contra o total aportado"]')).toHaveCount(1);
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

test("a resposta atrasada de um período antigo não sobrescreve o último escolhido", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await abrir(page);
    const main = page.locator("main");
    const rotulo = main.locator(".numeros .card b").nth(2);
    await expect(rotulo).toHaveText(DOZE);
    // A chamada de 6 meses passa a resolver bem depois da de 12; uma marca avisa quando ela chega
    // (num setTimeout seguinte, depois de a tela ter tratado a resposta).
    await page.evaluate(() =>
      import("./js/api.js").then(({ api }) => {
        const original = api.carteira.painel;
        window.__atrasadaChegou = false;
        api.carteira.painel = (meses, classe) =>
          original(meses, classe).then((r) =>
            meses === 6
              ? new Promise((ok) =>
                  setTimeout(() => {
                    ok(r);
                    setTimeout(() => {
                      window.__atrasadaChegou = true;
                    }, 0);
                  }, 1200),
                )
              : r,
          );
      }),
    );
    await main.getByRole("button", { name: "6 meses" }).click();
    await main.getByRole("button", { name: "12 meses" }).click();
    await expect(rotulo).toHaveText(DOZE);
    await page.waitForFunction(() => window.__atrasadaChegou === true);
    await expect(rotulo).toHaveText(DOZE);
    await expect(main.locator(".numeros .card b").nth(3)).toHaveText("Rendeu desde 01/26");
    await expect(main.getByRole("button", { name: "12 meses" })).toHaveAttribute("aria-pressed", "true");
  } finally {
    await app.close();
  }
});

test("falha ao trocar o período mantém os números, avisa nos gráficos e permite tentar de novo", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await abrir(page);
    const main = page.locator("main");
    await expect(main.locator(".numeros .card b").nth(2)).toHaveText(DOZE);
    await page.evaluate(() =>
      import("./js/api.js").then(({ api }) => {
        const original = api.carteira.painel;
        window.__falhar = true;
        api.carteira.painel = (m, c) => (window.__falhar && m === 6 ? Promise.reject(new Error("falha")) : original(m, c));
      }),
    );
    await main.getByRole("button", { name: "6 meses" }).click();
    await expect(main.getByRole("alert")).toContainText("Não foi possível carregar o período.");
    await expect(main.locator(".numeros .card b").nth(2)).toHaveText(DOZE);
    await expect(main.getByRole("button", { name: "12 meses" })).toHaveAttribute("aria-pressed", "true");
    // Clicar no botão já pressionado (12) tenta de novo e traz os gráficos de volta.
    await main.getByRole("button", { name: "12 meses" }).click();
    await expect(main.getByRole("alert")).toHaveCount(0);
    await expect(main.locator('svg[aria-label^="Rentabilidade mês a mês"]')).toHaveCount(1);
    await page.evaluate(() => {
      window.__falhar = false;
    });
    await main.getByRole("button", { name: "6 meses" }).click();
    await expect(main.locator(".numeros .card b").nth(2)).toHaveText("Em 6 meses");
    await expect(main.getByRole("alert")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("a escolha de 6 meses sobrevive ao redesenho depois de uma atualização de mercado", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    await abrir(page);
    const main = page.locator("main");
    await main.getByRole("button", { name: "6 meses" }).click();
    await expect(main.locator(".numeros .card b").nth(2)).toHaveText("Em 6 meses");
    // Marca os números atuais para saber quando a tela foi redesenhada.
    await page.evaluate(() => {
      document.querySelector("main .numeros").dataset.antigo = "1";
    });
    await atualizarAgora(page);
    await expect(main.locator(".numeros[data-antigo]")).toHaveCount(0);
    await expect(main.locator(".numeros .card b").nth(2)).toHaveText("Em 6 meses");
    await expect(main.getByRole("button", { name: "6 meses" })).toHaveAttribute("aria-pressed", "true");
    await expect(main.getByRole("button", { name: "12 meses" })).toHaveAttribute("aria-pressed", "false");
  } finally {
    await app.close();
  }
});

test("a saudação só diz que a carteira está atualizada quando nenhuma fonte tem erro", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await criar(page, CDB);
    // Com CDB a custo, os modelos são 5 e o índice 1 é a frase de "atualizada"; sem ela, 4.
    await page.evaluate(() => {
      Math.random = () => 0.25;
    });
    await abrir(page);
    const saudacao = page.locator("main").getByTestId("saudacao");
    await expect(saudacao).toContainText("Sua carteira está atualizada com os dados de mercado mais recentes.");
    await page.evaluate(() =>
      import("./js/api.js").then(({ api }) => {
        const original = api.mercado.status;
        api.mercado.status = async () => [...(await original()), { fonte: "b3", rotulo: "Cotações da B3", ultimaData: null, atualizadoEm: null, erro: "A B3 não respondeu." }];
        window.location.hash = "#/investimentos";
      }),
    );
    await expect(saudacao).toHaveCount(0);
    await abrir(page);
    await expect(saudacao).toBeVisible();
    await expect(saudacao).not.toContainText("atualizada");
  } finally {
    await app.close();
  }
});
