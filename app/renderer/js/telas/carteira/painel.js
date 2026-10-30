import { api } from "../../api.js";
import { formatarCentesimos, formatarData, formatarDecimos, formatarDiferenca, formatarReais } from "../../formato.js";
import { ajustavel, graficoBarras, graficoLinhas, graficoRosca, minigrafico } from "../../graficos.js";
import { modelosAplicaveis, saudacao } from "../../saudacao.js";
import { dataDasFontes, dataDoMercado, statusDoMercado, textoNota } from "../../mercado.js";
import { el } from "../_tabela.js";
import { rotulosPeriodo } from "./periodo.js";
import { altasEQuedas } from "./variacoes.js";

// Cor por classe (tokens em telas.css) e dos benchmarks do gráfico de rentabilidade.
const COR_CLASSE = { rf: "c1", acoes: "c2", fii: "c3", ext: "c4", outros: "c5" };
// BOVA11 em âmbar (c4): o verde (c3) se confundia com a linha da carteira.
const COR_BENCH = { cdi: "s-c1", ipca: "s-c2", bova11: "s-c4" };
const MESES = [12, 6];

// Data e frase sorteadas uma vez por abertura do app: o redesenho (troca de período, atualização
// de mercado) reaproveita as mesmas enquanto a frase continuar aplicável.
let saudacaoDaSessao = null;

// Período escolhido no seletor: sobrevive ao redesenho depois de uma atualização de mercado.
let mesesEscolhidos = 12;

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
  const c = el("div", undefined, testid === "numero-patrimonio" ? "card hero" : "card");
  if (testid) c.dataset.testid = testid;
  c.append(el("b", titulo), el("span", valor, "mid mono"));
  if (rodape !== undefined) c.append(el("small", rodape, "mono"));
  return c;
}

function numeros(p, meses) {
  const n = p.numeros;
  const rotulos = rotulosPeriodo(n.desde, meses);
  const comHistorico = temHistorico(p);
  const mes = cartao("No mês", formatarCentesimos(n.mesCentesimos), `CDI ${formatarCentesimos(n.mesCdiCentesimos)}`);
  const periodo = cartao(rotulos.periodo, comHistorico ? formatarCentesimos(n.periodoCentesimos) : "—", `CDI ${formatarCentesimos(n.periodoCdiCentesimos)}`, "numero-periodo");
  marcar(mes, n.mesCentesimos);
  if (comHistorico) marcar(periodo, n.periodoCentesimos);
  const rendeu = cartao(rotulos.rendeu, n.rendeuCentavos === null ? "—" : formatarReais(n.rendeuCentavos));
  marcar(rendeu, n.rendeuCentavos);
  const caixa = el("div", undefined, "numeros");
  caixa.append(
    cartao("Patrimônio total", formatarReais(n.patrimonioCentavos), `aportado ${formatarReais(n.aportadoCentavos)}`, "numero-patrimonio"),
    mes,
    periodo,
    rendeu,
    cartao("Total aportado", formatarReais(n.aportadoCentavos)),
  );
  ajustarCartoes(caixa);
  return caixa;
}

const transborda = (caixa) => [...caixa.querySelectorAll(".card > *")].some((e) => e.scrollWidth > e.clientWidth + 1);

// Uma linha de cinco cards quando todos os valores cabem; senão 3 + 2 (.apertado). Mede de novo só
// quando a largura muda: a troca de classe mexe na altura, não na largura, então não entra em laço.
function ajustarCartoes(caixa) {
  let largura = -1;
  const observador = new ResizeObserver(([entrada]) => {
    if (!caixa.isConnected) {
      observador.disconnect();
      return;
    }
    const nova = Math.round(entrada.contentRect.width);
    if (nova <= 0 || nova === largura) return;
    largura = nova;
    caixa.classList.remove("apertado");
    if (transborda(caixa)) caixa.classList.add("apertado");
  });
  observador.observe(caixa);
}

export function secao(titulo, subtitulo, ...filhos) {
  const s = el("section", undefined, "card bloco");
  const cab = el("div", undefined, "card-title");
  cab.append(el("h2", titulo, "bloco-titulo"));
  if (subtitulo) cab.append(el("small", subtitulo));
  s.append(cab, ...filhos);
  return s;
}

export function legenda(itens) {
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
  return secao("Por classe", "atual · alvo", corpo);
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

export const temHistorico = (p) => p.pontos.some((x) => x.retornoCentesimos !== null && x.retornoCentesimos !== undefined);

function graficosRentabilidade(p) {
  if (!temHistorico(p)) {
    return [secao("Rentabilidade", undefined, el("p", "Ainda não há histórico para comparar.", "hint"))];
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
    undefined,
    ajustavel((w) => graficoLinhas(p.datas, series, formatarCentesimos, "Rentabilidade acumulada da carteira contra CDI, IPCA e BOVA11", w)),
    legenda(itens),
  );
  const mensal = secao(
    "Mês a mês",
    "rentabilidade da carteira",
    ajustavel((w) => graficoBarras(p.datas, p.pontos.map((x) => x.retornoCentesimos), formatarCentesimos, "Rentabilidade mês a mês", w)),
  );
  return [acumulada, mensal];
}

function lista(titulo, testid, linhas) {
  const ul = el("ul", undefined, "lista");
  if (linhas.length === 0) ul.append(el("li", "Nada por aqui ainda.", "hint"));
  else ul.append(...linhas);
  const s = secao(titulo, undefined, ul);
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
  const variacoes = altasEQuedas(p.variacoes).map((v) => linhaPar(el("span", v.ticker), v.texto, v.classe));
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
    undefined,
    ajustavel((w) =>
      graficoLinhas(
        p.datas,
        [
          { rotulo: "Patrimônio", valores: p.pontos.map((x) => x.patrimonioCentavos), classe: "s-carteira" },
          { rotulo: "Total aportado", valores: p.pontos.map((x) => x.aportadoCentavos), classe: "s-bench", tracejada: true },
        ],
        reaisInteiros,
        "Evolução do patrimônio contra o total aportado",
        w,
      ),
    ),
    legenda([
      { rotulo: "Patrimônio", classe: "s-carteira" },
      { rotulo: "Total aportado", classe: "s-bench", tracejada: true },
    ]),
  );
  const linha1 = el("div", undefined, "graficos g1");
  linha1.append(evolucao, rosca(resumo));
  const classes = el("div", undefined, "classes");
  classes.append(...resumo.classes.map((c) => cartaoClasse(c, p.porClasse)));
  const linha2 = el("div", undefined, "graficos g2");
  linha2.append(...graficosRentabilidade(p));
  return [linha1, classes, linha2, listas(p)];
}

function topo(p, meses, aoTrocar, fontes) {
  // Sorteia de novo só se a frase da sessão deixou de valer (ex.: uma fonte passou a ter erro).
  const valeAinda = saudacaoDaSessao && modelosAplicaveis(p, saudacaoDaSessao.agora, fontes).includes(saudacaoDaSessao.texto);
  if (!valeAinda) {
    const agora = saudacaoDaSessao?.agora ?? new Date();
    saudacaoDaSessao = { ...saudacao({ agora, painel: p, sorteio: Math.random, fontes }), agora };
  }
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
    resumo = await api.carteira.resumo();
    if (!vigente()) return;
    if (resumo.totalAplicadoCentavos > 0) p = await api.carteira.painel(mesesEscolhidos);
  } catch (e) {
    if (!vigente()) return;
    console.error("Falha ao carregar a carteira:", e);
    erro(elemento, "Carteira");
    return;
  }
  if (!vigente()) return;

  if (!p) {
    const vazio = el("div", undefined, "vazio");
    const novo = el("a", "Lançar o primeiro investimento", "btn");
    novo.href = "#/investimentos/novo";
    vazio.append(el("p", "Sua carteira está vazia."), novo);
    const data = await dataDoMercado();
    if (!vigente()) return;
    elemento.replaceChildren(el("h1", "Carteira"), vazio, el("p", textoNota(data), "hint"));
    return;
  }

  const fontes = await statusDoMercado();
  if (!vigente()) return;

  let meses = mesesEscolhidos;
  let pedido = 0;
  // Depois de uma falha os gráficos dão lugar ao aviso: clicar no botão já pressionado tenta de novo.
  let falhou = false;
  const caixaNumeros = el("div", undefined, "numeros-caixa");
  const area = el("div", undefined, "painel-corpo");
  const aoTrocar = async (novo, botoes) => {
    if (novo === meses && !falhou) return;
    const anterior = meses;
    const marcarBotoes = (m) => botoes.forEach((b, i) => b.setAttribute("aria-pressed", String(MESES.slice().reverse()[i] === m)));
    meses = novo;
    marcarBotoes(novo);
    const meu = ++pedido;
    let outro;
    try {
      outro = await api.carteira.painel(novo);
    } catch (e) {
      if (!vigente() || meu !== pedido) return;
      console.error("Falha ao carregar o período:", e);
      meses = anterior;
      marcarBotoes(anterior);
      falhou = true;
      const aviso = el("p", "Não foi possível carregar o período.", "erro");
      aviso.setAttribute("role", "alert");
      area.replaceChildren(aviso);
      return;
    }
    if (!vigente() || meu !== pedido) return;
    falhou = false;
    mesesEscolhidos = novo;
    caixaNumeros.replaceChildren(numeros(outro, novo));
    area.replaceChildren(...desenhar(outro, resumo));
  };
  caixaNumeros.append(numeros(p, meses));
  area.append(...desenhar(p, resumo));
  elemento.replaceChildren(topo(p, meses, aoTrocar, fontes), caixaNumeros, area, el("p", textoNota(dataDasFontes(fontes)), "hint"));
}
