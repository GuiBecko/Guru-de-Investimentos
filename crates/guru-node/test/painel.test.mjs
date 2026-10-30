import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";
const { Core } = createRequire(import.meta.url)("../index.js");

const pastaFixtures = resolve(import.meta.dirname, "../../guru-core/tests/fixtures");
const abrir = () =>
  Core.open(mkdtempSync(join(tmpdir(), "guru-")), { pastaFixtures, hoje: "2026-10-28" });

const cdb = {
  tipo: "rf-cdb",
  instituicao: "Banco Inter",
  emissor: "Banco Inter",
  indexador: "pos",
  taxa: "100",
  benchmark: "cdi",
  dataAplicacao: "2026-08-31",
  valorAplicadoCentavos: 1000000,
  liquidez: "diaria",
  ir: "regressiva",
};

const codigo = (fn) => {
  try {
    fn();
  } catch (e) {
    return e.code;
  }
  return null;
};

test("painel devolve 13 datas para 12 meses e 7 para 6", () => {
  const core = abrir();
  const p12 = core.painel(12);
  assert.equal(p12.datas.length, 13);
  assert.equal(p12.datas[0], "2025-10-31");
  assert.equal(p12.datas[12], "2026-10-28");
  assert.equal(p12.pontos.length, 13);
  assert.equal(core.painel(6).datas.length, 7);
});

test("benchmarks: três sem classe; cinco com classe, e benchmarkPadrao só com classe", () => {
  const core = abrir();
  core.criarInvestimento(cdb);
  const geral = core.painel(12);
  assert.deepEqual(geral.benchmarks.map((b) => b.codigo), ["cdi", "ipca", "bova11"]);
  assert.equal(geral.benchmarkPadrao, undefined);
  const rf = core.painel(12, "rf");
  assert.equal(rf.benchmarks.length, 5);
  assert.deepEqual(rf.benchmarks.map((b) => b.codigo), ["cdi", "selic", "ipca", "bova11", "xfix11"]);
  assert.equal(rf.benchmarks[0].rotulo, "CDI");
  assert.equal(rf.benchmarkPadrao, "cdi");
});

test("campos ausentes chegam como null, não undefined", () => {
  const core = abrir();
  core.criarInvestimento(cdb);
  const p = core.painel(12);
  assert.equal(p.pontos[0].retornoCentesimos, null);
  assert.equal(typeof p.pontos[0].patrimonioCentavos, "number");
  // Sem mercado atualizado, os benchmarks inteiros ficam ausentes.
  assert.equal(p.benchmarks[0].valores, null);
  assert.equal(p.numeros.mesCdiCentesimos, null);
  assert.equal(p.numeros.periodoCdiCentesimos, null);
  assert.deepEqual(Object.keys(p.numeros).sort(), [
    "aportadoCentavos", "mesCdiCentesimos", "mesCentesimos", "patrimonioCentavos",
    "periodoCdiCentesimos", "periodoCentesimos", "rendeuCentavos",
  ]);
});

test("porClasse, instituições e variações têm o formato da ponte", () => {
  const core = abrir();
  core.criarInvestimento(cdb);
  const p = core.painel(6);
  assert.equal(p.porClasse[0].classe, "rf");
  assert.equal(typeof p.porClasse[0].rotulo, "string");
  assert.equal(p.porClasse[0].patrimonio.length, 7);
  assert.equal(p.instituicoes[0].nome, "Banco Inter");
  assert.equal(p.instituicoes[0].percentualDecimos, 1000);
  assert.deepEqual(p.variacoes, []);
});

test("meses fora de 6/12 dá PERIODO_INVALIDO; classe desconhecida dá NAO_ENCONTRADO", () => {
  const core = abrir();
  assert.equal(codigo(() => core.painel(3)), "PERIODO_INVALIDO");
  assert.equal(codigo(() => core.painel(12, "xyz")), "NAO_ENCONTRADO");
});
