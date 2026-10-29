import { el } from "../_tabela.js";

export const TEXTO_LEGAL =
  "Questionário baseado nas dimensões das regras de suitability da ANBIMA: objetivos, situação financeira e conhecimento. Não é recomendação de investimento.";

export function rodapeLegal() {
  return el("div", TEXTO_LEGAL, "legal");
}

export function alerta(texto) {
  const f = el("div", texto, "faixa");
  f.setAttribute("role", "alert");
  return f;
}
