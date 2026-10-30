import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { crc32 } from "node:zlib";
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
  assert.throws(
    () => c.aplicarImportacao(r.token, [{ chave, acao: "xyz" }]),
    { code: "DECISAO_INVALIDA" },
  );
  assert.throws(
    () => c.aplicarImportacao(r.token, [{ chave, acao: "vincular" }]),
    { code: "DECISAO_INVALIDA" },
  );
});

test("aplicar consome o token: a segunda chamada dá IMPORTACAO_EXPIRADA e não duplica", () => {
  const c = abrir();
  const r = c.compararImportacao(fixture("posicao-completa.xlsx"), "p.xlsx");
  const decisoes = r.itens.map((i) => ({ chave: i.chave, acao: "ignorar" }));
  assert.deepEqual(c.aplicarImportacao(r.token, decisoes).erros, []);
  assert.throws(() => c.aplicarImportacao(r.token, decisoes), { code: "IMPORTACAO_EXPIRADA" });
});

test("aplicar com erros de campo mantém o token para tentar de novo", () => {
  const c = abrir();
  const r = c.compararImportacao(fixture("posicao-completa.xlsx"), "p.xlsx");
  const d = [{ chave: r.itens[0].chave, acao: "adicionar" }];
  assert.ok(c.aplicarImportacao(r.token, d).erros.length > 0);
  assert.ok(c.aplicarImportacao(r.token, d).erros.length > 0);
});

test("token velho com decisão inválida ainda dá IMPORTACAO_EXPIRADA", () => {
  const c = abrir();
  assert.throws(() => c.aplicarImportacao("velho", [{ chave: "x", acao: "xyz" }]), {
    code: "IMPORTACAO_EXPIRADA",
  });
});

// `.xlsx` mínimo (zip sem compressão) com uma aba de células texto ou número (prefixo `#`).
function xlsx(aba, linhas) {
  const xml = (c) => (c.startsWith("#") ? `<c><v>${c.slice(1)}</v></c>` : `<c t="str"><v>${c}</v></c>`);
  const partes = [
    ["xl/workbook.xml", `<workbook xmlns:r="x"><sheets><sheet name="${aba}" sheetId="1" r:id="rId1"/></sheets></workbook>`],
    ["xl/_rels/workbook.xml.rels", `<Relationships><Relationship Id="rId1" Type="x/worksheet" Target="worksheets/s1.xml"/></Relationships>`],
    ["xl/worksheets/s1.xml", `<worksheet><sheetData>${linhas.map((l) => `<row>${l.map(xml).join("")}</row>`).join("")}</sheetData></worksheet>`],
  ];
  const locais = [];
  const central = [];
  let pos = 0;
  for (const [nome, texto] of partes) {
    const n = Buffer.from(nome);
    const d = Buffer.from(texto);
    const cab = Buffer.alloc(30);
    cab.writeUInt32LE(0x04034b50, 0);
    cab.writeUInt16LE(20, 4);
    cab.writeUInt32LE(crc32(d), 14);
    cab.writeUInt32LE(d.length, 18);
    cab.writeUInt32LE(d.length, 22);
    cab.writeUInt16LE(n.length, 26);
    const cen = Buffer.alloc(46);
    cen.writeUInt32LE(0x02014b50, 0);
    cen.writeUInt16LE(20, 4);
    cen.writeUInt16LE(20, 6);
    cen.writeUInt32LE(crc32(d), 16);
    cen.writeUInt32LE(d.length, 20);
    cen.writeUInt32LE(d.length, 24);
    cen.writeUInt16LE(n.length, 28);
    cen.writeUInt32LE(pos, 42);
    locais.push(cab, n, d);
    central.push(cen, n);
    pos += cab.length + n.length + d.length;
  }
  const dir = Buffer.concat(central);
  const fim = Buffer.alloc(22);
  fim.writeUInt32LE(0x06054b50, 0);
  fim.writeUInt16LE(partes.length, 8);
  fim.writeUInt16LE(partes.length, 10);
  fim.writeUInt32LE(dir.length, 12);
  fim.writeUInt32LE(pos, 16);
  return Buffer.concat([...locais, dir, fim]);
}

test("valor enorme na planilha vira aviso em vez de derrubar o processo", () => {
  const c = abrir();
  const bytes = xlsx("Tesouro Direto", [
    ["Produto", "Instituição", "Vencimento", "Valor Aplicado"],
    ["Tesouro Selic 2029", "Rico", "01/03/2029", "#79228162514264337593543950335"],
    ["Tesouro Selic 2027", "Rico", "01/03/2027", "#1000"],
  ]);
  const r = c.compararImportacao(bytes, "hostil.xlsx");
  assert.deepEqual(r.avisos, ["Valor fora do limite em Tesouro Selic 2029."]);
  assert.deepEqual(r.itens.map((i) => i.ativo), ["Tesouro Selic 2027"]);
});

test("comparação que falha descarta a anterior: o token antigo expira", () => {
  const c = abrir();
  const a = c.compararImportacao(fixture("posicao-completa.xlsx"), "a.xlsx");
  assert.throws(() => c.compararImportacao(Buffer.from("isto não é xlsx"), "x.xlsx"), {
    code: "ARQUIVO_INVALIDO",
  });
  assert.throws(() => c.aplicarImportacao(a.token, []), { code: "IMPORTACAO_EXPIRADA" });
});
