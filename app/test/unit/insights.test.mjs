import { test } from "node:test";
import assert from "node:assert/strict";
import { acaoDoInsight, rotuloGerado } from "../../renderer/js/telas/carteira/insights.js";

const insight = (acao, classe = null, classeRotulo = null) => ({ tipo: "meta", rotulo: "Meta", texto: "x", acao, classe, classeRotulo });

test("acaoDoInsight aporte leva ao Próximo aporte", () => {
  assert.deepEqual(acaoDoInsight(insight("aporte"), 0), { texto: "Simular próximo aporte →", href: "#/aporte" });
});

test("acaoDoInsight meta leva ao Perfil", () => {
  assert.deepEqual(acaoDoInsight(insight("meta"), 1), { texto: "Ver metas →", href: "#/perfil" });
});

test("acaoDoInsight classe leva ao detalhe da classe com o rótulo", () => {
  assert.deepEqual(acaoDoInsight(insight("classe", "acoes", "Ações"), 3), { texto: "Ver Ações →", href: "#/carteira/acoes" });
});

test("acaoDoInsight conversar leva ao Assistente com o índice", () => {
  assert.deepEqual(acaoDoInsight(insight("conversar"), 2), { texto: "Conversar sobre isso →", href: "#/assistente/insight/2" });
});

test("acaoDoInsight nenhuma não tem link", () => {
  assert.equal(acaoDoInsight(insight("nenhuma"), 4), null);
});

test("acaoDoInsight sempre devolve href interno", () => {
  const casos = [insight("aporte"), insight("meta"), insight("classe", "fii", "FIIs"), insight("conversar"), insight("nenhuma")];
  casos.forEach((i, n) => {
    const a = acaoDoInsight(i, n);
    if (a) assert.match(a.href, /^#\//);
  });
  // Uma classe estranha não escapa do endereço interno.
  const estranha = acaoDoInsight(insight("classe", "https://x.com", "X"), 0);
  assert.ok(estranha === null || estranha.href.startsWith("#/"));
});

test("rotuloGerado diz hoje no mesmo dia e dd/mm em outro", () => {
  const hoje = new Date(2026, 10, 3, 15, 0);
  assert.equal(rotuloGerado("2026-11-03", hoje), "Claude · hoje");
  assert.equal(rotuloGerado("2026-10-28", hoje), "Claude · 28/10");
});
