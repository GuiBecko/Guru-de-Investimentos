import { test } from "node:test";
import assert from "node:assert/strict";
import { analisarMarkdown } from "../../renderer/js/markdown.js";

const texto = (valor) => ({ tipo: "texto", valor });

test("parágrafos separados por linha em branco", () => {
  assert.deepEqual(analisarMarkdown("Olá!\n\nTudo bem?", []), [
    { tipo: "p", trechos: [texto("Olá!")] },
    { tipo: "p", trechos: [texto("Tudo bem?")] },
  ]);
});

test("linhas seguidas ficam no mesmo parágrafo", () => {
  assert.deepEqual(analisarMarkdown("Primeira\nsegunda", []), [{ tipo: "p", trechos: [texto("Primeira segunda")] }]);
});

test("negrito", () => {
  assert.deepEqual(analisarMarkdown("Isso é **importante** mesmo.", []), [
    { tipo: "p", trechos: [texto("Isso é "), { tipo: "negrito", valor: "importante" }, texto(" mesmo.")] },
  ]);
});

test("asteriscos sem par ficam como texto", () => {
  assert.deepEqual(analisarMarkdown("2 ** 3 e **aberto", []), [{ tipo: "p", trechos: [texto("2 ** 3 e **aberto")] }]);
});

test("lista com hífen", () => {
  assert.deepEqual(analisarMarkdown("Opções:\n\n- **Manter**: previsível.\n- Diversificar", []), [
    { tipo: "p", trechos: [texto("Opções:")] },
    {
      tipo: "ul",
      itens: [
        [{ tipo: "negrito", valor: "Manter" }, texto(": previsível.")],
        [texto("Diversificar")],
      ],
    },
  ]);
});

test("lista numerada", () => {
  assert.deepEqual(analisarMarkdown("1. Primeiro\n2. Segundo", []), [
    { tipo: "ol", itens: [[texto("Primeiro")], [texto("Segundo")]] },
  ]);
});

test("lista colada ao parágrafo começa um bloco novo", () => {
  assert.deepEqual(analisarMarkdown("Veja:\n- um", []), [
    { tipo: "p", trechos: [texto("Veja:")] },
    { tipo: "ul", itens: [[texto("um")]] },
  ]);
});

test("títulos de # a ###", () => {
  assert.deepEqual(analisarMarkdown("# Um\n## Dois\n### Três\n#### Quatro", []), [
    { tipo: "titulo", trechos: [texto("Um")] },
    { tipo: "titulo", trechos: [texto("Dois")] },
    { tipo: "titulo", trechos: [texto("Três")] },
    { tipo: "p", trechos: [texto("#### Quatro")] },
  ]);
});

test("marcador de fonte existente vira trecho fonte", () => {
  assert.deepEqual(analisarMarkdown("A Selic ficou em 15%[1][2].", [1, 2]), [
    { tipo: "p", trechos: [texto("A Selic ficou em 15%"), { tipo: "fonte", valor: 1 }, { tipo: "fonte", valor: 2 }, texto(".")] },
  ]);
});

test("marcador de fonte inexistente fica como texto", () => {
  assert.deepEqual(analisarMarkdown("Veja [3] e [1].", [1]), [
    { tipo: "p", trechos: [texto("Veja [3] e "), { tipo: "fonte", valor: 1 }, texto(".")] },
  ]);
});

test("<script> fica como texto literal", () => {
  const html = '<script>alert("x")</script> <b onclick="y">oi</b>';
  assert.deepEqual(analisarMarkdown(html, []), [{ tipo: "p", trechos: [texto(html)] }]);
});

test("texto vazio não gera bloco", () => {
  assert.deepEqual(analisarMarkdown("", []), []);
  assert.deepEqual(analisarMarkdown("\n\n  \n", []), []);
});

test("aceita um Set de números de fontes", () => {
  assert.deepEqual(analisarMarkdown("x[2]", new Set([2])), [{ tipo: "p", trechos: [texto("x"), { tipo: "fonte", valor: 2 }] }]);
});
