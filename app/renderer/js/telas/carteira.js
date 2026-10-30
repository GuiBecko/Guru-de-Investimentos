import { api } from "../api.js";
import { formatarData, formatarDecimos, formatarDiferenca, formatarReais } from "../formato.js";
import { caixaTabela, cabecalho, celulasValor, el } from "./_tabela.js";
import { dataDoMercado, textoNota } from "../mercado.js";

const COLUNAS = [
  ["Investimento"],
  ["Instituição"],
  ["Rendimento"],
  ["Aplicado", "r"],
  ["Atual", "r"],
  ["Rentab.", "r"],
  ["Vencimento"],
  ["Liquidez"],
];

function voltar() {
  const a = el("a", "Carteira ›", "migalha");
  a.href = "#/carteira";
  return a;
}

function erro(elemento, titulo) {
  const p = el("p", "Não foi possível carregar a carteira.");
  p.setAttribute("role", "alert");
  elemento.replaceChildren(el("h1", titulo), p);
}

function card(c) {
  // Classe sem investimento não tem tela de detalhe: o card não é link.
  const temDetalhe = c.quantidade > 0;
  const a = temDetalhe ? el("a", undefined, "card click") : el("div", undefined, "card");
  if (temDetalhe) a.href = `#/carteira/${c.classe}`;
  const n = c.quantidade === 1 ? "1 investimento" : `${c.quantidade} investimentos`;
  const base = el("div", undefined, "row");
  base.append(el("small", n), el("span", formatarDecimos(c.percentualDecimos), "mono"));
  a.append(el("b", c.rotulo), el("span", formatarReais(c.totalAtualCentavos), "mid mono"), base);
  if (c.alvoPercentual !== null && c.alvoPercentual !== undefined) {
    const alvo = el("div", undefined, "row");
    alvo.append(el("small", `alvo ${c.alvoPercentual}%`), el("span", formatarDiferenca(c.percentualDecimos, c.alvoPercentual), "mono"));
    a.append(alvo);
  }
  return a;
}

async function painel(elemento, vigente) {
  let resumo;
  try {
    resumo = await api.carteira.resumo();
  } catch (e) {
    if (!vigente()) return;
    console.error("Falha ao carregar a carteira:", e);
    erro(elemento, "Carteira");
    return;
  }
  if (!vigente()) return;

  const partes = [el("h1", "Carteira")];
  if (resumo.totalAplicadoCentavos === 0) {
    const vazio = el("div", undefined, "vazio");
    const novo = el("a", "Lançar o primeiro investimento", "btn");
    novo.href = "#/investimentos/novo";
    vazio.append(el("p", "Sua carteira está vazia."), novo);
    partes.push(vazio);
  } else {
    const patrimonio = el("div", undefined, "total");
    patrimonio.append(el("small", "Patrimônio"), el("div", formatarReais(resumo.totalAtualCentavos), "mid mono"));
    const aplicado = el("div", undefined, "total");
    aplicado.append(el("small", "Total aplicado"), el("div", formatarReais(resumo.totalAplicadoCentavos), "mono"));
    const total = el("div", undefined, "patrimonio");
    total.append(patrimonio, aplicado);
    const cards = el("div", undefined, "classes");
    cards.append(...resumo.classes.map(card));
    partes.push(total, cards);
  }
  const data = await dataDoMercado();
  if (!vigente()) return;
  partes.push(el("p", textoNota(data), "hint"));
  elemento.replaceChildren(...partes);
}

function linha(inv) {
  const tr = el("tr");
  const celulas = [
    inv.nome,
    inv.instituicao,
    inv.rendimento ?? "—",
    formatarReais(inv.valorAplicadoCentavos),
    formatarData(inv.vencimento),
    inv.liquidezRotulo ?? "—",
  ];
  celulas.forEach((texto, i) => {
    tr.append(el("td", texto, i === 3 ? "r mono" : i === 4 ? "mono" : ""));
    if (i === 3) tr.append(...celulasValor(inv));
  });
  return tr;
}

async function detalhe(elemento, classe, vigente) {
  let itens;
  let resumo;
  try {
    [itens, resumo] = await Promise.all([api.investimentos.listar(), api.carteira.resumo()]);
  } catch (e) {
    if (!vigente()) return;
    console.error("Falha ao carregar a classe:", e);
    erro(elemento, "Carteira");
    return;
  }
  if (!vigente()) return;

  const doClasse = itens.filter((i) => i.classe === classe);
  const info = resumo.classes.find((c) => c.classe === classe);
  if (doClasse.length === 0 || !info) {
    const vazio = el("div", undefined, "vazio");
    vazio.append(el("p", "Nenhum investimento nesta classe."), voltar());
    elemento.replaceChildren(vazio);
    return;
  }

  const topo = el("div", undefined, "row");
  const titulo = el("div");
  titulo.append(voltar(), el("h1", info.rotulo));
  const valor = el("div", undefined, "total");
  valor.append(el("small", "Total da classe"), el("div", formatarReais(info.totalAtualCentavos), "mid mono"));
  topo.append(titulo, valor);

  const tbody = el("tbody");
  tbody.append(...doClasse.map(linha));
  const rotulo = el("td", "Total aplicado / atual");
  rotulo.colSpan = 3;
  const rodape = el("tr");
  rodape.append(
    rotulo,
    el("td", formatarReais(info.totalAplicadoCentavos), "r mono"),
    el("td", formatarReais(info.totalAtualCentavos), "r mono"),
    el("td"),
    el("td"),
    el("td"),
  );
  const tfoot = el("tfoot");
  tfoot.append(rodape);
  const data = await dataDoMercado();
  if (!vigente()) return;
  elemento.replaceChildren(topo, caixaTabela(cabecalho(COLUNAS), tbody, tfoot), el("p", textoNota(data), "hint"));
}

export function render(elemento, parametros = [], vigente = () => true) {
  if (parametros.length > 0) return detalhe(elemento, parametros[0], vigente);
  return painel(elemento, vigente);
}
