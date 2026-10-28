// Dinheiro sempre em centavos inteiros; nada de ponto flutuante.

export function formatarReais(centavos) {
  const negativo = centavos < 0;
  const abs = Math.abs(centavos);
  const reais = Math.trunc(abs / 100);
  const resto = String(abs % 100).padStart(2, "0");
  const milhares = String(reais).replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return `${negativo ? "-" : ""}R$ ${milhares},${resto}`;
}

const COM_MILHAR = /^\d{1,3}(\.\d{3})+(,\d{1,2})?$/;
const SIMPLES = /^\d+(,\d{1,2})?$/;

// Texto digitado em reais -> centavos inteiros, ou null se vazio, ambíguo ou fora do limite.
export function paraCentavos(texto) {
  const t = String(texto).trim().replace(/^R\$\s*/, "");
  if (!COM_MILHAR.test(t) && !SIMPLES.test(t)) return null;
  const [inteira, fracao = ""] = t.replaceAll(".", "").split(",");
  const centavos = BigInt(inteira) * 100n + BigInt(fracao.padEnd(2, "0"));
  if (centavos > BigInt(Number.MAX_SAFE_INTEGER)) return null;
  return Number(centavos);
}

const TAXA = /^\d+([.,]\d+)?$/;

// Taxa digitada (vírgula ou ponto como decimal) -> texto com ponto decimal para o núcleo,
// sem passar por ponto flutuante. null se vazio ou fora do formato (milhar, sinal, letras).
export function paraTaxa(texto) {
  const t = String(texto ?? "").trim();
  if (!TAXA.test(t)) return null;
  return t.replace(",", ".");
}

export function formatarData(iso) {
  if (!iso) return "—";
  const [a, m, d] = iso.split("-");
  return `${d}/${m}/${a}`;
}

// Percentual de parte sobre total com uma casa e vírgula, em aritmética inteira ("89,0%").
export function formatarPercentual(parte, total) {
  const decimos = total > 0 ? Math.round((parte * 1000) / total) : 0;
  return `${Math.trunc(decimos / 10)},${decimos % 10}%`;
}
