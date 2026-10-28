import { api } from "../api.js";
import { emConstrucao } from "./_em-construcao.js";
import { formatarData, formatarReais } from "../formato.js";

const COLUNAS = [
  ["Investimento"],
  ["Tipo"],
  ["Instituição"],
  ["Rendimento"],
  ["Aplicado", "r"],
  ["Vencimento"],
  ["Liquidez"],
  [""],
];

function el(tag, texto, classe) {
  const e = document.createElement(tag);
  if (texto !== undefined) e.textContent = texto;
  if (classe) e.className = classe;
  return e;
}

function botaoNovo() {
  const a = el("a", "Novo investimento", "btn");
  a.href = "#/investimentos/novo";
  return a;
}

function confirmarApagar(inv) {
  return new Promise((resolve) => {
    const dialogo = el("dialog");
    const titulo = el("h2", `Apagar ${inv.nome}?`);
    const cancelar = el("button", "Cancelar", "btn ghost");
    const apagar = el("button", "Apagar", "btn");
    cancelar.type = apagar.type = "button";
    const acoes = el("div", undefined, "acoes");
    acoes.append(cancelar, apagar);
    const caixa = el("div", undefined, "caixa");
    caixa.append(titulo, acoes);
    dialogo.append(caixa);
    let resposta = false;
    cancelar.addEventListener("click", () => dialogo.close());
    apagar.addEventListener("click", () => {
      resposta = true;
      dialogo.close();
    });
    dialogo.addEventListener("close", () => {
      dialogo.remove();
      resolve(resposta);
    });
    document.body.append(dialogo);
    dialogo.showModal();
  });
}

function linha(inv, apagar) {
  const tr = document.createElement("tr");
  const celulas = [
    inv.nome,
    inv.tipoRotulo,
    inv.instituicao,
    inv.rendimento ?? "—",
    formatarReais(inv.valorAplicadoCentavos),
    formatarData(inv.vencimento),
    inv.liquidezRotulo ?? "—",
  ];
  celulas.forEach((texto, i) => {
    const td = el("td", texto, i === 4 ? "r mono" : i === 5 ? "mono" : "");
    tr.append(td);
  });
  const editar = el("a", "Editar", "btn ghost sm");
  editar.href = `#/investimentos/${inv.id}`;
  const botao = el("button", "Apagar", "btn ghost sm");
  botao.type = "button";
  botao.addEventListener("click", () => apagar(inv));
  const acoes = el("div", undefined, "acoes");
  acoes.append(editar, botao);
  const td = el("td");
  td.append(acoes);
  tr.append(td);
  return tr;
}

function tabela(lista, apagar) {
  const thead = el("thead");
  const trh = el("tr");
  for (const [texto, classe] of COLUNAS) {
    const th = el("th", texto, classe);
    if (!texto) th.setAttribute("aria-label", "Ações");
    trh.append(th);
  }
  thead.append(trh);
  const tbody = el("tbody");
  tbody.append(...lista.map((inv) => linha(inv, apagar)));

  const total = lista.reduce((soma, inv) => soma + inv.valorAplicadoCentavos, 0);
  const tr = el("tr");
  const rotulo = el("td", "Total aplicado");
  rotulo.colSpan = 4;
  tr.append(rotulo, el("td", formatarReais(total), "r mono"), el("td"), el("td"), el("td"));
  const tfoot = el("tfoot");
  tfoot.append(tr);

  const t = el("table", undefined, "tbl");
  t.append(thead, tbody, tfoot);
  const caixa = el("div", undefined, "tabela");
  caixa.append(t);
  return caixa;
}

async function lista(el_) {
  async function desenhar(aviso) {
    let itens;
    try {
      itens = await api.investimentos.listar();
    } catch (e) {
      console.error("Falha ao listar investimentos:", e);
      const p = el("p", "Não foi possível carregar os investimentos.");
      p.setAttribute("role", "alert");
      el_.replaceChildren(el("h1", "Investimentos"), p);
      return;
    }

    async function apagar(inv) {
      if (!(await confirmarApagar(inv))) return;
      try {
        await api.investimentos.apagar(inv.id);
        await desenhar();
      } catch (e) {
        if (e.code !== "NAO_ENCONTRADO") console.error("Falha ao apagar:", e);
        await desenhar(e.message);
      }
    }

    const topo = el("div", undefined, "row");
    topo.append(el("h1", "Investimentos"), botaoNovo());
    const partes = [topo];
    if (aviso) {
      const faixa = el("div", aviso, "faixa");
      faixa.setAttribute("role", "alert");
      partes.push(faixa);
    }
    if (itens.length === 0) {
      const vazio = el("div", undefined, "vazio");
      vazio.append(el("p", "Nenhum investimento ainda."));
      partes.push(vazio);
    } else {
      partes.push(tabela(itens, apagar));
    }
    el_.replaceChildren(...partes);
  }
  await desenhar();
}

export function render(elemento, parametros = []) {
  if (parametros.length > 0) {
    emConstrucao(elemento, "Investimentos");
    return;
  }
  return lista(elemento);
}
