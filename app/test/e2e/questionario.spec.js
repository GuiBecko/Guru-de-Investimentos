const { test, expect, _electron } = require("@playwright/test");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const appDir = path.resolve(__dirname, "../..");

function lancar() {
  return _electron.launch({
    args: ["."],
    cwd: appDir,
    env: { ...process.env, GURU_DATA_DIR: fs.mkdtempSync(path.join(os.tmpdir(), "guru-e2e-")) },
  });
}

const main = (page) => page.locator("main");

async function abrirTeste(page) {
  await page.evaluate(() => {
    window.location.hash = "#/perfil/teste";
  });
  await expect(main(page)).toContainText("pergunta 1 de 12");
}

async function responder(page, n, indice) {
  await expect(main(page)).toContainText(`pergunta ${n} de 12`);
  await page.getByRole("radio").nth(indice).click();
  await page.getByRole("button", { name: n === 12 ? "Ver meu perfil" : "Próxima pergunta" }).click();
}

async function responderTudo(page, indices, de = 1) {
  for (let i = de; i <= indices.length; i++) await responder(page, i, indices[i - 1]);
}

test("não mostra pontuação nem perfil durante o teste", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrirTeste(page);
    for (let n = 1; n <= 12; n++) {
      await expect(main(page)).toContainText(`pergunta ${n} de 12`);
      const texto = await main(page).innerText();
      for (const proibido of ["Conservador", "Moderado", "Arrojado", "pontos"]) {
        expect(texto).not.toContain(proibido);
      }
      expect(await main(page).locator("aside").innerText()).not.toContain("%");
      await responder(page, n, 2);
    }
    await expect(main(page)).toContainText("Seu perfil:");
  } finally {
    await app.close();
  }
});

test("voltar mantém e troca a resposta", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrirTeste(page);
    await responder(page, 1, 2);
    await responder(page, 2, 2);
    await expect(main(page)).toContainText("pergunta 3 de 12");
    await page.getByRole("button", { name: "Voltar" }).click();
    await expect(main(page)).toContainText("pergunta 2 de 12");
    await expect(page.getByRole("radio").nth(2)).toHaveAttribute("aria-checked", "true");
    // Troca para "Menos de 1 ano": a trava deve aparecer no fim.
    await page.getByRole("radio").nth(0).click();
    await expect(page.getByRole("radio").nth(2)).toHaveAttribute("aria-checked", "false");
    await page.getByRole("button", { name: "Próxima pergunta" }).click();
    await responderTudo(page, [2, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1], 3);
    await expect(main(page)).toContainText("Seu perfil: Conservador");
  } finally {
    await app.close();
  }
});

test("todas as primeiras opções dão Conservador", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrirTeste(page);
    await responderTudo(page, Array(12).fill(0));
    await expect(main(page)).toContainText("Seu perfil: Conservador");
    await expect(main(page)).toContainText("Prioriza a segurança e a liquidez.");
    await expect(page).toHaveURL(/#\/perfil\/resultado$/);
    const alocacao = main(page).getByRole("list", { name: "Alocação sugerida" }).getByRole("listitem");
    await expect(alocacao).toHaveCount(5);
    await expect(alocacao.nth(0)).toContainText("85%");
    await expect(alocacao.nth(1)).toContainText("5%");
    await expect(alocacao.nth(4)).toContainText("0%");
    await expect(main(page)).toContainText("Não é recomendação de investimento.");
  } finally {
    await app.close();
  }
});

test("prazo curto mostra o aviso da trava", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrirTeste(page);
    await responderTudo(page, [3, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3]);
    await expect(main(page)).toContainText("Seu perfil: Conservador");
    await expect(main(page).getByRole("alert")).toContainText(
      "Como você deve precisar do dinheiro em menos de 1 ano, o perfil ficou Conservador.",
    );
  } finally {
    await app.close();
  }
});

test("sair no meio não grava", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrirTeste(page);
    for (let n = 1; n <= 5; n++) await responder(page, n, 1);
    await page.getByRole("link", { name: "Carteira", exact: true }).click();
    await expect(main(page).getByRole("heading", { name: "Carteira" })).toBeVisible();
    await page.evaluate(() => {
      window.location.hash = "#/perfil/resultado";
    });
    await expect(main(page)).toContainText("Você ainda não fez o teste.");
    await expect(main(page).getByRole("link", { name: /teste/i })).toHaveAttribute("href", "#/perfil/teste");
  } finally {
    await app.close();
  }
});

test("usar esta alocação salva e volta ao perfil", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrirTeste(page);
    await responderTudo(page, Array(12).fill(0));
    await page.getByRole("button", { name: "Usar esta alocação" }).click();
    await expect(page).toHaveURL(/#\/perfil$/);
    const salva = await page.evaluate(() => import("./js/api.js").then(({ api }) => api.alocacao.ler()));
    expect(JSON.stringify(salva)).toContain("85");
  } finally {
    await app.close();
  }
});

test("ajustar a alocação leva ao editor", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrirTeste(page);
    await responderTudo(page, Array(12).fill(0));
    await page.getByRole("link", { name: "Ajustar a alocação" }).click();
    await expect(page).toHaveURL(/#\/perfil\/alocacao$/);
  } finally {
    await app.close();
  }
});

test("teclado escolhe a opção", async () => {
  const app = await lancar();
  try {
    const page = await app.firstWindow();
    await abrirTeste(page);
    await expect(page.getByRole("heading", { level: 2 })).toBeFocused();
    await page.getByRole("radio").first().focus();
    await page.keyboard.press("ArrowDown");
    await expect(page.getByRole("radio").nth(1)).toHaveAttribute("aria-checked", "true");
    await expect(page.getByRole("radio").nth(1)).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(main(page)).toContainText("pergunta 2 de 12");
    await expect(page.getByRole("heading", { level: 2 })).toBeFocused();
  } finally {
    await app.close();
  }
});
