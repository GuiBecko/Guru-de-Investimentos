import { api } from "../api.js";
import { analisarMarkdown, renderizarMarkdown } from "../markdown.js";
import { el } from "./_tabela.js";
import { confirmar } from "./_confirmar.js";
import { colunaConversas } from "./assistente/conversas.js";
import { painel } from "./assistente/painel.js";

const CONVITE = "Para conversar com a IA, configure a sua chave.";
const AVISO =
  "Conteúdo educativo gerado por IA com os seus dados e fontes da web. Não é recomendação individual de investimento. Confira as fontes.";
const SUGESTOES = [
  "O que mudou no mercado esta semana?",
  "Minha carteira está alinhada ao meu perfil?",
  "Quanto pago de IR nos meus investimentos?",
];
const INTERVALO = 150;

const esperar = (ms) => new Promise((r) => setTimeout(r, ms));

// A tela saiu de cena com um pedido em andamento: para e lê até o fim, para o pedido sair do
// mapa do addon. Nada aqui mexe na tela.
async function abandonar(pedido) {
  try {
    await api.ia.cancelar(pedido);
    for (let i = 0; i < 400; i++) {
      if ((await api.ia.progresso(pedido)).terminado) return;
      await esperar(INTERVALO);
    }
  } catch (e) {
    console.warn("Pedido da IA abandonado:", e);
  }
}

function convite() {
  const link = el("a", "Configurar IA", "btn");
  link.href = "#/ia";
  const card = el("div", undefined, "card vazio");
  card.append(el("p", CONVITE), link);
  return card;
}

function bolhaIa(texto, fontes, aoFonte) {
  const b = el("div", undefined, "bolha ia");
  b.append(renderizarMarkdown(analisarMarkdown(texto, fontes.map((f) => f.numero)), aoFonte));
  return b;
}

export async function render(raiz, _parametros, vigente = () => true) {
  const titulo = el("h1", "Assistente");
  let config;
  let lista;
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

  let atual = null; // ConversaJs aberta, ou null numa conversa nova
  let foco = null; // id da resposta cujas fontes o painel mostra (null: a última)
  let andamento = null; // { pergunta, pedido, parar, texto, fontes, buscando }

  // Peças fixas: o campo e o botão sobrevivem aos redesenhos.
  const colEsquerda = el("div");
  const colDireita = el("div");
  const mensagens = el("div", undefined, "mensagens");
  const sugestoes = el("div", undefined, "sugestoes");
  const buscando = el("p", undefined, "buscando");
  buscando.setAttribute("role", "status");
  buscando.setAttribute("aria-live", "polite");
  const area = el("textarea");
  area.id = "assistente-pergunta";
  area.rows = 3;
  area.placeholder = "Pergunte sobre sua carteira ou o mercado…";
  const rotulo = el("label", "Sua pergunta", "so-leitor");
  rotulo.htmlFor = area.id;
  const botao = el("button", "Enviar", "btn");
  botao.type = "button";
  const erro = el("p", undefined, "erro");
  erro.id = "e-assistente";
  erro.setAttribute("role", "alert");
  erro.hidden = true;
  const campo = el("div", undefined, "campo-pergunta");
  campo.append(rotulo, area, botao);
  const chat = el("section", undefined, "chat");
  chat.setAttribute("aria-label", "Conversa");
  chat.append(mensagens, sugestoes, buscando, campo, erro, el("p", AVISO, "legal"));
  const grade = el("div", undefined, "assistente");
  grade.append(colEsquerda, chat, colDireita);
  raiz.replaceChildren(titulo, grade);

  function mostrarErro(mensagem) {
    erro.textContent = mensagem ?? "";
    erro.hidden = !mensagem;
    if (mensagem) area.setAttribute("aria-describedby", erro.id);
    else area.removeAttribute("aria-describedby");
  }

  function focar(id, n) {
    foco = id;
    desenharPainel();
    const alvo = document.getElementById(`fonte-${n}`);
    alvo?.scrollIntoView({ block: "nearest" });
    alvo?.focus({ preventScroll: true });
  }

  function respostaEmFoco() {
    const ias = (atual?.mensagens ?? []).filter((m) => m.papel === "assistant");
    return ias.find((m) => m.id === foco) ?? ias[ias.length - 1] ?? null;
  }

  function desenharPainel() {
    const web = andamento ? andamento.fontes : (respostaEmFoco()?.fontes ?? []);
    colDireita.replaceChildren(painel(web, atual ? atual.fontesInternas : null));
  }

  function desenharLista() {
    colEsquerda.replaceChildren(
      colunaConversas({ lista, atualId: atual?.id ?? null, travada: !!andamento, aoNova: nova, aoAbrir: abrir, aoApagar: apagar }),
    );
  }

  function bolhaPendente() {
    const b = bolhaIa(andamento.texto, andamento.fontes);
    b.dataset.testid = "pendente";
    b.setAttribute("aria-busy", "true");
    if (!andamento.texto) b.append(el("small", "Escrevendo…"));
    return b;
  }

  function desenharChat() {
    const itens = [];
    for (const m of atual?.mensagens ?? []) {
      let b;
      if (m.papel === "user") b = el("div", m.texto, "bolha eu");
      else {
        b = bolhaIa(m.texto, m.fontes, (n) => focar(m.id, n));
        if (m.aviso) b.append(el("small", m.aviso, "aviso-ia"));
      }
      b.dataset.testid = "mensagem";
      b.dataset.papel = m.papel;
      itens.push(b);
    }
    if (andamento) {
      const eu = el("div", andamento.pergunta, "bolha eu");
      eu.dataset.testid = "pendente";
      itens.push(eu, bolhaPendente());
    }
    mensagens.replaceChildren(...itens);
    mensagens.scrollTop = mensagens.scrollHeight;
    const vazia = !andamento && !atual?.mensagens.length;
    sugestoes.replaceChildren(
      ...(vazia
        ? SUGESTOES.map((s) => {
            const b = el("button", s, "opt");
            b.type = "button";
            b.addEventListener("click", () => enviar(s));
            return b;
          })
        : []),
    );
    sugestoes.hidden = !vazia;
    buscando.textContent = andamento?.buscando ? "Buscando na web…" : "";
    botao.textContent = andamento ? "Parar" : "Enviar";
    botao.disabled = !!andamento?.parar;
  }

  function desenhar() {
    desenharLista();
    desenharChat();
    desenharPainel();
  }

  function falhar(mensagem) {
    const { pergunta } = andamento;
    andamento = null;
    // O texto da pergunta volta para o campo, sem apagar o que já tenha sido digitado.
    area.value = area.value.trim() ? `${pergunta}\n${area.value}` : pergunta;
    desenhar();
    mostrarErro(mensagem);
    area.focus();
  }

  async function concluir(conversaId) {
    try {
      const [c, l] = await Promise.all([api.ia.conversa(conversaId), api.ia.conversas()]);
      if (!vigente()) return;
      atual = c;
      lista = l;
      foco = null;
      andamento = null;
      desenhar();
    } catch (e) {
      console.error("Falha ao mostrar a resposta da IA:", e);
      if (!vigente()) return;
      andamento = null;
      desenhar();
      mostrarErro("A resposta foi gravada, mas não foi possível mostrá-la. Abra a conversa na lista.");
    }
  }

  async function acompanhar(pedido) {
    let ultimo = null;
    for (;;) {
      await esperar(INTERVALO);
      if (!vigente()) return abandonar(pedido);
      let p;
      try {
        p = await api.ia.progresso(pedido);
      } catch (e) {
        console.error("Falha ao acompanhar a resposta da IA:", e);
        if (!vigente()) return;
        return falhar("Não foi possível acompanhar a resposta. Tente de novo.");
      }
      if (!vigente()) return p.terminado ? undefined : abandonar(pedido);
      if (p.terminado) return p.erro ? falhar(p.erro.message) : concluir(p.conversaId);
      const assinatura = `${p.texto.length}|${p.fontes.length}|${p.buscando}`;
      if (assinatura === ultimo) continue;
      ultimo = assinatura;
      Object.assign(andamento, { texto: p.texto, fontes: p.fontes, buscando: p.buscando });
      mensagens.lastElementChild?.replaceWith(bolhaPendente());
      mensagens.scrollTop = mensagens.scrollHeight;
      buscando.textContent = p.buscando ? "Buscando na web…" : "";
      desenharPainel();
    }
  }

  async function enviar(texto) {
    const pergunta = (texto ?? area.value).trim();
    if (!pergunta || andamento) return;
    mostrarErro(null);
    if (texto === undefined) area.value = "";
    andamento = { pergunta, pedido: null, parar: false, texto: "", fontes: [], buscando: false };
    desenhar();
    let pedido;
    try {
      ({ pedido } = await api.ia.perguntar(atual?.id ?? null, pergunta));
    } catch (e) {
      console.error("Falha ao enviar a pergunta:", e);
      if (!vigente()) return;
      return falhar(e.message || "Não foi possível enviar a pergunta.");
    }
    if (!vigente()) return abandonar(pedido);
    andamento.pedido = pedido;
    if (andamento.parar) api.ia.cancelar(pedido).catch((e) => console.error("Falha ao parar a resposta:", e));
    await acompanhar(pedido);
  }

  function parar() {
    if (!andamento || andamento.parar) return;
    andamento.parar = true;
    botao.disabled = true;
    // Sem o número do pedido ainda, `enviar` cancela assim que ele chegar.
    if (andamento.pedido !== null) api.ia.cancelar(andamento.pedido).catch((e) => console.error("Falha ao parar a resposta:", e));
  }

  function nova() {
    atual = null;
    foco = null;
    mostrarErro(null);
    desenhar();
    area.focus();
  }

  async function abrir(id) {
    try {
      const c = await api.ia.conversa(id);
      if (!vigente() || andamento) return;
      atual = c;
      foco = null;
      mostrarErro(null);
      desenhar();
    } catch (e) {
      console.error("Falha ao abrir a conversa:", e);
      if (!vigente()) return;
      mostrarErro("Não foi possível abrir a conversa.");
    }
  }

  async function apagar(c) {
    if (!(await confirmar(`Apagar a conversa "${c.titulo}"?`)) || !vigente() || andamento) return;
    try {
      await api.ia.apagarConversa(c.id);
      const l = await api.ia.conversas();
      if (!vigente()) return;
      lista = l;
      if (atual?.id === c.id) atual = null;
      mostrarErro(null);
      desenhar();
    } catch (e) {
      console.error("Falha ao apagar a conversa:", e);
      if (!vigente()) return;
      mostrarErro("Não foi possível apagar a conversa.");
    }
  }

  botao.addEventListener("click", () => (andamento ? parar() : enviar()));
  area.addEventListener("keydown", (e) => {
    if (e.key !== "Enter" || e.shiftKey || e.isComposing) return;
    e.preventDefault();
    if (!andamento) enviar();
  });

  desenhar();
}
