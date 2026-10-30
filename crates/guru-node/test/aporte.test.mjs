import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
const { Core } = createRequire(import.meta.url)("../index.js");

const abrir = () => Core.open(mkdtempSync(join(tmpdir(), "guru-")));
const alvo = (a) => ["rf", "acoes", "fii", "ext", "outros"].map((classe, i) => ({ classe, percentual: a[i] }));

test("sem alocação-alvo: estado sem_alvo e aporte null", () => {
  const r = abrir().proximoAporte(100000);
  assert.equal(r.estado, "sem_alvo");
  assert.equal(r.aporte, null);
  assert.deepEqual(r.erros, []);
});

test("valor zero é inválido com erro em valorCentavos", () => {
  const c = abrir();
  assert.deepEqual(c.salvarAlocacaoAlvo(alvo([50, 25, 10, 10, 5])), []);
  const r = c.proximoAporte(0);
  assert.equal(r.estado, "invalido");
  assert.equal(r.erros[0].campo, "valorCentavos");
  assert.equal(r.aporte, null);
});

test("carteira vazia com alvo: divide o valor inteiro entre as classes", () => {
  const c = abrir();
  assert.deepEqual(c.salvarAlocacaoAlvo(alvo([50, 25, 10, 10, 5])), []);
  const r = c.proximoAporte(100000);
  assert.equal(r.estado, "dividido");
  assert.deepEqual(r.erros, []);
  assert.equal(r.aporte.valorCentavos, 100000);
  assert.equal(r.aporte.partes.reduce((s, p) => s + p.sugeridoCentavos, 0), 100000);
  const rf = r.aporte.partes.find((p) => p.classe === "rf");
  assert.equal(rf.rotulo.length > 0, true);
  assert.equal(rf.alvoPercentual, 50);
  assert.equal(typeof rf.situacao, "string");
});
