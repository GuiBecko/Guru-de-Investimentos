const test = require("node:test");
const assert = require("node:assert/strict");
const path = require("node:path");
const { executarSmoke } = require("../../main/smoke.js");

const DIR_APP = path.join("/pacote", "resources", "app.asar");
const DIR_RECURSOS = path.join("/pacote", "resources");
const DIR_ADDON = path.join(DIR_RECURSOS, "app.asar.unpacked", "node_modules", "@guru", "core-node");

// Monta as dependências falsas de um pacote saudável; cada teste estraga só o que interessa.
function deps(extra = {}) {
  const linhas = [];
  const chamadas = { abrirCore: [], listar: [] };
  const presentes = new Set([path.join(DIR_APP, "renderer", "index.html"), path.join(DIR_APP, "avisos-terceiros.txt")]);
  const base = {
    pasta: "/tmp/guru-smoke-abc",
    versaoApp: "0.9.0",
    empacotado: true,
    abrirCore: (pasta, opcoes) => {
      chamadas.abrirCore.push([pasta, opcoes]);
      return { info: () => ({ coreVersion: "0.9.0", schemaVersion: 7 }) };
    },
    existe: (caminho) => presentes.has(caminho),
    listar: (pasta) => {
      chamadas.listar.push(pasta);
      return pasta === DIR_ADDON ? ["guru-core-node.linux-x64-gnu.node"] : [];
    },
    dirApp: DIR_APP,
    dirRecursos: DIR_RECURSOS,
    plataforma: "linux",
    arquitetura: "x64",
    log: (linha) => linhas.push(linha),
  };
  return { opcoes: { ...base, ...extra }, linhas, chamadas, presentes };
}

test("smoke_ok", async () => {
  const { opcoes, linhas } = deps();
  assert.equal(await executarSmoke(opcoes), 0);
  assert.deepEqual(linhas, ["GURU_SMOKE_OK app=0.9.0 nucleo=0.9.0 esquema=7"]);
});

test("smoke_usa_pasta_temporaria_e_cofre_em_memoria", async () => {
  const { opcoes, chamadas } = deps();
  await executarSmoke(opcoes);
  assert.equal(chamadas.abrirCore.length, 1);
  assert.equal(chamadas.abrirCore[0][0], "/tmp/guru-smoke-abc");
  assert.deepEqual(chamadas.abrirCore[0][1], { cofreMemoria: true });
});

test("addon_que_nao_carrega", async () => {
  const { opcoes, linhas } = deps({
    abrirCore: () => {
      throw new Error("Cannot find module '@guru/core-node'");
    },
  });
  assert.equal(await executarSmoke(opcoes), 1);
  assert.equal(linhas.length, 1);
  assert.match(linhas[0], /^GURU_SMOKE_FALHOU ADDON_INDISPONIVEL: /);
});

test("arquivo_do_app_faltando", async () => {
  const { opcoes, linhas, presentes } = deps();
  presentes.delete(path.join(DIR_APP, "avisos-terceiros.txt"));
  assert.equal(await executarSmoke(opcoes), 1);
  assert.match(linhas[0], /^GURU_SMOKE_FALHOU PACOTE_INCOMPLETO: .*avisos-terceiros\.txt/);
});

test("pacote_com_codigo_do_addon", async () => {
  const { opcoes, linhas, presentes } = deps();
  presentes.add(path.join(DIR_APP, "node_modules", "@guru", "core-node", "src"));
  assert.equal(await executarSmoke(opcoes), 1);
  assert.match(linhas[0], /^GURU_SMOKE_FALHOU PACOTE_INCOMPLETO: /);
});

test("pacote_com_testes_do_addon", async () => {
  const { opcoes, linhas, presentes } = deps();
  presentes.add(path.join(DIR_APP, "node_modules", "@guru", "core-node", "test"));
  assert.equal(await executarSmoke(opcoes), 1);
  assert.match(linhas[0], /^GURU_SMOKE_FALHOU PACOTE_INCOMPLETO: /);
});

test("pacote_sem_node_da_arquitetura", async () => {
  const { opcoes, linhas } = deps({
    plataforma: "darwin",
    arquitetura: "arm64",
    listar: () => ["guru-core-node.darwin-x64.node"],
  });
  assert.equal(await executarSmoke(opcoes), 1);
  assert.match(linhas[0], /^GURU_SMOKE_FALHOU PACOTE_INCOMPLETO: /);
});

test("pacote_sem_pasta_do_addon", async () => {
  const { opcoes, linhas } = deps({
    listar: () => {
      throw Object.assign(new Error("ENOENT"), { code: "ENOENT" });
    },
  });
  assert.equal(await executarSmoke(opcoes), 1);
  assert.match(linhas[0], /^GURU_SMOKE_FALHOU PACOTE_INCOMPLETO: /);
});

test("nucleo_com_outra_versao", async () => {
  const { opcoes, linhas } = deps({
    abrirCore: () => ({ info: () => ({ coreVersion: "0.8.0", schemaVersion: 7 }) }),
  });
  assert.equal(await executarSmoke(opcoes), 1);
  assert.match(linhas[0], /^GURU_SMOKE_FALHOU VERSAO_DIVERGENTE: /);
});

test("fora_do_pacote_nao_confere_o_asar", async () => {
  const { opcoes, linhas, chamadas } = deps({ empacotado: false });
  assert.equal(await executarSmoke(opcoes), 0);
  assert.equal(chamadas.listar.length, 0);
  assert.deepEqual(linhas, ["GURU_SMOKE_OK app=0.9.0 nucleo=0.9.0 esquema=7"]);
});
