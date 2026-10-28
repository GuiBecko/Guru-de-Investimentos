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
