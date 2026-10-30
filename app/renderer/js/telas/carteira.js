import { api } from "../api.js";
import { formatarCentesimos, formatarData, formatarReais } from "../formato.js";
import { graficoLinhas } from "../graficos.js";
import { caixaTabela, cabecalho, celulasValor, el } from "./_tabela.js";
import { dataDoMercado, textoNota } from "../mercado.js";
import { legenda, painel, secao } from "./carteira/painel.js";

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

function seletorBenchmark(benchmarks, escolhido, aoEscolher) {
  const grupo = el("div", undefined, "seletor");
  grupo.setAttribute("role", "group");
  grupo.setAttribute("aria-label", "Benchmark");
  const botoes = benchmarks.map((b) => {
    const botao = el("button", b.codigo === "bova11" || b.codigo === "xfix11" ? b.codigo.toUpperCase() : b.rotulo, "btn ghost sm");
    botao.type = "button";
    botao.setAttribute("aria-pressed", String(b.codigo === escolhido));
    botao.addEventListener("click", () => {
      botoes.forEach((x) => x.setAttribute("aria-pressed", String(x === botao)));
      aoEscolher(b);
    });
    return botao;
  });
  grupo.append(...botoes);
  return grupo;
}

// Card do gráfico: a classe contra o benchmark escolhido. Os cinco benchmarks já vêm no painel,
// então trocar de um para outro só redesenha.
function cartaoBenchmark(p, rotuloClasse) {
  const grafico = el("div");
  const legendaDoGrafico = el("div");
  const desenhar = (b) => {
    const series = [{ rotulo: "Classe", valores: p.pontos.map((x) => x.acumuladoCentesimos), classe: "s-carteira" }];
    const itens = [{ rotulo: `Sua ${rotuloClasse.toLowerCase()}`, classe: "s-carteira" }];
    itens.push({ rotulo: b.rotulo, classe: "s-bench", tracejada: true, texto: b.valores ? "" : "sem dados" });
    if (b.valores) series.push({ rotulo: b.rotulo, valores: b.valores, classe: "s-bench", tracejada: true });
    grafico.replaceChildren(graficoLinhas(p.datas, series, formatarCentesimos, "Rentabilidade acumulada da classe contra o benchmark"));
    const nova = [legenda(itens)];
    if (!b.valores) nova.push(el("p", `${b.rotulo}: sem dados no período.`, "hint"));
    legendaDoGrafico.replaceChildren(...nova);
  };
  const inicial = p.benchmarks.find((b) => b.codigo === p.benchmarkPadrao) ?? p.benchmarks[0];
  desenhar(inicial);
  const cartao = secao("Rentabilidade acumulada", undefined, seletorBenchmark(p.benchmarks, inicial.codigo, desenhar), grafico, legendaDoGrafico);
  cartao.classList.add("bloco-benchmark");
  return cartao;
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

  // O painel da classe só existe para classe com investimento: por isso vem depois da checagem acima.
  let p;
  try {
    p = await api.carteira.painel(12, classe);
  } catch (e) {
    if (!vigente()) return;
    console.error("Falha ao carregar o histórico da classe:", e);
    erro(elemento, "Carteira");
    return;
  }
  if (!vigente()) return;

  const topo = el("div", undefined, "row");
  const titulo = el("div");
  titulo.append(voltar(), el("h1", info.rotulo));
  const valor = el("div", undefined, "total");
  valor.append(el("small", "Valor"), el("div", formatarReais(info.totalAtualCentavos), "mid mono"));
  const periodo = el("div", undefined, "total");
  const centesimos = p.numeros.periodoCentesimos;
  const sinal = centesimos > 0 ? " up" : centesimos < 0 ? " down" : "";
  periodo.append(el("small", "Em 12 meses"), el("div", formatarCentesimos(centesimos), `mid mono${sinal}`));
  const numeros = el("div", undefined, "numeros-classe");
  numeros.append(valor, periodo);
  topo.append(titulo, numeros);

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
  elemento.replaceChildren(topo, cartaoBenchmark(p, info.rotulo), caixaTabela(cabecalho(COLUNAS), tbody, tfoot), el("p", textoNota(data), "hint"));
}

export function render(elemento, parametros = [], vigente = () => true) {
  if (parametros.length > 0) return detalhe(elemento, parametros[0], vigente);
  return painel(elemento, vigente, erro);
}
