const { test, expect } = require("@playwright/test");
const { lancar } = require("./_app");

const main = (page) => page.locator("main");
const ir = (page, hash) =>
  page.evaluate((h) => {
    window.location.hash = h;
  }, hash);
const chamar = (page, fn) => page.evaluate(`import("./js/api.js").then(({ api }) => (${fn})(api))`);

const AVISO =
  "Conteúdo educativo gerado por IA com os seus dados e fontes da web. Pode conter erros. Não é recomendação individual de investimento. Confira as fontes.";
const SUGESTOES = [
  "O que mudou no mercado esta semana?",
  "Minha carteira está alinhada ao meu perfil?",
  "Quanto pago de IR nos meus investimentos?",
];

const campo = (page) => main(page).getByLabel("Sua pergunta");
const botao = (page, nome) => main(page).getByRole("button", { name: nome, exact: true });
const conversas = (page) => main(page).getByTestId("conversa");
const respostas = (page) => main(page).locator('[data-testid="mensagem"][data-papel="assistant"]');
const perguntas = (page) => main(page).locator('[data-testid="mensagem"][data-papel="user"]');

async function abrirAssistente(page) {
  await page.getByRole("link", { name: "Assistente", exact: true }).click();
  await expect(main(page).getByRole("heading", { name: "Assistente" })).toBeVisible();
}

// Salva a chave pela tela Configurar IA (cofre em memória, conferência pelo ClienteFixture).
async function salvarChave(page) {
  await page.getByRole("link", { name: "Configurar IA", exact: true }).click();
  await page.getByLabel("Chave da API da Anthropic").fill("sk-ant-teste-boa-12345678-4f2a");
  await main(page).getByRole("button", { name: "Salvar chave" }).click();
  await expect(main(page).getByText("ativo", { exact: true })).toBeVisible();
}

// Abre o app com a chave salva e a tela Assistente aberta; no fim, nenhum erro de console ou CSP.
async function comChave(fn) {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const erros = [];
    page.on("pageerror", (e) => erros.push(String(e)));
    page.on("console", (m) => {
      if (m.type() === "error") erros.push(m.text());
    });
    await salvarChave(page);
    await abrirAssistente(page);
    await fn(page);
    expect(erros).toEqual([]);
  } finally {
    await app.close();
  }
}

async function perguntar(page, texto) {
  await campo(page).fill(texto);
  await campo(page).press("Enter");
}

test("sem chave mostra o convite para configurar", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrirAssistente(page);
    await expect(main(page)).toContainText("Para conversar com a IA, configure a sua chave.");
    const link = main(page).getByRole("link", { name: "Configurar IA" });
    await expect(link).toHaveAttribute("href", "#/ia");
    await expect(campo(page)).toHaveCount(0);
    await link.click();
    await expect(main(page).getByRole("heading", { name: "Sua IA" })).toBeVisible();
  } finally {
    await app.close();
  }
});

test("com chave, uma sugestão recebe a resposta e a conversa entra na lista", () =>
  comChave(async (page) => {
    for (const s of SUGESTOES) await expect(botao(page, s)).toBeVisible();
    await expect(main(page)).toContainText(AVISO);
    await expect(conversas(page)).toHaveCount(0);
    await botao(page, SUGESTOES[1]).click();
    await expect(perguntas(page)).toHaveText([SUGESTOES[1]]);
    await expect(respostas(page)).toHaveCount(1);
    await expect(respostas(page).first()).toContainText("A decisão é sua.");
    await expect(respostas(page).first().locator("strong", { hasText: "Manter como está" })).toBeVisible();
    await expect(respostas(page).first().locator("li")).toHaveCount(2);
    await expect(conversas(page)).toHaveCount(1);
    await expect(conversas(page).first()).toContainText(SUGESTOES[1]);
    await expect(botao(page, "Enviar")).toBeVisible();
    await expect(botao(page, SUGESTOES[0])).toHaveCount(0);
    await expect(main(page)).toContainText(AVISO);
  }));

test("#busca mostra os marcadores 1 e 2 e as fontes da web no painel", () =>
  comChave(async (page) => {
    await perguntar(page, "O que o Copom decidiu? #busca");
    const resposta = respostas(page).first();
    await expect(resposta).toContainText("O Copom manteve a Selic em 15% ao ano");
    await expect(resposta.locator("sup a")).toHaveText(["1", "2", "1"]);
    await expect(resposta).not.toContainText("[1]");
    const painel = main(page).getByTestId("fontes-web");
    const fontes = painel.getByTestId("fonte");
    await expect(fontes).toHaveCount(2);
    for (let i = 0; i < 2; i++) {
      await expect(fontes.nth(i)).toContainText(String(i + 1));
      await expect(fontes.nth(i).getByRole("link")).toHaveAttribute("href", /^https:\/\//);
    }
    await expect(main(page).getByText("O que a IA está vendo")).toBeVisible();
    await expect(main(page).getByTestId("seus-dados")).toContainText("Carteira");
    // O marcador leva à fonte sem trocar de tela.
    await resposta.locator("sup a").first().click();
    await expect(page).toHaveURL(/#\/assistente$/);
    await expect(main(page).getByRole("heading", { name: "Assistente" })).toBeVisible();
  }));

test("uma segunda pergunta na mesma conversa funciona", () =>
  comChave(async (page) => {
    await perguntar(page, "Primeira pergunta");
    await expect(respostas(page)).toHaveCount(1);
    await expect(respostas(page).first()).toContainText("A decisão é sua.");
    await perguntar(page, "E agora? #busca");
    await expect(respostas(page)).toHaveCount(2);
    await expect(respostas(page).nth(1)).toContainText("O Copom manteve a Selic");
    await expect(perguntas(page)).toHaveText(["Primeira pergunta", "E agora? #busca"]);
    await expect(conversas(page)).toHaveCount(1);
    const salvas = await chamar(page, "async (api) => { const [c] = await api.ia.conversas(); return (await api.ia.conversa(c.id)).mensagens.length; }");
    expect(salvas).toBe(4);
  }));

test("#erro401 mostra a mensagem da chave e devolve o texto ao campo", () =>
  comChave(async (page) => {
    await perguntar(page, "Pergunta que falha #erro401");
    const erro = main(page).getByRole("alert");
    await expect(erro).toHaveText("A Anthropic recusou a sua chave. Confira em Configurar IA.");
    await expect(campo(page)).toHaveValue("Pergunta que falha #erro401");
    await expect(main(page).getByTestId("mensagem")).toHaveCount(0);
    await expect(conversas(page)).toHaveCount(0);
    await expect(botao(page, "Enviar")).toBeEnabled();
  }));

test("Parar no meio da resposta mostra Resposta interrompida e não grava nada", () =>
  comChave(async (page) => {
    await campo(page).fill("Me fale da carteira");
    // Os dois cliques na mesma tarefa: o "Parar" chega antes do primeiro evento da fixture
    // (que manda um evento a cada 20 ms), então a resposta está sempre em andamento.
    await page.evaluate(() => {
      const b = [...document.querySelectorAll("main button")].find((x) => x.textContent === "Enviar");
      b.click();
      if (b.textContent !== "Parar") throw new Error(`o botão deveria ser Parar, é ${b.textContent}`);
      b.click();
    });
    await expect(main(page).getByRole("alert")).toHaveText("Resposta interrompida.");
    await expect(campo(page)).toHaveValue("Me fale da carteira");
    await expect(main(page).getByTestId("mensagem")).toHaveCount(0);
    await expect(conversas(page)).toHaveCount(0);
    expect(await chamar(page, "(api) => api.ia.conversas()")).toEqual([]);
    // Depois de parar, uma pergunta nova funciona.
    await campo(page).press("Enter");
    await expect(respostas(page).first()).toContainText("A decisão é sua.");
    await expect(main(page).getByRole("alert")).toHaveCount(0);
  }));

test("se acompanhar a resposta falha, o pedido é cancelado e nada é gravado", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await salvarChave(page);
    await abrirAssistente(page);
    await page.evaluate(() =>
      import("./js/api.js").then(({ api }) => {
        const cancelar = api.ia.cancelar;
        window.cancelados = [];
        api.ia.progresso = () => Promise.reject(Object.assign(new Error("falhou"), { code: "ERRO_INTERNO" }));
        api.ia.cancelar = (pedido) => {
          window.cancelados.push(pedido);
          return cancelar(pedido);
        };
      }),
    );
    await perguntar(page, "Pergunta sem acompanhamento");
    await expect(main(page).getByRole("alert")).toHaveText("Não foi possível acompanhar a resposta. Tente de novo.");
    await expect(campo(page)).toHaveValue("Pergunta sem acompanhamento");
    expect(await page.evaluate(() => window.cancelados.length)).toBe(1);
    // Espera mais que a fixture inteira (22 eventos a 20 ms): cancelado, nada foi gravado.
    await page.waitForTimeout(1000);
    expect(await chamar(page, "(api) => api.ia.conversas()")).toEqual([]);
  } finally {
    await app.close();
  }
});

test("apagar a conversa a tira da lista", () =>
  comChave(async (page) => {
    await perguntar(page, "Uma conversa para apagar");
    await expect(conversas(page)).toHaveCount(1);
    await conversas(page).first().getByRole("button", { name: "Apagar", exact: true }).click();
    const dialogo = page.getByRole("dialog");
    await expect(dialogo).toContainText("Apagar a conversa");
    await dialogo.getByRole("button", { name: "Apagar" }).click();
    await expect(conversas(page)).toHaveCount(0);
    await expect(main(page).getByTestId("mensagem")).toHaveCount(0);
    await expect(botao(page, SUGESTOES[0])).toBeVisible();
    expect(await chamar(page, "(api) => api.ia.conversas()")).toEqual([]);
  }));

test("Nova conversa e abrir uma conversa da lista", () =>
  comChave(async (page) => {
    await perguntar(page, "Conversa A");
    await expect(respostas(page)).toHaveCount(1);
    await botao(page, "Nova conversa").click();
    await expect(main(page).getByTestId("mensagem")).toHaveCount(0);
    await perguntar(page, "Conversa B #busca");
    await expect(respostas(page)).toHaveCount(1);
    await expect(conversas(page)).toHaveCount(2);
    await conversas(page).filter({ hasText: "Conversa A" }).getByRole("button", { name: /Conversa A/ }).click();
    await expect(perguntas(page)).toHaveText(["Conversa A"]);
    await expect(main(page).getByTestId("fontes-web").getByTestId("fonte")).toHaveCount(0);
  }));

test("sair da tela no meio da resposta não deixa nada gravado", () =>
  comChave(async (page) => {
    await campo(page).fill("Pergunta abandonada");
    await page.evaluate(() => {
      [...document.querySelectorAll("main button")].find((x) => x.textContent === "Enviar").click();
      window.location.hash = "#/carteira";
    });
    await expect(main(page).getByRole("heading", { name: "Carteira" })).toBeVisible();
    // Espera mais que a fixture inteira (22 eventos a 20 ms) para ver que nada foi gravado.
    await page.waitForTimeout(1000);
    expect(await chamar(page, "(api) => api.ia.conversas()")).toEqual([]);
  }));

test("Shift+Enter quebra a linha sem enviar", () =>
  comChave(async (page) => {
    await campo(page).fill("linha 1");
    await campo(page).press("Shift+Enter");
    await campo(page).pressSequentially("linha 2");
    await expect(campo(page)).toHaveValue("linha 1\nlinha 2");
    await expect(main(page).getByTestId("mensagem")).toHaveCount(0);
  }));
