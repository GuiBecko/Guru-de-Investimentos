import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { escala, dominioComZero, caminhoLinha, arcosRosca, indiceMaisProximo } from "../../renderer/js/graficos.js";

test("escala mapeia o domínio na faixa e trata domínio degenerado", () => {
  assert.equal(escala([0, 10], [0, 100])(5), 50);
  assert.equal(escala([0, 10], [100, 0])(10), 0);
  assert.equal(escala([3, 3], [0, 100])(3), 50);
});

test("dominioComZero ignora null e sempre inclui 0", () => {
  assert.deepEqual(dominioComZero([null, 2, 5]), [0, 5]);
  assert.deepEqual(dominioComZero([-3, null, 4]), [-3, 4]);
  assert.deepEqual(dominioComZero([null, null]), [0, 0]);
});

test("caminhoLinha quebra a linha em null com novo M", () => {
  const d = caminhoLinha([{ x: 0, y: 1 }, { x: 1, y: 2 }, null, { x: 3, y: 4 }]);
  assert.equal(d.match(/M/g).length, 2);
  assert.equal(d, "M0 1L1 2M3 4");
  assert.equal(caminhoLinha([null]), "");
});

test("arcosRosca: dois arcos, fatia zero vazia, fatia única é anel", () => {
  const arcos = arcosRosca([1, 1], 50, 30);
  assert.equal(arcos.length, 2);
  assert.ok(arcos.every((a) => a.startsWith("M") && !a.includes("NaN")));
  assert.deepEqual(arcosRosca([0, 1], 50, 30)[0], "");
  const anel = arcosRosca([1], 50, 30)[0];
  assert.equal((anel.match(/A/g) || []).length, 4);
  assert.ok(!anel.includes("NaN"));
  assert.deepEqual(arcosRosca([0, 0], 50, 30), ["", ""]);
});

test("indiceMaisProximo", () => {
  assert.equal(indiceMaisProximo([0, 10, 20], 12), 1);
  assert.equal(indiceMaisProximo([0, 10, 20], 100), 2);
  assert.equal(indiceMaisProximo([0, 10, 20], -5), 0);
});

test("graficos.js não tem cores fixas", () => {
  const texto = readFileSync(new URL("../../renderer/js/graficos.js", import.meta.url), "utf8");
  assert.doesNotMatch(texto, /#[0-9a-fA-F]{3,6}\b/);
  assert.doesNotMatch(texto, /\.style\b|setAttribute\(\s*["']style|innerHTML/);
});
