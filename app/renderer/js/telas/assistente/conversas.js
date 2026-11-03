// Coluna das conversas: "Nova conversa" e a lista, com "Apagar" em cada uma.
import { el } from "../_tabela.js";

const dataLocal = (iso) => {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "" : d.toLocaleDateString("pt-BR");
};

// `travada`: uma resposta em andamento impede trocar ou apagar conversas.
export function colunaConversas({ lista, atualId, travada, aoNova, aoAbrir, aoApagar }) {
  const col = el("div", undefined, "conversas");
  const nova = el("button", "Nova conversa", "btn");
  nova.type = "button";
  nova.disabled = travada;
  nova.addEventListener("click", aoNova);
  const ul = el("ul", undefined, "lista-conversas");
  ul.setAttribute("aria-label", "Conversas");
  for (const c of lista) {
    const li = el("li");
    li.dataset.testid = "conversa";
    const abrir = el("button", undefined, "abrir");
    abrir.type = "button";
    abrir.disabled = travada;
    abrir.append(el("span", c.titulo, "titulo-conversa"), el("small", dataLocal(c.atualizadaEm)));
    if (c.id === atualId) {
      li.classList.add("on");
      abrir.setAttribute("aria-current", "true");
    }
    abrir.addEventListener("click", () => aoAbrir(c.id));
    const apagar = el("button", "Apagar", "link");
    apagar.type = "button";
    apagar.disabled = travada;
    apagar.addEventListener("click", () => aoApagar(c));
    li.append(abrir, apagar);
    ul.append(li);
  }
  col.append(nova);
  if (lista.length) col.append(ul);
  else col.append(el("small", "Nenhuma conversa ainda."));
  return col;
}
