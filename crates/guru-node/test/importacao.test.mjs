import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
const { Core } = createRequire(import.meta.url)("../index.js");

const abrir = () => Core.open(mkdtempSync(join(tmpdir(), "guru-")));
const fixture = (nome) =>
  readFileSync(new URL(`../../guru-core/tests/fixtures/b3/${nome}`, import.meta.url));

test("comparar a posição completa numa carteira vazia dá só Novos, com faltando", () => {
  const c = abrir();
  const r = c.compararImportacao(fixture("posicao-completa.xlsx"), "posicao.xlsx");
  assert.equal(typeof r.token, "string");
  assert.equal(r.arquivo, "posicao.xlsx");
  assert.ok(r.linhasLidas > 0);
  assert.ok(r.itens.length > 0);
  for (const i of r.itens) {
    assert.equal(i.estado, "novo");
    assert.equal(i.estadoRotulo, "novo");
    assert.ok(i.acoes.includes("adicionar"));
    assert.equal(i.acaoPadrao, "adicionar");
    assert.ok(i.sugestao.tipo);
    assert.ok(Array.isArray(i.faltando));
    assert.ok(i.chave);
  }
  assert.ok(r.itens.some((i) => i.faltando.length > 0));
  assert.equal(c.ultimaImportacao(), null);
});

test("aplicar com os dados completos grava tudo e conta os itens", () => {
  const c = abrir();
  const r = c.compararImportacao(fixture("posicao-completa.xlsx"), "posicao.xlsx");
  const decisoes = r.itens.map((i) => {
    const s = { ...i.sugestao };
    for (const f of i.faltando) {
      if (f.campo === "valorAplicadoCentavos") s.valorAplicadoCentavos = 100000;
      if (f.campo === "precoMedioCentavos") s.precoMedioCentavos = 1000;
      if (f.campo === "taxa") s.taxa = "10";
      if (f.campo === "dataAplicacao") s.dataAplicacao = "2024-01-10";
      if (f.campo === "instituicao") s.instituicao = "Banco Inter";
      if (f.campo === "emissor") s.emissor = "Banco Inter";
    }
    return { chave: i.chave, acao: "adicionar", dados: s };
  });
  const res = c.aplicarImportacao(r.token, decisoes);
  assert.deepEqual(res.erros, []);
  assert.equal(res.aplicadas, r.itens.length);
  assert.equal(c.listarInvestimentos().length, r.itens.length);
  assert.match(c.ultimaImportacao(), /^\d{4}-\d{2}-\d{2}$/);
});

test("aplicar um novo incompleto não grava e devolve os campos", () => {
  const c = abrir();
  const r = c.compararImportacao(fixture("posicao-completa.xlsx"), "posicao.xlsx");
  const res = c.aplicarImportacao(r.token, [{ chave: r.itens[0].chave, acao: "adicionar" }]);
  assert.equal(res.aplicadas, 0);
  assert.equal(res.erros[0].chave, r.itens[0].chave);
  assert.ok(res.erros[0].campos.length > 0);
  assert.equal(c.listarInvestimentos().length, 0);
});

test("token velho dá IMPORTACAO_EXPIRADA", () => {
  const c = abrir();
  const a = c.compararImportacao(fixture("posicao-completa.xlsx"), "a.xlsx");
  const b = c.compararImportacao(fixture("posicao-bdr-rf.xlsx"), "b.xlsx");
  assert.notEqual(a.token, b.token);
  assert.throws(() => c.aplicarImportacao(a.token, []), { code: "IMPORTACAO_EXPIRADA" });
  assert.throws(() => abrir().aplicarImportacao("x", []), { code: "IMPORTACAO_EXPIRADA" });
});

test("bytes inválidos dão ARQUIVO_INVALIDO", () => {
  assert.throws(() => abrir().compararImportacao(Buffer.from("isto não é xlsx"), "x.xlsx"), {
    code: "ARQUIVO_INVALIDO",
  });
});

test("decisão com ação desconhecida ou vincular sem id dá DECISAO_INVALIDA", () => {
  const c = abrir();
  const r = c.compararImportacao(fixture("posicao-bdr-rf.xlsx"), "p.xlsx");
  const chave = r.itens[0].chave;
  assert.throws(() => c.aplicarImportacao(r.token, [{ chave, acao: "xyz" }]), { code: "DECISAO_INVALIDA" });
  assert.throws(() => c.aplicarImportacao(r.token, [{ chave, acao: "vincular" }]), { code: "DECISAO_INVALIDA" });
});
