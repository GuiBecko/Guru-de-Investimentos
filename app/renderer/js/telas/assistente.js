import { api } from "../api.js";
import { el } from "./_tabela.js";
import { confirmar } from "./_confirmar.js";
import { criarChat } from "./assistente/chat.js";
import { colunaConversas } from "./assistente/conversas.js";
import { painel } from "./assistente/painel.js";

const CONVITE = "Para conversar com a IA, configure a sua chave.";
const SUGESTOES = [
  "O que mudou no mercado esta semana?",
  "Minha carteira está alinhada ao meu perfil?",
  "Quanto pago de IR nos meus investimentos?",
];
const NAO_ABRIU = "Não foi possível abrir a conversa.";

function convite() {
  const link = el("a", "Configurar IA", "btn");
  link.href = "#/ia";
  const card = el("div", undefined, "card vazio");
  card.append(el("p", CONVITE), link);
  return card;
}

// `#/assistente/<id>` abre a conversa `id`; sem ela, a tela abre numa conversa nova.
export async function render(raiz, parametros = [], vigente = () => true) {
  const titulo = el("h1", "Assistente");
  const pedidaId = /^\d+$/.test(parametros[0] ?? "") ? Number(parametros[0]) : null;
  let config;
  let lista;
  let inicial = null;
  let naoAbriu = parametros.length > 0 && pedidaId === null;
  try {
    [config, lista] = await Promise.all([api.ia.config(), api.ia.conversas()]);
  } catch (e) {
    console.error("Falha ao abrir o Assistente:", e);
    if (!vigente()) return;
    const p = el("p", "Não foi possível abrir esta tela.");
    p.setAttribute("role", "alert");
    raiz.replaceChildren(titulo, p);
    return;
  }
  if (!vigente()) return;
  if (!config.chave) {
    raiz.replaceChildren(titulo, convite());
    return;
  }
  if (pedidaId !== null) {
    try {
      inicial = await api.ia.conversa(pedidaId);
    } catch (e) {
      console.warn("Conversa pedida pelo endereço não abriu:", e);
      naoAbriu = true;
    }
    if (!vigente()) return;
  }

  const colEsquerda = el("div");
  const colDireita = el("div");
  let andava = false;

  const chat = criarChat({
    vigente,
    sugestoes: SUGESTOES,
    placeholder: "Pergunte sobre sua carteira ou o mercado…",
    aoMudar: ({ emAndamento }) => {
      const terminou = andava && !emAndamento;
      andava = emAndamento;
      desenhar();
      // Uma resposta terminou: a conversa pode ter nascido ou mudado de data.
      if (terminou) atualizarLista();
    },
  });

  const grade = el("div", undefined, "assistente");
  grade.append(colEsquerda, chat.elemento, colDireita);
  raiz.replaceChildren(titulo, grade);

  function desenhar() {
    const { web, internas } = chat.fontes();
    colDireita.replaceChildren(painel(web, internas));
    colEsquerda.replaceChildren(
      colunaConversas({
        lista,
        atualId: chat.conversa()?.id ?? null,
        travada: chat.emAndamento(),
        aoNova: nova,
        aoAbrir: abrir,
        aoApagar: apagar,
      }),
    );
  }

  async function atualizarLista() {
    try {
      const l = await api.ia.conversas();
      if (!vigente()) return;
      lista = l;
      desenhar();
    } catch (e) {
      console.error("Falha ao atualizar a lista de conversas:", e);
    }
  }

  function nova() {
    chat.mostrar(null);
    chat.focarCampo();
  }

  async function abrir(id) {
    try {
      const c = await api.ia.conversa(id);
      if (!vigente() || chat.emAndamento()) return;
      chat.mostrar(c);
    } catch (e) {
      console.error("Falha ao abrir a conversa:", e);
      if (!vigente()) return;
      chat.erro(NAO_ABRIU);
    }
  }

  async function apagar(c) {
    if (!(await confirmar(`Apagar a conversa "${c.titulo}"?`)) || !vigente() || chat.emAndamento()) return;
    try {
      await api.ia.apagarConversa(c.id);
      const l = await api.ia.conversas();
      if (!vigente()) return;
      lista = l;
      chat.mostrar(chat.conversa()?.id === c.id ? null : chat.conversa());
    } catch (e) {
      console.error("Falha ao apagar a conversa:", e);
      if (!vigente()) return;
      chat.erro("Não foi possível apagar a conversa.");
    }
  }

  chat.mostrar(inicial);
  if (naoAbriu) chat.erro(NAO_ABRIU);
}
