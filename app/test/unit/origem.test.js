const { test } = require("node:test");
const assert = require("node:assert/strict");
const { origemPermitida } = require("../../main/origem.js");

const esperada = "file:///x/renderer/index.html";

test("aceita a página esperada, com ou sem hash e query", () => {
  assert.equal(origemPermitida("file:///x/renderer/index.html", esperada), true);
  assert.equal(origemPermitida("file:///x/renderer/index.html#/investimentos", esperada), true);
  assert.equal(origemPermitida("file:///x/renderer/index.html?a=1", esperada), true);
});

test("recusa outro caminho, protocolo ou URL inválida", () => {
  assert.equal(origemPermitida("file:///x/renderer/erro.html", esperada), false);
  assert.equal(origemPermitida("https://exemplo.com/", esperada), false);
  assert.equal(origemPermitida("file:///y/renderer/index.html", esperada), false);
  assert.equal(origemPermitida("", esperada), false);
  assert.equal(origemPermitida("não é url", esperada), false);
});
