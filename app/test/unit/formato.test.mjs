import { test } from "node:test";
import assert from "node:assert/strict";
import { formatarReais, paraCentavos, paraTaxa, formatarData } from "../../renderer/js/formato.js";

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

test("formatarPercentual usa uma casa e vírgula, sem erro de ponto flutuante", async () => {
  const { formatarPercentual } = await import("../../renderer/js/formato.js");
  assert.equal(formatarPercentual(3800000, 4271300), "89,0%");
  assert.equal(formatarPercentual(471300, 4271300), "11,0%");
  assert.equal(formatarPercentual(1, 3), "33,3%");
  assert.equal(formatarPercentual(1, 1), "100,0%");
  assert.equal(formatarPercentual(5, 1000), "0,5%");
  assert.equal(formatarPercentual(0, 0), "0,0%");
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
