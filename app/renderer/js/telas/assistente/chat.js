// O chat com a IA: balões, sugestões, campo com Enviar/Parar, "Buscando na web…" e o aviso fixo.
// Serve à tela Assistente e à gaveta do Próximo aporte. A resposta é acompanhada a cada 150 ms
// e, se a tela sai de cena no meio dela, o pedido é parado e lido até o fim (abandono).
import { api } from "../../api.js";
import { analisarMarkdown, renderizarMarkdown } from "../../markdown.js";
import { el } from "../_tabela.js";

const AVISO =
  "Conteúdo educativo gerado por IA com os seus dados e fontes da web. Não é recomendação individual de investimento. Confira as fontes.";
const INTERVALO = 150;

const esperar = (ms) => new Promise((r) => setTimeout(r, ms));

let ultimoId = 0;

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

function bolhaIa(texto, fontes, aoFonte) {
  const b = el("div", undefined, "bolha ia");
  b.append(renderizarMarkdown(analisarMarkdown(texto, fontes.map((f) => f.numero)), aoFonte));
  return b;
}

// `foco` (o do Próximo aporte) vai só na primeira pergunta, quando a conversa nasce.
// `aoMudar({ conversa, emAndamento })` roda depois de cada redesenho.
export function criarChat({ vigente, sugestoes: textosSugestoes, placeholder, foco = null, aoMudar = () => {} }) {
  const n = ++ultimoId;
  let atual = null; // ConversaJs aberta, ou null numa conversa nova
  let emFoco = null; // id da resposta cujas fontes o painel mostra (null: a última)
  let andamento = null; // { pergunta, pedido, parar, texto, fontes, buscando }

  const mensagens = el("div", undefined, "mensagens");
  const sugestoes = el("div", undefined, "sugestoes");
  const buscando = el("p", undefined, "buscando");
  buscando.setAttribute("role", "status");
  buscando.setAttribute("aria-live", "polite");
  const area = el("textarea");
  area.id = `chat-pergunta-${n}`;
  area.rows = 3;
  area.placeholder = placeholder;
  const rotulo = el("label", "Sua pergunta", "so-leitor");
  rotulo.htmlFor = area.id;
  const botao = el("button", "Enviar", "btn");
  botao.type = "button";
  const erro = el("p", undefined, "erro");
  erro.id = `chat-erro-${n}`;
  erro.setAttribute("role", "alert");
  erro.hidden = true;
  const campo = el("div", undefined, "campo-pergunta");
  campo.append(rotulo, area, botao);
  const chat = el("section", undefined, "chat");
  chat.setAttribute("aria-label", "Conversa");
  chat.append(mensagens, sugestoes, buscando, campo, erro, el("p", AVISO, "legal"));

  const avisar = () => aoMudar({ conversa: atual, emAndamento: !!andamento });

  function mostrarErro(mensagem) {
    erro.textContent = mensagem ?? "";
    erro.hidden = !mensagem;
    if (mensagem) area.setAttribute("aria-describedby", erro.id);
    else area.removeAttribute("aria-describedby");
  }

  function respostaEmFoco() {
    const ias = (atual?.mensagens ?? []).filter((m) => m.papel === "assistant");
    return ias.find((m) => m.id === emFoco) ?? ias[ias.length - 1] ?? null;
  }

  function fontes() {
    return {
      web: andamento ? andamento.fontes : (respostaEmFoco()?.fontes ?? []),
      internas: atual ? atual.fontesInternas : null,
    };
  }

  function focar(id, numero) {
    emFoco = id;
    avisar();
    const alvo = document.getElementById(`fonte-${numero}`);
    alvo?.scrollIntoView({ block: "nearest" });
    alvo?.focus({ preventScroll: true });
  }

  function bolhaPendente() {
    const b = bolhaIa(andamento.texto, andamento.fontes);
    b.dataset.testid = "pendente";
    b.setAttribute("aria-busy", "true");
    if (!andamento.texto) b.append(el("small", "Escrevendo…"));
    return b;
  }

  function pintar() {
    const itens = [];
    for (const m of atual?.mensagens ?? []) {
      let b;
      if (m.papel === "user") b = el("div", m.texto, "bolha eu");
      else {
        b = bolhaIa(m.texto, m.fontes, (num) => focar(m.id, num));
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
        ? textosSugestoes.map((s) => {
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
    pintar();
    avisar();
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
      const c = await api.ia.conversa(conversaId);
      if (!vigente()) return;
      atual = c;
      emFoco = null;
      andamento = null;
      desenhar();
    } catch (e) {
      console.error("Falha ao mostrar a resposta da IA:", e);
      if (!vigente()) return;
      andamento = null;
      desenhar();
      mostrarErro("A resposta foi gravada, mas não foi possível mostrá-la. Abra a conversa no Assistente.");
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
        // Sem acompanhar, o pedido não pode seguir sozinho: pede para parar, como o Parar.
        api.ia.cancelar(pedido).catch(() => {});
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
      avisar();
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
      const resposta = atual ? await api.ia.perguntar(atual.id, pergunta) : await api.ia.perguntar(null, pergunta, foco ?? undefined);
      ({ pedido } = resposta);
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

  botao.addEventListener("click", () => (andamento ? parar() : enviar()));
  area.addEventListener("keydown", (e) => {
    if (e.key !== "Enter" || e.shiftKey || e.isComposing) return;
    e.preventDefault();
    if (!andamento) enviar();
  });

  pintar();

  return {
    elemento: chat,
    // Troca a conversa mostrada (null: conversa nova). Ignorado com uma resposta em andamento.
    mostrar(conversa) {
      if (andamento) return;
      atual = conversa;
      emFoco = null;
      mostrarErro(null);
      desenhar();
    },
    conversa: () => atual,
    emAndamento: () => !!andamento,
    preencher(texto) {
      area.value = texto;
      area.focus();
    },
    focarCampo: () => area.focus(),
    erro: mostrarErro,
    fontes,
  };
}
