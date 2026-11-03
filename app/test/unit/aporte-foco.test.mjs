import { test } from "node:test";
import assert from "node:assert/strict";
import { resumoContexto } from "../../renderer/js/telas/aporte/gaveta.js";

test("resumoContexto com valor, direções, texto livre e meta", () => {
  assert.equal(
    resumoContexto({
      valorCentavos: 200000,
      direcoes: ["novo", "acoes"],
      direcaoLivre: "  quero algo\ncom liquidez ",
      metaNome: "Entrada do apartamento",
    }),
    'Contexto: aporte de R$ 2.000,00 · direção: Ações, Experimentar algo novo, "quero algo com liquidez" · meta: Entrada do apartamento · mais as partes ligadas em Configurar IA',
  );
});

test("resumoContexto sem direção nem texto livre diz livre", () => {
  assert.equal(
    resumoContexto({ valorCentavos: 150050, direcoes: [], direcaoLivre: "   ", metaNome: "Viagem" }),
    "Contexto: aporte de R$ 1.500,50 · direção: livre · meta: Viagem · mais as partes ligadas em Configurar IA",
  );
});

test("resumoContexto sem meta diz nenhuma", () => {
  assert.equal(
    resumoContexto({ valorCentavos: 200000, direcoes: ["renda_fixa"], direcaoLivre: "", metaNome: null }),
    "Contexto: aporte de R$ 2.000,00 · direção: Renda fixa · meta: nenhuma · mais as partes ligadas em Configurar IA",
  );
  assert.equal(
    resumoContexto({ valorCentavos: 200000, direcoes: [], direcaoLivre: "só liquidez", metaNome: "" }),
    'Contexto: aporte de R$ 2.000,00 · direção: "só liquidez" · meta: nenhuma · mais as partes ligadas em Configurar IA',
  );
});
