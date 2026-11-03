// Painel "O que a IA está vendo": fontes da web numeradas e os seus dados enviados na conversa.
import { formatarData } from "../../formato.js";
import { el } from "../_tabela.js";

// `page_age` vem como a Anthropic mandou ("October 22, 2026", ISO...). Mostra em dd/mm/aaaa
// quando dá para ler a data; senão, do jeito que veio.
export function dataDaFonte(texto) {
  if (!texto) return null;
  if (/^\d{4}-\d{2}-\d{2}$/.test(texto)) return formatarData(texto);
  const ms = Date.parse(texto);
  if (Number.isNaN(ms)) return texto;
  return new Date(ms).toLocaleDateString("pt-BR");
}

const plural = (n) => (n === 1 ? "1 fonte" : `${n} fontes`);

function fonteWeb(f) {
  const item = el("div", undefined, "src");
  item.id = `fonte-${f.numero}`;
  item.tabIndex = -1;
  item.dataset.testid = "fonte";
  const topo = el("div", undefined, "row");
  topo.append(el("span", String(f.numero), "n"));
  const data = dataDaFonte(f.data);
  if (data) topo.append(el("small", data));
  item.append(topo);
  if (/^https:\/\//.test(f.url)) {
    // Abre no navegador pelo handler de janelas do main (só https).
    const a = el("a", f.titulo || f.url);
    a.href = f.url;
    a.target = "_blank";
    a.rel = "noopener noreferrer";
    item.append(a);
  } else {
    item.append(el("b", f.titulo || f.url));
  }
  item.append(el("span", f.dominio, "dom"));
  if (f.trecho) item.append(el("span", f.trecho, "took"));
  return item;
}

function fonteInterna(f) {
  const item = el("div", undefined, "src");
  const topo = el("div", undefined, "row");
  topo.append(el("b", f.rotulo));
  const lado = el("span", undefined, "row");
  if (f.dataDados) lado.append(el("small", formatarData(f.dataDados)));
  if (f.antigo) lado.append(el("span", "antigo", "pill wa"));
  topo.append(lado);
  item.append(topo);
  return item;
}

// `web`: fontes da resposta em foco; `internas`: as da conversa, ou null numa conversa nova.
export function painel(web, internas) {
  const card = el("aside", undefined, "card vendo");
  card.setAttribute("aria-label", "O que a IA está vendo");
  const topo = el("div", undefined, "row");
  topo.append(el("b", "O que a IA está vendo"), el("small", plural(web.length + (internas?.length ?? 0))));
  const lista = el("div", undefined, "fontes");
  lista.dataset.testid = "fontes-web";
  if (web.length) lista.append(...web.map(fonteWeb));
  else lista.append(el("small", "Nenhuma fonte da web nesta resposta."));
  const dados = el("div", undefined, "fontes");
  dados.dataset.testid = "seus-dados";
  if (internas === null) dados.append(el("small", "Aparecem aqui quando a conversa começar."));
  else if (internas.length) dados.append(...internas.map(fonteInterna));
  else dados.append(el("small", "Nenhum dado seu foi enviado nesta conversa."));
  card.append(topo, el("small", "Fontes da web", "rotulo-sec"), lista, el("small", "Seus dados", "rotulo-sec"), dados);
  return card;
}
