import { api } from "../../api.js";
import { el } from "../_tabela.js";

const TOTAL = 12;

function falha(raiz, mensagem) {
  const p = el("p", mensagem);
  p.setAttribute("role", "alert");
  raiz.replaceChildren(el("h1", "Perfil de investidor"), p);
}

export async function render(raiz, vigente) {
  let dados;
  try {
    dados = await api.perfil.questionario();
  } catch (e) {
    console.error("Falha ao carregar o questionário:", e);
    if (vigente()) falha(raiz, "Não foi possível carregar o questionário.");
    return;
  }
  if (!vigente()) return;

  // As respostas vivem só aqui: sair da tela descarta tudo.
  const respostas = Array(TOTAL).fill(null);
  let indice = 0;
  let aviso = "";
  let enviando = false;

  async function enviar() {
    enviando = true;
    desenhar(false);
    try {
      const retorno = await api.perfil.registrar(respostas);
      if (!vigente()) return;
      if (retorno.erros && retorno.erros.length > 0) {
        aviso = retorno.erros.map((x) => x.mensagem).join(" ");
        enviando = false;
        desenhar();
        return;
      }
      window.location.hash = "#/perfil/resultado";
    } catch (e) {
      console.error("Falha ao registrar o perfil:", e);
      if (!vigente()) return;
      aviso = "Não foi possível registrar o seu perfil. Tente de novo.";
      enviando = false;
      desenhar();
    }
  }

  function avancar() {
    if (respostas[indice] === null || enviando) return;
    aviso = "";
    if (indice === TOTAL - 1) {
      enviar();
      return;
    }
    indice += 1;
    desenhar();
  }

  function grupo(pergunta) {
    const g = el("div", undefined, "opcoes");
    g.setAttribute("role", "radiogroup");
    const titulo = el("h2", pergunta.texto, "mid");
    titulo.id = "pergunta-titulo";
    titulo.tabIndex = -1;
    g.setAttribute("aria-labelledby", titulo.id);
    const marcada = respostas[indice];
    const botoes = pergunta.opcoes.map((texto, i) => {
      const b = el("button", texto, "opt");
      b.type = "button";
      b.setAttribute("role", "radio");
      b.setAttribute("aria-checked", String(marcada === i));
      b.tabIndex = marcada === i || (marcada === null && i === 0) ? 0 : -1;
      b.classList.toggle("sel", marcada === i);
      return b;
    });
    function escolher(i, foco) {
      respostas[indice] = i;
      botoes.forEach((b, j) => {
        b.setAttribute("aria-checked", String(j === i));
        b.classList.toggle("sel", j === i);
        b.tabIndex = j === i ? 0 : -1;
      });
      if (foco) botoes[i].focus();
      proxima.disabled = enviando;
    }
    botoes.forEach((b, i) => b.addEventListener("click", () => escolher(i, false)));
    g.addEventListener("keydown", (ev) => {
      const i = botoes.indexOf(document.activeElement);
      if (i < 0) return;
      const n = botoes.length;
      let destino = null;
      if (ev.key === "ArrowDown" || ev.key === "ArrowRight") destino = (i + 1) % n;
      else if (ev.key === "ArrowUp" || ev.key === "ArrowLeft") destino = (i + n - 1) % n;
      else if (ev.key === "Home") destino = 0;
      else if (ev.key === "End") destino = n - 1;
      else if (ev.key === "Enter") {
        ev.preventDefault();
        if (respostas[indice] === null) escolher(i, false);
        avancar();
        return;
      }
      if (destino === null) return;
      ev.preventDefault();
      escolher(destino, true);
    });
    g.append(...botoes);
    return { g, titulo };
  }

  let proxima;

  function desenhar(foco = true) {
    const pergunta = dados.perguntas[indice];
    const n = indice + 1;

    const topo = el("div", undefined, "row");
    topo.append(el("small", `${pergunta.bloco} · pergunta ${n} de ${TOTAL}`), el("small", "Método ANBIMA"));

    const barra = el("div", undefined, `bar p-${indice}`);
    barra.setAttribute("role", "progressbar");
    barra.setAttribute("aria-label", "Progresso do teste");
    barra.setAttribute("aria-valuemin", "0");
    barra.setAttribute("aria-valuemax", String(TOTAL));
    barra.setAttribute("aria-valuenow", String(indice));
    barra.append(el("i"));

    const { g, titulo } = grupo(pergunta);

    const voltar = el("button", "Voltar", "btn ghost");
    voltar.type = "button";
    voltar.disabled = indice === 0 || enviando;
    voltar.addEventListener("click", () => {
      aviso = "";
      indice -= 1;
      desenhar();
    });
    proxima = el("button", n === TOTAL ? "Ver meu perfil" : "Próxima pergunta", "btn");
    proxima.type = "button";
    proxima.disabled = respostas[indice] === null || enviando;
    proxima.addEventListener("click", avancar);
    const acoes = el("div", undefined, "row");
    acoes.append(voltar, proxima);

    const pergCard = el("div", undefined, "card pergunta");
    pergCard.append(titulo, g);
    if (aviso) {
      const faixa = el("div", aviso, "faixa");
      faixa.setAttribute("role", "alert");
      pergCard.append(faixa);
    }
    pergCard.append(acoes);

    const lateral = el("aside", undefined, "card");
    lateral.append(el("b", "Por que perguntamos isso"), el("span", pergunta.porque));
    for (const f of dados.avisosFixos.slice(0, 1)) lateral.append(el("small", f));
    for (const f of dados.avisosFixos.slice(1)) lateral.append(el("div", f, "legal"));

    const colunas = el("div", undefined, "grid2");
    colunas.append(pergCard, lateral);
    raiz.replaceChildren(topo, barra, colunas);
    if (foco) titulo.focus();
  }

  desenhar();
}
