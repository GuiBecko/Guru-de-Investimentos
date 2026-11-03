// Markdown mínimo das respostas da IA. `analisarMarkdown` é pura (sem DOM); `renderizarMarkdown`
// monta a árvore só com elementos criados e `textContent`. Nada vira HTML: `<script>` é texto.
//
// Bloco: {tipo: "p"|"titulo", trechos} ou {tipo: "ul"|"ol", itens: trechos[]}.
// Trecho: {tipo: "texto"|"negrito", valor: string} ou {tipo: "fonte", valor: número}.

const TITULO = /^#{1,3}\s+(.*)$/;
const ITEM_UL = /^[-*]\s+(.*)$/;
const ITEM_OL = /^\d+\.\s+(.*)$/;
const INLINE = /\*\*(\S(?:.*?\S)?)\*\*|\[(\d+)\]/g;

function trechos(texto, fontes) {
  const saida = [];
  const texto_ = (valor) => {
    if (!valor) return;
    const ultimo = saida[saida.length - 1];
    if (ultimo?.tipo === "texto") ultimo.valor += valor;
    else saida.push({ tipo: "texto", valor });
  };
  let i = 0;
  for (const m of texto.matchAll(INLINE)) {
    texto_(texto.slice(i, m.index));
    if (m[1] !== undefined) saida.push({ tipo: "negrito", valor: m[1] });
    else if (fontes.has(Number(m[2]))) saida.push({ tipo: "fonte", valor: Number(m[2]) });
    else texto_(m[0]);
    i = m.index + m[0].length;
  }
  texto_(texto.slice(i));
  return saida;
}

export function analisarMarkdown(texto, numerosDeFontes = []) {
  const fontes = new Set([...numerosDeFontes].map(Number));
  const blocos = [];
  let paragrafo = null;
  let lista = null;
  const fechar = () => {
    if (paragrafo) blocos.push({ tipo: "p", trechos: trechos(paragrafo.join(" "), fontes) });
    if (lista) blocos.push({ tipo: lista.tipo, itens: lista.itens.map((t) => trechos(t, fontes)) });
    paragrafo = lista = null;
  };
  for (const bruta of String(texto ?? "").split(/\r?\n/)) {
    const linha = bruta.trim();
    if (!linha) {
      fechar();
      continue;
    }
    const titulo = linha.match(TITULO);
    if (titulo) {
      fechar();
      blocos.push({ tipo: "titulo", trechos: trechos(titulo[1], fontes) });
      continue;
    }
    const ul = linha.match(ITEM_UL);
    const ol = ul ? null : linha.match(ITEM_OL);
    if (ul || ol) {
      const tipo = ul ? "ul" : "ol";
      if (paragrafo || lista?.tipo !== tipo) fechar();
      lista ??= { tipo, itens: [] };
      lista.itens.push((ul ?? ol)[1]);
      continue;
    }
    if (lista) fechar();
    paragrafo ??= [];
    paragrafo.push(linha);
  }
  fechar();
  return blocos;
}

function irParaFonte(n) {
  const alvo = document.getElementById(`fonte-${n}`);
  if (!alvo) return;
  alvo.scrollIntoView({ block: "nearest" });
  alvo.focus({ preventScroll: true });
}

function preencher(pai, lista, aoFonte) {
  for (const t of lista) {
    if (t.tipo === "negrito") {
      const b = document.createElement("strong");
      b.textContent = t.valor;
      pai.append(b);
    } else if (t.tipo === "fonte") {
      const sup = document.createElement("sup");
      const a = document.createElement("a");
      a.href = `#fonte-${t.valor}`;
      a.textContent = String(t.valor);
      a.setAttribute("aria-label", `Fonte ${t.valor}`);
      // O hash é do roteador: o clique só rola até a fonte, sem trocar de tela.
      a.addEventListener("click", (e) => {
        e.preventDefault();
        (aoFonte ?? irParaFonte)(t.valor);
      });
      sup.append(a);
      pai.append(sup);
    } else {
      pai.append(document.createTextNode(t.valor));
    }
  }
}

// `aoFonte(n)`, opcional, troca o que o clique num marcador faz (o padrão rola até `#fonte-n`).
export function renderizarMarkdown(blocos, aoFonte) {
  const frag = document.createDocumentFragment();
  for (const b of blocos) {
    if (b.tipo === "ul" || b.tipo === "ol") {
      const lista = document.createElement(b.tipo);
      for (const item of b.itens) {
        const li = document.createElement("li");
        preencher(li, item, aoFonte);
        lista.append(li);
      }
      frag.append(lista);
    } else {
      const p = document.createElement("p");
      if (b.tipo === "titulo") {
        const forte = document.createElement("strong");
        preencher(forte, b.trechos, aoFonte);
        p.append(forte);
      } else {
        preencher(p, b.trechos, aoFonte);
      }
      frag.append(p);
    }
  }
  return frag;
}
