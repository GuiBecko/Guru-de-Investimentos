const { defineConfig } = require("@playwright/test");

// Os e2e rodam com a janela escondida (ver main.js) para não atrapalhar quem está na máquina.
process.env.GURU_JANELA_OCULTA = "1";

module.exports = defineConfig({
  testDir: "test/e2e",
  timeout: 30000,
  workers: 1,
  reporter: "list",
  use: { trace: "retain-on-failure" },
});
