const { test, expect } = require("@playwright/test");
const { lancar } = require("./_app");

const main = (page) => page.locator("main");
const ir = (page, hash) =>
  page.evaluate((h) => {
    window.location.hash = h;
  }, hash);
const chamar = (page, fn, arg) => page.evaluate(`import("./js/api.js").then(({ api }) => (${fn})(api, ${JSON.stringify(arg ?? null)}))`);
const tokens = async (page) => (await chamar(page, "(api) => api.ia.config()")).uso.tokens;

const ANO = new Date().getFullYear();
const ITUB4 = {
  tipo: "rv-acao",
  instituicao: "Rico",
  ticker: "ITUB4",
  quantidade: 150,
  precoMedioCentavos: 3142,
  dataAplicacao: "2025-05-10",
};
const CDB = {
  tipo: "rf-cdb",
  instituicao: "Banco Inter",
  emissor: "Banco Inter",
  indexador: "pos",
  taxa: "102",
  benchmark: "cdi",
  dataAplicacao: "2025-03-15",
  vencimento: `${ANO + 3}-03-15`,
  valorAplicadoCentavos: 900000,
  liquidez: "vencimento",
  ir: "regressiva",
};
const CONVITE = "Para ver insights e notícias da sua carteira, configure a sua chave.";
const ETIQUETAS = ["Rebalanceamento", "Meta", "Vencimento", "Concentração", "Mercado"];
const VENCIMENTO = "Nenhum investimento vence nos próximos 12 meses; o CDB vence em 15/03/2028.";
const INVALIDA = "A IA devolveu os insights num formato inesperado. Tente de novo.";
const DESLIGADA = "Ligue “Buscar na web” em Configurar IA para buscar notícias.";

function vigiarErros(page) {
  const erros = [];
  page.on("pageerror", (e) => erros.push(String(e)));
  page.on("console", (m) => {
    if (m.type() === "error") erros.push(m.text());
  });
  return erros;
}

async function preparar(page) {
  await chamar(page, "(api, d) => api.investimentos.criar(d)", ITUB4);
  await chamar(page, "(api, d) => api.investimentos.criar(d)", CDB);
}

async function abrirCarteira(page) {
  await ir(page, "#/investimentos");
  await expect(main(page).getByRole("heading", { name: "Investimentos" })).toBeVisible();
  await ir(page, "#/carteira");
  await expect(main(page).getByTestId("saudacao")).toBeVisible();
}

async function abrirIa(page) {
  await ir(page, "#/ia");
  await expect(main(page).getByRole("heading", { name: "Configurar IA" })).toBeVisible();
}

async function salvarChave(page) {
  await abrirIa(page);
  await page.getByLabel("Chave da API da Anthropic").fill("sk-ant-teste-boa-12345678-4f2a");
  await main(page).getByRole("button", { name: "Salvar chave" }).click();
  await expect(main(page).getByText("ativo", { exact: true })).toBeVisible();
}

const coluna = (page) => main(page).getByTestId("coluna-ia");
const cartoes = (page) => coluna(page).getByTestId("insight");

test("sem chave, a Carteira mostra o convite da coluna", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const erros = vigiarErros(page);
    await preparar(page);
    await abrirCarteira(page);
    await expect(coluna(page)).toContainText(CONVITE);
    await expect(coluna(page).getByRole("link", { name: "Configurar IA" })).toHaveAttribute("href", "#/ia");
    await expect(coluna(page).getByRole("button")).toHaveCount(0);
    expect(erros).toEqual([]);
  } finally {
    await app.close();
  }
});

test("com chave, insights e notícias só com clique, guardados e com link", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const erros = vigiarErros(page);
    // O núcleo data a geração pelo dia do modo de teste (GURU_MERCADO_HOJE); a página usa o mesmo dia.
    await page.clock.setFixedTime(new Date(`${process.env.GURU_MERCADO_HOJE}T12:00:00`));
    await salvarChave(page);
    await preparar(page);
    await abrirCarteira(page);

    // Abrir a Carteira não chama a IA.
    await expect(coluna(page)).toContainText("Nenhum insight gerado ainda.");
    await expect(coluna(page)).toContainText("Cada geração usa a sua chave da Anthropic.");
    await expect(coluna(page)).toContainText("Gerado por IA com os seus dados. Pode conter erros. Não é recomendação individual de investimento.");
    expect(await tokens(page)).toBe(0);

    // O começo e o fim da geração saem numa região viva (role="status") do bloco.
    const statusInsights = coluna(page).getByRole("region", { name: "Insights" }).getByRole("status");
    await expect(statusInsights).toHaveText("");
    const começo = await page.evaluate(() => {
      const bloco = document.querySelector("[data-testid=coluna-ia] .ia-insights");
      [...bloco.querySelectorAll("button")].find((b) => b.textContent === "Gerar insights").click();
      return document.querySelector("[data-testid=coluna-ia] .ia-insights [role=status]").textContent;
    });
    expect(começo).toBe("Gerando insights…");
    await expect(cartoes(page)).toHaveCount(5);
    await expect(statusInsights).toHaveText("Insights gerados.");
    await expect(cartoes(page).locator(".pill")).toHaveText(ETIQUETAS);
    await expect(cartoes(page).first().locator(".pill")).toHaveClass(/\bwa\b/);
    await expect(cartoes(page).nth(1).locator(".pill")).not.toHaveClass(/\bwa\b/);
    await expect(coluna(page).getByText("Claude · hoje", { exact: true })).toBeVisible();
    await expect(coluna(page).getByRole("link", { name: "Simular próximo aporte →" })).toHaveAttribute("href", "#/aporte");
    await expect(coluna(page).getByRole("link", { name: "Ver Ações →" })).toHaveAttribute("href", "#/carteira/acoes");
    await expect(coluna(page).getByRole("button", { name: "Gerar de novo" })).toBeVisible();
    const uso = await tokens(page);
    expect(uso).toBeGreaterThan(0);

    // Voltar à Carteira mostra os mesmos, sem gastar.
    await abrirCarteira(page);
    await expect(cartoes(page)).toHaveCount(5);
    await expect(cartoes(page).locator(".pill")).toHaveText(ETIQUETAS);
    expect(await tokens(page)).toBe(uso);

    // Saída fora do esquema: a mensagem aparece e os anteriores continuam.
    await abrirIa(page);
    await page.getByLabel("Instruções para a sua IA").fill("#insights-invalido");
    await main(page).getByRole("button", { name: "Salvar instruções" }).click();
    await expect(page.getByTestId("parte-instrucoes")).toContainText("#insights-invalido");
    await abrirCarteira(page);
    await coluna(page).getByRole("button", { name: "Gerar de novo" }).click();
    await expect(coluna(page).getByRole("alert")).toHaveText(INVALIDA);
    await expect(cartoes(page)).toHaveCount(5);
    await expect(cartoes(page).locator(".pill")).toHaveText(ETIQUETAS);
    await expect(coluna(page).getByRole("button", { name: "Gerar de novo" })).toBeEnabled();

    // Conversar sobre isso: o Assistente abre com o campo preenchido, sem enviar nada.
    const antesDoChat = await tokens(page);
    await coluna(page).getByRole("link", { name: "Conversar sobre isso →" }).click();
    await expect(page).toHaveURL(/#\/assistente\/insight\/2$/);
    await expect(main(page).getByLabel("Sua pergunta")).toHaveValue(`Sobre este insight do painel: "${VENCIMENTO}" `);
    await expect(main(page).getByTestId("pendente")).toHaveCount(0);
    await expect(main(page).getByTestId("mensagem")).toHaveCount(0);
    expect(await tokens(page)).toBe(antesDoChat);

    // Índice inválido: o Assistente normal, sem erro.
    await ir(page, "#/assistente/insight/9");
    await expect(main(page).getByLabel("Sua pergunta")).toHaveValue("");
    await expect(main(page).getByRole("alert")).toBeHidden();

    // Notícias: só as citadas com https, cinco à vista e o resto no botão.
    await abrirCarteira(page);
    const secao = coluna(page).getByTestId("noticias");
    await expect(secao).toContainText("Notícias da sua carteira");
    const statusNoticias = secao.getByRole("status");
    await expect(statusNoticias).toHaveText("");
    const começoNoticias = await page.evaluate(() => {
      const bloco = document.querySelector("[data-testid=noticias]");
      [...bloco.querySelectorAll("button")].find((b) => b.textContent === "Buscar notícias").click();
      // O clique redesenha a coluna: a seção é lida de novo.
      return document.querySelector("[data-testid=noticias] [role=status]").textContent;
    });
    expect(começoNoticias).toBe("Buscando notícias na web…");
    const links = secao.getByTestId("noticia");
    await expect(links).toHaveCount(5);
    await expect(statusNoticias).toHaveText("Notícias atualizadas.");
    for (const href of await links.evaluateAll((as) => as.map((a) => a.getAttribute("href")))) expect(href).toMatch(/^https:\/\//);
    await expect(secao.locator('a[href^="http:"]')).toHaveCount(0);
    await expect(links.first()).toContainText("Itaú Unibanco tem lucro recorde no terceiro trimestre");
    await expect(links.first()).toContainText("infomoney.com.br");
    await expect(links.first().locator(".pill")).toHaveText("ITUB4");
    await expect(links.filter({ hasText: "XPTO conclui oferta de ações" }).locator(".pill")).toHaveText("Mercado");
    await expect(secao).not.toContainText("Ibovespa fecha outubro em alta");
    await secao.getByRole("button", { name: "Ver todas as notícias →" }).click();
    await expect(links).toHaveCount(6);
    await expect(links.nth(5)).toContainText("Ibovespa fecha outubro em alta");
    await expect(secao.getByRole("button", { name: "Buscar de novo" })).toBeVisible();

    // A parte Mercado do contexto leva as 3 primeiras.
    await abrirIa(page);
    await expect(page.getByTestId("parte-mercado")).toContainText("automático · 3 notícias");

    // Com a busca desligada, a mensagem no lugar do botão.
    await page.getByRole("switch", { name: "Buscar na web" }).uncheck();
    await expect(page.getByRole("switch", { name: "Buscar na web" })).not.toBeChecked();
    await abrirCarteira(page);
    await expect(coluna(page).getByTestId("noticias")).toContainText(DESLIGADA);
    await expect(coluna(page).getByTestId("noticias").getByRole("link", { name: "Configurar IA" })).toHaveAttribute("href", "#/ia");
    await expect(coluna(page).getByTestId("noticias").getByRole("button", { name: /Buscar/ })).toHaveCount(0);
    await expect(coluna(page).getByTestId("noticia")).toHaveCount(5);

    expect(erros).toEqual([]);
  } finally {
    await app.close();
  }
});

test("sair da Carteira durante a geração não para, e voltar mostra o resultado", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const erros = vigiarErros(page);
    await salvarChave(page);
    await preparar(page);
    await abrirCarteira(page);
    await coluna(page).getByRole("button", { name: "Gerar insights" }).click();
    await ir(page, "#/investimentos");
    await expect(main(page).getByRole("heading", { name: "Investimentos" })).toBeVisible();
    await ir(page, "#/carteira");
    await expect(cartoes(page)).toHaveCount(5);
    await expect(coluna(page).getByRole("alert")).toBeHidden();
    expect(erros).toEqual([]);
  } finally {
    await app.close();
  }
});
