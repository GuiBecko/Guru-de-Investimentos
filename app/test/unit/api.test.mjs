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
  } finally {
    delete globalThis.window;
  }
});
