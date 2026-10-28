const test = require("node:test");
const assert = require("node:assert/strict");
const { normalizarErro } = require("../../main/erro.js");

test("mantém code em SCREAMING_SNAKE_CASE e a mensagem", () => {
  const e = Object.assign(new Error("Não achei."), { code: "NAO_ENCONTRADO" });
  assert.deepEqual(normalizarErro(e), { code: "NAO_ENCONTRADO", message: "Não achei.", original: false });
});

test("code fora do formato vira ERRO_DESCONHECIDO sem vazar a mensagem", () => {
  for (const code of ["ENOENT_x", "erro", "1ABC", "A-B", "", 42, undefined, null]) {
    const e = Object.assign(new Error("segredo: /home/x"), { code });
    const r = normalizarErro(e);
    assert.equal(r.code, "ERRO_DESCONHECIDO");
    assert.equal(r.message, "Ocorreu um erro inesperado.");
    assert.equal(r.original, true);
  }
});

test("aceita valores que não são Error", () => {
  assert.equal(normalizarErro(null).code, "ERRO_DESCONHECIDO");
  assert.equal(normalizarErro("falhou").code, "ERRO_DESCONHECIDO");
});
