import { api } from "../../api.js";
import { formatarData } from "../../formato.js";
import { el } from "../_tabela.js";
import { alerta, rodapeLegal } from "./_comum.js";
import { leitura, editor } from "./alocacao.js";
import { coluna as colunaMetas } from "./metas.js";

const MAX = 2000;
// Texto digitado e ainda não salvo: sobrevive à ida e volta ao editor de alocação.
let rascunho = null;
const EXEMPLOS = [
  "Quero construir um patrimônio de R$ 50.000 investido",
  "Quero manter o patrimônio que já tenho",
  "Quero viver de renda a partir dos 55",
];

function falha(raiz, mensagem) {
  const p = el("p", mensagem);
  p.setAttribute("role", "alert");
  raiz.replaceChildren(el("h1", "Perfil e objetivo"), p);
}

function convite() {
  const card = el("div", undefined, "card");
  const link = el("a", "Fazer o teste", "btn");
  link.href = "#/perfil/teste";
  card.append(
    el("b", "Descubra seu perfil de investidor"),
    el("p", "São 12 perguntas, leva uns 3 minutos. O resultado só aparece no fim."),
    link,
  );
  return card;
}

function cardObjetivo(inicial, vigente) {
  const card = el("div", undefined, "card");
  const rotulo = el("label", "Por que você está investindo? O que seria sucesso para você?", "titulo-campo");
  rotulo.htmlFor = "objetivo-txt";
  const dica = el("small", "Escreva com suas palavras. A IA vai usar este texto nas análises.");
  dica.id = "objetivo-dica";
  const texto = el("textarea");
  texto.id = "objetivo-txt";
  texto.rows = 5;
  texto.maxLength = MAX;
  texto.value = rascunho ?? inicial ?? "";
  texto.setAttribute("aria-describedby", dica.id);
  const contador = el("small", undefined, "mono contador");
  const atualizar = () => {
    contador.textContent = `${texto.value.length}/${MAX}`;
  };
  atualizar();
  const status = el("span", undefined, "hint");
  status.setAttribute("role", "status");
  const erro = el("span", undefined, "erro");
  erro.setAttribute("role", "alert");
  erro.hidden = true;
  texto.addEventListener("input", () => {
    rascunho = texto.value;
    atualizar();
    status.textContent = "";
  });

  const chips = el("div", undefined, "chips");
  for (const ex of EXEMPLOS) {
    const b = el("button", ex, "chip");
    b.type = "button";
    b.addEventListener("click", () => {
      texto.value = texto.value.trim() === "" ? ex : `${texto.value}\n\n${ex}`;
      rascunho = texto.value;
      atualizar();
      status.textContent = "";
      texto.focus();
    });
    chips.append(b);
  }

  const salvar = el("button", "Salvar objetivo", "btn");
  salvar.type = "button";
  salvar.addEventListener("click", async () => {
    salvar.disabled = true;
    status.textContent = "";
    erro.hidden = true;
    try {
      const erros = await api.objetivo.salvar(texto.value);
      if (!vigente()) return;
      if (erros.length > 0) {
        erro.textContent = erros.map((x) => x.mensagem).join(" ");
        erro.hidden = false;
      } else {
        rascunho = null;
        status.textContent = "Objetivo salvo.";
      }
    } catch (e) {
      console.error("Falha ao salvar o objetivo:", e);
      if (!vigente()) return;
      erro.textContent = "Não foi possível salvar o objetivo.";
      erro.hidden = false;
    }
    salvar.disabled = false;
  });
  const linha = el("div", undefined, "row acoes-resultado");
  linha.append(salvar, status);
  card.append(rotulo, dica, texto, contador, el("small", "Exemplos:"), chips, erro, linha);
  return card;
}

export async function render(raiz, editando, vigente) {
  let perfil, alocacao, objetivo, metas;
  try {
    [perfil, alocacao, objetivo, metas] = await Promise.all([
      api.perfil.atual(),
      api.alocacao.ler(),
      api.objetivo.ler(),
      api.metas.listar(),
    ]);
  } catch (e) {
    console.error("Falha ao carregar o perfil e o objetivo:", e);
    if (vigente()) falha(raiz, "Não foi possível carregar esta tela.");
    return;
  }
  if (!vigente()) return;

  let alvo;
  if (editando) {
    alvo = await editor(alocacao, perfil, vigente);
    if (!vigente()) return;
    if (!alvo) {
      falha(raiz, "Não foi possível abrir a edição da alocação.");
      return;
    }
  } else {
    alvo = leitura(alocacao, perfil);
  }

  const topo = el("div", undefined, "row");
  topo.append(el("h1", "Perfil e objetivo"));
  if (perfil) {
    const direitaTopo = el("div", undefined, "row");
    const refazer = el("a", "Refazer o teste", "migalha");
    refazer.href = "#/perfil/teste";
    direitaTopo.append(el("span", `Perfil ${perfil.perfilRotulo} · feito em ${formatarData(perfil.feitoEm)}`, "pill"), refazer);
    topo.append(direitaTopo);
  }
  const partes = [topo];
  if (!perfil) partes.push(convite());
  else if (perfil.vencido) partes.push(alerta("Seu perfil tem mais de 2 anos. Refaça o teste."));

  const esquerda = el("div", undefined, "coluna");
  esquerda.append(cardObjetivo(objetivo, vigente), alvo);
  const direita = colunaMetas(metas);
  const grade = el("div", undefined, "grid2");
  grade.append(esquerda, direita);
  partes.push(grade, rodapeLegal());
  raiz.replaceChildren(...partes);
}
