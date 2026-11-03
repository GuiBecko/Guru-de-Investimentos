import { AVISOS } from "../../avisos.js";
import { el } from "../_tabela.js";

export function rodapeLegal() {
  return el("div", AVISOS.RODAPE_PERFIL, "legal");
}

export function alerta(texto) {
  const f = el("div", texto, "faixa");
  f.setAttribute("role", "alert");
  return f;
}
