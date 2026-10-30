const { defineConfig } = require("@playwright/test");
const path = require("node:path");

// Os e2e rodam com a janela escondida (ver main.js) para não atrapalhar quem está na máquina.
process.env.GURU_JANELA_OCULTA = "1";

// Modo de teste do mercado: o atualizador lê as fixtures em vez de baixar, com a data de mercado fixa.
process.env.GURU_MERCADO_FIXTURES = path.resolve(__dirname, "../crates/guru-core/tests/fixtures");
process.env.GURU_MERCADO_HOJE = "2026-10-28";

module.exports = defineConfig({
  testDir: "test/e2e",
  timeout: 30000,
  workers: 1,
  reporter: "list",
  use: { trace: "retain-on-failure" },
});
