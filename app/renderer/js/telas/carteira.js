import { api } from "../api.js";
import { formatarData, formatarReais } from "../formato.js";
import { caixaTabela, cabecalho, celulasValor, el } from "./_tabela.js";
import { dataDoMercado, textoNota } from "../mercado.js";
import { painel } from "./carteira/painel.js";

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
  return painel(elemento, vigente, erro);
}
