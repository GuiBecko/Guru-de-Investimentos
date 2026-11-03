// Cada e2e cria uma pasta de dados nova no temporário do sistema. Para não acumular milhares
// delas (o /tmp do Linux costuma ser pequeno e fica cheio), a rodada inteira aponta o temporário
// para uma pasta própria, criada aqui e apagada no fim. Os workers e o Electron herdam o ambiente.
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

module.exports = async () => {
  const raiz = fs.mkdtempSync(path.join(os.tmpdir(), "guru-rodada-e2e-"));
  for (const nome of ["TMPDIR", "TEMP", "TMP"]) process.env[nome] = raiz;
  return async () => {
    fs.rmSync(raiz, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  };
};
