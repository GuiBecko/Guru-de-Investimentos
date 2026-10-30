import { formatarDecimos } from "../../formato.js";

// Até 3 altas (maiores primeiro) e até 3 quedas (a mais forte primeiro), com a seta e a classe `up`/`down`. Variação zero não entra.
export function altasEQuedas(variacoes) {
  const altas = variacoes.filter((v) => v.variacaoDecimos > 0).slice(0, 3);
  const quedas = variacoes.filter((v) => v.variacaoDecimos < 0).slice(-3).reverse();
  return [...altas, ...quedas].map((v) => ({
    ticker: v.ticker,
    texto: `${v.variacaoDecimos > 0 ? "▲" : "▼"} ${formatarDecimos(Math.abs(v.variacaoDecimos))}`,
    classe: v.variacaoDecimos > 0 ? "up" : "down",
  }));
}
