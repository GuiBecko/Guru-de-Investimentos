import { formatarData, formatarReais, formatarRentabilidade } from "../formato.js";

// Peças de DOM compartilhadas pelas telas de tabela.
export function el(tag, texto, classe) {
  const e = document.createElement(tag);
  if (texto !== undefined) e.textContent = texto;
  if (classe) e.className = classe;
  return e;
}

export function cabecalho(colunas) {
  const tr = el("tr");
  for (const [texto, classe] of colunas) {
    const th = el("th", texto, classe);
    if (!texto) th.setAttribute("aria-label", "Ações");
    tr.append(th);
  }
  const thead = el("thead");
  thead.append(tr);
  return thead;
}

export function caixaTabela(thead, tbody, tfoot) {
  const t = el("table", undefined, "tbl");
  t.append(thead, tbody);
  if (tfoot) t.append(tfoot);
  const caixa = el("div", undefined, "tabela");
  caixa.append(t);
  return caixa;
}

// Marca do valor sem fonte: ativo que não tem preço oficial x ticker sem cotação (ou sem dados).
export function marcaSemValor(origem) {
  return origem === "sem_fonte" ? "sem fonte de preço" : "sem cotação";
}

// Células "Atual" e "Rentab." de um investimento. Sem fonte ou sem dados o "Atual" é o valor aplicado,
// marcado "sem fonte de preço" ou "sem cotação", e a rentabilidade é "—". O valor é bruto, antes do IR.
export function celulasValor(inv) {
  const atual = el("td", formatarReais(inv.valorAtualCentavos), "r mono");
  const rentab = el("td", undefined, "r mono");
  if (inv.origemValor === "sem_fonte" || inv.origemValor === "sem_dados") {
    atual.append(el("span", marcaSemValor(inv.origemValor), "sem-cotacao"));
    rentab.textContent = "—";
    atual.title = `${inv.origemValorRotulo}; valor bruto, antes do IR`;
  } else {
    rentab.textContent = formatarRentabilidade(inv.rentabilidadeDecimos);
    if (inv.rentabilidadeDecimos > 0) rentab.classList.add("up");
    else if (inv.rentabilidadeDecimos < 0) rentab.classList.add("down");
    atual.title = `${inv.origemValorRotulo} de ${formatarData(inv.dataValor)}; valor bruto, antes do IR`;
  }
  return [atual, rentab];
}
