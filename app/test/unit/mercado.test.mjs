import { test } from "node:test";
import assert from "node:assert/strict";
import { menorData, diaMes, textoIndicador, precisaAtualizar } from "../../renderer/js/mercado.js";

const fontes = [
  { fonte: "cdi", rotulo: "CDI", ultimaData: "2026-10-28" },
  { fonte: "selic", rotulo: "Selic", ultimaData: "2026-10-27" },
  { fonte: "ipca", rotulo: "IPCA", ultimaData: "2026-09-01" },
  { fonte: "tesouro", rotulo: "Tesouro Direto" },
];

test("menorData ignora o IPCA mensal e as fontes sem dados", () => {
  assert.equal(menorData(fontes), "2026-10-27");
  assert.equal(menorData([{ fonte: "ipca", ultimaData: "2026-09-01" }]), null);
  assert.equal(menorData([]), null);
});

test("textos do indicador", () => {
  assert.equal(diaMes("2026-10-28"), "28/10");
  assert.equal(textoIndicador(fontes), "Dados de mercado de 27/10");
  assert.equal(textoIndicador([]), "Dados de mercado: ainda não baixados");
});

test("precisaAtualizar compara com o dia local", () => {
  const agora = new Date(2026, 9, 30, 12, 0, 0);
  const hoje = new Date(2026, 9, 30, 8, 0, 0).toISOString();
  const ontem = new Date(2026, 9, 29, 20, 0, 0).toISOString();
  assert.equal(precisaAtualizar([], agora), true);
  assert.equal(precisaAtualizar([{ atualizadoEm: hoje }, { atualizadoEm: hoje }], agora), false);
  assert.equal(precisaAtualizar([{ atualizadoEm: hoje }, { atualizadoEm: ontem }], agora), true);
  assert.equal(precisaAtualizar([{ atualizadoEm: hoje }, {}], agora), true);
});
