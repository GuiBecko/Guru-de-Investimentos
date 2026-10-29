import { emConstrucao } from "./_em-construcao.js";
import { render as questionario } from "./perfil/questionario.js";
import { render as resultado } from "./perfil/resultado.js";

export function render(el, parametros = [], vigente = () => true) {
  if (parametros[0] === "teste") return questionario(el, vigente);
  if (parametros[0] === "resultado") return resultado(el, vigente);
  emConstrucao(el, "Perfil e objetivo");
}
