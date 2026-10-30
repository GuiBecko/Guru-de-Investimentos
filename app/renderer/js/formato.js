// Dinheiro sempre em centavos inteiros; nada de ponto flutuante.

export function formatarReais(centavos) {
  centavos = Math.round(centavos);
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

// Décimos de ponto percentual (inteiro do núcleo) -> "89,0%". Sem ponto flutuante.
export function formatarDecimos(decimos) {
  return `${Math.trunc(decimos / 10)},${decimos % 10}%`;
}

// Diferença entre o percentual atual (décimos) e o alvo (pontos inteiros): "+7,0 p.p.", "−8,0 p.p."
// ou "no alvo" quando a distância é menor que meio ponto.
export function formatarDiferenca(decimos, alvoPercentual) {
  const dif = decimos - alvoPercentual * 10;
  if (Math.abs(dif) < 5) return "no alvo";
  const abs = Math.abs(dif);
  return `${dif > 0 ? "+" : "\u2212"}${Math.trunc(abs / 10)},${abs % 10} p.p.`;
}

const MESES = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];

// "2030-12" -> "dez/2030".
export function formatarPrazo(aaaamm) {
  const [a, m] = String(aaaamm).split("-");
  return `${MESES[Number(m) - 1] ?? m}/${a}`;
}

// Rentabilidade em décimos de ponto percentual -> "+17,1%", "\u22123,2%" ou "0,0%". Sem ponto flutuante.
export function formatarRentabilidade(decimos) {
  const abs = Math.abs(decimos);
  const corpo = `${Math.trunc(abs / 10)},${abs % 10}%`;
  if (decimos > 0) return `+${corpo}`;
  return decimos < 0 ? `\u2212${corpo}` : corpo;
}

// Centésimos de ponto percentual -> "+1,07%", "−23,5%", "0,00%"; null -> "—". Duas casas abaixo
// de 10% e uma acima (meio para cima). Sem ponto flutuante.
export function formatarCentesimos(c) {
  if (c === null || c === undefined || !Number.isFinite(c)) return "—";
  c = Math.round(c);
  const abs = Math.abs(c);
  let corpo;
  if (abs < 1000) {
    corpo = `${Math.trunc(abs / 100)},${String(abs % 100).padStart(2, "0")}%`;
  } else {
    const dec = Math.floor((abs + 5) / 10);
    corpo = `${Math.trunc(dec / 10)},${dec % 10}%`;
  }
  if (c > 0) return `+${corpo}`;
  return c < 0 ? `−${corpo}` : corpo;
}
