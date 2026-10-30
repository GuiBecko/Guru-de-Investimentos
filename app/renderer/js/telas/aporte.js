import { api } from "../api.js";
import { formatarDecimos, formatarReais, formatoDeReais, paraCentavos } from "../formato.js";
import { el } from "./_tabela.js";
import { COR_CLASSE } from "./carteira/painel.js";

const CONVITE = "Para dividir o aporte, defina a sua alocação-alvo.";
const MENSAGEM_VALOR = "Informe o valor em reais, como 2.000,00.";
const MENSAGEM_ALTO = "Valor alto demais.";
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

function cartaoParte(p) {
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
  return c;
}

function resultado(a) {
  const caixa = el("div", undefined, "coluna");
  caixa.append(
    el("p", `Patrimônio de ${formatarReais(a.patrimonioCentavos)} passa a ${formatarReais(a.patrimonioCentavos + a.valorCentavos)} com o aporte de ${formatarReais(a.valorCentavos)}.`),
  );
  if (a.patrimonioCentavos === 0) caixa.append(el("p", "Carteira vazia: o aporte foi dividido pela alocação-alvo.", "sub"));
  caixa.append(...a.partes.map(cartaoParte));
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
  try {
    alvo = await api.alocacao.ler();
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
  const botao = el("button", "Dividir", "btn");
  botao.type = "submit";
  const linha = el("div", undefined, "row");
  linha.append(campo, botao);
  form.append(linha);

  const saida = el("div");
  saida.setAttribute("aria-live", "polite");
  const rodape = el("p", AVISO, "legal");
  raiz.replaceChildren(titulo, form, saida, rodape);

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
      saida.replaceChildren();
      mostrarErro(formatoDeReais(input.value) ? MENSAGEM_ALTO : MENSAGEM_VALOR);
      return;
    }
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
        saida.replaceChildren(resultado(r.aporte));
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
