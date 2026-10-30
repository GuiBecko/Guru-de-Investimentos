import { test } from "node:test";
import assert from "node:assert/strict";
import { saudacao, modelosAplicaveis } from "../../renderer/js/saudacao.js";

const vazio = {
  numeros: { mesCentesimos: null, mesCdiCentesimos: null, rendeuCentavos: 0 },
  variacoes: [],
  vencimentos: [],
};
const hoje = new Date(2026, 9, 28, 9, 0);

test("cumprimento pela hora", () => {
  const f = (h) => saudacao({ agora: new Date(2026, 9, 28, h, 0), painel: vazio, sorteio: () => 0 }).frase;
  assert.equal(f(9), "Bom dia. Bom ver você por aqui.");
  assert.equal(f(5), "Bom dia. Bom ver você por aqui.");
  assert.ok(f(13).startsWith("Boa tarde. "));
  assert.ok(f(12).startsWith("Boa tarde. "));
  assert.ok(f(22).startsWith("Boa noite. "));
  assert.ok(f(3).startsWith("Boa noite. "));
});

test("data por extenso", () => {
  const { data } = saudacao({ agora: new Date(2026, 9, 28, 9), painel: vazio, sorteio: () => 0 });
  assert.equal(data, "quarta, 28 de outubro de 2026");
});

test("modelos condicionais", () => {
  assert.equal(modelosAplicaveis(vazio, hoje).length, 4);
  const acima = { ...vazio, numeros: { ...vazio.numeros, mesCentesimos: 200, mesCdiCentesimos: 90 } };
  assert.ok(modelosAplicaveis(acima, hoje).includes("Seu mês está acima do CDI até agora."));
  const abaixo = { ...vazio, numeros: { ...vazio.numeros, mesCentesimos: 10, mesCdiCentesimos: 90 } };
  assert.ok(modelosAplicaveis(abaixo, hoje).includes("O mês está abaixo do CDI até agora. Meses assim fazem parte."));
  const nulo = { ...vazio, numeros: { ...vazio.numeros, mesCentesimos: 10, mesCdiCentesimos: null } };
  assert.equal(modelosAplicaveis(nulo, hoje).length, 4);
  const destaque = { ...vazio, variacoes: [{ ticker: "PETR4", variacaoDecimos: 123 }, { ticker: "X", variacaoDecimos: -5 }] };
  assert.ok(modelosAplicaveis(destaque, hoje).includes("PETR4 é o destaque do mês, com +12,3%."));
  const queda = { ...vazio, variacoes: [{ ticker: "X", variacaoDecimos: -5 }] };
  assert.equal(modelosAplicaveis(queda, hoje).length, 4);
  const venc = { ...vazio, vencimentos: [{ id: 1, nome: "CDB Banco", vencimento: "2026-11-27", atualCentavos: 1 }] };
  assert.ok(modelosAplicaveis(venc, hoje).includes("CDB Banco vence em 27/11."));
  const longe = { ...vazio, vencimentos: [{ id: 1, nome: "CDB Banco", vencimento: "2026-11-28", atualCentavos: 1 }] };
  assert.equal(modelosAplicaveis(longe, hoje).length, 4);
  const rendeu = { ...vazio, numeros: { ...vazio.numeros, rendeuCentavos: 123456 } };
  assert.ok(modelosAplicaveis(rendeu, hoje).includes("Seus investimentos renderam R$ 1.234,56 no período."));
});

test("sorteio escolhe entre os aplicáveis", () => {
  const acima = { ...vazio, numeros: { ...vazio.numeros, mesCentesimos: 200, mesCdiCentesimos: 90 } };
  const { frase } = saudacao({ agora: hoje, painel: acima, sorteio: () => 0.999 });
  assert.equal(frase, "Bom dia. Seu mês está acima do CDI até agora.");
});
