import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";
const { Core } = createRequire(import.meta.url)("../index.js");

const pastaIa = resolve(import.meta.dirname, "../../guru-core/tests/fixtures/ia");
const CHAVE = "sk-ant-api03-chave-de-teste-4f2a";
const abrir = () => Core.open(mkdtempSync(join(tmpdir(), "guru-")), { pastaIa, cofreMemoria: true });

const esperar = async (core, pedido) => {
  for (let i = 0; i < 500; i++) {
    const p = core.iaProgresso(pedido);
    if (p.terminado) return p;
    await new Promise((r) => setTimeout(r, 10));
  }
  throw new Error("pedido não terminou");
};

const codigo = (f) => {
  try {
    f();
  } catch (e) {
    return e.code ?? e.status;
  }
  return null;
};

test("salvar chave valida mascara e nunca devolve a chave inteira", async () => {
  const core = abrir();
  assert.equal(core.configIa().chave, null);
  assert.deepEqual(await core.salvarChaveIa(CHAVE), []);
  const config = core.configIa();
  assert.equal(config.chave, "sk-ant-…4f2a");
  assert.equal(config.modelos.length, 3);
  const retornos = [
    config,
    core.contextoIa(),
    core.conversas(),
    await core.salvarChaveIa(CHAVE),
    core.salvarConfigIa({ modelo: config.modelo, instrucoes: "x", apenasPercentuais: true, nomesInstituicoes: false, buscaWeb: true }),
  ];
  const { pedido } = core.perguntarIa(null, "Oi");
  retornos.push(core.iaProgresso(pedido), await esperar(core, pedido).catch(() => null), core.conversas());
  for (const r of retornos) assert.ok(!JSON.stringify(r).includes(CHAVE));
});

test("chave recusada e mal formada voltam como erro de campo", async () => {
  const core = abrir();
  const [e1] = await core.salvarChaveIa("sk-ant-api03-recusada-xxxxxxxx");
  assert.equal(e1.campo, "chave");
  const [e2] = await core.salvarChaveIa("abc");
  assert.equal(e2.campo, "chave");
  assert.equal(core.configIa().chave, null);
  core.removerChaveIa();
});

test("remover a chave limpa a máscara", async () => {
  const core = abrir();
  await core.salvarChaveIa(CHAVE);
  core.removerChaveIa();
  assert.equal(core.configIa().chave, null);
});

test("perguntar sem chave termina com IA_SEM_CHAVE", async () => {
  const core = abrir();
  const { pedido } = core.perguntarIa(null, "Oi");
  assert.equal((await esperar(core, pedido)).erro.code, "IA_SEM_CHAVE");
});

test("perguntarIa devolve já e o polling termina com texto e conversaId", async () => {
  const core = abrir();
  await core.salvarChaveIa(CHAVE);
  const r = core.perguntarIa(null, "Como está a carteira?");
  assert.equal(typeof r.pedido, "number");
  const p = await esperar(core, r.pedido);
  assert.equal(p.erro, null);
  assert.ok(p.texto.length > 0);
  assert.equal(typeof p.conversaId, "number");
  // Lido como terminado, o pedido sai do mapa.
  assert.equal(codigo(() => core.iaProgresso(r.pedido)), "NAO_ENCONTRADO");
  const c = core.conversa(p.conversaId);
  assert.equal(c.mensagens.length, 2);
  assert.equal(core.conversas().length, 1);
  // Segunda pergunta na mesma conversa.
  const p2 = await esperar(core, core.perguntarIa(p.conversaId, "E agora?").pedido);
  assert.equal(p2.conversaId, p.conversaId);
  assert.equal(core.conversa(p.conversaId).mensagens.length, 4);
  core.apagarConversa(p.conversaId);
  assert.equal(core.conversas().length, 0);
});

test("#erro401 termina com IA_CHAVE_RECUSADA", async () => {
  const core = abrir();
  await core.salvarChaveIa(CHAVE);
  const p = await esperar(core, core.perguntarIa(null, "#erro401 Oi").pedido);
  assert.equal(p.erro.code, "IA_CHAVE_RECUSADA");
  assert.equal(p.conversaId, null);
});

test("cancelarIa termina com IA_CANCELADA", async () => {
  const core = abrir();
  await core.salvarChaveIa(CHAVE);
  const { pedido } = core.perguntarIa(null, "Oi");
  core.cancelarIa(pedido);
  assert.equal((await esperar(core, pedido)).erro.code, "IA_CANCELADA");
  assert.equal(core.conversas().length, 0);
});

test("pedido desconhecido é NAO_ENCONTRADO e cancelar nele não faz nada", () => {
  const core = abrir();
  assert.equal(codigo(() => core.iaProgresso(999)), "NAO_ENCONTRADO");
  core.cancelarIa(999);
});

test("config e contexto: salvar, validar e editar partes", () => {
  const core = abrir();
  const c = core.configIa();
  const dados = { modelo: c.modelo, instrucoes: "Seja breve.", apenasPercentuais: true, nomesInstituicoes: true, buscaWeb: false };
  assert.deepEqual(core.salvarConfigIa(dados), []);
  assert.equal(core.configIa().instrucoes, "Seja breve.");
  assert.equal(core.salvarConfigIa({ ...dados, modelo: "xxx" })[0].campo, "modelo");
  const ctx = core.contextoIa();
  assert.ok(ctx.partes.length > 0 && ctx.tokens > 0);
  const parte = ctx.partes[0].parte;
  assert.deepEqual(core.salvarParteContexto(parte, false, null), []);
  assert.equal(core.contextoIa().partes[0].ligada, false);
  assert.equal(codigo(() => core.conversa(12345)), "NAO_ENCONTRADO");
});
