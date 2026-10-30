const { test, expect } = require("@playwright/test");
const { lancar } = require("./_app");
const path = require("node:path");

const FIXTURES = path.resolve(__dirname, "../../../crates/guru-core/tests/fixtures/b3");
const COMPLETA = path.join(FIXTURES, "posicao-completa.xlsx");
const DESCONHECIDA = path.join(FIXTURES, "posicao-aba-desconhecida.xlsx");

const abrir = (arquivo) => lancar(undefined, { GURU_TESTE_ARQUIVO_B3: arquivo });

async function irParaImportar(page) {
  await page.evaluate(() => {
    window.location.hash = "#/investimentos/importar";
  });
  await expect(page.getByRole("heading", { name: "Importar da B3" })).toBeVisible();
}

async function escolher(page) {
  await page.getByRole("button", { name: "Escolher planilha" }).click();
}

const item = (page, ativo) => page.locator("tbody.item", { hasText: ativo });

async function ignorarTodosExceto(page, ativo) {
  const linhas = page.locator("tbody.item");
  const n = await linhas.count();
  for (let i = 0; i < n; i++) {
    const linha = linhas.nth(i);
    if (!(await linha.textContent()).includes(ativo)) {
      await linha.getByRole("combobox", { name: "O que fazer" }).selectOption({ label: "Ignorar" });
    }
  }
}

async function importarItub4(page) {
  await irParaImportar(page);
  await escolher(page);
  await expect(page.locator(".marcas").getByText("9 novos")).toBeVisible();
  await ignorarTodosExceto(page, "ITUB4");
  const itub = item(page, "ITUB4");
  await itub.getByLabel("Data da primeira compra").fill("2025-01-10");
  await itub.getByLabel("Preço médio").fill("30,50");
  await page.getByRole("button", { name: "Aplicar 1 alteração", exact: true }).click();
}

test("tela inicial mostra o título, a orientação e o botão", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await expect(page.getByText("Exporte a posição em investidor.b3.com.br → Extratos → Posição → Baixar Excel.")).toBeVisible();
    await expect(page.getByRole("button", { name: "Escolher planilha" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Importar da B3" })).toHaveCount(0);
    await page.evaluate(() => {
      window.location.hash = "#/investimentos";
    });
    await expect(page.getByRole("link", { name: "Importar da B3" })).toBeVisible();
  } finally {
    await app.close();
  }
});

test("carteira vazia: 9 novos, completa o ITUB4, ignora o resto e aplica", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await escolher(page);
    await expect(page.getByText("posicao-completa.xlsx")).toBeVisible();
    await expect(page.getByText("Área do Investidor B3 · 9 linhas lidas")).toBeVisible();
    await expect(page.locator(".marcas").getByText("9 novos")).toBeVisible();
    for (const t of ["Situação", "Ativo", "Corretora", "No app hoje", "Na planilha B3", "O que fazer"]) {
      await expect(page.getByRole("columnheader", { name: t })).toBeVisible();
    }
    await expect(page.getByRole("button", { name: "Aplicar 9 alterações", exact: true })).toBeVisible();
    await expect(page.getByText("Fundos e previdência normalmente não aparecem na B3. Por padrão, ficam como estão.")).toBeVisible();

    await ignorarTodosExceto(page, "ITUB4");
    const itub = item(page, "ITUB4");
    await itub.getByLabel("Data da primeira compra").fill("2025-01-10");
    await itub.getByLabel("Preço médio").fill("30,50");
    await expect(page.getByRole("button", { name: "Aplicar 1 alteração", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Aplicar 1 alteração", exact: true }).click();

    await expect(page.getByRole("heading", { name: "Investimentos" })).toBeVisible();
    await expect(page.getByText("1 alteração aplicada.")).toBeVisible();
    await expect(page.locator("main tbody tr")).toHaveCount(1);
    await expect(page.locator("main tbody tr").first()).toContainText("ITUB4");
    await expect(page.getByTestId("importacao-ultima")).toHaveText(/^Última importação B3: \d\d\/\d\d\/\d{4}$/);
  } finally {
    await app.close();
  }
});

test("importar a mesma planilha de novo mostra o ITUB4 como igual e não duplica", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    await importarItub4(page);
    await expect(page.getByText("1 alteração aplicada.")).toBeVisible();
    await irParaImportar(page);
    await escolher(page);
    await expect(page.locator(".marcas").getByText("8 novos")).toBeVisible();
    await expect(page.locator(".marcas").getByText("1 igual")).toBeVisible();
    await expect(page.locator("tbody.item", { hasText: "ITUB4" })).toBeHidden();
    await page.getByRole("button", { name: "1 igual" }).click();
    await expect(item(page, "ITUB4")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("novo incompleto mostra o erro no campo e não grava nada", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await escolher(page);
    await page.getByRole("button", { name: "Aplicar 9 alterações", exact: true }).click();
    const itub = item(page, "ITUB4");
    await expect(itub.locator(".erro", { hasText: "Informe o preço médio." })).toBeVisible();
    await expect(itub.getByLabel("Preço médio")).toHaveAttribute("aria-invalid", "true");
    await expect(page.getByRole("heading", { name: "Importar da B3" })).toBeVisible();
    await page.evaluate(() => {
      window.location.hash = "#/investimentos";
    });
    await expect(page.getByText("Nenhum investimento ainda.")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("o fundo cadastrado aparece como fora da B3 com Manter", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    await page.evaluate(() =>
      import("./js/api.js").then(({ api }) =>
        api.investimentos.criar({
          tipo: "fundo",
          instituicao: "XP Investimentos",
          emissor: "Fundo Verde",
          dataAplicacao: "2025-03-15",
          valorAplicadoCentavos: 620000,
          liquidez: "d30",
        }),
      ),
    );
    await irParaImportar(page);
    await escolher(page);
    const fundo = item(page, "Fundo Verde");
    await expect(fundo).toContainText("fora da B3");
    await expect(fundo.getByRole("combobox", { name: "O que fazer" }).locator("option:checked")).toHaveText("Manter");
    await expect(page.locator(".marcas").getByText("1 fora da B3")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("planilha com aba desconhecida mostra a mensagem de arquivo inválido", async () => {
  const app = await abrir(DESCONHECIDA);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await escolher(page);
    await expect(page.getByRole("alert")).toBeVisible();
    await expect(page.getByRole("button", { name: "Escolher planilha" })).toBeVisible();
  } finally {
    await app.close();
  }
});

test("cancelar o diálogo mantém a tela inicial", async () => {
  const app = await abrir("cancelar");
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await escolher(page);
    await expect(page.getByRole("button", { name: "Escolher planilha" })).toBeVisible();
    await expect(page.getByRole("alert")).toHaveCount(0);
    await expect(page.locator("tbody.item")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("Cancelar volta para a lista", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await escolher(page);
    await page.getByRole("link", { name: "Cancelar" }).click();
    await expect(page.getByRole("heading", { name: "Investimentos" })).toBeVisible();
  } finally {
    await app.close();
  }
});
