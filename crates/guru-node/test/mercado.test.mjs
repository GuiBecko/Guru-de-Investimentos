import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";
const { Core } = createRequire(import.meta.url)("../index.js");

const pastaFixtures = resolve(import.meta.dirname, "../../guru-core/tests/fixtures");
const abrir = () =>
  Core.open(mkdtempSync(join(tmpdir(), "guru-")), { pastaFixtures, hoje: "2026-10-28" });

const cdb = {
  tipo: "rf-cdb",
  instituicao: "Banco Inter",
  emissor: "Banco Inter",
  indexador: "pos",
  taxa: "100",
  benchmark: "cdi",
  dataAplicacao: "2026-08-31",
  valorAplicadoCentavos: 1000000,
  liquidez: "diaria",
  ir: "regressiva",
};

test("statusMercado lista as cinco fontes com rótulos, ainda sem dados", () => {
  const status = abrir().statusMercado();
  assert.deepEqual(
    status.map((s) => [s.fonte, s.rotulo]),
    [
      ["cdi", "CDI"],
      ["selic", "Selic"],
      ["ipca", "IPCA"],
      ["tesouro", "Tesouro Direto"],
      ["b3", "Cotações da B3"],
    ],
  );
  assert.equal(status[0].ultimaData, undefined);
});

test("atualizarMercado resolve com 5 fontes e o CDB passa a ter valor calculado", async () => {
  const core = abrir();
  const r = await core.atualizarMercado();
  assert.equal(r.fontes.length, 5);
  assert.ok(r.novos > 0);
  assert.equal(r.fontes[0].ultimaData, "2026-10-28");
  const criado = core.criarInvestimento(cdb);
  assert.deepEqual(criado.erros, []);
  const i = core.investimento(criado.id);
  assert.equal(i.origemValor, "calculo");
  assert.equal(i.origemValorRotulo, "calculado pelo índice");
  assert.equal(i.dataValor, "2026-10-28");
  assert.ok(i.valorAtualCentavos > i.valorAplicadoCentavos);
  assert.ok(i.rentabilidadeDecimos > 0);
  assert.equal(core.resumoCarteira().totalAtualCentavos, i.valorAtualCentavos);
  assert.equal(core.listarInvestimentos()[0].valorAtualCentavos, i.valorAtualCentavos);
  assert.equal(core.statusMercado()[0].ultimaData, "2026-10-28");
});

test("atualizar de novo não grava linhas repetidas", async () => {
  const core = abrir();
  await core.atualizarMercado();
  const segunda = await core.atualizarMercado();
  assert.equal(segunda.novos, 0);
});

test("hoje inválido, ou sem pastaFixtures, é recusado", () => {
  const dir = mkdtempSync(join(tmpdir(), "guru-"));
  assert.throws(() => Core.open(dir, { pastaFixtures, hoje: "28/10/2026" }), {
    code: "MERCADO_CONFIG_INVALIDA",
  });
  assert.throws(() => Core.open(dir, { hoje: "2026-10-28" }), {
    code: "MERCADO_CONFIG_INVALIDA",
  });
});

test("falha de banco na atualização rejeita com o code do núcleo", async () => {
  const dir = mkdtempSync(join(tmpdir(), "guru-"));
  const core = Core.open(dir, { pastaFixtures, hoje: "2026-10-28" });
  // Sem o -wal/-shm da conexão aberta, o arquivo de texto é tudo o que a tarefa encontra.
  for (const f of readdirSync(dir)) rmSync(join(dir, f));
  writeFileSync(join(dir, "guru.db"), "isto não é um banco");
  await assert.rejects(core.atualizarMercado(), { code: "DB_INVALID" });
});
