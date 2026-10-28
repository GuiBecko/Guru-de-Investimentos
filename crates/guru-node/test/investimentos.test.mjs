import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
const { Core } = createRequire(import.meta.url)("../index.js");

const abrir = () => Core.open(mkdtempSync(join(tmpdir(), "guru-")));

const cdb = (extra = {}) => ({
  tipo: "rf-cdb",
  instituicao: "Banco Inter",
  emissor: "Banco Inter",
  indexador: "pos",
  taxa: "102",
  benchmark: "cdi",
  dataAplicacao: "2024-01-10",
  valorAplicadoCentavos: 2000000,
  liquidez: "diaria",
  ir: "regressiva",
  ...extra,
});

test("catálogo tem 17 tipos e a LCI isenta com FGC", () => {
  const c = abrir().catalogo();
  assert.equal(c.tipos.length, 17);
  const lci = c.tipos.find((t) => t.codigo === "rf-lci");
  assert.equal(lci.irPadrao, "isento");
  assert.equal(lci.irEditavel, false);
  assert.equal(lci.garantia, "fgc");
  assert.equal(c.classes[0].codigo, "rf");
  assert.equal(c.classes.length, 5);
  assert.equal(c.indexadores.length, 4);
  assert.equal(c.benchmarks.length, 2);
  assert.equal(c.liquidezes.length, 5);
  assert.equal(c.irs.length, 2);
  assert.equal(c.garantias.length, 3);
  assert.equal(c.instituicoes[0], "Banco Inter");
  const td = c.tipos.find((t) => t.codigo === "td-selic");
  assert.equal(td.indexadorForcado, "selic");
  assert.equal(td.liquidezForcada, "d1");
  assert.equal(td.exigeEmissor, false);
  assert.equal(td.liquidezPadrao, "d1");
  const cripto = c.tipos.find((t) => t.codigo === "cripto");
  assert.equal(cripto.exigeEmissor, true);
  assert.equal(cripto.liquidezPadrao, "diaria");
  assert.equal(c.tipos.find((t) => t.codigo === "rv-acao").liquidezPadrao, undefined);
});

test("cria um CDB e lê de volta com rótulos", () => {
  const core = abrir();
  const r = core.criarInvestimento(cdb());
  assert.deepEqual(r.erros, []);
  assert.equal(typeof r.id, "number");
  const inv = core.investimento(r.id);
  assert.equal(inv.rendimento, "102% do CDI");
  assert.equal(inv.garantiaRotulo, "FGC");
  assert.equal(inv.classe, "rf");
  assert.equal(inv.classeRotulo, "Renda fixa");
  assert.equal(inv.indexadorRotulo, "Pós-fixado");
  assert.equal(inv.liquidezRotulo, "Diária");
  assert.equal(inv.irRotulo, "Tabela regressiva");
  assert.equal(inv.nome, "CDB Banco Inter");
  assert.equal(core.listarInvestimentos().length, 1);
});

test("criar inválido devolve erros e não grava", () => {
  const core = abrir();
  const r = core.criarInvestimento(cdb({ emissor: undefined }));
  assert.equal(r.id, undefined);
  assert.ok(r.erros.some((e) => e.campo === "emissor"));
  assert.equal(core.listarInvestimentos().length, 0);
});

test("atualiza e apaga; inexistente lança NAO_ENCONTRADO", () => {
  const core = abrir();
  const { id } = core.criarInvestimento(cdb());
  const r = core.atualizarInvestimento(id, cdb({ taxa: "110" }));
  assert.deepEqual(r.erros, []);
  assert.equal(core.investimento(id).rendimento, "110% do CDI");
  core.apagarInvestimento(id);
  assert.throws(() => core.apagarInvestimento(id), { code: "NAO_ENCONTRADO" });
  assert.throws(() => core.investimento(id), { code: "NAO_ENCONTRADO" });
  assert.throws(() => core.atualizarInvestimento(id, cdb()), { code: "NAO_ENCONTRADO" });
});

test("resumo da carteira agrupa por classe", () => {
  const core = abrir();
  core.criarInvestimento(cdb());
  const r = core.resumoCarteira();
  assert.equal(r.totalAplicadoCentavos, 2000000);
  assert.equal(r.classes[0].rotulo, "Renda fixa");
  assert.equal(r.classes[0].classe, "rf");
  assert.equal(r.classes[0].quantidade, 1);
});
