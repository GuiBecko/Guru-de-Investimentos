import { test } from "node:test";
import assert from "node:assert/strict";
import { desembrulhar } from "../../renderer/js/api.js";

test("devolve o value quando ok", async () => {
  assert.equal(await desembrulhar(Promise.resolve({ ok: true, value: 1 })), 1);
});

test("rejeita com Error que carrega o code e a message", async () => {
  await assert.rejects(
    desembrulhar(Promise.resolve({ ok: false, code: "DB_INVALID", message: "x" })),
    (e) => e instanceof Error && e.code === "DB_INVALID" && e.message === "x",
  );
});

test("api.investimentos e api.carteira repassam à ponte e desembrulham", async () => {
  const chamadas = [];
  const canal = (nome) => (...args) => {
    chamadas.push([nome, ...args]);
    return Promise.resolve({ ok: true, value: nome });
  };
  globalThis.window = {
    guru: {
      app: { info: canal("info") },
      investimentos: Object.fromEntries(
        ["catalogo", "listar", "ler", "criar", "atualizar", "apagar"].map((n) => [n, canal(n)]),
      ),
      carteira: { resumo: canal("resumo") },
      perfil: Object.fromEntries(["questionario", "registrar", "atual"].map((n) => [n, canal(n)])),
      alocacao: Object.fromEntries(["ler", "salvar"].map((n) => [n, canal(n)])),
      objetivo: Object.fromEntries(["ler", "salvar"].map((n) => [n, canal(n)])),
      metas: Object.fromEntries(
        ["listar", "ler", "criar", "atualizar", "apagar"].map((n) => [n, canal(n)]),
      ),
      mercado: Object.fromEntries(["atualizar", "status"].map((n) => [n, canal(n)])),
    },
  };
  try {
    const { api } = await import("../../renderer/js/api.js");
    assert.equal(await api.investimentos.criar({ a: 1 }), "criar");
    assert.equal(await api.investimentos.atualizar(2, { b: 2 }), "atualizar");
    assert.equal(await api.investimentos.apagar(3), "apagar");
    assert.equal(await api.investimentos.ler(4), "ler");
    assert.equal(await api.investimentos.listar(), "listar");
    assert.equal(await api.investimentos.catalogo(), "catalogo");
    assert.equal(await api.carteira.resumo(), "resumo");
    assert.deepEqual(chamadas[1], ["atualizar", 2, { b: 2 }]);
    assert.equal(await api.perfil.questionario(), "questionario");
    assert.equal(await api.perfil.registrar([0]), "registrar");
    assert.equal(await api.perfil.atual(), "atual");
    assert.equal(await api.alocacao.ler(), "ler");
    assert.equal(await api.alocacao.salvar([]), "salvar");
    assert.equal(await api.objetivo.ler(), "ler");
    assert.equal(await api.objetivo.salvar("x"), "salvar");
    assert.equal(await api.metas.listar(), "listar");
    assert.equal(await api.metas.ler(1), "ler");
    assert.equal(await api.metas.criar({}), "criar");
    assert.equal(await api.metas.atualizar(1, {}), "atualizar");
    assert.equal(await api.metas.apagar(1), "apagar");
    assert.deepEqual(chamadas.at(-1), ["apagar", 1]);
    assert.equal(await api.mercado.atualizar(), "atualizar");
    assert.equal(await api.mercado.status(), "status");
  } finally {
    delete globalThis.window;
  }
});
