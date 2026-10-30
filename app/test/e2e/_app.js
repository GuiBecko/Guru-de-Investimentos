// Lançador comum dos e2e. A atualização automática de mercado roda ao abrir o app (contra as
// fixtures) e, ao terminar, redesenha as telas de leitura e grava no banco. Se o teste já
// estiver criando investimentos nessa hora, o resultado depende de quem chega primeiro (e, no
// Windows, a escrita concorrente ainda deixa tudo lento). Por isso `firstWindow()` só devolve a
// janela depois que a atualização de abertura terminou: o rodapé mostra a data dos dados e o
// botão "Atualizar" voltou a ficar habilitado. Daí em diante só o que o teste fizer mexe nos dados.
const { expect, _electron } = require("@playwright/test");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const appDir = path.resolve(__dirname, "../..");

async function esperarMercadoAssentado(page, timeout = 30000) {
  const mercado = page.getByTestId("mercado");
  // "Dados de mercado: ainda não baixados" é o estado antes da atualização, por isso exige a data.
  await expect(mercado.getByTestId("mercado-texto")).toHaveText(/^Dados de mercado de \d\d\/\d\d$/, { timeout });
  await expect(mercado.getByRole("button", { name: "Atualizar", exact: true })).toBeEnabled({ timeout });
}

async function lancar(dir = fs.mkdtempSync(path.join(os.tmpdir(), "guru-e2e-")), extraEnv = {}) {
  const app = await _electron.launch({
    args: ["."],
    cwd: appDir,
    env: { ...process.env, GURU_DATA_DIR: dir, ...extraEnv },
  });
  const original = app.firstWindow.bind(app);
  let pronta;
  app.firstWindow = () => {
    pronta ??= original().then(async (page) => {
      await esperarMercadoAssentado(page);
      return page;
    });
    return pronta;
  };
  return app;
}

module.exports = { lancar, esperarMercadoAssentado };
