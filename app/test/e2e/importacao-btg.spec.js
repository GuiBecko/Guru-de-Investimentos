const { test, expect } = require("@playwright/test");
const { lancar } = require("./_app");
const path = require("node:path");

const FIXTURES = path.resolve(__dirname, "../../../crates/guru-core/tests/fixtures/btg");
const COMPLETO = path.join(FIXTURES, "extrato-completo.pdf");
const DESCONHECIDA = path.join(FIXTURES, "extrato-secao-desconhecida.pdf");
const OUTRO_BANCO = path.join(FIXTURES, "outro-banco.pdf");

// A data fixa dos e2e (28/10/2026) é anterior à referência do saldo do extrato (30/10/2026): o
// saldo só vale até a data de hoje, então estes testes avançam o "hoje" do mercado.
const HOJE = { GURU_MERCADO_HOJE: "2026-11-06" };
const abrir = (arquivo) => lancar(undefined, { GURU_TESTE_ARQUIVO_BTG: arquivo, ...HOJE });

async function irParaImportar(page) {
  await page.evaluate(() => {
    window.location.hash = "#/investimentos/importar/btg";
  });
  await expect(page.getByRole("heading", { name: "Importar da BTG" })).toBeVisible();
}

test("os dois botões de importação aparecem em Investimentos", async () => {
  const app = await abrir(COMPLETO);
  try {
    const page = await app.firstWindow();
    await page.evaluate(() => {
      window.location.hash = "#/investimentos";
    });
    await expect(page.getByRole("link", { name: "Importar da B3" })).toBeVisible();
    await page.getByRole("link", { name: "Importar da BTG" }).click();
    await expect(page.getByRole("heading", { name: "Importar da BTG" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Escolher PDF" })).toBeVisible();
    await expect(page.getByText("Gere o “Extrato da Conta Investimento” em PDF no app ou no site da BTG e escolha o arquivo aqui.")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("importar o extrato completo numa carteira vazia", async () => {
  const app = await abrir(COMPLETO);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await page.getByRole("button", { name: "Escolher PDF" }).click();
    await expect(page.getByText("Extrato BTG de 31/10/2026 · 7 ativos lidos")).toBeVisible();
    await expect(page.getByRole("columnheader", { name: "No extrato BTG" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Escolher outro PDF" })).toBeVisible();
    await expect(page.getByText("Conta corrente e valores em trânsito não são importados.")).toBeVisible();
    await page.getByRole("button", { name: /^Aplicar \d+ alterações$/ }).click();
    await expect(page.getByRole("heading", { name: "Investimentos" })).toBeVisible();
    const fundo = page.locator("main tbody tr", { hasText: "FUNDO EXEMPLO RF LP" });
    await expect(fundo).toBeVisible();
    await expect(fundo.locator("[title*='saldo do extrato da corretora']")).toBeVisible();
    await expect(page.getByTestId("importacao-ultima")).toHaveText(/^Última importação: \d\d\/\d\d\/\d{4}$/);
  } finally {
    await app.close();
  }
});

test("seção desconhecida aparece como aviso", async () => {
  const app = await abrir(DESCONHECIDA);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await page.getByRole("button", { name: "Escolher PDF" }).click();
    await expect(page.locator(".avisos").getByText("Tesouro Direto")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("PDF de outro banco mostra o erro", async () => {
  const app = await abrir(OUTRO_BANCO);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await page.getByRole("button", { name: "Escolher PDF" }).click();
    await expect(page.getByRole("alert").getByText("Este PDF não é um Extrato da Conta Investimento da BTG.")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("cancelar o diálogo não muda nada", async () => {
  const app = await lancar(undefined, { GURU_TESTE_ARQUIVO_BTG: "cancelar" });
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await page.getByRole("button", { name: "Escolher PDF" }).click();
    await expect(page.getByRole("button", { name: "Escolher PDF" })).toBeEnabled();
    await expect(page.locator(".cartao-arquivo")).toHaveCount(0);
    await expect(page.getByRole("alert")).toBeHidden();
  } finally {
    await app.close();
  }
});

test("ativo lançado à mão em outra corretora aparece divergente com Ignorar e Vincular", async () => {
  const app = await abrir(COMPLETO);
  try {
    const page = await app.firstWindow();
    await page.evaluate(() =>
      import("./js/api.js").then(({ api }) =>
        api.investimentos.criar({
          tipo: "rv-acao",
          instituicao: "Genial",
          ticker: "ABCD3",
          dataAplicacao: "2025-01-10",
          quantidade: 10,
          precoMedioCentavos: 1000,
        }),
      ),
    );
    await irParaImportar(page);
    await page.getByRole("button", { name: "Escolher PDF" }).click();
    const linha = page.locator("tbody.item", { hasText: "ABCD3" });
    await expect(linha).toContainText("divergente");
    const acao = linha.getByRole("combobox", { name: "O que fazer" });
    await expect(acao.locator("option:checked")).toHaveText("Ignorar");
    await expect(acao.locator("option", { hasText: "Vincular a" })).toHaveCount(1);
    await expect(acao.locator("option", { hasText: "Vincular a" })).toContainText("· Genial ·");
  } finally {
    await app.close();
  }
});
