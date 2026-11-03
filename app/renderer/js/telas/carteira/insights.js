// Coluna da IA no painel da Carteira: os insights e as notícias da carteira. Nada aqui chama a IA
// sozinho: cada geração sai de um botão, porque gasta a chave do usuário. O resultado fica no
// núcleo com a data e aparece até a próxima geração.
import { api } from "../../api.js";
import { formatarData } from "../../formato.js";
import { el } from "../_tabela.js";
import { dataDaFonte } from "../assistente/painel.js";

const CONVITE = "Para ver insights e notícias da sua carteira, configure a sua chave.";
const NOTA_CUSTO = "Cada geração usa a sua chave da Anthropic.";
const AVISO = "Gerado por IA com os seus dados. Não é recomendação individual de investimento.";
const SEM_INSIGHTS = "Nenhum insight gerado ainda.";
const SEM_NOTICIAS = "Nenhuma notícia recente encontrada sobre os seus ativos.";
const BUSCA_DESLIGADA = "Ligue “Buscar na web” em Configurar IA para buscar notícias.";
const FALHA_ACOMPANHAR = "Não foi possível acompanhar a geração. Tente de novo.";
const FALHA_MOSTRAR = "O resultado foi gravado, mas não foi possível mostrá-lo. Abra a Carteira de novo.";
const VISIVEIS = 5;
const INTERVALO = 150;
const CLASSES = new Set(["rf", "acoes", "fii", "ext", "outros"]);

const TEXTOS = {
  insights: { gerar: "Gerar insights", deNovo: "Gerar de novo", andando: "Gerando insights…" },
  noticias: { gerar: "Buscar notícias", deNovo: "Buscar de novo", andando: "Buscando notícias na web…" },
};

// Pedidos em andamento por tipo: { pedido, parar, fim }. Ficam no módulo porque sair da Carteira
// não para a geração (já foi paga); voltar à tela retoma o acompanhamento.
const andamento = { insights: null, noticias: null };
// Só o último laço de acompanhamento de cada tipo vale (uma abertura nova da tela passa à frente).
const voltas = { insights: 0, noticias: 0 };

const esperar = (ms) => new Promise((r) => setTimeout(r, ms));

// O link de cada insight para o lugar certo do app; `null` sem ação. O endereço é sempre interno.
export function acaoDoInsight(insight, indice) {
  switch (insight?.acao) {
    case "aporte":
      return { texto: "Simular próximo aporte →", href: "#/aporte" };
    case "meta":
      return { texto: "Ver metas →", href: "#/perfil" };
    case "classe":
      if (!CLASSES.has(insight.classe)) return null;
      return { texto: `Ver ${insight.classeRotulo ?? insight.classe} →`, href: `#/carteira/${insight.classe}` };
    case "conversar":
      if (!Number.isInteger(indice) || indice < 0) return null;
      return { texto: "Conversar sobre isso →", href: `#/assistente/insight/${indice}` };
    default:
      return null;
  }
}

const doisDigitos = (n) => String(n).padStart(2, "0");
const isoLocal = (d) => `${d.getFullYear()}-${doisDigitos(d.getMonth() + 1)}-${doisDigitos(d.getDate())}`;

// "hoje" no mesmo dia do relógio local; senão dd/mm. `geradoEm` vem em aaaa-mm-dd.
function quando(geradoEm, hoje) {
  if (geradoEm === isoLocal(hoje)) return "hoje";
  const [, m, d] = String(geradoEm).split("-");
  return `${d}/${m}`;
}

export function rotuloGerado(geradoEm, hoje = new Date()) {
  return `Claude · ${quando(geradoEm, hoje)}`;
}

// Acompanha o pedido do tipo até o fim. Sair da tela só para este laço: o pedido segue no addon,
// e a próxima abertura da coluna lê o fim (o pedido terminado sai do mapa na primeira leitura).
async function acompanhar(tipo, vigente, aoFim) {
  const volta = ++voltas[tipo];
  const viva = () => vigente() && volta === voltas[tipo];
  const a = andamento[tipo];
  if (!a) return;
  while (!a.fim) {
    await esperar(INTERVALO);
    if (!viva()) return;
    if (a.fim) break;
    if (a.pedido === null) continue; // o número do pedido ainda não chegou
    try {
      const p = await api.ia.progresso(a.pedido);
      if (p.terminado) a.fim = { erro: p.erro ?? null };
    } catch (e) {
      console.error("Falha ao acompanhar a geração da IA:", e);
      api.ia.cancelar(a.pedido).catch(() => {});
      a.fim = { erro: { code: "", message: FALHA_ACOMPANHAR } };
    }
  }
  if (!viva() || andamento[tipo] !== a) return;
  andamento[tipo] = null;
  await aoFim(tipo, a);
}

function botao(texto, classe, foco) {
  const b = el("button", texto, classe);
  b.type = "button";
  if (foco) b.dataset.foco = foco;
  return b;
}

function alerta(mensagem) {
  const p = el("p", mensagem, "erro");
  p.setAttribute("role", "alert");
  return p;
}

function linkIa() {
  const a = el("a", "Configurar IA");
  a.href = "#/ia";
  return a;
}

function cartaoInsight(insight, indice) {
  const c = el("div", undefined, "insight");
  c.dataset.testid = "insight";
  c.append(el("span", insight.rotulo, insight.tipo === "rebalanceamento" ? "pill wa" : "pill"), el("p", insight.texto));
  const acao = acaoDoInsight(insight, indice);
  if (acao) {
    const a = el("a", acao.texto, "acao-insight");
    a.href = acao.href;
    c.append(a);
  }
  return c;
}

function linkNoticia(n) {
  const a = el("a", undefined, "noticia");
  a.dataset.testid = "noticia";
  // Abre no navegador pelo handler de janelas do main (só https).
  a.href = n.url;
  a.target = "_blank";
  a.rel = "noopener noreferrer";
  const meta = el("span", undefined, "m");
  const partes = [n.dominio, dataDaFonte(n.data)].filter(Boolean);
  meta.append(document.createTextNode(`${partes.join(" · ")} · `), el("span", n.etiqueta, "pill mute"));
  a.append(el("span", n.titulo, "t"), meta);
  return a;
}

// A coluna inteira; carrega sozinha (configuração e painel da IA) e retoma o que estava andando.
export function colunaIa(vigente) {
  const raiz = el("aside", undefined, "card coluna-ia");
  raiz.dataset.testid = "coluna-ia";
  raiz.setAttribute("aria-label", "Insights e notícias");
  raiz.append(el("small", "Carregando…", "hint"));

  let config = null;
  let painel = null;
  let todas = false;
  const erros = { insights: null, noticias: null };

  function acoes(tipo, gerado) {
    const t = TEXTOS[tipo];
    const linha = el("div", undefined, "acoes-ia");
    const a = andamento[tipo];
    if (a) {
      const status = botao(t.andando, "btn sm");
      status.disabled = true;
      const parar = botao("Parar", "btn ghost sm", tipo);
      parar.disabled = a.parar;
      parar.addEventListener("click", () => pararPedido(tipo));
      linha.append(status, parar);
    } else {
      const gerar = botao(gerado ? t.deNovo : t.gerar, "btn sm", tipo);
      gerar.addEventListener("click", () => iniciar(tipo));
      linha.append(gerar);
    }
    return linha;
  }

  function blocoInsights() {
    const caixa = el("section", undefined, "ia-insights");
    caixa.setAttribute("aria-label", "Insights");
    const cab = el("div", undefined, "row");
    cab.append(el("b", "Insights"));
    if (painel.insights) cab.append(el("span", rotuloGerado(painel.insights.geradoEm), "pill"));
    caixa.append(cab);
    const itens = painel.insights?.itens ?? [];
    if (itens.length) caixa.append(...itens.map(cartaoInsight));
    else caixa.append(el("p", SEM_INSIGHTS, "hint"));
    caixa.append(acoes("insights", !!painel.insights));
    if (erros.insights) caixa.append(alerta(erros.insights));
    caixa.append(el("small", NOTA_CUSTO, "hint"));
    return caixa;
  }

  function blocoNoticias() {
    const caixa = el("section", undefined, "ia-noticias");
    caixa.dataset.testid = "noticias";
    caixa.setAttribute("aria-label", "Notícias da sua carteira");
    const cab = el("div", undefined, "row");
    cab.append(el("b", "Notícias da sua carteira"));
    const g = painel.noticias;
    if (g) cab.append(el("small", quando(g.geradoEm, new Date()) === "hoje" ? "hoje" : formatarData(g.geradoEm)));
    caixa.append(cab);
    if (g) {
      const validas = g.itens.filter((n) => /^https:\/\//.test(n.url));
      if (validas.length === 0) caixa.append(el("p", SEM_NOTICIAS, "hint"));
      else {
        const lista = el("div", undefined, "newsfeed");
        lista.append(...(todas ? validas : validas.slice(0, VISIVEIS)).map(linkNoticia));
        caixa.append(lista);
        if (!todas && validas.length > VISIVEIS) {
          const mais = botao("Ver todas as notícias →", "link");
          mais.addEventListener("click", () => {
            todas = true;
            desenhar();
            raiz.querySelectorAll("[data-testid=noticia]")[VISIVEIS]?.focus();
          });
          caixa.append(mais);
        }
      }
    }
    if (painel.buscaWeb || andamento.noticias) caixa.append(acoes("noticias", !!g));
    else {
      const aviso = el("p", undefined, "hint");
      aviso.append(document.createTextNode(`${BUSCA_DESLIGADA} `), linkIa());
      caixa.append(aviso);
    }
    if (erros.noticias) caixa.append(alerta(erros.noticias));
    return caixa;
  }

  function desenhar() {
    // O botão clicado some no redesenho: o foco passa ao que ocupa o mesmo lugar.
    const foco = raiz.contains(document.activeElement) ? document.activeElement.dataset?.foco : undefined;
    if (!config.chave) {
      const link = linkIa();
      link.className = "btn sm";
      raiz.replaceChildren(el("p", CONVITE), link);
      return;
    }
    raiz.replaceChildren(blocoInsights(), blocoNoticias(), el("p", AVISO, "legal"));
    if (foco) raiz.querySelector(`[data-foco="${foco}"]:not(:disabled)`)?.focus();
  }

  async function aoFim(tipo, a) {
    const cancelada = a.parar && a.fim.erro?.code === "IA_CANCELADA";
    if (a.fim.erro && !cancelada) {
      erros[tipo] = a.fim.erro.message || FALHA_ACOMPANHAR;
      desenhar();
      return;
    }
    try {
      const p = await api.ia.painel();
      if (!vigente()) return;
      painel = p;
      if (tipo === "noticias") todas = false;
    } catch (e) {
      console.error("Falha ao reler o painel da IA:", e);
      if (!vigente()) return;
      erros[tipo] = FALHA_MOSTRAR;
    }
    desenhar();
  }

  async function iniciar(tipo) {
    if (andamento[tipo]) return;
    const a = { pedido: null, parar: false, fim: null };
    andamento[tipo] = a;
    erros[tipo] = null;
    desenhar();
    try {
      const { pedido } = await (tipo === "insights" ? api.ia.gerarInsights() : api.ia.buscarNoticias());
      a.pedido = pedido;
      if (a.parar) api.ia.cancelar(pedido).catch((e) => console.error("Falha ao parar a geração:", e));
    } catch (e) {
      console.error("Falha ao pedir a geração da IA:", e);
      a.fim = { erro: { code: e.code ?? "", message: e.message || FALHA_ACOMPANHAR } };
    }
    // Fora de cena, o pedido segue: a próxima abertura da coluna retoma o acompanhamento.
    if (!vigente()) return;
    await acompanhar(tipo, vigente, aoFim);
  }

  function pararPedido(tipo) {
    const a = andamento[tipo];
    if (!a || a.parar) return;
    a.parar = true;
    desenhar();
    // Sem o número do pedido ainda, `iniciar` cancela assim que ele chegar.
    if (a.pedido !== null) api.ia.cancelar(a.pedido).catch((e) => console.error("Falha ao parar a geração:", e));
  }

  (async () => {
    try {
      [config, painel] = await Promise.all([api.ia.config(), api.ia.painel()]);
    } catch (e) {
      console.error("Falha ao carregar a coluna da IA:", e);
      if (!vigente()) return;
      raiz.replaceChildren(alerta("Não foi possível carregar os insights."));
      return;
    }
    if (!vigente()) return;
    desenhar();
    for (const tipo of ["insights", "noticias"]) if (andamento[tipo]) acompanhar(tipo, vigente, aoFim);
  })();

  return raiz;
}
