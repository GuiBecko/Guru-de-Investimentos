const { test, expect } = require("@playwright/test");
const { lancar } = require("./_app");

const main = (page) => page.locator("main");
const ir = (page, hash) =>
  page.evaluate((h) => {
    window.location.hash = h;
  }, hash);
const chamar = (page, fn, arg) => page.evaluate(`import("./js/api.js").then(({ api }) => (${fn})(api, ${JSON.stringify(arg ?? null)}))`);

const ANO = new Date().getFullYear();
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
const ALVO = [
  { classe: "rf", percentual: 40 },
  { classe: "acoes", percentual: 30 },
  { classe: "fii", percentual: 15 },
  { classe: "ext", percentual: 15 },
  { classe: "outros", percentual: 0 },
];
const CONVITE = "Para conversar com a IA sobre cada classe, configure a sua chave.";
const SUGESTOES = [
  "Quais investimentos de Ações combinam com a direção que escolhi?",
  "Compare duas opções para esse valor, com prós e contras.",
  "Quais custos, impostos e riscos devo olhar nesta classe?",
];
const CONTEXTO =
  'Contexto: aporte de R$ 2.000,00 · direção: Ações, Experimentar algo novo, "quero algo com liquidez" · meta: Entrada do apartamento · mais as partes ligadas em Configurar IA';

// Alvo, um CDB e uma meta pela api; depois abre o Próximo aporte.
async function preparar(page) {
  await chamar(page, "(api, a) => api.alocacao.salvar(a)", ALVO);
  await chamar(page, "(api, d) => api.investimentos.criar(d)", CDB);
  await chamar(page, "(api, m) => api.metas.criar(m)", { nome: "Entrada do apartamento", valorAlvoCentavos: 15000000, prazo: `${ANO + 4}-12` });
  await ir(page, "#/aporte");
  await expect(main(page).getByRole("heading", { name: "Próximo aporte" })).toBeVisible();
}

async function salvarChave(page) {
  await page.getByRole("link", { name: "Configurar IA", exact: true }).click();
  await page.getByLabel("Chave da API da Anthropic").fill("sk-ant-teste-boa-12345678-4f2a");
  await main(page).getByRole("button", { name: "Salvar chave" }).click();
  await expect(main(page).getByText("ativo", { exact: true })).toBeVisible();
}

const dividir = async (page, texto) => {
  await main(page).getByLabel("Quanto vai aportar?").fill(texto);
  await main(page).getByRole("button", { name: "Dividir" }).click();
};
const valores = async (page) => {
  await expect(main(page).getByTestId("parte").first()).toBeVisible();
  return main(page).getByTestId("parte").locator(".mid").allTextContents();
};
const gaveta = (page) => main(page).getByTestId("gaveta");
const respostas = (loc) => loc.locator('[data-testid="mensagem"][data-papel="assistant"]');

function vigiarErros(page) {
  const erros = [];
  page.on("pageerror", (e) => erros.push(String(e)));
  page.on("console", (m) => {
    if (m.type() === "error") erros.push(m.text());
  });
  return erros;
}

test("sem chave, dividir mostra o convite e nenhum Conversar com a IA", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const erros = vigiarErros(page);
    await preparar(page);
    await dividir(page, "2.000,00");
    await expect(main(page).getByTestId("parte").first()).toBeVisible();
    await expect(main(page)).toContainText(CONVITE);
    await expect(main(page).getByRole("link", { name: "Configurar IA" })).toHaveAttribute("href", "#/ia");
    await expect(main(page).getByRole("button", { name: /Conversar com a IA/ })).toHaveCount(0);
    await expect(gaveta(page)).toHaveCount(0);
    expect(erros).toEqual([]);
  } finally {
    await app.close();
  }
});

test("com chave, a gaveta de uma classe conversa com o foco do aporte", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const erros = vigiarErros(page);
    await salvarChave(page);
    await preparar(page);

    // A meta cadastrada aparece no select; direção e meta não mudam a divisão.
    const meta = main(page).getByLabel("Para qual meta?");
    await expect(meta.locator("option")).toHaveText(["Nenhuma meta específica", "Entrada do apartamento"]);
    await dividir(page, "2.000,00");
    const semDirecao = await valores(page);
    const grupo = main(page).getByRole("group", { name: "Para onde quer levar esse dinheiro?" });
    for (const d of ["Renda fixa", "Ações", "FIIs", "Exterior", "Experimentar algo novo", "Mais segurança"])
      await expect(grupo.getByLabel(d, { exact: true })).toBeVisible();
    await grupo.getByLabel("Ações", { exact: true }).check();
    await grupo.getByLabel("Experimentar algo novo", { exact: true }).check();
    const livre = main(page).getByLabel("Outra direção ou detalhe (opcional)");
    await expect(livre).toHaveAttribute("maxlength", "300");
    await livre.fill("quero algo com liquidez");
    await meta.selectOption({ label: "Entrada do apartamento" });
    await main(page).getByRole("button", { name: "Dividir" }).click();
    await expect(main(page).getByTestId("parte")).toHaveCount(semDirecao.length);
    expect(await valores(page)).toEqual(semDirecao);
    await expect(main(page)).not.toContainText(CONVITE);

    // Toda classe tem o botão; abrir a de Ações mostra o título e o contexto, sem chamar a IA.
    await expect(main(page).getByRole("button", { name: /^Conversar com a IA sobre / })).toHaveCount(semDirecao.length);
    const acoes = main(page).getByTestId("parte").filter({ hasText: "Ações" });
    const valorAcoes = await acoes.locator(".mid").textContent();
    await main(page).getByRole("button", { name: "Conversar com a IA sobre Ações" }).click();
    const g = gaveta(page);
    await expect(g.getByRole("heading", { name: `Ações · ${valorAcoes}` })).toBeVisible();
    await expect(g.getByText("Claude", { exact: true })).toBeVisible();
    await expect(g.getByText(CONTEXTO, { exact: true })).toBeVisible();
    await expect(g.getByLabel("Sua pergunta")).toHaveAttribute("placeholder", "Pergunte sobre esta classe…");
    for (const s of SUGESTOES) await expect(g.getByRole("button", { name: s, exact: true })).toBeVisible();
    await expect(g.getByRole("link", { name: "Registrar aporte" })).toHaveAttribute("href", "#/investimentos/novo");
    await expect(g.getByRole("link", { name: "Ver no Assistente" })).toHaveCount(0);
    await expect(g.getByText("O que a IA está vendo")).toBeVisible();
    expect((await chamar(page, "(api) => api.ia.config()")).uso.tokens).toBe(0);

    // A primeira sugestão: o formulário fica desabilitado durante a resposta.
    const durante = await page.evaluate((s) => {
      [...document.querySelectorAll("[data-testid=gaveta] button")].find((b) => b.textContent === s).click();
      return {
        valor: document.getElementById("aporte-valor").matches(":disabled"),
        meta: document.getElementById("aporte-meta").matches(":disabled"),
        direcao: document.getElementById("aporte-direcao").matches(":disabled"),
        dividir: [...document.querySelectorAll("main button")].find((b) => b.textContent === "Dividir").matches(":disabled"),
      };
    }, SUGESTOES[0]);
    expect(durante).toEqual({ valor: true, meta: true, direcao: true, dividir: true });
    await expect(respostas(g)).toHaveCount(1);
    await expect(respostas(g).first()).toContainText("A decisão é sua.");
    await expect(g.getByTestId("seus-dados")).toContainText("Próximo aporte");
    await expect(main(page).getByLabel("Quanto vai aportar?")).toBeEnabled();

    // Uma segunda pergunta na mesma conversa.
    await g.getByLabel("Sua pergunta").fill("E para quem quer liquidez?");
    await g.getByLabel("Sua pergunta").press("Enter");
    await expect(respostas(g)).toHaveCount(2);

    // #erro401 mostra a mensagem da chave e devolve o texto ao campo.
    await g.getByLabel("Sua pergunta").fill("Pergunta que falha #erro401");
    await g.getByLabel("Sua pergunta").press("Enter");
    await expect(g.getByRole("alert")).toHaveText("A Anthropic recusou a sua chave. Confira em Configurar IA.");
    await expect(g.getByLabel("Sua pergunta")).toHaveValue("Pergunta que falha #erro401");
    await expect(respostas(g)).toHaveCount(2);

    // "Ver no Assistente" abre a conversa com as 4 mensagens, e a lista mostra a etiqueta.
    const ver = g.getByRole("link", { name: "Ver no Assistente" });
    await expect(ver).toHaveAttribute("href", /^#\/assistente\/\d+$/);
    await ver.click();
    await expect(main(page).getByRole("heading", { name: "Assistente" })).toBeVisible();
    await expect(main(page).getByTestId("mensagem")).toHaveCount(4);
    await expect(main(page).locator('[data-testid="mensagem"][data-papel="user"]').first()).toHaveText(SUGESTOES[0]);
    await expect(main(page).getByTestId("conversa")).toHaveCount(1);
    await expect(main(page).getByTestId("conversa").first()).toContainText("Próximo aporte · Ações");
    await expect(main(page).getByTestId("conversa").first()).toHaveClass(/\bon\b/);

    // De volta ao aporte: mudar a direção com a gaveta aberta oferece "Nova conversa".
    await ir(page, "#/aporte");
    await dividir(page, "2.000,00");
    await main(page).getByRole("button", { name: "Conversar com a IA sobre FIIs" }).click();
    await expect(gaveta(page).getByRole("heading", { name: /^FIIs · / })).toBeVisible();
    await expect(gaveta(page)).toContainText("direção: livre · meta: nenhuma");
    await expect(gaveta(page).getByRole("button", { name: "Nova conversa" })).toHaveCount(0);
    await main(page).getByRole("group", { name: "Para onde quer levar esse dinheiro?" }).getByLabel("FIIs", { exact: true }).check();
    await gaveta(page).getByRole("button", { name: "Nova conversa" }).click();
    await expect(gaveta(page)).toContainText("direção: FIIs · meta: nenhuma");
    await expect(gaveta(page).getByRole("button", { name: "Nova conversa" })).toHaveCount(0);

    // Abrir outra classe troca a gaveta; dividir de novo fecha.
    await main(page).getByRole("button", { name: "Conversar com a IA sobre Exterior" }).click();
    await expect(gaveta(page)).toHaveCount(1);
    await expect(gaveta(page).getByRole("heading", { name: /^Exterior · / })).toBeVisible();
    await main(page).getByRole("button", { name: "Dividir" }).click();
    await expect(gaveta(page)).toHaveCount(0);
    await expect(main(page).getByTestId("parte").first()).toBeVisible();
    // E "Fechar" fecha.
    await main(page).getByRole("button", { name: "Conversar com a IA sobre Ações" }).click();
    await gaveta(page).getByRole("button", { name: "Fechar" }).click();
    await expect(gaveta(page)).toHaveCount(0);
    expect(erros).toEqual([]);
  } finally {
    await app.close();
  }
});

test("#/assistente/<id> inexistente cai na tela normal com a mensagem", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await salvarChave(page);
    await ir(page, "#/assistente/9999");
    await expect(main(page).getByRole("heading", { name: "Assistente" })).toBeVisible();
    await expect(main(page).getByRole("alert")).toHaveText("Não foi possível abrir a conversa.");
    await expect(main(page).getByRole("button", { name: "O que mudou no mercado esta semana?" })).toBeVisible();
  } finally {
    await app.close();
  }
});
