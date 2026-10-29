import { api } from "../../api.js";
import { el } from "../_tabela.js";
import { alerta, rodapeLegal } from "./_comum.js";

function falha(raiz, mensagem) {
  const p = el("p", mensagem);
  p.setAttribute("role", "alert");
  raiz.replaceChildren(el("h1", "Seu perfil"), p);
}

export async function render(raiz, vigente) {
  let atual;
  try {
    atual = await api.perfil.atual();
  } catch (e) {
    console.error("Falha ao carregar o perfil:", e);
    if (vigente()) falha(raiz, "Não foi possível carregar o seu perfil.");
    return;
  }
  if (!vigente()) return;

  if (!atual) {
    const link = el("a", "Fazer o teste de perfil", "btn");
    link.href = "#/perfil/teste";
    const vazio = el("div", undefined, "vazio");
    vazio.append(el("p", "Você ainda não fez o teste."), link);
    raiz.replaceChildren(el("h1", "Seu perfil"), vazio);
    return;
  }

  const partes = [el("h1", `Seu perfil: ${atual.perfilRotulo}`), el("p", atual.descricao)];
  if (atual.vencido) partes.push(alerta("Seu perfil tem mais de 2 anos. Refaça o teste."));
  for (const a of atual.avisos) partes.push(alerta(a));

  const lista = el("ul", undefined, "alocacao");
  lista.setAttribute("aria-label", "Alocação sugerida");
  for (const item of atual.alocacaoSugerida) {
    const li = el("li");
    li.append(el("span", item.rotulo), el("span", `${item.percentual}%`, "mono"));
    lista.append(li);
  }
  const card = el("div", undefined, "card");
  card.append(el("b", "Alocação sugerida"), lista);
  partes.push(card);

  const erro = el("div", undefined, "faixa");
  erro.setAttribute("role", "alert");
  erro.hidden = true;

  const usar = el("button", "Usar esta alocação", "btn");
  usar.type = "button";
  usar.addEventListener("click", async () => {
    usar.disabled = true;
    try {
      const erros = await api.alocacao.salvar(
        atual.alocacaoSugerida.map((x) => ({ classe: x.classe, percentual: x.percentual })),
      );
      if (!vigente()) return;
      if (erros.length > 0) {
        erro.textContent = erros.map((x) => x.mensagem).join(" ");
        erro.hidden = false;
        usar.disabled = false;
        return;
      }
      window.location.hash = "#/perfil";
    } catch (e) {
      console.error("Falha ao salvar a alocação:", e);
      if (!vigente()) return;
      erro.textContent = "Não foi possível salvar a alocação.";
      erro.hidden = false;
      usar.disabled = false;
    }
  });
  const ajustar = el("a", "Ajustar a alocação", "btn ghost");
  ajustar.href = "#/perfil/alocacao";
  const acoes = el("div", undefined, "row acoes-resultado");
  acoes.append(ajustar, usar);
  partes.push(erro, acoes);

  partes.push(rodapeLegal());
  raiz.replaceChildren(...partes);
  raiz.querySelector("h1").tabIndex = -1;
  raiz.querySelector("h1").focus();
}
