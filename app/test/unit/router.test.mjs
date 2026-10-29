import { test } from "node:test";
import assert from "node:assert/strict";
import { TELAS, resolver } from "../../renderer/js/router.js";

test("resolve cada tela pelo hash", () => {
  assert.equal(resolver("#/aporte").tela.id, "aporte");
  assert.equal(resolver("#/ia").tela.titulo, "Configurar IA");
  assert.deepEqual(resolver("#/aporte").parametros, []);
});

test("segmentos depois da tela viram parâmetros", () => {
  const novo = resolver("#/investimentos/novo");
  assert.equal(novo.tela.id, "investimentos");
  assert.deepEqual(novo.parametros, ["novo"]);
  const rf = resolver("#/carteira/rf");
  assert.equal(rf.tela.id, "carteira");
  assert.deepEqual(rf.parametros, ["rf"]);
});

test("barra final e segmentos vazios não viram parâmetros", () => {
  assert.deepEqual(resolver("#/investimentos/").parametros, []);
  assert.deepEqual(resolver("#/carteira//rf").parametros, ["rf"]);
});

test("hash vazio ou desconhecido cai na carteira sem parâmetros", () => {
  for (const h of ["", "#", "#/", "#/nao-existe", "#/nao-existe/x"]) {
    const r = resolver(h);
    assert.equal(r.tela.id, "carteira");
    assert.deepEqual(r.parametros, []);
  }
});

test("menu na ordem do protótipo", () => {
  assert.deepEqual(
    TELAS.map((t) => t.titulo),
    ["Carteira", "Próximo aporte", "Assistente", "Investimentos", "Perfil e objetivo", "Configurar IA"],
  );
});
