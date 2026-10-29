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
const campos = (page) => main(page).locator('input[inputmode="numeric"]');

async function comPerfil(page) {
  await chamar(page, "(api) => api.perfil.registrar(Array(12).fill(0))");
}

async function preencher(page, valores) {
  for (let i = 0; i < valores.length; i++) await campos(page).nth(i).fill(String(valores[i]));
}

test("sem perfil mostra o convite e o botão leva ao teste", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await ir(page, "#/perfil");
    await expect(main(page).getByRole("heading", { name: "Perfil e objetivo" })).toBeVisible();
    await expect(main(page)).toContainText("Descubra seu perfil de investidor");
    await expect(main(page)).toContainText("São 12 perguntas, leva uns 3 minutos. O resultado só aparece no fim.");
    await expect(main(page)).toContainText("Faça o teste de perfil para receber uma sugestão, ou defina a sua.");
    await main(page).getByRole("link", { name: "Fazer o teste" }).click();
    await expect(page).toHaveURL(/#\/perfil\/teste$/);
  } finally {
    await app.close();
  }
});

test("com perfil mostra a pílula com a data de hoje e o rodapé", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const antes = new Date();
    await comPerfil(page);
    await ir(page, "#/perfil");
    // Mesma data local do núcleo; aceita a data de antes ou de depois do registro (virada do dia).
    const fmt = (d) => [d.getDate(), d.getMonth() + 1].map((n) => String(n).padStart(2, "0")).join("/") + `/${d.getFullYear()}`;
    const datas = [antes, new Date()].map(fmt);
    await expect(main(page)).toContainText(new RegExp(`Perfil Conservador · feito em (${[...new Set(datas)].join("|")})`));
    await expect(main(page).getByRole("link", { name: "Refazer o teste" })).toHaveAttribute("href", "#/perfil/teste");
    await expect(main(page)).toContainText("Não é recomendação de investimento.");
    await expect(main(page)).toContainText("Metas");
  } finally {
    await app.close();
  }
});

test("objetivo: digitar, salvar, reabrir o app e o texto está lá", async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "guru-e2e-"));
  let app = await lancar(dir);
  try {
    let page = await app.firstWindow();
    await ir(page, "#/perfil");
    const texto = main(page).getByRole("textbox", { name: /Por que você está investindo/ });
    await texto.fill("Quero comprar um apartamento");
    await expect(main(page)).toContainText("28/2000");
    await main(page).getByRole("button", { name: "Salvar objetivo" }).click();
    await expect(main(page)).toContainText("Objetivo salvo.");
    await app.close();
    app = await lancar(dir);
    page = await app.firstWindow();
    await ir(page, "#/perfil");
    await expect(main(page).getByRole("textbox", { name: /Por que você está investindo/ })).toHaveValue(
      "Quero comprar um apartamento",
    );
  } finally {
    await app.close();
  }
});

test("rascunho do objetivo sobrevive à ida e volta ao editor de alocação", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await ir(page, "#/perfil");
    const texto = main(page).getByRole("textbox", { name: /Por que você está investindo/ });
    await texto.fill("Rascunho ainda não salvo");
    await main(page).getByRole("button", { name: "Definir" }).click();
    await expect(page).toHaveURL(/#\/perfil\/alocacao$/);
    await expect(texto).toHaveValue("Rascunho ainda não salvo");
    await main(page).getByRole("button", { name: "Cancelar" }).click();
    await expect(page).toHaveURL(/#\/perfil$/);
    await expect(texto).toHaveValue("Rascunho ainda não salvo");
  } finally {
    await app.close();
  }
});

test("exemplo clicado entra no texto", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await ir(page, "#/perfil");
    await main(page).getByRole("button", { name: "Quero viver de renda a partir dos 55" }).click();
    await expect(main(page).getByRole("textbox", { name: /Por que você está investindo/ })).toHaveValue(
      "Quero viver de renda a partir dos 55",
    );
    // Com texto, o exemplo é acrescentado depois de uma linha em branco.
    await main(page).getByRole("button", { name: "Quero manter o patrimônio que já tenho" }).click();
    await expect(main(page).getByRole("textbox", { name: /Por que você está investindo/ })).toHaveValue(
      "Quero viver de renda a partir dos 55\n\nQuero manter o patrimônio que já tenho",
    );
  } finally {
    await app.close();
  }
});

test("alocação: ajustar para 60/20/10/10/0 salva e mostra", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await ir(page, "#/perfil");
    await main(page).getByRole("button", { name: "Definir" }).click();
    await expect(page).toHaveURL(/#\/perfil\/alocacao$/);
    await expect(campos(page)).toHaveCount(5);
    await preencher(page, [60, 20, 10, 10, 0]);
    await expect(main(page)).toContainText("Soma: 100%");
    await main(page).getByRole("button", { name: "Salvar", exact: true }).click();
    await expect(page).toHaveURL(/#\/perfil$/);
    const linhas = main(page).getByRole("table").getByRole("row");
    await expect(linhas).toHaveCount(6);
    await expect(linhas.nth(1)).toContainText("60%");
    await expect(linhas.nth(2)).toContainText("20%");
    await expect(main(page).getByRole("button", { name: "Ajustar" })).toBeVisible();
  } finally {
    await app.close();
  }
});

test("soma 99 fica em alerta e o núcleo recusa sem gravar", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await ir(page, "#/perfil/alocacao");
    await preencher(page, [60, 20, 10, 9, 0]);
    await expect(main(page).getByText("Soma: 99%")).toHaveClass(/alerta/);
    await main(page).getByRole("button", { name: "Salvar", exact: true }).click();
    await expect(main(page)).toContainText("A soma precisa dar 100%. Hoje dá 99%.");
    await expect(page).toHaveURL(/#\/perfil\/alocacao$/);
    expect(await chamar(page, "(api) => api.alocacao.ler()")).toBeNull();
  } finally {
    await app.close();
  }
});

test("decimal, negativo e vazio são recusados no campo sem enviar", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await ir(page, "#/perfil/alocacao");
    await preencher(page, [100, 0, 0, 0, 0]);
    for (const ruim of ["12,5", "-1", ""]) {
      await campos(page).nth(1).fill(ruim);
      await main(page).getByRole("button", { name: "Salvar", exact: true }).click();
      await expect(main(page)).toContainText("Use um número inteiro de 0 a 100.");
      await expect(campos(page).nth(1)).toHaveAttribute("aria-invalid", "true");
      await expect(page).toHaveURL(/#\/perfil\/alocacao$/);
    }
    // Digitar no campo limpa o erro dele.
    await campos(page).nth(1).fill("0");
    await expect(campos(page).nth(1)).not.toHaveAttribute("aria-invalid", "true");
    await expect(main(page)).not.toContainText("Use um número inteiro de 0 a 100.");
    // O campo inválido não entra na soma ao vivo.
    await expect(main(page)).toContainText("Soma: 100%");
    expect(await chamar(page, "(api) => api.alocacao.ler()")).toBeNull();
  } finally {
    await app.close();
  }
});

test("usar a sugestão do perfil preenche os valores; sem perfil o botão não aparece", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await ir(page, "#/perfil/alocacao");
    await expect(main(page).getByRole("button", { name: "Usar a sugestão do perfil" })).toHaveCount(0);
    await comPerfil(page);
    await ir(page, "#/perfil");
    await ir(page, "#/perfil/alocacao");
    await preencher(page, [20, 20, 20, 20, 20]);
    await main(page).getByRole("button", { name: "Usar a sugestão do perfil" }).click();
    const valores = await campos(page).evaluateAll((l) => l.map((i) => i.value));
    expect(valores).toEqual(["85", "5", "5", "5", "0"]);
    await expect(main(page)).toContainText("Soma: 100%");
    await main(page).getByRole("button", { name: "Cancelar" }).click();
    await expect(page).toHaveURL(/#\/perfil$/);
  } finally {
    await app.close();
  }
});
