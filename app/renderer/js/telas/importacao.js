// Tela "Importar da B3": escolhe a planilha, compara com a carteira, deixa completar os campos
// que faltam nos itens novos e aplica tudo de uma vez.
import { api } from "../api.js";
import { atualizarMercado } from "../mercado.js";
import { atualizarUltimaImportacao } from "../importacao-ultima.js";
import { formatarData, formatarReais, paraCentavos } from "../formato.js";
import { montarDados } from "./formulario-investimento/dados.js";
import { ROTULO_TAXA } from "./formulario-investimento.js";
import { caixaTabela, cabecalho, el } from "./_tabela.js";

const MENSAGEM_EXPIRADA = "A comparação expirou. Escolha a planilha de novo.";
const COLUNAS = [["Situação"], ["Ativo"], ["Corretora"], ["No app hoje", "r"], ["Na planilha B3", "r"], ["O que fazer"]];
const MUDAM = new Set(["adicionar", "usar_b3", "vincular", "remover"]);
const CLASSE_ESTADO = { novo: "", atualizar: "ok", divergente: "wa", fora_da_b3: "no", igual: "mute" };

// Aviso de sucesso que a lista de investimentos mostra ao receber o usuário. Some quando ele sai
// da lista; os redesenhos da própria lista (atualização de mercado) mantêm o aviso.
let aviso = null;
let ouvindo = false;
export function consumirAviso() {
  return aviso;
}
function guardarAviso(texto) {
  aviso = texto;
  if (ouvindo) return;
  ouvindo = true;
  window.addEventListener("hashchange", () => {
    if (aviso && !/^#\/investimentos\/?$/.test(window.location.hash)) aviso = null;
  });
}

const plural = (n, singular, pl) => `${n} ${n === 1 ? singular : pl}`;
const alteracoes = (n) => plural(n, "alteração", "alterações");
const semPrefixo = (centavos) => formatarReais(centavos).replace(/^R\$ /, "");

function opcao(valor, texto) {
  const o = el("option", texto);
  o.value = valor;
  return o;
}

// --- Campos que o usuário completa (mesmos rótulos e padrão `.field` do formulário) ---

const DATAS = new Set(["dataAplicacao", "vencimento", "dataCarencia"]);
const REAIS = new Set(["precoMedioCentavos", "valorAplicadoCentavos"]);
const ORDEM = ["tipo", "emissor", "ticker", "instituicao", "indexador", "taxa", "quantidade", "precoMedioCentavos", "dataAplicacao", "vencimento", "valorAplicadoCentavos", "liquidez", "dataCarencia", "ir"];
const ROTULOS = {
  tipo: "Tipo",
  emissor: "Emissor",
  ticker: "Ticker",
  instituicao: "Corretora ou banco",
  indexador: "Indexador",
  quantidade: "Quantidade",
  precoMedioCentavos: "Preço médio",
  dataAplicacao: "Data da aplicação",
  vencimento: "Vencimento",
  valorAplicadoCentavos: "Valor aplicado",
  liquidez: "Liquidez",
  dataCarencia: "Data da carência",
  ir: "Imposto de renda",
};

// Texto de cada campo a partir da sugestão, no formato que `montarDados` espera.
function textosDaSugestao(s) {
  const reais = (c) => (c === undefined || c === null ? "" : semPrefixo(c));
  return {
    instituicao: s.instituicao,
    emissor: s.emissor,
    ticker: s.ticker,
    indexador: s.indexador,
    taxa: s.taxa === undefined || s.taxa === null ? "" : String(s.taxa).replace(".", ","),
    benchmark: s.benchmark,
    dataAplicacao: s.dataAplicacao,
    vencimento: s.vencimento,
    valor: reais(s.valorAplicadoCentavos),
    quantidade: s.quantidade === undefined || s.quantidade === null ? "" : String(s.quantidade),
    preco: reais(s.precoMedioCentavos),
    liquidez: s.liquidez,
    dataCarencia: s.dataCarencia,
    ir: s.ir,
    observacao: s.observacao,
    metaId: s.metaId,
  };
}

function resumoDaPlanilha(s, tipo) {
  const partes = [tipo?.rotulo ?? "tipo a escolher"];
  const nome = s.ticker ?? s.emissor;
  if (nome) partes.push(nome);
  if (s.quantidade !== undefined && s.quantidade !== null) partes.push(`quantidade ${s.quantidade}`);
  if (s.vencimento) partes.push(`vence em ${formatarData(s.vencimento)}`);
  if (s.valorAplicadoCentavos !== undefined && s.valorAplicadoCentavos !== null) partes.push(`valor ${formatarReais(s.valorAplicadoCentavos)}`);
  return `Da planilha: ${partes.join(" · ")}.`;
}

// Uma linha de item: `tbody.item` com a linha da tabela e, embaixo, os campos a completar.
function criarItem(item, n, catalogo, nomes) {
  const tipos = new Map(catalogo.tipos.map((t) => [t.codigo, t]));
  const sugestao = item.sugestao;
  const faltam = new Set(item.faltando.map((f) => f.campo));

  const tbody = el("tbody", undefined, "item");
  tbody.dataset.chave = item.chave;
  tbody.hidden = item.estado === "igual";

  // --- ações ---
  const rotuloAcao = (codigo) => {
    if (codigo === "adicionar") return "Adicionar";
    if (codigo === "ignorar") return "Ignorar";
    if (codigo === "usar_b3") return item.estado === "atualizar" ? "Atualizar" : "Usar valor da B3";
    if (codigo === "manter") return item.estado === "fora_da_b3" ? "Manter" : "Manter o meu";
    if (codigo === "remover") return "Remover";
    return codigo;
  };
  const acao = el("select");
  acao.id = `i${n}-acao`;
  for (const codigo of item.acoes) {
    if (codigo === "vincular") {
      for (const id of item.investimentos) acao.append(opcao(`vincular:${id}`, `Vincular a ${nomes.get(id) ?? `investimento ${id}`}`));
    } else {
      acao.append(opcao(codigo, rotuloAcao(codigo)));
    }
  }
  acao.value = item.acaoPadrao;
  acao.setAttribute("aria-label", "O que fazer");

  const tr = el("tr");
  const pill = el("span", item.estadoRotulo, `pill ${CLASSE_ESTADO[item.estado] ?? ""}`.trim());
  const tdSituacao = el("td");
  tdSituacao.append(pill);
  const tdAcao = el("td");
  tdAcao.append(acao);
  tr.append(
    tdSituacao,
    el("td", item.ativo),
    el("td", item.instituicao),
    el("td", item.noApp ?? "—", "r mono"),
    el("td", item.naB3 ?? (item.estado === "fora_da_b3" ? "não consta" : "—"), "r mono"),
    tdAcao,
  );

  // --- campos do Adicionar ---
  const detalhe = el("tr", undefined, "detalhe");
  const celula = el("td");
  celula.colSpan = COLUNAS.length;
  detalhe.append(celula);
  const campos = {}; // nome -> { wrap, controle, erro, label }
  const geral = el("span", undefined, "erro");
  geral.hidden = true;
  let tipoControle = null;

  const tipoAtual = () => (tipoControle ? tipos.get(tipoControle.value) : tipos.get(sugestao?.tipo));

  if (sugestao) {
    const resumo = el("small", resumoDaPlanilha(sugestao, tipoAtual()));
    const grade = el("div", undefined, "grid4");
    const t0 = tipoAtual();
    const nomesCampos = ORDEM.filter((nome) => {
      if (faltam.has(nome)) return true;
      if (nome === "dataCarencia") return t0 && t0.forma !== "renda_variavel";
      if (nome === "liquidez") return sugestao.liquidez !== undefined && sugestao.liquidez !== null;
      if (nome === "ir") return sugestao.ir !== undefined && sugestao.ir !== null;
      return false;
    });

    for (const nome of nomesCampos) {
      const id = `i${n}-${nome}`;
      let controle;
      let corpo;
      if (nome === "tipo") {
        controle = el("select");
        controle.append(opcao("", "Escolha…"));
        for (const t of catalogo.tipos) controle.append(opcao(t.codigo, t.rotulo));
        controle.value = sugestao.tipo && tipos.has(sugestao.tipo) ? sugestao.tipo : "";
        tipoControle = controle;
      } else if (nome === "indexador") {
        controle = el("select");
        for (const o of catalogo.indexadores) controle.append(opcao(o.codigo, o.rotulo));
        controle.value = sugestao.indexador ?? "pos";
      } else if (nome === "liquidez" || nome === "ir") {
        controle = el("select");
        for (const o of nome === "liquidez" ? catalogo.liquidezes : catalogo.irs) controle.append(opcao(o.codigo, o.rotulo));
        controle.value = sugestao[nome];
      } else {
        controle = el("input");
        controle.type = DATAS.has(nome) ? "date" : "text";
        if (REAIS.has(nome) || nome === "quantidade" || nome === "taxa") {
          controle.className = "num";
          controle.inputMode = "decimal";
        }
        if (nome === "ticker") controle.className = "caixa-alta";
        if (nome === "taxa") controle.value = sugestao.taxa === undefined || sugestao.taxa === null ? "" : String(sugestao.taxa).replace(".", ",");
        else if (REAIS.has(nome)) controle.value = sugestao[nome] === undefined || sugestao[nome] === null ? "" : semPrefixo(sugestao[nome]);
        else if (sugestao[nome] !== undefined && sugestao[nome] !== null) controle.value = String(sugestao[nome]);
      }
      controle.id = id;
      if (REAIS.has(nome)) {
        const afixo = el("div", undefined, "affix");
        afixo.append(el("span", "R$"), controle);
        corpo = afixo;
        controle.addEventListener("blur", () => {
          const c = paraCentavos(controle.value);
          if (c !== null) controle.value = semPrefixo(c);
        });
      } else if (nome === "taxa") {
        const sufixo = el("span");
        const afixo = el("div", undefined, "affix");
        afixo.append(controle, sufixo);
        corpo = afixo;
        campos[nome] = { sufixo };
      } else {
        corpo = controle;
      }
      const wrap = el("div", undefined, "field");
      const label = el("label", ROTULOS[nome] ?? nome);
      label.htmlFor = id;
      const erro = el("span", undefined, "erro");
      erro.id = `e-${id}`;
      erro.hidden = true;
      wrap.append(label, corpo);
      if (nome === "liquidez" || nome === "ir") {
        const marca = el("span", undefined, "auto");
        marca.id = `${id}-auto`;
        wrap.append(marca);
        campos[nome] = { ...campos[nome], marca };
      }
      wrap.append(erro);
      campos[nome] = { ...campos[nome], wrap, controle, erro, label };
      grade.append(wrap);
    }

    // Texto e estado dos campos que dependem do tipo escolhido.
    const ajustar = () => {
      const t = tipoAtual();
      resumo.textContent = resumoDaPlanilha(sugestao, t);
      const indexador = t?.indexadorForcado ?? campos.indexador?.controle.value ?? sugestao.indexador ?? "pos";
      if (campos.taxa) {
        const [rotulo, sufixo] = ROTULO_TAXA[indexador] ?? ROTULO_TAXA.pos;
        campos.taxa.label.textContent = rotulo;
        campos.taxa.sufixo.textContent = sufixo;
      }
      if (campos.indexador) {
        campos.indexador.controle.disabled = !!t?.indexadorForcado;
        if (t?.indexadorForcado) campos.indexador.controle.value = t.indexadorForcado;
      }
      if (campos.liquidez) {
        campos.liquidez.controle.disabled = !!t?.liquidezForcada;
        if (t?.liquidezForcada) campos.liquidez.controle.value = t.liquidezForcada;
        campos.liquidez.marca.textContent = t?.liquidezForcada ? "preenchido pelo tipo" : "sugerido pelo catálogo, confira";
      }
      if (campos.ir) {
        const fixo = t?.forma === "renda_fixa" && !t.irEditavel;
        campos.ir.controle.disabled = fixo;
        if (fixo && t.irPadrao) campos.ir.controle.value = t.irPadrao;
        campos.ir.marca.textContent = fixo ? "preenchido pelo tipo" : "sugerido pelo catálogo, confira";
      }
      if (campos.dataCarencia) {
        const liquidez = campos.liquidez?.controle.value ?? sugestao.liquidez;
        campos.dataCarencia.wrap.hidden = !(liquidez === "carencia" || faltam.has("dataCarencia"));
      }
      if (campos.emissor && t) campos.emissor.label.textContent = t.forma === "valor" ? "Nome" : "Emissor";
      if (campos.dataAplicacao && t) campos.dataAplicacao.label.textContent = t.forma === "renda_variavel" ? "Data da primeira compra" : "Data da aplicação";
    };
    detalhe.addEventListener("input", ajustar);
    detalhe.addEventListener("change", ajustar);
    ajustar();
    const corpoDetalhe = el("div", undefined, "detalhe-corpo");
    corpoDetalhe.append(resumo, grade, geral);
    celula.append(corpoDetalhe);
  }

  tbody.append(tr, detalhe);

  // --- comportamento ---
  const escolha = () => {
    const [codigo, id] = acao.value.split(":");
    return { codigo, id: id === undefined ? undefined : Number(id) };
  };
  const temErro = () => !geral.hidden || Object.values(campos).some((c) => c.erro && !c.erro.hidden);
  const mostrarDetalhe = () => {
    detalhe.hidden = !((escolha().codigo === "adicionar" && sugestao) || temErro());
  };
  mostrarDetalhe();

  function limparErros() {
    geral.hidden = true;
    geral.textContent = "";
    for (const c of Object.values(campos)) {
      if (!c.erro) continue;
      c.erro.hidden = true;
      c.erro.textContent = "";
      c.controle.removeAttribute("aria-invalid");
      c.controle.removeAttribute("aria-describedby");
    }
    mostrarDetalhe();
  }

  // Devolve o primeiro controle com erro, para o foco.
  function marcarErros(erros) {
    const soltos = [];
    let primeiro = null;
    for (const { campo, mensagem } of erros) {
      const c = campos[campo === "benchmark" ? "taxa" : campo];
      if (!c?.erro) {
        soltos.push(mensagem);
        continue;
      }
      c.erro.textContent = mensagem;
      c.erro.hidden = false;
      c.controle.setAttribute("aria-invalid", "true");
      c.controle.setAttribute("aria-describedby", c.erro.id);
      c.wrap.hidden = false;
      primeiro ??= c.controle;
    }
    if (soltos.length) {
      geral.textContent = soltos.join(" ");
      geral.hidden = false;
    }
    detalhe.hidden = false;
    return primeiro;
  }

  // Dados completos para o `Adicionar`: a sugestão com o que o usuário digitou por cima.
  function dados() {
    const t = tipoAtual();
    if (!t) return { dados: { ...sugestao }, erros: [] };
    const v = textosDaSugestao(sugestao);
    const le = (nome, alvo) => {
      if (campos[nome]?.controle) v[alvo ?? nome] = campos[nome].controle.value;
    };
    for (const nome of ["emissor", "ticker", "instituicao", "indexador", "taxa", "quantidade", "dataAplicacao", "vencimento", "liquidez", "dataCarencia", "ir"]) le(nome);
    le("valorAplicadoCentavos", "valor");
    le("precoMedioCentavos", "preco");
    return montarDados(t, v);
  }

  return {
    chave: item.chave,
    tbody,
    acao: escolha,
    dados,
    marcarErros,
    limparErros,
    aoMudar(fn) {
      acao.addEventListener("change", () => {
        mostrarDetalhe();
        fn();
      });
    },
  };
}

// --- Tela ---

function marcas(itens) {
  const contagem = {};
  for (const i of itens) contagem[i.estado] = (contagem[i.estado] ?? 0) + 1;
  const texto = {
    novo: (c) => plural(c, "novo", "novos"),
    atualizar: (c) => `${c} atualizar`,
    divergente: (c) => plural(c, "divergente", "divergentes"),
    fora_da_b3: (c) => `${c} fora da B3`,
    igual: (c) => plural(c, "igual", "iguais"),
  };
  const caixa = el("span", undefined, "marcas");
  for (const estado of ["novo", "atualizar", "divergente", "fora_da_b3", "igual"]) {
    if (contagem[estado]) caixa.append(el("span", texto[estado](contagem[estado]), `pill ${CLASSE_ESTADO[estado]}`.trim()));
  }
  return caixa;
}

export async function render(raiz, _parametros = [], vigente = () => true) {
  aviso = null;
  let comparacao = null;
  let linhas = [];
  let ocupado = false;

  const titulo = el("h1", "Importar da B3");
  const botaoEscolher = el("button", "Escolher planilha", "btn");
  botaoEscolher.type = "button";
  const topo = el("div", undefined, "row");
  topo.append(titulo, botaoEscolher);
  const alerta = el("div", undefined, "faixa");
  alerta.setAttribute("role", "alert");
  alerta.hidden = true;
  const corpo = el("div", undefined, "importacao-corpo");
  raiz.replaceChildren(topo, alerta, corpo);

  const dizer = (texto) => {
    alerta.textContent = texto ?? "";
    alerta.hidden = !texto;
  };

  function inicial() {
    comparacao = null;
    linhas = [];
    botaoEscolher.textContent = "Escolher planilha";
    corpo.replaceChildren(el("p", "Exporte a posição em investidor.b3.com.br → Extratos → Posição → Baixar Excel.", "hint"));
  }

  function desenhar(dados, catalogo, nomes) {
    comparacao = dados;
    botaoEscolher.textContent = "Escolher outra planilha";
    linhas = dados.itens.map((item, n) => criarItem(item, n, catalogo, nomes));
    const itens = dados.itens;

    const cartao = el("div", undefined, "card row cartao-arquivo");
    const nome = el("span");
    nome.append(el("b", dados.arquivo), " ", el("small", `· Área do Investidor B3 · ${plural(dados.linhasLidas, "linha lida", "linhas lidas")}`));
    cartao.append(nome, marcas(itens));
    const partes = [cartao];

    if (dados.avisos.length) {
      const faixa = el("div", undefined, "faixa");
      const lista = el("ul", undefined, "avisos");
      lista.append(...dados.avisos.map((a) => el("li", a)));
      faixa.append(lista);
      partes.push(faixa);
    }

    const iguais = linhas.filter((_, i) => itens[i].estado === "igual");
    const tabelaCorpo = [];
    for (const l of linhas) tabelaCorpo.push(l.tbody);
    if (iguais.length) {
      const alternar = el("button", plural(iguais.length, "igual", "iguais"), "btn ghost sm");
      alternar.type = "button";
      alternar.setAttribute("aria-expanded", "false");
      alternar.addEventListener("click", () => {
        const abrir = alternar.getAttribute("aria-expanded") !== "true";
        alternar.setAttribute("aria-expanded", String(abrir));
        for (const l of iguais) l.tbody.hidden = !abrir;
      });
      const td = el("td");
      td.colSpan = COLUNAS.length;
      td.append(alternar);
      const tr = el("tr");
      tr.append(td);
      const rodape = el("tbody", undefined, "iguais-linha");
      rodape.append(tr);
      tabelaCorpo.push(rodape);
    }
    const caixa = caixaTabela(cabecalho(COLUNAS), el("tbody"));
    const tabela = caixa.querySelector("table");
    tabela.querySelector(":scope > tbody").remove();
    tabela.append(...tabelaCorpo);
    partes.push(caixa);

    const nota = el("small", "Fundos e previdência normalmente não aparecem na B3. Por padrão, ficam como estão.");
    const cancelar = el("a", "Cancelar", "btn ghost");
    cancelar.href = "#/investimentos";
    const aplicar = el("button", undefined, "btn");
    aplicar.type = "button";
    const botoes = el("span", undefined, "row");
    botoes.append(cancelar, aplicar);
    const rodape = el("div", undefined, "row");
    rodape.append(nota, botoes);
    partes.push(rodape);

    const contar = () => linhas.filter((l) => MUDAM.has(l.acao().codigo)).length;
    const atualizarBotao = () => {
      aplicar.textContent = `Aplicar ${alteracoes(contar())}`;
    };
    for (const l of linhas) l.aoMudar(atualizarBotao);
    atualizarBotao();
    aplicar.addEventListener("click", () => aplicarTudo(aplicar));

    corpo.replaceChildren(...partes);
  }

  async function aplicarTudo(botao) {
    if (ocupado || !comparacao) return;
    dizer(null);
    for (const l of linhas) l.limparErros();

    const decisoes = [];
    let primeiro = null;
    comparacao.itens.forEach((item, i) => {
      if (item.estado === "igual") return;
      const l = linhas[i];
      const { codigo, id } = l.acao();
      const decisao = { chave: item.chave, acao: codigo };
      if (id !== undefined) decisao.idVinculo = id;
      if (codigo === "adicionar" && item.sugestao) {
        const { dados, erros } = l.dados();
        if (erros.length) primeiro ??= l.marcarErros(erros);
        decisao.dados = dados;
      }
      decisoes.push(decisao);
    });
    if (primeiro) {
      dizer("Há campos a corrigir. Nada foi aplicado.");
      primeiro.focus();
      return;
    }

    ocupado = true;
    botao.disabled = true;
    try {
      const resultado = await api.importacao.aplicar(comparacao.token, decisoes);
      if (resultado.erros.length === 0) {
        // Gravou: busca em segundo plano o valor atual do que entrou e atualiza o rodapé do menu.
        atualizarMercado();
        atualizarUltimaImportacao();
        if (!vigente()) return;
        guardarAviso(`${alteracoes(resultado.aplicadas)} ${resultado.aplicadas === 1 ? "aplicada" : "aplicadas"}.`);
        window.location.hash = "#/investimentos";
        return;
      }
      if (!vigente()) return;
      let foco = null;
      for (const { chave, campos } of resultado.erros) {
        const l = linhas.find((x) => x.chave === chave);
        foco ??= l?.marcarErros(campos);
      }
      dizer("Há campos a corrigir. Nada foi aplicado.");
      foco?.focus();
    } catch (e) {
      if (!vigente()) return;
      if (e.code === "IMPORTACAO_EXPIRADA") {
        inicial();
        dizer(MENSAGEM_EXPIRADA);
      } else {
        console.error("Falha ao aplicar a importação:", e);
        dizer("Não foi possível aplicar as alterações.");
      }
    } finally {
      ocupado = false;
      botao.disabled = false;
    }
  }

  async function escolher() {
    if (ocupado) return;
    ocupado = true;
    botaoEscolher.disabled = true;
    try {
      const r = await api.importacao.escolher();
      if (!vigente()) return;
      if (r.cancelado) return;
      const precisaNomes = r.itens.some((i) => i.acoes.includes("vincular"));
      const [catalogo, lista] = await Promise.all([api.investimentos.catalogo(), precisaNomes ? api.investimentos.listar() : []]);
      if (!vigente()) return;
      dizer(null);
      desenhar(r, catalogo, new Map(lista.map((i) => [i.id, i.nome])));
    } catch (e) {
      if (!vigente()) return;
      if (e.code === "ARQUIVO_INVALIDO") {
        dizer(e.message);
      } else {
        console.error("Falha ao ler a planilha:", e);
        dizer("Não foi possível ler a planilha.");
      }
    } finally {
      ocupado = false;
      botaoEscolher.disabled = false;
    }
  }

  botaoEscolher.addEventListener("click", escolher);
  inicial();
}
