import { test } from "node:test";
import assert from "node:assert/strict";
import { rotulosPeriodo, rotuloDaClasse } from "../../renderer/js/telas/carteira/periodo.js";

test("sem `desde`, o período é o do seletor", () => {
  assert.deepEqual(rotulosPeriodo(null, 12), { periodo: "Em 12 meses", rendeu: "Rendeu em 12 meses" });
  assert.deepEqual(rotulosPeriodo(undefined, 6), { periodo: "Em 6 meses", rendeu: "Rendeu em 6 meses" });
});

test("com `desde`, o período começa no primeiro aporte", () => {
  assert.deepEqual(rotulosPeriodo("2026-04-15", 12), { periodo: "Desde 04/26", rendeu: "Rendeu desde 04/26" });
});

test("legenda da classe no detalhe concorda em gênero e número", () => {
  assert.equal(rotuloDaClasse("rf"), "Sua renda fixa");
  assert.equal(rotuloDaClasse("acoes"), "Suas ações");
  assert.equal(rotuloDaClasse("fii"), "Seus FIIs");
  assert.equal(rotuloDaClasse("ext"), "Seus investimentos no exterior");
  assert.equal(rotuloDaClasse("outros"), "Seus outros investimentos");
});
