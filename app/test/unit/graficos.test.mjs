import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { escala, dominioComZero, caminhoLinha, pontosIsolados, arcosRosca, indiceMaisProximo, margemEsquerda, resumoUltimoPonto } from "../../renderer/js/graficos.js";

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

// Cor hexadecimal de 3, 4, 6 ou 8 dígitos.
const COR = /#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})\b/;

test("a regex de cor pega 3, 4, 6 e 8 dígitos e ignora rotas", () => {
  for (const cor of ["#abc", "#abcd", "#a1b2c3", "#a1b2c3d4"]) assert.match(`fill: ${cor};`, COR, cor);
  for (const rota of ["#/carteira/acoes", "#/aporte", "#/investimentos/12"]) assert.doesNotMatch(rota, COR, rota);
});

test("gráficos e telas do painel não têm cores fixas nem style", () => {
  for (const arquivo of ["graficos.js", "telas/carteira/painel.js", "telas/carteira.js"]) {
    const texto = readFileSync(new URL(`../../renderer/js/${arquivo}`, import.meta.url), "utf8");
    assert.doesNotMatch(texto, COR, arquivo);
    assert.doesNotMatch(texto, /\.style\b|setAttribute\(\s*["']style|innerHTML/, arquivo);
  }
});

test("resumoUltimoPonto dá o último valor de cada série", () => {
  const fmt = (v) => `${v}%`;
  const series = [
    { rotulo: "Carteira", valores: [0, 1, 2.94] },
    { rotulo: "IPCA", valores: [0, 4.48, null] },
    { rotulo: "BOVA11", valores: [null, null] },
  ];
  assert.equal(resumoUltimoPonto(series, fmt), "Carteira 2.94%, IPCA 4.48%");
  assert.equal(resumoUltimoPonto([], fmt), "");
});

test("pontosIsolados acha só o ponto entre dois null", () => {
  const a = { x: 0, y: 0 }, b = { x: 1, y: 1 }, c = { x: 2, y: 2 }, d = { x: 3, y: 3 };
  assert.deepEqual(pontosIsolados([a, null, b, null, c, d]), [a, b]);
  assert.deepEqual(pontosIsolados([a, b]), []);
  assert.deepEqual(pontosIsolados([null, c]), [c]);
});

test("margemEsquerda cabe o maior rótulo e nunca fica abaixo de 64", () => {
  assert.equal(margemEsquerda(["0,00%", "+1,07%"]), 64);
  assert.equal(margemEsquerda(["R$ 0", "R$ 1.242.107"]), Math.ceil(12 * 6.2) + 10);
});

test("dominioComZero trata NaN e Infinity como lacuna", () => {
  assert.deepEqual(dominioComZero([NaN, Infinity, 3, -Infinity]), [0, 3]);
});
