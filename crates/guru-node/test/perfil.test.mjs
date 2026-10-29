import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
const { Core } = createRequire(import.meta.url)("../index.js");

const abrir = () => Core.open(mkdtempSync(join(tmpdir(), "guru-")));

test("questionário tem 12 perguntas e nenhuma opção traz pontos", () => {
  const q = abrir().questionario();
  assert.equal(q.perguntas.length, 12);
  assert.equal(q.avisosFixos.length, 2);
  for (const p of q.perguntas) {
    assert.equal(p.opcoes.length, 4);
    assert.equal(typeof p.opcoes[0], "string");
    assert.deepEqual(Object.keys(p).sort(), ["bloco", "numero", "opcoes", "porque", "texto"]);
  }
});

test("doze zeros dão Conservador e a alocação sugerida soma 100", () => {
  const c = abrir();
  assert.equal(c.perfilAtual(), null);
  const r = c.registrarPerfil(Array(12).fill(0));
  assert.deepEqual(r.erros, []);
  assert.equal(r.perfil.perfil, "conservador");
  assert.equal(r.perfil.perfilRotulo, "Conservador");
  assert.equal(r.perfil.vencido, false);
  assert.equal(r.perfil.alocacaoSugerida.reduce((s, a) => s + a.percentual, 0), 100);
  assert.equal(c.perfilAtual().perfil, "conservador");
  assert.equal(c.alocacaoAlvo().length, 5);
});

test("11 respostas dão erro no campo respostas e nada é gravado", () => {
  const c = abrir();
  const r = c.registrarPerfil(Array(11).fill(0));
  assert.equal(r.perfil, undefined);
  assert.equal(r.erros[0].campo, "respostas");
  assert.equal(c.perfilAtual(), null);
});

test("alocação-alvo: soma 99 é recusada; 100 grava", () => {
  const c = abrir();
  const v = (a) => ["rf", "acoes", "fii", "ext", "outros"].map((classe, i) => ({ classe, percentual: a[i] }));
  assert.equal(c.alocacaoAlvo(), null);
  assert.equal(c.salvarAlocacaoAlvo(v([50, 25, 10, 10, 4]))[0].campo, "alocacao");
  assert.equal(c.alocacaoAlvo(), null);
  assert.deepEqual(c.salvarAlocacaoAlvo(v([50, 25, 10, 10, 5])), []);
  assert.equal(c.alocacaoAlvo()[0].percentual, 50);
});

test("objetivo: grava, lê e apaga com texto vazio", () => {
  const c = abrir();
  assert.equal(c.objetivo(), null);
  assert.deepEqual(c.salvarObjetivo("Aposentadoria"), []);
  assert.equal(c.objetivo(), "Aposentadoria");
  c.salvarObjetivo("  ");
  assert.equal(c.objetivo(), null);
});
