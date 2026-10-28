import { test } from "node:test";
import assert from "node:assert/strict";
import { TELAS, resolver } from "../../renderer/js/router.js";

test("resolve cada tela pelo hash", () => {
  assert.equal(resolver("#/aporte").id, "aporte");
  assert.equal(resolver("#/ia").titulo, "Configurar IA");
});

test("hash vazio ou desconhecido cai na carteira", () => {
  for (const h of ["", "#", "#/", "#/nao-existe"]) assert.equal(resolver(h).id, "carteira");
});

test("menu na ordem do protótipo", () => {
  assert.deepEqual(
    TELAS.map((t) => t.titulo),
    ["Carteira", "Próximo aporte", "Assistente", "Investimentos", "Perfil e objetivo", "Configurar IA"],
  );
});
