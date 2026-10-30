// Estado da última atualização dos dados de mercado e o indicador do rodapé do menu.
import { api } from "./api.js";

function el(tag, texto, classe) {
  const e = document.createElement(tag);
  if (texto !== undefined) e.textContent = texto;
  if (classe) e.className = classe;
  return e;
}

// Menor `ultimaData` entre as fontes que têm dados, em "YYYY-MM-DD"; null se nenhuma.
// O IPCA fica de fora de propósito: ele é mensal e sua `ultimaData` é o primeiro dia do mês
// publicado (YYYY-MM-01), que sempre ficaria "atrás" das séries diárias sem indicar defasagem.
export function menorData(fontes) {
  const datas = (fontes ?? []).filter((f) => f.fonte !== "ipca" && f.ultimaData).map((f) => f.ultimaData);
  return datas.length === 0 ? null : datas.reduce((a, b) => (b < a ? b : a));
}

// "2026-10-28" -> "28/10".
export function diaMes(iso) {
  const [, m, d] = String(iso).split("-");
  return `${d}/${m}`;
}

export function textoIndicador(fontes) {
  const data = menorData(fontes);
  return data ? `Dados de mercado de ${diaMes(data)}` : "Dados de mercado: ainda não baixados";
}

function isoLocal(data) {
  const p = (n) => String(n).padStart(2, "0");
  return `${data.getFullYear()}-${p(data.getMonth() + 1)}-${p(data.getDate())}`;
}

// Alguma fonte sem atualização feita hoje (no relógio local)? Sem fontes conhecidas, sim.
export function precisaAtualizar(fontes, agora = new Date()) {
  if (!fontes || fontes.length === 0) return true;
  const hoje = isoLocal(agora);
  return fontes.some((f) => !f.atualizadoEm || isoLocal(new Date(f.atualizadoEm)) !== hoje);
}

// Data dos dados para notas nas telas ("dd/mm"), ou null.
export async function dataDoMercado() {
  try {
    const data = menorData(await api.mercado.status());
    return data ? diaMes(data) : null;
  } catch (e) {
    console.error("Falha ao ler o estado do mercado:", e);
    return null;
  }
}

function mostrarFalhas(fontes) {
  const dialogo = el("dialog");
  const lista = el("ul", undefined, "falhas");
  for (const f of fontes.filter((x) => x.erro)) {
    const li = el("li");
    li.append(el("b", f.rotulo), el("span", f.erro));
    lista.append(li);
  }
  const fechar = el("button", "Fechar", "btn ghost");
  fechar.type = "button";
  fechar.addEventListener("click", () => dialogo.close());
  const acoes = el("div", undefined, "acoes");
  acoes.append(fechar);
  const caixa = el("div", undefined, "caixa");
  caixa.append(el("h2", "Fontes com falha"), lista, acoes);
  dialogo.append(caixa);
  dialogo.addEventListener("close", () => dialogo.remove());
  document.body.append(dialogo);
  dialogo.showModal();
}

export function iniciarMercado(raiz) {
  let fontes = [];
  let atualizando = false;

  function desenhar() {
    const texto = el("span", textoIndicador(fontes));
    texto.dataset.testid = "mercado-texto";
    const partes = [texto];
    if (fontes.some((f) => f.erro)) {
      const marca = el("button", "com falhas", "falhas-marca");
      marca.type = "button";
      marca.addEventListener("click", () => mostrarFalhas(fontes));
      partes.push(marca);
    }
    const botao = el("button", atualizando ? "Atualizando…" : "Atualizar", "btn ghost sm");
    botao.type = "button";
    botao.disabled = atualizando;
    botao.addEventListener("click", atualizar);
    partes.push(botao);
    raiz.replaceChildren(...partes);
  }

  async function atualizar() {
    if (atualizando) return;
    atualizando = true;
    desenhar();
    try {
      fontes = (await api.mercado.atualizar()).fontes;
    } catch (e) {
      console.error("Falha ao atualizar os dados de mercado:", e);
      try {
        fontes = await api.mercado.status();
      } catch (e2) {
        console.error("Falha ao ler o estado do mercado:", e2);
      }
    }
    atualizando = false;
    desenhar();
    // Redesenha a tela atual com os valores novos, sem pisar num formulário aberto.
    if (!/^#\/investimentos\/./.test(window.location.hash)) window.dispatchEvent(new Event("hashchange"));
  }

  desenhar();
  (async () => {
    try {
      fontes = await api.mercado.status();
    } catch (e) {
      console.error("Falha ao ler o estado do mercado:", e);
    }
    if (atualizando) return;
    desenhar();
    if (precisaAtualizar(fontes)) await atualizar();
  })();
}
