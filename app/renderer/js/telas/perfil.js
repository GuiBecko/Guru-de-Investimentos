import { render as questionario } from "./perfil/questionario.js";
import { render as resultado } from "./perfil/resultado.js";
import { render as painel } from "./perfil/painel.js";

export function render(el, parametros = [], vigente = () => true) {
  if (parametros[0] === "teste") return questionario(el, vigente);
  if (parametros[0] === "resultado") return resultado(el, vigente);
  return painel(el, parametros[0] === "alocacao", vigente);
}
