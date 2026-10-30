import { test } from "node:test";
import assert from "node:assert/strict";
import { altasEQuedas } from "../../renderer/js/telas/carteira/variacoes.js";

const v = (ticker, variacaoDecimos) => ({ ticker, variacaoDecimos });

test("pega as 3 maiores altas e as 3 maiores quedas, com seta e classe", () => {
  const lista = [v("A", 90), v("B", 70), v("C", 50), v("D", 30), v("E", 0), v("F", -20), v("G", -40), v("H", -60), v("I", -80)];
  const r = altasEQuedas(lista);
  assert.deepEqual(r.map((x) => x.ticker), ["A", "B", "C", "I", "H", "G"]);
  assert.deepEqual(r[0], { ticker: "A", texto: "▲ 9,0%", classe: "up" });
  assert.deepEqual(r[3], { ticker: "I", texto: "▼ 8,0%", classe: "down" });
});

test("poucas variações: sem repetir o mesmo ticker nas duas pontas", () => {
  assert.deepEqual(altasEQuedas([v("A", 10)]).map((x) => x.ticker), ["A"]);
  assert.deepEqual(altasEQuedas([v("A", -10)]).map((x) => x.ticker), ["A"]);
  assert.deepEqual(altasEQuedas([]), []);
});
