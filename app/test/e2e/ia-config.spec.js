const { test, expect } = require("@playwright/test");
const { lancar } = require("./_app");

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
  valorAplicadoCentavos: 2000000,
  liquidez: "vencimento",
  ir: "regressiva",
};

const main = (page) => page.locator("main");
const ir = (page, hash) =>
  page.evaluate((h) => {
    window.location.hash = h;
  }, hash);

async function abrir(page) {
  await page.getByRole("link", { name: "Configurar IA", exact: true }).click();
  await expect(main(page).getByRole("heading", { name: "Sua IA" })).toBeVisible();
}

async function comApp(fn) {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrir(page);
    await fn(page);
  } finally {
    await app.close();
  }
}

const tokens = async (page) => Number((await page.getByTestId("ctx-tokens").textContent()).replace(/\D/g, ""));

test("chave com formato inválido mostra a mensagem de formato", () =>
  comApp(async (page) => {
    await page.getByLabel("Chave da API da Anthropic").fill("abc");
    await main(page).getByRole("button", { name: "Salvar chave" }).click();
    await expect(main(page)).toContainText("Isso não parece uma chave da Anthropic (começa com sk-ant-).");
    await expect(page.getByLabel("Chave da API da Anthropic")).toHaveAttribute("aria-invalid", "true");
  }));

test("chave recusada pela Anthropic mostra a mensagem", () =>
  comApp(async (page) => {
    await page.getByLabel("Chave da API da Anthropic").fill("sk-ant-teste-recusada-0000");
    await main(page).getByRole("button", { name: "Salvar chave" }).click();
    await expect(main(page)).toContainText("A Anthropic recusou esta chave.");
    await expect(main(page).getByRole("button", { name: "Remover chave" })).toHaveCount(0);
  }));

test("chave boa fica ativa, mascarada, e pode ser removida", () =>
  comApp(async (page) => {
    await expect(main(page).getByRole("button", { name: "Remover chave" })).toHaveCount(0);
    await page.getByLabel("Chave da API da Anthropic").fill("sk-ant-teste-boa-12345678-4f2a");
    await main(page).getByRole("button", { name: "Salvar chave" }).click();
    await expect(main(page).getByText("ativo", { exact: true })).toBeVisible();
    await expect(main(page)).toContainText("sk-ant-…4f2a");
    await expect(page.getByLabel("Chave da API da Anthropic")).toHaveValue("");
    await main(page).getByRole("button", { name: "Remover chave" }).click();
    await expect(main(page).getByText("ativo", { exact: true })).toHaveCount(0);
    await expect(main(page)).not.toContainText("sk-ant-…");
    await expect(main(page).getByRole("button", { name: "Remover chave" })).toHaveCount(0);
  }));

test("provedores futuros aparecem com V2", () =>
  comApp(async (page) => {
    for (const nome of ["OpenAI", "Google Gemini", "Modelo local (Ollama)"]) await expect(main(page)).toContainText(nome);
    await expect(main(page).getByText("V2", { exact: true })).toHaveCount(3);
  }));

test("trocar o modelo persiste depois de navegar e voltar", () =>
  comApp(async (page) => {
    await page.getByLabel("Modelo").selectOption("claude-haiku-5-5");
    await expect(main(page)).toContainText("Modelo: Haiku 5.5 · mais barato");
    await ir(page, "#/carteira");
    await abrir(page);
    await expect(page.getByLabel("Modelo")).toHaveValue("claude-haiku-5-5");
  }));

test("desligar Carteira persiste e baixa o total de tokens", () =>
  comApp(async (page) => {
    const antes = await tokens(page);
    const chave = page.getByRole("switch", { name: "Carteira", exact: true });
    await expect(chave).toBeChecked();
    await chave.uncheck();
    await expect.poll(() => tokens(page)).toBeLessThan(antes);
    await ir(page, "#/carteira");
    await abrir(page);
    await expect(page.getByRole("switch", { name: "Carteira", exact: true })).not.toBeChecked();
  }));

test("editar Perfil e objetivo marca como editado e volta ao automático", () =>
  comApp(async (page) => {
    const bloco = page.getByTestId("parte-perfil");
    await bloco.getByRole("button", { name: "editar" }).click();
    await bloco.getByRole("textbox").fill("Texto escrito por mim.");
    await bloco.getByRole("button", { name: "Salvar", exact: true }).click();
    await expect(bloco).toContainText("editado por você");
    await expect(bloco).toContainText("Texto escrito por mim.");
    await bloco.getByRole("button", { name: "voltar ao automático" }).click();
    await expect(bloco).not.toContainText("editado por você");
  }));

test("instruções contam caracteres e salvam", () =>
  comApp(async (page) => {
    await page.getByLabel("Instruções para a sua IA").fill("Sem cripto.");
    await expect(main(page)).toContainText("11/4.000");
    await main(page).getByRole("button", { name: "Salvar instruções" }).click();
    await expect(page.getByTestId("parte-instrucoes")).toContainText("Sem cripto.");
  }));

test("com Apenas percentuais ligada o texto da Carteira não tem R$", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await page.evaluate((d) => import("./js/api.js").then(({ api }) => api.investimentos.criar(d)), CDB);
    await abrir(page);
    await expect(page.getByRole("switch", { name: "Apenas percentuais" })).toBeChecked();
    const bloco = page.getByTestId("parte-carteira");
    await bloco.getByRole("button", { name: "editar" }).click();
    const texto = await bloco.getByRole("textbox").inputValue();
    expect(texto).toContain("CDB");
    expect(texto).not.toContain("R$");
  } finally {
    await app.close();
  }
});
