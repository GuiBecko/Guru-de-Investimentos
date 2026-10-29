const { test, expect, _electron } = require("@playwright/test");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const appDir = path.resolve(__dirname, "../..");

function lancar(dir = fs.mkdtempSync(path.join(os.tmpdir(), "guru-e2e-"))) {
  return _electron.launch({ args: ["."], cwd: appDir, env: { ...process.env, GURU_DATA_DIR: dir } });
}

const main = (page) => page.locator("main");
const ir = (page, hash) =>
  page.evaluate((h) => {
    window.location.hash = h;
  }, hash);
const chamar = (page, fn, arg) => page.evaluate(`import("./js/api.js").then(({ api }) => (${fn})(api, ${JSON.stringify(arg ?? null)}))`);

const CDB = {
  tipo: "rf-cdb",
  instituicao: "Banco Inter",
  emissor: "Banco Inter",
  indexador: "pos",
  taxa: "102",
  benchmark: "cdi",
  dataAplicacao: "2025-03-15",
  vencimento: "2028-03-15",
  valorAplicadoCentavos: 2000000,
  liquidez: "vencimento",
  ir: "regressiva",
};

async function novaMeta(page, { nome, valor, prazo }) {
  await ir(page, "#/perfil/metas/nova");
  await expect(main(page).getByRole("heading", { name: "Nova meta" })).toBeVisible();
  await page.getByLabel("Nome", { exact: true }).fill(nome);
  await page.getByLabel("Valor da meta").fill(valor);
  await page.getByLabel("Prazo").fill(prazo);
  await main(page).getByRole("button", { name: "Salvar" }).click();
}

test("sem metas mostra o texto e o card Nova meta", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await ir(page, "#/perfil");
    await expect(main(page)).toContainText("Metas ajudam a acompanhar objetivos com valor e prazo, como a entrada de um imóvel.");
    await expect(main(page).getByRole("link", { name: "+ Nova meta" })).toHaveAttribute("href", "#/perfil/metas/nova");
  } finally {
    await app.close();
  }
});

test("cria uma meta e o card mostra prazo, valores e a situação sem investimentos", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await novaMeta(page, { nome: "Entrada do apartamento", valor: "150.000,00", prazo: "2030-12" });
    await expect(page).toHaveURL(/#\/perfil$/);
    const card = main(page).getByRole("link", { name: /Entrada do apartamento/ });
    await expect(card).toContainText("dez/2030");
    await expect(card).toContainText("R$ 0,00 / R$ 150.000,00");
    await expect(card).toContainText("Ainda sem investimentos vinculados.");
    await expect(card).toContainText(/aporte necessário R\$ [\d.]+,\d{2}\/mês/);
    await expect(main(page)).not.toContainText("Metas ajudam a acompanhar");
  } finally {
    await app.close();
  }
});

test("vincular um CDB pelo formulário atualiza o acumulado e o aporte necessário", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await chamar(page, "(api) => api.metas.criar({ nome: 'Entrada', valorAlvoCentavos: 15000000, prazo: '2030-12' })");
    await ir(page, "#/investimentos/novo");
    await page.getByLabel("Emissor").fill("Banco Inter");
    await page.getByLabel("Rendimento", { exact: true }).fill("102");
    await page.getByLabel("Data da aplicação").fill("2025-03-15");
    await page.getByLabel("Vencimento").fill("2028-03-15");
    await page.getByLabel("Valor aplicado").fill("20.000,00");
    await page.getByLabel("Vincular a uma meta").selectOption({ label: "Entrada" });
    await page.getByRole("button", { name: "Salvar", exact: true }).click();
    await expect(page.locator("main tbody tr").first()).toContainText("CDB Banco Inter");
    await ir(page, "#/perfil");
    const card = main(page).getByRole("link", { name: /Entrada/ });
    await expect(card).toContainText("R$ 20.000,00 / R$ 150.000,00");
    await expect(card).toContainText(/aporte necessário R\$ [\d.]+,\d{2}\/mês/);
    await expect(card).not.toContainText("Ainda sem investimentos vinculados.");
  } finally {
    await app.close();
  }
});

test("prazo no passado, nome e valor vazios mostram a mensagem em cada campo", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await novaMeta(page, { nome: "", valor: "", prazo: "2020-01" });
    await expect(page).toHaveURL(/#\/perfil\/metas\/nova$/);
    await expect(page.locator("#e-nome")).toHaveText("Informe o nome da meta.");
    await expect(page.locator("#e-valorAlvoCentavos")).toHaveText("Informe o valor da meta.");
    await expect(page.locator("#e-prazo")).toHaveText("O prazo precisa ser este mês ou depois.");
    await expect(page.getByLabel("Nome", { exact: true })).toHaveAttribute("aria-invalid", "true");
    expect(await chamar(page, "(api) => api.metas.listar()")).toHaveLength(0);
  } finally {
    await app.close();
  }
});

test("editar uma meta troca os valores e Cancelar volta sem gravar", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const { id } = await chamar(page, "(api) => api.metas.criar({ nome: 'Viagem', valorAlvoCentavos: 1000000, prazo: '2030-06' })");
    await ir(page, "#/perfil");
    await main(page).getByRole("link", { name: /Viagem/ }).click();
    await expect(page).toHaveURL(new RegExp(`#/perfil/metas/${id}$`));
    await expect(main(page).getByRole("heading", { name: "Editar meta" })).toBeVisible();
    await expect(page.getByLabel("Valor da meta")).toHaveValue("10.000,00");
    await expect(page.getByLabel("Prazo")).toHaveValue("2030-06");
    await page.getByLabel("Nome", { exact: true }).fill("Viagem ao Japão");
    await main(page).getByRole("link", { name: "Cancelar" }).click();
    await expect(page).toHaveURL(/#\/perfil$/);
    await expect(main(page)).toContainText("Viagem");
    await expect(main(page)).not.toContainText("Japão");
    await main(page).getByRole("link", { name: /Viagem/ }).click();
    await page.getByLabel("Nome", { exact: true }).fill("Viagem ao Japão");
    await main(page).getByRole("button", { name: "Salvar" }).click();
    await expect(main(page)).toContainText("Viagem ao Japão");
  } finally {
    await app.close();
  }
});

test("apagar meta com investimento vinculado mantém o investimento, que abre com Nenhuma", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const { id } = await chamar(page, "(api) => api.metas.criar({ nome: 'Entrada', valorAlvoCentavos: 15000000, prazo: '2030-12' })");
    await chamar(page, "(api, d) => api.investimentos.criar(d)", { ...CDB, metaId: id });
    await ir(page, `#/perfil/metas/${id}`);
    await main(page).getByRole("button", { name: "Apagar meta" }).click();
    const dialogo = page.getByRole("dialog");
    await expect(dialogo).toContainText("Apagar a meta Entrada? Os investimentos vinculados continuam na carteira.");
    await dialogo.getByRole("button", { name: "Cancelar" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    expect(await chamar(page, "(api) => api.metas.listar()")).toHaveLength(1);

    await main(page).getByRole("button", { name: "Apagar meta" }).click();
    await page.getByRole("dialog").getByRole("button", { name: "Apagar meta" }).click();
    await expect(page).toHaveURL(/#\/perfil$/);
    await expect(main(page)).not.toContainText("Entrada");
    await ir(page, "#/investimentos");
    await expect(page.locator("main tbody tr").first()).toContainText("CDB Banco Inter");
    await page.getByRole("link", { name: "Editar" }).click();
    await expect(page.getByLabel("Vincular a uma meta")).toHaveValue("");
    await expect(page.getByLabel("Vincular a uma meta").locator("option:checked")).toHaveText("Nenhuma");
  } finally {
    await app.close();
  }
});

test("ao editar o investimento mostra a meta vinculada", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const { id } = await chamar(page, "(api) => api.metas.criar({ nome: 'Entrada', valorAlvoCentavos: 15000000, prazo: '2030-12' })");
    const inv = await chamar(page, "(api, d) => api.investimentos.criar(d)", { ...CDB, metaId: id });
    await ir(page, `#/investimentos/${inv.id}`);
    await expect(page.getByLabel("Vincular a uma meta").locator("option:checked")).toHaveText("Entrada");
  } finally {
    await app.close();
  }
});
