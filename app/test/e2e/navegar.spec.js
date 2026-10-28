const { test, expect, _electron } = require("@playwright/test");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const appDir = path.resolve(__dirname, "../..");
const TELAS = [
  ["carteira", "Carteira"],
  ["aporte", "Próximo aporte"],
  ["assistente", "Assistente"],
  ["investimentos", "Investimentos"],
  ["perfil", "Perfil e objetivo"],
  ["ia", "Configurar IA"],
];

function lancar() {
  return _electron.launch({
    args: ["."],
    cwd: appDir,
    env: { ...process.env, GURU_DATA_DIR: fs.mkdtempSync(path.join(os.tmpdir(), "guru-e2e-")) },
  });
}

test("clicar no menu troca a tela e marca o item", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const conteudo = page.locator("main");
    await expect(conteudo.getByRole("heading", { name: "Carteira" })).toBeVisible();
    await page.getByRole("link", { name: "Assistente" }).click();
    await expect(conteudo.getByRole("heading", { name: "Assistente" })).toBeVisible();
    await expect(conteudo).toContainText("Em construção");
    await expect(page.locator("nav a.on")).toHaveCount(1);
    await expect(page.locator("nav a.on")).toHaveText("Assistente");
  } finally {
    await app.close();
  }
});

test("nenhum erro de CSP no console", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    const erros = [];
    page.on("console", (m) => m.type() === "error" && erros.push(m.text()));
    page.on("pageerror", (e) => erros.push(String(e)));
    await page.reload();
    for (const [id, titulo] of TELAS) {
      await page.getByRole("link", { name: titulo, exact: true }).click();
      await expect(page.locator("main").getByRole("heading", { name: titulo })).toBeVisible();
      await expect(page.locator("nav a.on")).toHaveAttribute("href", `#/${id}`);
    }
    expect(erros).toEqual([]);
  } finally {
    await app.close();
  }
});
