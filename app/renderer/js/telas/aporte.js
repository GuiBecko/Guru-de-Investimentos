import { api } from "../api.js";
import { formatarDecimos, formatarReais, formatoDeReais, paraCentavos } from "../formato.js";
import { el } from "./_tabela.js";
import { COR_CLASSE } from "./carteira/painel.js";
import { DIRECOES, MAX_DIRECAO_LIVRE, gaveta } from "./aporte/gaveta.js";

const CONVITE = "Para dividir o aporte, defina a sua alocação-alvo.";
const MENSAGEM_VALOR = "Informe o valor em reais, como 2.000,00.";
const MENSAGEM_ALTO = "Valor alto demais.";
const CONVITE_IA = "Para conversar com a IA sobre cada classe, configure a sua chave.";
const AVISO =
  "Cálculo de rebalanceamento pela sua alocação-alvo, sem vender nada. Investimentos sem cotação contam pelo valor aplicado. Não é recomendação individual de investimento.";

const semPercento = (decimos) => formatarDecimos(decimos).replace(/%$/, "");

export function situacao(p) {
  const d = p.percentualAtualDecimos - 10 * p.alvoPercentual;
  let texto;
  if (p.situacao === "abaixo") texto = `${semPercento(Math.abs(d))} p.p. abaixo do alvo.`;
  else if (p.situacao === "acima" && p.sugeridoCentavos > 0)
    return `${semPercento(Math.abs(d))} p.p. acima do alvo hoje, mas recebe parte porque o aporte é grande em relação à carteira.`;
  else if (p.situacao === "acima") texto = `${semPercento(Math.abs(d))} p.p. acima do alvo.`;
  else if (p.situacao === "perto") texto = "Perto do alvo.";
  else texto = "Fora da sua alocação-alvo.";
  return p.sugeridoCentavos === 0 ? `${texto} Nada neste aporte.` : texto;
}

// `aoConversar(p)`: com a chave, o cartão ganha "Conversar com a IA".
function cartaoParte(p, aoConversar) {
  const c = el("div", undefined, "card");
  c.dataset.testid = "parte";
  const topo = el("div", undefined, "row");
  const rotuloClasse = el("span", undefined, "classe-rotulo");
  const marca = el("i", undefined, `marca f-${COR_CLASSE[p.classe] ?? "c5"}`);
  marca.setAttribute("aria-hidden", "true");
  rotuloClasse.append(marca, el("b", p.rotulo));
  topo.append(rotuloClasse, el("span", formatarReais(p.sugeridoCentavos), "mid mono"));
  c.append(
    topo,
    el("small", `atual ${formatarDecimos(p.percentualAtualDecimos)} → depois ${formatarDecimos(p.percentualDepoisDecimos)} · alvo ${p.alvoPercentual}%`, "mono"),
    el("small", situacao(p)),
  );
  if (aoConversar) {
    const b = el("button", "Conversar com a IA", "btn ghost conversar");
    b.type = "button";
    b.dataset.classe = p.classe;
    b.setAttribute("aria-label", `Conversar com a IA sobre ${p.rotulo}`);
    b.addEventListener("click", () => aoConversar(p));
    c.append(b);
  }
  return c;
}

function conviteIa() {
  const link = el("a", "Configurar IA", "btn");
  link.href = "#/ia";
  const card = el("div", undefined, "card vazio");
  card.append(el("p", CONVITE_IA), link);
  return card;
}

function resultado(a, aoConversar) {
  const caixa = el("div", undefined, "coluna");
  caixa.append(
    el("p", `Patrimônio de ${formatarReais(a.patrimonioCentavos)} passa a ${formatarReais(a.patrimonioCentavos + a.valorCentavos)} com o aporte de ${formatarReais(a.valorCentavos)}.`),
  );
  if (a.patrimonioCentavos === 0) caixa.append(el("p", "Carteira vazia: o aporte foi dividido pela alocação-alvo.", "sub"));
  if (!aoConversar) caixa.append(conviteIa());
  caixa.append(...a.partes.map((p) => cartaoParte(p, aoConversar)));
  return caixa;
}

function convite() {
  const botao = el("a", "Definir alocação-alvo", "btn");
  botao.href = "#/perfil/alocacao";
  const card = el("div", undefined, "card");
  card.append(el("p", CONVITE), botao);
  return card;
}

export async function render(raiz, _parametros, vigente = () => true) {
  const titulo = el("h1", "Próximo aporte");
  let alvo;
  let metas;
  let comChave;
  try {
    // Metas e IA só enriquecem a tela: sem elas, a divisão continua funcionando.
    [alvo, metas, comChave] = await Promise.all([
      api.alocacao.ler(),
      api.metas.listar().catch((e) => {
        console.warn("Metas indisponíveis no Próximo aporte:", e);
        return [];
      }),
      api.ia.config().then(
        (c) => !!c.chave,
        (e) => {
          console.warn("Configuração da IA indisponível no Próximo aporte:", e);
          return false;
        },
      ),
    ]);
  } catch (e) {
    console.error("Falha ao ler a alocação-alvo:", e);
    if (!vigente()) return;
    const p = el("p", "Não foi possível abrir esta tela.");
    p.setAttribute("role", "alert");
    raiz.replaceChildren(titulo, p);
    return;
  }
  if (!vigente()) return;
  if (alvo === null) {
    raiz.replaceChildren(titulo, convite());
    return;
  }

  const form = el("form", undefined, "card form");
  form.noValidate = true;
  const campo = el("div", undefined, "field");
  const input = el("input", undefined, "num");
  input.id = "aporte-valor";
  input.type = "text";
  input.inputMode = "decimal";
  input.autocomplete = "off";
  const rotulo = el("label", "Quanto vai aportar?");
  rotulo.htmlFor = input.id;
  const afixo = el("div", undefined, "affix");
  afixo.append(el("span", "R$"), input);
  const erro = el("span", undefined, "erro");
  erro.id = "aporte-erro";
  erro.hidden = true;
  erro.setAttribute("role", "alert");
  campo.append(rotulo, afixo, erro);
  const campoMeta = el("div", undefined, "field");
  const meta = el("select");
  meta.id = "aporte-meta";
  const rotuloMeta = el("label", "Para qual meta?");
  rotuloMeta.htmlFor = meta.id;
  const nenhuma = el("option", "Nenhuma meta específica");
  nenhuma.value = "";
  meta.append(nenhuma);
  for (const m of metas) {
    const o = el("option", m.nome);
    o.value = String(m.id);
    meta.append(o);
  }
  campoMeta.append(rotuloMeta, meta);
  const botao = el("button", "Dividir", "btn");
  botao.type = "submit";
  const linha = el("div", undefined, "row linha-aporte");
  linha.append(campo, campoMeta);

  const direcoes = el("fieldset", undefined, "direcoes");
  direcoes.append(el("legend", "Para onde quer levar esse dinheiro?"));
  const chips = el("div", undefined, "chips");
  for (const [codigo, rotuloDirecao] of DIRECOES) {
    const caixa = el("input");
    caixa.type = "checkbox";
    caixa.value = codigo;
    caixa.id = `aporte-direcao-${codigo}`;
    const chip = el("label", undefined, "chip-check");
    chip.htmlFor = caixa.id;
    chip.append(caixa, el("span", rotuloDirecao));
    chips.append(chip);
  }
  direcoes.append(chips);
  const campoLivre = el("div", undefined, "field");
  const livre = el("input");
  livre.id = "aporte-direcao";
  livre.type = "text";
  livre.maxLength = MAX_DIRECAO_LIVRE;
  livre.autocomplete = "off";
  const rotuloLivre = el("label", "Outra direção ou detalhe (opcional)");
  rotuloLivre.htmlFor = livre.id;
  campoLivre.append(rotuloLivre, livre);
  // Um fieldset em volta de tudo deixa desabilitar o formulário inteiro durante uma resposta.
  const tudo = el("fieldset", undefined, "envolve");
  tudo.append(linha, direcoes, campoLivre, botao);
  form.append(tudo);

  const saida = el("div");
  saida.setAttribute("aria-live", "polite");
  const rodape = el("p", AVISO, "legal");
  const esquerda = el("div", undefined, "coluna");
  esquerda.append(form, saida, rodape);
  const direita = el("div", undefined, "lado-gaveta");
  const grade = el("div", undefined, "aporte-grade");
  grade.append(esquerda, direita);
  raiz.replaceChildren(titulo, grade);

  // Divisão mostrada agora (para o foco da gaveta) e a gaveta aberta.
  let dividido = null;
  let tokenGaveta = 0;
  let aberta = null;

  function focoAtual(p) {
    return {
      classe: p.classe,
      valorCentavos: dividido.valorCentavos,
      direcoes: [...chips.querySelectorAll("input:checked")].map((c) => c.value),
      direcaoLivre: livre.value,
      metaId: meta.value ? Number(meta.value) : null,
    };
  }

  function travar(emAndamento) {
    tudo.disabled = emAndamento;
    for (const b of saida.querySelectorAll("button.conversar")) b.disabled = emAndamento;
  }

  function fecharGaveta() {
    tokenGaveta++;
    aberta = null;
    direita.replaceChildren();
    grade.classList.remove("com-gaveta");
    travar(false);
    for (const b of saida.querySelectorAll("button.conversar")) b.removeAttribute("aria-expanded");
  }

  function abrirGaveta(p) {
    fecharGaveta();
    const meu = tokenGaveta;
    const foco = focoAtual(p);
    const g = gaveta({
      vigente: () => vigente() && meu === tokenGaveta,
      parte: p,
      foco,
      metaNome: metas.find((m) => m.id === foco.metaId)?.nome ?? null,
      aoFechar: fecharGaveta,
      aoMudar: ({ emAndamento }) => travar(emAndamento),
      aoNova: () => abrirGaveta(p),
    });
    aberta = g;
    direita.replaceChildren(g);
    grade.classList.add("com-gaveta");
    for (const b of saida.querySelectorAll("button.conversar")) {
      if (b.dataset.classe === p.classe) b.setAttribute("aria-expanded", "true");
    }
    g.querySelector("textarea")?.focus();
  }

  // Direção ou meta mudou com a gaveta aberta: vale para uma conversa nova.
  const focoMudou = () => aberta?.dispatchEvent(new CustomEvent("guru:foco-mudou"));
  direcoes.addEventListener("change", focoMudou);
  meta.addEventListener("change", focoMudou);
  livre.addEventListener("input", focoMudou);

  function mostrarErro(mensagem) {
    erro.textContent = mensagem;
    erro.hidden = false;
    input.setAttribute("aria-invalid", "true");
    input.setAttribute("aria-describedby", erro.id);
    input.focus();
  }
  function limpar() {
    erro.hidden = true;
    erro.textContent = "";
    input.removeAttribute("aria-invalid");
    input.removeAttribute("aria-describedby");
  }

  let ocupado = false;
  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    if (ocupado) return;
    limpar();
    const centavos = paraCentavos(input.value);
    if (centavos === null) {
      fecharGaveta();
      saida.replaceChildren();
      mostrarErro(formatoDeReais(input.value) ? MENSAGEM_ALTO : MENSAGEM_VALOR);
      return;
    }
    fecharGaveta();
    dividido = null;
    ocupado = botao.disabled = true;
    try {
      const r = await api.aporte.dividir(centavos);
      if (!vigente()) return;
      if (r.estado === "sem_alvo") {
        raiz.replaceChildren(titulo, convite());
      } else if (r.estado === "invalido") {
        saida.replaceChildren();
        mostrarErro(r.erros.map((x) => x.mensagem).join(" "));
      } else {
        dividido = r.aporte;
        saida.replaceChildren(resultado(r.aporte, comChave ? abrirGaveta : null));
      }
    } catch (err) {
      console.error("Falha ao dividir o aporte:", err);
      if (!vigente()) return;
      saida.replaceChildren();
      mostrarErro("Não foi possível dividir o aporte.");
    } finally {
      ocupado = botao.disabled = false;
    }
  });
}
