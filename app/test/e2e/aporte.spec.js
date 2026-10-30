const { test, expect } = require("@playwright/test");
const { lancar } = require("./_app");

const main = (page) => page.locator("main");
const ir = (page, hash) =>
  page.evaluate((h) => {
    window.location.hash = h;
  }, hash);
const chamar = (page, fn, arg) => page.evaluate(`import("./js/api.js").then(({ api }) => (${fn})(api, ${JSON.stringify(arg ?? null)}))`);

const CONVITE = "Para dividir o aporte, defina a sua alocação-alvo.";
const AVISO =
  "Cálculo de rebalanceamento pela sua alocação-alvo, sem vender nada. Investimentos sem cotação contam pelo valor aplicado. Não é recomendação individual de investimento.";

const CDB = {
  tipo: "rf-cdb",
  instituicao: "Banco Inter",
  emissor: "Banco Inter",
  indexador: "pos",
  taxa: "102",
  benchmark: "cdi",
  dataAplicacao: "2025-03-15",
  vencimento: `${new Date().getFullYear() + 3}-03-15`,
  valorAplicadoCentavos: 900000,
  liquidez: "vencimento",
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

// Define a alocação-alvo pela tela de Perfil: Renda fixa, Ações, FIIs, Exterior, Outros.
async function definirAlvo(page, valores) {
  await ir(page, "#/perfil/alocacao");
  const campos = main(page).locator('input[inputmode="numeric"]');
  for (let i = 0; i < valores.length; i++) await campos.nth(i).fill(String(valores[i]));
  await main(page).getByRole("button", { name: "Salvar", exact: true }).click();
  await expect(page).toHaveURL(/#\/perfil$/);
}

const campoValor = (page) => main(page).getByLabel("Quanto vai aportar?");
const dividir = async (page, texto) => {
  await campoValor(page).fill(texto);
  await main(page).getByRole("button", { name: "Dividir" }).click();
};
const centavosDe = (t) => Number(t.replace(/\D/g, ""));
const cartao = (page, rotulo) => main(page).getByTestId("parte").filter({ hasText: rotulo });

test("sem alvo mostra o convite e leva ao perfil", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await ir(page, "#/aporte");
    await expect(main(page).getByRole("heading", { name: "Próximo aporte" })).toBeVisible();
    await expect(main(page)).toContainText(CONVITE);
    await expect(main(page).getByLabel("Quanto vai aportar?")).toHaveCount(0);
    await main(page).getByRole("link", { name: "Definir alocação-alvo" }).click();
    await expect(page).toHaveURL(/#\/perfil\/alocacao$/);
  } finally {
    await app.close();
  }
});

test("divide pela alocação com a carteira vazia", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await definirAlvo(page, [40, 40, 20, 0, 0]);
    await ir(page, "#/aporte");
    await expect(main(page)).not.toContainText(CONVITE);
    await dividir(page, "1.000,00");
    await expect(main(page).getByTestId("parte")).toHaveCount(3);
    await expect(cartao(page, "Renda fixa")).toContainText("R$ 400,00");
    await expect(cartao(page, "Ações")).toContainText("R$ 400,00");
    await expect(cartao(page, "FIIs")).toContainText("R$ 200,00");
    await expect(cartao(page, "Renda fixa")).toContainText("atual 0,0% → depois 40,0% · alvo 40%");
    await expect(main(page)).toContainText("Carteira vazia: o aporte foi dividido pela alocação-alvo.");
    await expect(main(page)).toContainText("Patrimônio de R$ 0,00 passa a R$ 1.000,00 com o aporte de R$ 1.000,00.");
    await expect(main(page)).toContainText(AVISO);
    await expect(main(page).locator(".legal")).toContainText(AVISO);
  } finally {
    await app.close();
  }
});

test("classe acima do alvo recebe zero", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await definirAlvo(page, [40, 40, 20, 0, 0]);
    await chamar(page, "(api, d) => api.investimentos.criar(d)", CDB);
    await chamar(page, "(api, d) => api.investimentos.criar(d)", ITUB4);
    await ir(page, "#/aporte");
    await dividir(page, "4.000,00");
    await expect(main(page).getByTestId("parte").first()).toBeVisible();
    await expect(cartao(page, "Renda fixa")).toContainText("R$ 0,00");
    await expect(cartao(page, "Renda fixa")).toContainText("acima do alvo.");
    await expect(cartao(page, "Renda fixa")).toContainText("Nada neste aporte.");
    await expect(main(page)).not.toContainText("Carteira vazia");
    const valores = (await main(page).getByTestId("parte").locator(".mid").allTextContents()).map(centavosDe);
    expect(valores.reduce((a, b) => a + b, 0)).toBe(400000);
    expect(valores.filter((v) => v > 0)).toHaveLength(2);
    expect(valores.filter((v) => v === 0)).toHaveLength(1);
  } finally {
    await app.close();
  }
});

test("classe acima do alvo que recebe parte explica o motivo", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await definirAlvo(page, [40, 40, 20, 0, 0]);
    await chamar(page, "(api, d) => api.investimentos.criar(d)", CDB);
    await chamar(page, "(api, d) => api.investimentos.criar(d)", ITUB4);
    await ir(page, "#/aporte");
    await dividir(page, "20.000,00");
    const rf = cartao(page, "Renda fixa");
    await expect(rf).toContainText("acima do alvo hoje, mas recebe parte porque o aporte é grande em relação à carteira.");
    await expect(rf).not.toContainText("Nada neste aporte.");
    await expect(rf.locator(".mid")).not.toHaveText("R$ 0,00");
  } finally {
    await app.close();
  }
});

test("valor absurdo mostra 'Valor alto demais.' e o resultado é anunciado", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await definirAlvo(page, [40, 40, 20, 0, 0]);
    await ir(page, "#/aporte");
    await dividir(page, "99999999999999999999");
    await expect(main(page)).toContainText("Valor alto demais.");
    await expect(main(page)).not.toContainText("Informe o valor em reais");
    await expect(main(page).locator('[aria-live="polite"]')).toHaveCount(1);
  } finally {
    await app.close();
  }
});

test("renderizar o resultado não gera erro de console nem violação de CSP", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const erros = [];
    page.on("pageerror", (e) => erros.push(String(e)));
    page.on("console", (m) => {
      if (m.type() === "error") erros.push(m.text());
    });
    await definirAlvo(page, [40, 40, 20, 0, 0]);
    await ir(page, "#/aporte");
    await dividir(page, "1.000,00");
    await expect(main(page).getByTestId("parte")).toHaveCount(3);
    expect(erros).toEqual([]);
  } finally {
    await app.close();
  }
});

test("valor inválido mostra erro sem calcular", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await definirAlvo(page, [40, 40, 20, 0, 0]);
    await ir(page, "#/aporte");
    await dividir(page, "abc");
    await expect(main(page)).toContainText("Informe o valor em reais, como 2.000,00.");
    await expect(campoValor(page)).toHaveAttribute("aria-invalid", "true");
    await expect(campoValor(page)).toHaveAttribute("aria-describedby", /./);
    await dividir(page, "0,50");
    await expect(main(page)).toContainText("Informe um valor de pelo menos R$ 1,00.");
    await expect(main(page)).not.toContainText("Informe o valor em reais");
    await expect(main(page).getByTestId("parte")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("trocar de tela logo após dividir não gera erro", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const erros = [];
    page.on("pageerror", (e) => erros.push(String(e)));
    await definirAlvo(page, [40, 40, 20, 0, 0]);
    await ir(page, "#/aporte");
    await campoValor(page).fill("1.000,00");
    await campoValor(page).press("Enter");
    await page.getByRole("link", { name: "Carteira", exact: true }).click();
    await expect(main(page).getByRole("heading", { name: "Carteira" })).toBeVisible();
    await expect(main(page).getByTestId("parte")).toHaveCount(0);
    expect(erros).toEqual([]);
  } finally {
    await app.close();
  }
});
