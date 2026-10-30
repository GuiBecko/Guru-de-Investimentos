import { test } from "node:test";
import assert from "node:assert/strict";
import { formatarReais, formatoDeReais, paraCentavos, paraTaxa, formatarData } from "../../renderer/js/formato.js";

test("formatarReais", () => {
  assert.equal(formatarReais(2000000), "R$ 20.000,00");
  assert.equal(formatarReais(1), "R$ 0,01");
  assert.equal(formatarReais(0), "R$ 0,00");
  assert.equal(formatarReais(123456789), "R$ 1.234.567,89");
});

test("paraCentavos aceita formatos brasileiros", () => {
  assert.equal(paraCentavos("20.000,00"), 2000000);
  assert.equal(paraCentavos("20000"), 2000000);
  assert.equal(paraCentavos("20000,5"), 2000050);
  assert.equal(paraCentavos("R$ 1.234,56"), 123456);
  assert.equal(paraCentavos("R$ 1.234,5"), 123450);
  assert.equal(paraCentavos("0,01"), 1);
  assert.equal(paraCentavos("  15,00  "), 1500);
  assert.equal(paraCentavos("1.234"), 123400);
});

test("paraCentavos recusa o ambíguo ou inválido", () => {
  for (const t of ["", "   ", "abc", "-5", "1,234,56", "1.2.3", "1.23", "1,234", "10,", ",5", "1e3", "1.0000,00"]) {
    assert.equal(paraCentavos(t), null, `"${t}"`);
  }
});

test("paraCentavos recusa acima do inteiro seguro", () => {
  assert.equal(paraCentavos("90.071.992.547.409,91"), Number.MAX_SAFE_INTEGER);
  assert.equal(paraCentavos("90.071.992.547.409,92"), null);
  assert.equal(paraCentavos("99999999999999999999"), null);
});

test("formatarData", () => {
  assert.equal(formatarData("2028-03-15"), "15/03/2028");
  assert.equal(formatarData(null), "—");
  assert.equal(formatarData(""), "—");
  assert.equal(formatarData(undefined), "—");
});

test("formatarDecimos usa uma casa e vírgula a partir de inteiros", async () => {
  const { formatarDecimos } = await import("../../renderer/js/formato.js");
  assert.equal(formatarDecimos(890), "89,0%");
  assert.equal(formatarDecimos(333), "33,3%");
  assert.equal(formatarDecimos(1000), "100,0%");
  assert.equal(formatarDecimos(5), "0,5%");
  assert.equal(formatarDecimos(0), "0,0%");
});

test("formatarDiferenca dá o sinal, uma casa e 'no alvo' abaixo de meio ponto", async () => {
  const { formatarDiferenca } = await import("../../renderer/js/formato.js");
  assert.equal(formatarDiferenca(520, 45), "+7,0 p.p.");
  assert.equal(formatarDiferenca(370, 45), "\u22128,0 p.p.");
  assert.equal(formatarDiferenca(0, 15), "\u221215,0 p.p.");
  assert.equal(formatarDiferenca(452, 45), "no alvo");
  assert.equal(formatarDiferenca(446, 45), "no alvo");
  assert.equal(formatarDiferenca(455, 45), "+0,5 p.p.");
  assert.equal(formatarDiferenca(445, 45), "\u22120,5 p.p.");
});

test("formatarPrazo usa mês abreviado em minúsculas", async () => {
  const { formatarPrazo } = await import("../../renderer/js/formato.js");
  assert.equal(formatarPrazo("2030-12"), "dez/2030");
  assert.equal(formatarPrazo("2031-01"), "jan/2031");
});

test("paraTaxa aceita vírgula ou ponto e devolve texto com ponto", () => {
  assert.equal(paraTaxa("102"), "102");
  assert.equal(paraTaxa("102,5"), "102.5");
  assert.equal(paraTaxa("102.5"), "102.5");
  assert.equal(paraTaxa("0,0905"), "0.0905");
  assert.equal(paraTaxa("0.0905"), "0.0905");
  assert.equal(paraTaxa("  12,5  "), "12.5");
});

test("paraTaxa recusa o que é ambíguo ou não é número", () => {
  for (const t of [",5", "5,", ".5", "5.", "1.000,5", "1,2,3", "1.2.3", "-5", "+5", "1e3", "abc", "5%", "1 2", "", "  ", undefined]) {
    assert.equal(paraTaxa(t), null, JSON.stringify(t));
  }
});

test("formatarRentabilidade", async () => {
  const { formatarRentabilidade } = await import("../../renderer/js/formato.js");
  assert.equal(formatarRentabilidade(171), "+17,1%");
  assert.equal(formatarRentabilidade(5), "+0,5%");
  assert.equal(formatarRentabilidade(0), "0,0%");
  assert.equal(formatarRentabilidade(-32), "−3,2%");
  assert.equal(formatarRentabilidade(-5), "−0,5%");
});

test("formatarCentesimos", async () => {
  const { formatarCentesimos } = await import("../../renderer/js/formato.js");
  assert.equal(formatarCentesimos(107), "+1,07%");
  assert.equal(formatarCentesimos(-2350), "−23,5%");
  assert.equal(formatarCentesimos(0), "0,00%");
  assert.equal(formatarCentesimos(null), "—");
  assert.equal(formatarCentesimos(-5), "−0,05%");
  assert.equal(formatarCentesimos(999), "+9,99%");
  assert.equal(formatarCentesimos(1005), "+10,1%");
});

test("formatadores arredondam entrada fracionária", async () => {
  const { formatarCentesimos, formatarReais } = await import("../../renderer/js/formato.js");
  assert.equal(formatarCentesimos(166.666666666667), "+1,67%");
  assert.equal(formatarCentesimos(-1666.67), "−16,7%");
  assert.equal(formatarCentesimos(NaN), "—");
  assert.equal(formatarReais(1234.5), "R$ 12,35");
  assert.equal(formatarReais(100.4), "R$ 1,00");
  assert.equal(formatarReais(-166.67), "-R$ 1,67");
});

test("formatoDeReais reconhece o formato mesmo quando o valor passa do limite", () => {
  assert.equal(formatoDeReais("2.000,00"), true);
  assert.equal(formatoDeReais("R$ 50"), true);
  assert.equal(formatoDeReais("99999999999999999999"), true);
  assert.equal(paraCentavos("99999999999999999999"), null);
  assert.equal(formatoDeReais("abc"), false);
  assert.equal(formatoDeReais("1.00"), false);
  assert.equal(formatoDeReais(""), false);
});
