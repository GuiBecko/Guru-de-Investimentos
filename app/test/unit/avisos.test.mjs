import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { AVISOS } from "../../renderer/js/avisos.js";

const aqui = path.dirname(fileURLToPath(import.meta.url));
const renderer = path.resolve(aqui, "../../renderer");
const doc = path.resolve(aqui, "../../../docs/avisos.md");

const NOMES = [
  "RECOMENDACAO",
  "IA",
  "MERCADO",
  "PRIVACIDADE",
  "LICENCA",
  "REVISAO_JURIDICA",
  "RODAPE_CHAT",
  "RODAPE_INSIGHTS",
  "RODAPE_APORTE",
  "RODAPE_PERFIL",
];

test("AVISOS tem os dez textos, congelados", () => {
  assert.deepEqual(Object.keys(AVISOS), NOMES);
  assert.ok(Object.isFrozen(AVISOS));
  for (const n of NOMES) assert.ok(typeof AVISOS[n] === "string" && AVISOS[n].length > 0, n);
});

test("cada texto de AVISOS aparece igual numa citação de docs/avisos.md", () => {
  const citacoes = new Set(
    fs
      .readFileSync(doc, "utf8")
      .split(/\r?\n/)
      .filter((l) => l.startsWith("> "))
      .map((l) => l.slice(2)),
  );
  for (const n of NOMES) assert.ok(citacoes.has(AVISOS[n]), `${n} diferente em docs/avisos.md`);
});

function arquivosJs(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) return arquivosJs(p);
    return e.name.endsWith(".js") ? [p] : [];
  });
}

test("nenhum .js do renderer fora de avisos.js escreve “recomendação”", () => {
  const avisos = path.join(renderer, "js", "avisos.js");
  const culpados = arquivosJs(renderer).filter(
    (p) => p !== avisos && /recomendação/i.test(fs.readFileSync(p, "utf8")),
  );
  assert.deepEqual(culpados.map((p) => path.relative(renderer, p)), []);
});

test("os textos cobrem CVM, erro da IA e os destinos dos dados", () => {
  assert.match(AVISOS.RECOMENDACAO, /CVM/);
  assert.match(AVISOS.IA, /podem conter erros/);
  assert.match(AVISOS.PRIVACIDADE, /Anthropic/);
  assert.match(AVISOS.PRIVACIDADE, /GitHub/);
});

test("os rodapés do chat e dos insights avisam que pode haver erro", () => {
  assert.match(AVISOS.RODAPE_CHAT, /Pode conter erros\./);
  assert.match(AVISOS.RODAPE_INSIGHTS, /Pode conter erros\./);
});
