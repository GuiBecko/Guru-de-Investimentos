// Atualização do app na interface: a faixa da barra lateral e a seção "Atualizações" da tela Sobre.
// As duas desenham o mesmo estado, lido do processo principal ao abrir e a cada 1 s enquanto ele é
// "procurando" ou "baixando". Nada baixa nem reinicia sem clique.
import { api } from "./api.js";

const INTERVALO_MS = 1000;
const ACOMPANHAR = new Set(["procurando", "baixando"]);
const NA_FAIXA = new Set(["disponivel", "baixando", "baixada"]);

export const TEXTOS = {
  ajuste: "Procurar atualizações ao abrir o app",
  dica: "Consulta a página de versões do projeto no GitHub. Nada é baixado sem o seu clique.",
  procurar: "Procurar agora",
  baixar: "Baixar e instalar",
  reiniciar: "Reiniciar e atualizar",
  github: "Ver no GitHub",
};

// A linha de estado; vazia em "parado" e em "erro" (o erro vai na linha de alerta).
export function textoEstado(e) {
  if (!e) return "";
  switch (e.estado) {
    case "indisponivel":
      return "A procura de atualizações só funciona no app instalado.";
    case "procurando":
      return "Procurando atualizações…";
    case "atualizado":
      return `Você está na versão mais recente (${e.versaoAtual}).`;
    case "disponivel":
      return `Nova versão disponível: ${e.versaoNova}.`;
    case "baixando":
      return `Baixando a versão ${e.versaoNova}… ${e.progresso ?? 0}%`;
    case "baixada":
      return `Versão ${e.versaoNova} baixada. Reinicie para atualizar.`;
    default:
      return "";
  }
}

export function naFaixa(e) {
  return Boolean(e && NA_FAIXA.has(e.estado));
}

function el(tag, texto, classe) {
  const e = document.createElement(tag);
  if (texto !== undefined) e.textContent = texto;
  if (classe) e.className = classe;
  return e;
}

// --- O estado compartilhado ---

let atual = null;
let erroAcao = null;
let ocupado = false;
let timer = null;
const ouvintes = new Set();

function publicar() {
  for (const o of [...ouvintes]) {
    if (!o.vigente()) {
      ouvintes.delete(o);
      continue;
    }
    o.desenhar();
  }
}

function agendar() {
  for (const o of [...ouvintes]) if (!o.vigente()) ouvintes.delete(o);
  if (timer || ouvintes.size === 0 || !atual || !ACOMPANHAR.has(atual.estado)) return;
  timer = setTimeout(() => {
    timer = null;
    ler();
  }, INTERVALO_MS);
}

async function ler() {
  try {
    atual = await api.atualizacao.estado();
  } catch (e) {
    console.error("Falha ao ler o estado da atualização:", e);
  }
  publicar();
  agendar();
}

// Roda uma ação (procurar, baixar, reiniciar, salvar o ajuste). A leitura logo depois da chamada
// já vê "procurando" ou "baixando" e liga o acompanhamento; o fim da ação lê o estado de novo.
async function acao(chamada) {
  if (ocupado) return;
  ocupado = true;
  erroAcao = null;
  publicar();
  const pedido = chamada();
  ler();
  try {
    await pedido;
  } catch (e) {
    erroAcao = e && e.message ? e.message : "Ocorreu um erro inesperado.";
  } finally {
    ocupado = false;
  }
  await ler();
}

function mensagemDeErro() {
  return erroAcao ?? (atual && atual.erro ? atual.erro.message : null);
}

function inscrever(desenhar, vigente) {
  ouvintes.add({ desenhar, vigente });
  desenhar();
  ler();
}

// Os botões de "disponivel" e "baixada", iguais na faixa e na tela Sobre.
function botoesDaVersao(e) {
  const itens = [];
  if (e.estado === "disponivel" && e.instalavel) {
    const b = el("button", TEXTOS.baixar, "btn sm");
    b.type = "button";
    b.disabled = ocupado;
    b.addEventListener("click", () => acao(() => api.atualizacao.baixar()));
    itens.push(b);
  } else if (e.estado === "disponivel" && typeof e.pagina === "string" && e.pagina.startsWith("https://")) {
    const a = el("a", TEXTOS.github, "btn ghost sm");
    a.href = e.pagina;
    itens.push(a);
  } else if (e.estado === "baixada") {
    const b = el("button", TEXTOS.reiniciar, "btn sm");
    b.type = "button";
    b.disabled = ocupado || e.reiniciando;
    b.addEventListener("click", () => acao(() => api.atualizacao.reiniciar()));
    itens.push(b);
  }
  return itens;
}

function linhas() {
  const status = el("p", undefined, "atualizacao-estado");
  status.setAttribute("role", "status");
  const alerta = el("p", undefined, "erro");
  alerta.setAttribute("role", "alert");
  alerta.hidden = true;
  const acoes = el("div", undefined, "acoes-atualizacao");
  return { status, alerta, acoes };
}

function trocarTexto(no, texto) {
  if (no.textContent !== texto) no.textContent = texto;
}

function mostrarAlerta(alerta, mensagem) {
  trocarTexto(alerta, mensagem ?? "");
  alerta.hidden = !mensagem;
}

// A faixa da barra lateral: só em "disponivel", "baixando" e "baixada".
export function iniciarFaixa(raiz) {
  if (!raiz) return;
  const { status, alerta, acoes } = linhas();
  raiz.replaceChildren(status, alerta, acoes);
  inscrever(
    () => {
      const mostrar = naFaixa(atual);
      raiz.hidden = !mostrar;
      if (!mostrar) return;
      trocarTexto(status, textoEstado(atual));
      mostrarAlerta(alerta, mensagemDeErro());
      acoes.replaceChildren(...botoesDaVersao(atual));
    },
    () => true,
  );
}

let ultimoId = 0;

// A seção "Atualizações" da tela Sobre, dentro de `lugar`. Para de acompanhar quando a tela sai.
export function montarSecao(lugar, vigente) {
  const id = ++ultimoId;
  const caixa = el("input");
  caixa.type = "checkbox";
  caixa.setAttribute("role", "switch");
  caixa.id = `atualizacao-ajuste-${id}`;
  const rotulo = el("label", undefined, "row");
  rotulo.htmlFor = caixa.id;
  rotulo.append(caixa, el("span", TEXTOS.ajuste));
  const dica = el("p", TEXTOS.dica, "dica-ajuste");
  dica.id = `atualizacao-dica-${id}`;
  caixa.setAttribute("aria-describedby", dica.id);
  caixa.addEventListener("change", () => {
    const valor = caixa.checked;
    acao(() => api.atualizacao.salvarConfig({ procurarAoAbrir: valor }));
  });

  const procurar = el("button", TEXTOS.procurar, "btn ghost sm");
  procurar.type = "button";
  procurar.addEventListener("click", () => acao(() => api.atualizacao.procurar()));

  const { status, alerta, acoes } = linhas();
  lugar.replaceChildren(rotulo, dica, status, alerta, acoes);
  lugar.classList.add("atualizacoes");

  inscrever(() => {
    if (!atual) {
      caixa.disabled = true;
      return;
    }
    // Durante a gravação, a caixa fica como o usuário deixou; no fim, o estado lido manda.
    if (!ocupado) caixa.checked = atual.procurarAoAbrir;
    caixa.disabled = ocupado;
    trocarTexto(status, textoEstado(atual));
    mostrarAlerta(alerta, mensagemDeErro());
    const botoes = [];
    if (atual.estado !== "indisponivel") {
      procurar.disabled = ocupado || ACOMPANHAR.has(atual.estado) || atual.estado === "baixada";
      botoes.push(procurar);
    }
    acoes.replaceChildren(...botoes, ...botoesDaVersao(atual));
  }, vigente);
}
