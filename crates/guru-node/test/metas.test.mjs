import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
const { Core } = createRequire(import.meta.url)("../index.js");

const abrir = () => Core.open(mkdtempSync(join(tmpdir(), "guru-")));

const cdb = (extra = {}) => ({
  tipo: "rf-cdb", instituicao: "Banco Inter", emissor: "Banco Inter", indexador: "pos",
  taxa: "102", benchmark: "cdi", dataAplicacao: "2024-01-10", valorAplicadoCentavos: 2000000,
  liquidez: "diaria", ir: "regressiva", ...extra,
});
const ano = new Date().getFullYear() + 2;

test("meta: criar, listar sem aportes, vincular CDB e ver o acumulado", () => {
  const c = abrir();
  const r = c.criarMeta({ nome: "Viagem", valorAlvoCentavos: 10000000, prazo: `${ano}-12` });
  assert.deepEqual(r.erros, []);
  const [m] = c.listarMetas();
  assert.equal(m.situacao, "sem_aportes");
  assert.equal(m.acumuladoCentavos, 0);
  assert.equal(c.criarInvestimento(cdb({ metaId: r.id })).erros.length, 0);
  assert.equal(c.meta(r.id).acumuladoCentavos, 2000000);
  const [i] = c.listarInvestimentos();
  assert.equal(i.metaId, r.id);
  assert.equal(i.metaNome, "Viagem");
});

test("meta inválida devolve erros; atualizar e apagar", () => {
  const c = abrir();
  assert.equal(c.criarMeta({ nome: "", valorAlvoCentavos: 1, prazo: "2000-01" }).erros.length >= 2, true);
  const { id } = c.criarMeta({ nome: "A", valorAlvoCentavos: 100, prazo: `${ano}-01` });
  assert.deepEqual(c.atualizarMeta(id, { nome: "B", valorAlvoCentavos: 200, prazo: `${ano}-02` }).erros, []);
  assert.equal(c.meta(id).nome, "B");
  const v = c.criarInvestimento(cdb({ metaId: id }));
  c.apagarMeta(id);
  assert.equal(c.listarMetas().length, 0);
  assert.equal(c.investimento(v.id).metaId, undefined);
  assert.throws(() => c.apagarMeta(999), (e) => e.code === "NAO_ENCONTRADO");
});

test("investimento com metaId inexistente dá erro no campo metaId", () => {
  const r = abrir().criarInvestimento(cdb({ metaId: 999 }));
  assert.equal(r.erros[0].campo, "metaId");
});

test("resumo traz percentualDecimos e alvoPercentual", () => {
  const c = abrir();
  c.criarInvestimento(cdb());
  const rf = c.resumoCarteira().classes.find((x) => x.classe === "rf");
  assert.equal(rf.percentualDecimos, 1000);
  assert.equal(rf.alvoPercentual, undefined);
});
