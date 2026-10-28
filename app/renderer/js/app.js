import { api } from "./api.js";
import { iniciar } from "./router.js";

function mostrarVersoes(core, esquema) {
  document.querySelector('[data-testid="core-version"]').textContent = core;
  document.querySelector('[data-testid="schema-version"]').textContent = esquema;
}

try {
  iniciar(document.querySelector(".app"));
} catch (e) {
  console.error("Falha ao iniciar o roteador:", e);
}

api.app
  .info()
  .then((info) => mostrarVersoes(info.coreVersion, String(info.schemaVersion)))
  .catch((e) => {
    console.error("Falha ao ler as versões:", e);
    mostrarVersoes("—", "—");
  });
