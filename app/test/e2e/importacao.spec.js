const { test, expect } = require("@playwright/test");
const { lancar } = require("./_app");
const path = require("node:path");

const FIXTURES = path.resolve(__dirname, "../../../crates/guru-core/tests/fixtures/b3");
const COMPLETA = path.join(FIXTURES, "posicao-completa.xlsx");
const DESCONHECIDA = path.join(FIXTURES, "posicao-aba-desconhecida.xlsx");
const PREFIXO = path.join(FIXTURES, "posicao-prefixo-desconhecido.xlsx");

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
  // A comparação chega depois do clique: sem esperar, a contagem pode dar zero linhas.
  await expect(page.locator(".cartao-arquivo")).toBeVisible();
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
    await expect(itub.locator(".erro", { hasText: "Informe a data da primeira compra." })).toBeVisible();
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
    await expect(page.getByRole("alert")).toHaveText(
      "Não encontrei nenhuma aba da posição da B3 neste arquivo. Aba Aluguel de Ações não reconhecida. Aba Renda Fixa: falta a coluna Vencimento.",
    );
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

test("com tudo em Ignorar o botão Aplicar fica desabilitado", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await escolher(page);
    await ignorarTodosExceto(page, "NENHUM");
    await expect(page.getByRole("button", { name: "Aplicar 0 alterações", exact: true })).toBeDisabled();
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

async function criarCdb(page, taxa) {
  const r = await page.evaluate(
    (t) =>
      import("./js/api.js").then(({ api }) =>
        api.investimentos.criar({
          tipo: "rf-cdb",
          instituicao: "Banco Inter",
          emissor: "Banco Fictício S/A",
          indexador: "pos",
          taxa: t,
          benchmark: "cdi",
          dataAplicacao: "2025-03-15",
          vencimento: "2029-03-15",
          valorAplicadoCentavos: 2000000,
          liquidez: "vencimento",
          ir: "regressiva",
        }),
      ),
    taxa,
  );
  expect(r.erros).toEqual([]);
}

test("dois CDBs do mesmo banco e vencimento: vincular um à linha da B3 grava o código", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    await criarCdb(page, "102");
    await criarCdb(page, "110");
    await irParaImportar(page);
    await escolher(page);
    const cdb = item(page, "CDB BANCO FICTICIO S/A");
    await expect(cdb).toContainText("divergente");
    const acao = cdb.getByRole("combobox", { name: "O que fazer" });
    await expect(acao.locator("option", { hasText: "Vincular a" })).toHaveCount(2);
    // Os dois candidatos ainda não foram casados: continuam em "fora da B3".
    await expect(page.locator(".marcas").getByText("2 fora da B3")).toBeVisible();
    await ignorarTodosExceto(page, "Banco Fictício S/A");
    await acao.selectOption({ label: "Vincular a CDB Banco Fictício S/A · 110% do CDI · aplicado R$ 20.000,00" });
    await page.getByRole("button", { name: "Aplicar 1 alteração", exact: true }).click();
    await expect(page.getByText("1 alteração aplicada.")).toBeVisible();

    // Na próxima importação a linha casa pelo código com o CDB de 110%; o de 102% fica fora da B3.
    await irParaImportar(page);
    await escolher(page);
    await expect(page.locator(".marcas").getByText("1 igual")).toBeVisible();
    await expect(page.locator(".marcas").getByText("1 fora da B3")).toBeVisible();
    await page.getByRole("button", { name: "1 igual" }).click();
    await expect(item(page, "CDB BANCO FICTICIO S/A")).toContainText("igual");
  } finally {
    await app.close();
  }
});

const DATA_EMISSAO = "Data da aplicação (confira: veio a data de emissão)";

test("renda fixa com prefixo desconhecido: escolher CDB mostra os campos do tipo e aplicar grava", async () => {
  const app = await abrir(PREFIXO);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await escolher(page);
    await ignorarTodosExceto(page, "LF BANCO FICTICIO S/A");
    const lf = item(page, "LF BANCO FICTICIO S/A");
    await expect(lf.getByLabel("Valor aplicado")).toBeHidden();
    await lf.getByLabel("Tipo", { exact: true }).selectOption({ label: "CDB" });
    await expect(lf.getByLabel("Rendimento")).toBeVisible();
    await expect(lf.getByLabel("Valor aplicado")).toBeVisible();
    await expect(lf.getByLabel("Liquidez")).toHaveValue("vencimento");
    await expect(lf.getByLabel(DATA_EMISSAO)).toHaveValue("2024-03-15");
    await lf.getByLabel("Rendimento").fill("102");
    await lf.getByLabel("Valor aplicado").fill("10.000,00");
    await page.getByRole("button", { name: "Aplicar 1 alteração", exact: true }).click();
    await expect(page.getByText("1 alteração aplicada.")).toBeVisible();
    await expect(page.locator("main tbody tr")).toHaveCount(1);
    await expect(page.locator("main tbody tr").first()).toContainText("CDB");
  } finally {
    await app.close();
  }
});

test("renda fixa com prefixo desconhecido: aplicar sem escolher o tipo marca o campo e não grava", async () => {
  const app = await abrir(PREFIXO);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await escolher(page);
    await ignorarTodosExceto(page, "LF BANCO FICTICIO S/A");
    await page.getByRole("button", { name: "Aplicar 1 alteração", exact: true }).click();
    const lf = item(page, "LF BANCO FICTICIO S/A");
    await expect(lf.locator(".erro", { hasText: "Escolha o tipo." })).toBeVisible();
    await expect(lf.getByLabel("Tipo", { exact: true })).toHaveAttribute("aria-invalid", "true");
    await expect(page.getByRole("alert")).toHaveText("Há campos a corrigir. Nada foi aplicado.");
    await page.evaluate(() => {
      window.location.hash = "#/investimentos";
    });
    await expect(page.getByText("Nenhum investimento ainda.")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("a data de emissão vem como data da aplicação editável, para conferir", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await escolher(page);
    const cdb = item(page, "CDB BANCO EXEMPLO S/A");
    const data = cdb.getByLabel(DATA_EMISSAO);
    await expect(data).toBeVisible();
    await expect(data).toBeEditable();
    await expect(data).not.toHaveValue("");
  } finally {
    await app.close();
  }
});

test("outra planilha que falha descarta a comparação mostrada e volta à tela inicial", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    await irParaImportar(page);
    await escolher(page);
    await expect(page.locator(".marcas").getByText("9 novos")).toBeVisible();
    await app.evaluate((_electron, arquivo) => {
      process.env.GURU_TESTE_ARQUIVO_B3 = arquivo;
    }, DESCONHECIDA);
    await page.getByRole("button", { name: "Escolher outra planilha" }).click();
    await expect(page.getByRole("alert")).toContainText("Não encontrei nenhuma aba da posição da B3 neste arquivo.");
    await expect(page.locator("tbody.item")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Escolher planilha" })).toBeVisible();
  } finally {
    await app.close();
  }
});

test("só com itens iguais o botão vira Concluir importação e grava a data", async () => {
  const app = await abrir(COMPLETA);
  try {
    const page = await app.firstWindow();
    const r = await page.evaluate(() =>
      import("./js/api.js").then(({ api }) =>
        api.investimentos.criar({
          tipo: "rv-acao",
          instituicao: "Rico",
          ticker: "ITUB4",
          dataAplicacao: "2025-01-10",
          quantidade: 150,
          precoMedioCentavos: 3050,
        }),
      ),
    );
    expect(r.erros).toEqual([]);
    await expect(page.getByTestId("importacao-ultima")).toBeHidden();
    await irParaImportar(page);
    await escolher(page);
    await expect(page.locator(".marcas").getByText("1 igual")).toBeVisible();
    await ignorarTodosExceto(page, "ITUB4");
    const concluir = page.getByRole("button", { name: "Concluir importação", exact: true });
    await expect(concluir).toBeEnabled();
    await concluir.click();
    await expect(page.getByRole("heading", { name: "Investimentos" })).toBeVisible();
    await expect(page.getByText("Importação concluída, sem alterações na carteira.")).toBeVisible();
    await expect(page.getByTestId("importacao-ultima")).toHaveText(/^Última importação B3: \d\d\/\d\d\/\d{4}$/);
  } finally {
    await app.close();
  }
});
