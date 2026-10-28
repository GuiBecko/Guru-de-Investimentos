import test from "node:test";
import assert from "node:assert/strict";
import {
  formatarTaxa,
  legendaRendimento,
  montarDados,
} from "../../renderer/js/telas/formulario-investimento/dados.js";

const CDB = { codigo: "rf-cdb", forma: "renda_fixa", exigeEmissor: true, irPadrao: "regressiva", irEditavel: false };
const TESOURO = { codigo: "td-ipca", forma: "renda_fixa", exigeEmissor: false, indexadorForcado: "ipca", liquidezForcada: "d1", irPadrao: "regressiva", irEditavel: false };
const ACAO = { codigo: "rv-acao", forma: "renda_variavel" };
const FUNDO = { codigo: "fundo", forma: "valor", exigeEmissor: true };

test("formatarTaxa usa vírgula e tira zeros à direita", () => {
  assert.equal(formatarTaxa("102"), "102");
  assert.equal(formatarTaxa("102.50"), "102,5");
  assert.equal(formatarTaxa("0.0905"), "0,0905");
  assert.equal(formatarTaxa("012.0"), "12");
  assert.equal(formatarTaxa(""), null);
  assert.equal(formatarTaxa("1,5"), null);
});

test("legendaRendimento segue o texto do núcleo", () => {
  assert.equal(legendaRendimento("pos", "102", "CDI"), "102% do CDI");
  assert.equal(legendaRendimento("pre", "12.5", "CDI"), "12,5% ao ano");
  assert.equal(legendaRendimento("ipca", "6.2", "CDI"), "IPCA + 6,2% ao ano");
  assert.equal(legendaRendimento("selic", "0.0905", "CDI"), "Selic + 0,0905% ao ano");
  assert.equal(legendaRendimento("pos", "", "CDI"), "");
});

const base = {
  instituicao: "Banco Inter", emissor: "Banco Inter", ticker: "ITUB4", indexador: "pos", taxa: "102",
  benchmark: "cdi", dataAplicacao: "2025-03-15", vencimento: "2028-03-15", valor: "20.000,00",
  quantidade: "150", preco: "31,42", liquidez: "vencimento", dataCarencia: "2026-01-01", ir: "regressiva", observacao: "",
};

test("CDB leva só os campos de renda fixa, em centavos", () => {
  const { dados, erros } = montarDados(CDB, base);
  assert.deepEqual(erros, []);
  assert.deepEqual(dados, {
    tipo: "rf-cdb", instituicao: "Banco Inter", emissor: "Banco Inter", indexador: "pos", taxa: "102",
    benchmark: "cdi", dataAplicacao: "2025-03-15", vencimento: "2028-03-15",
    valorAplicadoCentavos: 2000000, liquidez: "vencimento", ir: "regressiva",
  });
});

test("benchmark só vai com pós-fixado e carência só com liquidez carência", () => {
  const { dados } = montarDados(CDB, { ...base, indexador: "pre", liquidez: "carencia" });
  assert.equal("benchmark" in dados, false);
  assert.equal(dados.dataCarencia, "2026-01-01");
});

test("o emissor segue exigeEmissor do catálogo, não a garantia nem o indexador", () => {
  assert.equal("emissor" in montarDados({ ...CDB, exigeEmissor: false }, base).dados, false);
  assert.equal("emissor" in montarDados({ ...TESOURO, exigeEmissor: true }, base).dados, true);
});

test("Tesouro manda os valores forçados e nenhum emissor", () => {
  const { dados } = montarDados(TESOURO, { ...base, indexador: "pos", liquidez: "diaria" });
  assert.equal(dados.indexador, "ipca");
  assert.equal(dados.liquidez, "d1");
  assert.equal("emissor" in dados, false);
  assert.equal("benchmark" in dados, false);
});

test("ação leva ticker, quantidade e preço e nada de renda fixa", () => {
  const { dados } = montarDados(ACAO, base);
  assert.deepEqual(dados, {
    tipo: "rv-acao", instituicao: "Banco Inter", dataAplicacao: "2025-03-15",
    ticker: "ITUB4", quantidade: 150, precoMedioCentavos: 3142,
  });
});

test("fundo leva nome, valor e liquidez, sem taxa nem IR", () => {
  const { dados } = montarDados(FUNDO, base);
  assert.equal(dados.emissor, "Banco Inter");
  assert.equal(dados.valorAplicadoCentavos, 2000000);
  for (const c of ["taxa", "indexador", "ir", "ticker"]) assert.equal(c in dados, false);
});

test("valor mal digitado vira erro de formato e não 0", () => {
  const { dados, erros } = montarDados(CDB, { ...base, valor: "1,234,56" });
  assert.equal("valorAplicadoCentavos" in dados, false);
  assert.equal(erros[0].campo, "valorAplicadoCentavos");
});

test("campos vazios são omitidos para o núcleo reclamar", () => {
  const { dados, erros } = montarDados(CDB, { ...base, emissor: "", valor: "", taxa: "" });
  assert.deepEqual(erros, []);
  for (const c of ["emissor", "valorAplicadoCentavos", "taxa"]) assert.equal(c in dados, false);
});
