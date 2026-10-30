import { api } from "../../api.js";
import { formatarCentesimos, formatarData, formatarDecimos, formatarDiferenca, formatarReais } from "../../formato.js";
import { graficoBarras, graficoLinhas, graficoRosca, minigrafico } from "../../graficos.js";
import { saudacao } from "../../saudacao.js";
import { dataDoMercado, textoNota } from "../../mercado.js";
import { el } from "../_tabela.js";

// Cor por classe (tokens em telas.css) e dos benchmarks do gráfico de rentabilidade.
const COR_CLASSE = { rf: "c1", acoes: "c2", fii: "c3", ext: "c4", outros: "c5" };
const COR_BENCH = { cdi: "s-c1", ipca: "s-c2", bova11: "s-c3" };
const MESES = [12, 6];

// Data e frase sorteadas uma vez por abertura do app: o redesenho (troca de período, atualização
// de mercado) reaproveita as mesmas.
let saudacaoDaSessao = null;

// Reais inteiros nos eixos e na dica do gráfico de evolução: com centavos o rótulo não cabe na margem.
const reaisInteiros = (centavos) => formatarReais(Math.round(centavos / 100) * 100).replace(/,00$/, "");

function sinal(centesimos) {
  if (centesimos === null || centesimos === undefined) return "";
  if (centesimos > 0) return "up";
  return centesimos < 0 ? "down" : "";
}

function marcar(cartao, valor) {
  const classe = sinal(valor);
  if (classe) cartao.querySelector(".mid").classList.add(classe);
}

function cartao(titulo, valor, rodape, testid) {
  const c = el("div", undefined, "card");
  if (testid) c.dataset.testid = testid;
  c.append(el("b", titulo), el("span", valor, "mid mono"));
  if (rodape !== undefined) c.append(el("small", rodape, "mono"));
  return c;
}

function numeros(p, meses) {
  const n = p.numeros;
  const mes = cartao("No mês", formatarCentesimos(n.mesCentesimos), `CDI ${formatarCentesimos(n.mesCdiCentesimos)}`);
  const periodo = cartao(`Em ${meses} meses`, formatarCentesimos(n.periodoCentesimos), `CDI ${formatarCentesimos(n.periodoCdiCentesimos)}`, "numero-periodo");
  marcar(mes, n.mesCentesimos);
  marcar(periodo, n.periodoCentesimos);
  const rendeu = cartao(`Rendeu em ${meses} meses`, n.rendeuCentavos === null ? "—" : formatarReais(n.rendeuCentavos));
  marcar(rendeu, n.rendeuCentavos);
  const caixa = el("div", undefined, "numeros");
  caixa.append(
    cartao("Patrimônio total", formatarReais(n.patrimonioCentavos), `aportado ${formatarReais(n.aportadoCentavos)}`, "numero-patrimonio"),
    mes,
    periodo,
    rendeu,
    cartao("Total aportado", formatarReais(n.aportadoCentavos)),
  );
  return caixa;
}

function secao(titulo, ...filhos) {
  const s = el("section", undefined, "card bloco");
  s.append(el("h2", titulo, "bloco-titulo"), ...filhos);
  return s;
}

function legenda(itens) {
  const ul = el("ul", undefined, "legenda");
  for (const { rotulo, classe, texto, tracejada } of itens) {
    const li = el("li");
    const marca = el("i", undefined, `marca ${classe}${tracejada ? " tracejada" : ""}`);
    marca.setAttribute("aria-hidden", "true");
    li.append(marca, el("span", rotulo), el("small", texto ?? "", "mono"));
    ul.append(li);
  }
  return ul;
}

function rosca(resumo) {
  const classes = resumo.classes.filter((c) => c.totalAtualCentavos > 0);
  const fatias = classes.map((c) => ({ rotulo: c.rotulo, valor: c.totalAtualCentavos, classe: `f-${COR_CLASSE[c.classe] ?? "c5"}` }));
  const itens = classes.map((c) => ({
    rotulo: c.rotulo,
    classe: `f-${COR_CLASSE[c.classe] ?? "c5"}`,
    texto: `${formatarDecimos(c.percentualDecimos)}${c.alvoPercentual === null || c.alvoPercentual === undefined ? "" : ` · alvo ${c.alvoPercentual}%`}`,
  }));
  const corpo = el("div", undefined, "rosca-corpo");
  corpo.append(graficoRosca(fatias, "Distribuição do patrimônio por classe"), legenda(itens));
  return secao("Por classe", corpo);
}

function cartaoClasse(c, porClasse) {
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
  const serie = porClasse.find((s) => s.classe === c.classe)?.patrimonio.filter(Number.isFinite) ?? [];
  if (serie.length > 1) a.append(minigrafico(serie, `s-${COR_CLASSE[c.classe] ?? "c5"}`));
  return a;
}




function graficosRentabilidade(p) {
  const temHistorico = p.pontos.some((x) => x.retornoCentesimos !== null && x.retornoCentesimos !== undefined);
  if (!temHistorico) {
    return [secao("Rentabilidade", el("p", "Ainda não há histórico para comparar.", "hint"))];
  }
  const series = [{ rotulo: "Carteira", valores: p.pontos.map((x) => x.acumuladoCentesimos), classe: "s-carteira" }];
  const itens = [{ rotulo: "Carteira", classe: "s-carteira" }];
  for (const b of p.benchmarks) {
    const classe = COR_BENCH[b.codigo] ?? "s-bench";
    itens.push({ rotulo: b.rotulo, classe, tracejada: true, texto: b.valores ? "" : "sem dados" });
    if (b.valores) series.push({ rotulo: b.rotulo, valores: b.valores, classe, tracejada: true });
  }
  const acumulada = secao(
    "Rentabilidade acumulada",
    graficoLinhas(p.datas, series, formatarCentesimos, "Rentabilidade acumulada da carteira contra CDI, IPCA e BOVA11"),
    legenda(itens),
  );
  const mensal = secao(
    "Mês a mês",
    graficoBarras(p.datas, p.pontos.map((x) => x.retornoCentesimos), formatarCentesimos, "Rentabilidade mês a mês"),
  );
  return [acumulada, mensal];
}

function lista(titulo, testid, linhas) {
  const ul = el("ul", undefined, "lista");
  if (linhas.length === 0) ul.append(el("li", "Nada por aqui ainda.", "hint"));
  else ul.append(...linhas);
  const s = secao(titulo, ul);
  s.dataset.testid = testid;
  return s;
}

function linhaPar(esquerda, direita, classe) {
  const li = el("li", undefined, "row");
  li.append(esquerda, el("span", direita, `mono ${classe ?? ""}`.trim()));
  return li;
}

function listas(p) {
  const instituicoes = p.instituicoes.map((i) => linhaPar(el("span", i.nome), formatarDecimos(i.percentualDecimos)));
  const altas = p.variacoes.filter((v) => v.variacaoDecimos > 0).slice(0, 3);
  const quedas = p.variacoes.filter((v) => v.variacaoDecimos < 0).slice(-3).reverse();
  const fmt = (v) => `${v.variacaoDecimos > 0 ? "▲" : "▼"} ${formatarDecimos(Math.abs(v.variacaoDecimos))}`;
  const variacoes = [...altas, ...quedas].map((v) => linhaPar(el("span", v.ticker), fmt(v), v.variacaoDecimos > 0 ? "up" : "down"));
  const vencimentos = p.vencimentos.map((v) => {
    const a = el("a", v.nome);
    a.href = `#/investimentos/${v.id}`;
    return linhaPar(a, formatarData(v.vencimento));
  });
  const caixa = el("div", undefined, "listas");
  caixa.append(
    lista("Por instituição", "lista-instituicoes", instituicoes),
    lista("Altas e quedas no mês", "lista-variacoes", variacoes),
    lista("Próximos vencimentos", "lista-vencimentos", vencimentos),
  );
  return caixa;
}

function desenhar(p, resumo) {
  const evolucao = secao(
    "Evolução do patrimônio",
    graficoLinhas(
      p.datas,
      [
        { rotulo: "Patrimônio", valores: p.pontos.map((x) => x.patrimonioCentavos), classe: "s-carteira" },
        { rotulo: "Total aportado", valores: p.pontos.map((x) => x.aportadoCentavos), classe: "s-bench", tracejada: true },
      ],
      reaisInteiros,
      "Evolução do patrimônio contra o total aportado",
    ),
    legenda([
      { rotulo: "Patrimônio", classe: "s-carteira" },
      { rotulo: "Total aportado", classe: "s-bench", tracejada: true },
    ]),
  );
  const classes = el("div", undefined, "classes");
  classes.append(...resumo.classes.map((c) => cartaoClasse(c, p.porClasse)));
  const graficos = el("div", undefined, "graficos");
  graficos.append(evolucao, rosca(resumo), ...graficosRentabilidade(p));
  return [graficos, classes, listas(p)];
}

function topo(p, meses, aoTrocar) {
  if (!saudacaoDaSessao) saudacaoDaSessao = saudacao({ agora: new Date(), painel: p, sorteio: Math.random });
  const texto = el("div", undefined, "saudacao-texto");
  const data = el("small", saudacaoDaSessao.data, "mono");
  data.dataset.testid = "data-extenso";
  const frase = el("h1", saudacaoDaSessao.frase, "frase");
  frase.dataset.testid = "saudacao";
  texto.append(data, frase);

  const seletor = el("div", undefined, "seletor");
  seletor.setAttribute("role", "group");
  seletor.setAttribute("aria-label", "Período");
  const botoes = MESES.slice().reverse().map((m) => {
    const b = el("button", `${m} meses`, "btn ghost sm");
    b.type = "button";
    b.setAttribute("aria-pressed", String(m === meses));
    b.addEventListener("click", () => aoTrocar(m, botoes));
    return b;
  });
  seletor.append(...botoes);
  const aportar = el("a", "Tenho dinheiro para aportar", "btn sm");
  aportar.href = "#/aporte";
  const acoes = el("div", undefined, "acoes-topo");
  acoes.append(seletor, aportar);
  const caixa = el("header", undefined, "topo-painel");
  caixa.append(texto, acoes);
  return caixa;
}

export async function painel(elemento, vigente, erro) {
  let resumo;
  let p;
  try {
    [resumo, p] = await Promise.all([api.carteira.resumo(), api.carteira.painel(12)]);
  } catch (e) {
    if (!vigente()) return;
    console.error("Falha ao carregar a carteira:", e);
    erro(elemento, "Carteira");
    return;
  }
  if (!vigente()) return;

  if (resumo.totalAplicadoCentavos === 0) {
    const vazio = el("div", undefined, "vazio");
    const novo = el("a", "Lançar o primeiro investimento", "btn");
    novo.href = "#/investimentos/novo";
    vazio.append(el("p", "Sua carteira está vazia."), novo);
    const data = await dataDoMercado();
    if (!vigente()) return;
    elemento.replaceChildren(el("h1", "Carteira"), vazio, el("p", textoNota(data), "hint"));
    return;
  }

  const data = await dataDoMercado();
  if (!vigente()) return;

  let meses = 12;
  let pedido = 0;
  const corpo = el("div", undefined, "painel-corpo");
  const aoTrocar = async (novo, botoes) => {
    if (novo === meses) return;
    meses = novo;
    botoes.forEach((b, i) => b.setAttribute("aria-pressed", String(MESES.slice().reverse()[i] === novo)));
    const meu = ++pedido;
    let outro;
    try {
      outro = await api.carteira.painel(novo);
    } catch (e) {
      if (!vigente() || meu !== pedido) return;
      console.error("Falha ao carregar o período:", e);
      corpo.replaceChildren(el("p", "Não foi possível carregar o período.", "erro"));
      return;
    }
    if (!vigente() || meu !== pedido) return;
    corpo.replaceChildren(numeros(outro, novo), ...desenhar(outro, resumo));
  };
  corpo.append(numeros(p, 12), ...desenhar(p, resumo));
  elemento.replaceChildren(topo(p, 12, aoTrocar), corpo, el("p", textoNota(data), "hint"));
}
