// Gráficos em SVG escritos à mão. A geometria é pura (testada em Node); a construção usa o DOM só
// dentro das funções. Cores e tracejados vêm de classes de CSS (telas.css), nunca do JS, porque a
// CSP proíbe o atributo style. A dica de hover é um <g class="dica"> desenhado dentro do SVG.

const NS = "http://www.w3.org/2000/svg";

const arred = (n) => String(Math.round(n * 100) / 100);

// ---------- geometria pura ----------

export function escala([min, max], [a, b]) {
  if (min === max) return () => (a + b) / 2;
  return (v) => a + ((v - min) / (max - min)) * (b - a);
}

export function dominioComZero(valores) {
  let min = 0;
  let max = 0;
  for (const v of valores) {
    if (!Number.isFinite(v)) continue;
    if (v < min) min = v;
    if (v > max) max = v;
  }
  return [min, max];
}

export function caminhoLinha(pontos) {
  let d = "";
  let quebrado = true;
  for (const p of pontos) {
    if (!p) {
      quebrado = true;
      continue;
    }
    d += `${quebrado ? "M" : "L"}${arred(p.x)} ${arred(p.y)}`;
    quebrado = false;
  }
  return d;
}

const ponto = (r, ang) => `${arred(r * Math.cos(ang))} ${arred(r * Math.sin(ang))}`;

// Centro em (0, 0); começa no topo, sentido horário.
export function arcosRosca(fatias, raio, raioInterno) {
  const total = fatias.reduce((s, v) => s + (v > 0 ? v : 0), 0);
  if (total <= 0) return fatias.map(() => "");
  let acum = 0;
  return fatias.map((v) => {
    if (!(v > 0)) return "";
    const frac = v / total;
    if (frac >= 1) {
      return (
        `M0 ${arred(-raio)}A${raio} ${raio} 0 1 1 0 ${arred(raio)}A${raio} ${raio} 0 1 1 0 ${arred(-raio)}Z` +
        `M0 ${arred(-raioInterno)}A${raioInterno} ${raioInterno} 0 1 0 0 ${arred(raioInterno)}` +
        `A${raioInterno} ${raioInterno} 0 1 0 0 ${arred(-raioInterno)}Z`
      );
    }
    const a0 = -Math.PI / 2 + acum * 2 * Math.PI;
    acum += frac;
    const a1 = -Math.PI / 2 + acum * 2 * Math.PI;
    const grande = frac > 0.5 ? 1 : 0;
    return (
      `M${ponto(raio, a0)}A${raio} ${raio} 0 ${grande} 1 ${ponto(raio, a1)}` +
      `L${ponto(raioInterno, a1)}A${raioInterno} ${raioInterno} 0 ${grande} 0 ${ponto(raioInterno, a0)}Z`
    );
  });
}

// Pontos sem vizinho (null dos dois lados): a linha não os desenha, então viram círculos.
export function pontosIsolados(pontos) {
  return pontos.filter((p, i) => p && !pontos[i - 1] && !pontos[i + 1]);
}

const ehNum = (v) => Number.isFinite(v);

export function indiceMaisProximo(xs, x) {
  let melhor = 0;
  for (let i = 1; i < xs.length; i++) {
    if (Math.abs(xs[i] - x) < Math.abs(xs[melhor] - x)) melhor = i;
  }
  return melhor;
}

// "2026-10-28" -> "10/26"
export function mesAno(iso) {
  const [a, m] = String(iso).split("-");
  return `${m}/${String(a).slice(2)}`;
}

// ---------- construção (DOM) ----------

function el(nome, attrs = {}, texto) {
  const e = document.createElementNS(NS, nome);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  if (texto !== undefined) e.textContent = texto;
  return e;
}

function svgBase(w, h, rotuloAcessivel, classe = "chart") {
  const svg = el("svg", { viewBox: `0 0 ${w} ${h}`, role: "img", "aria-label": rotuloAcessivel, class: classe });
  return svg;
}

const M = { e: 56, d: 12, t: 12, b: 26 };
const W = 640;
const H = 240;

function eixoY(svg, y, dominio, formatarValor) {
  const ticks = dominio[0] === dominio[1] ? [Math.round(dominio[0])] : [0, 1, 2, 3].map((i) => Math.round(dominio[0] + ((dominio[1] - dominio[0]) * i) / 3));
  for (const v of ticks) {
    const py = y(v);
    svg.append(el("line", { class: "grade", x1: M.e, x2: W - M.d, y1: py, y2: py }));
    svg.append(el("text", { class: "eixo", x: M.e - 6, y: py + 3, "text-anchor": "end" }, formatarValor(v)));
  }
  if (dominio[0] < 0 && dominio[1] > 0) {
    const y0 = y(0);
    svg.append(el("line", { class: "zero", x1: M.e, x2: W - M.d, y1: y0, y2: y0 }));
  }
}

function eixoX(svg, datas, xs) {
  const passo = Math.max(1, Math.ceil(datas.length / 7));
  datas.forEach((d, i) => {
    if (i % passo !== 0 && i !== datas.length - 1) return;
    if (i !== datas.length - 1 && datas.length - 1 - i < passo / 2) return;
    svg.append(el("text", { class: "eixo", x: xs[i], y: H - 8, "text-anchor": "middle" }, mesAno(d)));
  });
}

// Dica desenhada no SVG: retângulo + linhas de texto posicionados por atributos x/y.
function criarDica() {
  const g = el("g", { class: "dica", visibility: "hidden", "pointer-events": "none" });
  const fundo = el("rect", { class: "dica-fundo", rx: 4, ry: 4, width: 0, height: 0 });
  g.append(fundo);
  return {
    g,
    mostrar(x, y, linhas, largura, altura) {
      g.replaceChildren(fundo);
      const w = Math.max(...linhas.map((l) => l.length)) * 6.4 + 16;
      const h = linhas.length * 15 + 8;
      const px = Math.min(Math.max(x + 10 > largura - w ? x - w - 10 : x + 10, 0), largura - w);
      const py = Math.min(Math.max(y, 0), altura - h);
      fundo.setAttribute("x", px);
      fundo.setAttribute("y", py);
      fundo.setAttribute("width", w);
      fundo.setAttribute("height", h);
      linhas.forEach((l, i) => g.append(el("text", { class: i === 0 ? "dica-titulo" : "dica-texto", x: px + 8, y: py + 17 + i * 15 }, l)));
      g.setAttribute("visibility", "visible");
    },
    esconder() {
      g.setAttribute("visibility", "hidden");
    },
  };
}

function xDoMouse(svg, ev) {
  const r = svg.getBoundingClientRect();
  return r.width ? ((ev.clientX - r.left) / r.width) * W : 0;
}

function ligarHover(svg, xs, dica, linhasDe, guia) {
  if (xs.length === 0) return;
  const alvo = el("rect", { class: "alvo", x: M.e, y: M.t, width: W - M.e - M.d, height: H - M.t - M.b });
  alvo.addEventListener("mousemove", (ev) => {
    const i = indiceMaisProximo(xs, xDoMouse(svg, ev));
    if (guia) {
      guia.setAttribute("x1", xs[i]);
      guia.setAttribute("x2", xs[i]);
      guia.setAttribute("visibility", "visible");
    }
    dica.mostrar(xs[i], M.t, linhasDe(i), W, H);
  });
  alvo.addEventListener("mouseleave", () => {
    dica.esconder();
    if (guia) guia.setAttribute("visibility", "hidden");
  });
  svg.append(alvo);
}

export function graficoLinhas(datas, series, formatarValor, rotuloAcessivel) {
  const svg = svgBase(W, H, rotuloAcessivel);
  const dominio = dominioComZero(series.flatMap((s) => s.valores));
  const y = escala(dominio, [H - M.b, M.t]);
  const xs = datas.map((_, i) => escala([0, Math.max(1, datas.length - 1)], [M.e, W - M.d])(i));
  eixoY(svg, y, dominio, formatarValor);
  eixoX(svg, datas, xs);
  const guia = el("line", { class: "guia", x1: 0, x2: 0, y1: M.t, y2: H - M.b, visibility: "hidden" });
  svg.append(guia);
  for (const s of series) {
    const pontos = s.valores.map((v, i) => (ehNum(v) ? { x: xs[i], y: y(v) } : null));
    svg.append(el("path", { class: `serie ${s.classe}${s.tracejada ? " tracejada" : ""}`, d: caminhoLinha(pontos), fill: "none" }));
    for (const p of pontosIsolados(pontos)) svg.append(el("circle", { class: `ponto ${s.classe}`, cx: arred(p.x), cy: arred(p.y), r: 2 }));
  }
  const dica = criarDica();
  ligarHover(
    svg,
    xs,
    dica,
    (i) => [
      mesAno(datas[i]),
      ...series.map((s) => `${s.rotulo}: ${ehNum(s.valores[i]) ? formatarValor(s.valores[i]) : "—"}`),
    ],
    guia,
  );
  svg.append(dica.g);
  return svg;
}

export function graficoBarras(datas, valores, formatarValor, rotuloAcessivel) {
  const svg = svgBase(W, H, rotuloAcessivel);
  const dominio = dominioComZero(valores);
  const y = escala(dominio, [H - M.b, M.t]);
  const banda = (W - M.e - M.d) / Math.max(1, datas.length);
  const xs = datas.map((_, i) => M.e + banda * (i + 0.5));
  eixoY(svg, y, dominio, formatarValor);
  eixoX(svg, datas, xs);
  const y0 = y(0);
  valores.forEach((v, i) => {
    if (!ehNum(v) || v === 0) return;
    const topo = Math.min(y(v), y0);
    svg.append(
      el("rect", {
        class: v > 0 ? "barra up" : "barra down",
        x: arred(xs[i] - banda * 0.35),
        y: arred(topo),
        width: arred(banda * 0.7),
        height: arred(Math.abs(y(v) - y0)),
      }),
    );
  });
  const dica = criarDica();
  ligarHover(svg, xs, dica, (i) => [mesAno(datas[i]), ehNum(valores[i]) ? formatarValor(valores[i]) : "—"], null);
  svg.append(dica.g);
  return svg;
}

export function graficoRosca(fatias, rotuloAcessivel) {
  const R = 90;
  const svg = el("svg", { viewBox: `${-R - 4} ${-R - 4} ${2 * R + 8} ${2 * R + 8}`, role: "img", "aria-label": rotuloAcessivel, class: "chart rosca" });
  const arcos = arcosRosca(fatias.map((f) => f.valor), R, R * 0.62);
  const total = fatias.reduce((s, f) => s + (f.valor > 0 ? f.valor : 0), 0);
  const centro = el("text", { class: "rosca-texto", x: 0, y: 4, "text-anchor": "middle" }, "");
  fatias.forEach((f, i) => {
    if (!arcos[i]) return;
    const p = el("path", { class: `fatia ${f.classe}`, d: arcos[i] });
    p.addEventListener("mouseenter", () => {
      centro.textContent = `${f.rotulo} ${Math.round((f.valor / total) * 100)}%`;
    });
    p.addEventListener("mouseleave", () => {
      centro.textContent = "";
    });
    svg.append(p);
  });
  svg.append(centro);
  return svg;
}

export function minigrafico(valores, classe) {
  const w = 80;
  const h = 24;
  const svg = el("svg", { viewBox: `0 0 ${w} ${h}`, role: "img", "aria-label": "Tendência", class: "mini" });
  const dominio = [Math.min(...valores), Math.max(...valores)];
  const y = escala(dominio, [h - 2, 2]);
  const x = escala([0, Math.max(1, valores.length - 1)], [1, w - 1]);
  svg.append(el("path", { class: `serie ${classe}`, d: caminhoLinha(valores.map((v, i) => ({ x: x(i), y: y(v) }))), fill: "none" }));
  return svg;
}
