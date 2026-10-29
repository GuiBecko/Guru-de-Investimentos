import { api } from "../../api.js";
import { el, cabecalho, caixaTabela } from "../_tabela.js";

const INTEIRO = /^\d{1,3}$/;
const MSG_CAMPO = "Use um número inteiro de 0 a 100.";

// Modo leitura: tabela com a alocação-alvo e o botão "Ajustar" (ou o convite, se não houver).
export function leitura(alocacao, perfil) {
  const card = el("div", undefined, "card");
  card.append(el("b", "Alocação-alvo"));
  if (!alocacao) {
    card.append(
      el(
        "p",
        perfil
          ? "Você ainda não definiu a sua alocação-alvo. Defina a sua ou use a sugestão do perfil."
          : "Faça o teste de perfil para receber uma sugestão, ou defina a sua.",
      ),
    );
    card.append(botaoEditar("Definir"));
    return card;
  }
  const corpo = el("tbody");
  for (const a of alocacao) {
    const tr = el("tr");
    tr.append(el("td", a.rotulo), el("td", `${a.percentual}%`, "r mono"));
    corpo.append(tr);
  }
  card.append(caixaTabela(cabecalho([["Classe"], ["Alvo", "r"]]), corpo), botaoEditar("Ajustar"));
  return card;
}

function botaoEditar(texto) {
  const b = el("button", texto, "btn ghost");
  b.type = "button";
  b.addEventListener("click", () => {
    window.location.hash = "#/perfil/alocacao";
  });
  return b;
}

// Modo edição: um campo inteiro por classe, soma ao vivo e as ações.
export async function editor(alocacao, perfil, vigente) {
  let classes;
  try {
    classes = (await api.investimentos.catalogo()).classes;
  } catch (e) {
    console.error("Falha ao carregar as classes:", e);
    return null;
  }
  if (!vigente()) return null;

  const base = new Map();
  for (const c of classes) base.set(c.codigo, "0");
  for (const a of alocacao ?? perfil?.alocacaoSugerida ?? []) base.set(a.classe, String(a.percentual));

  const card = el("div", undefined, "card");
  card.append(el("b", "Alocação-alvo"));
  const grade = el("div", undefined, "grid-alocacao");
  const entradas = new Map();
  const erros = new Map();
  for (const c of classes) {
    const campo = el("div", undefined, "field");
    const rotulo = el("label", c.rotulo);
    rotulo.htmlFor = `a-${c.codigo}`;
    const entrada = el("input", undefined, "num");
    entrada.id = `a-${c.codigo}`;
    entrada.type = "text";
    entrada.inputMode = "numeric";
    entrada.autocomplete = "off";
    entrada.value = base.get(c.codigo);
    const erro = el("span", undefined, "erro");
    erro.id = `a-${c.codigo}-erro`;
    erro.hidden = true;
    entrada.addEventListener("input", atualizarSoma);
    entradas.set(c.codigo, entrada);
    erros.set(c.codigo, erro);
    campo.append(rotulo, entrada, erro);
    grade.append(campo);
  }

  const soma = el("output", undefined, "soma mono");
  soma.setAttribute("aria-live", "polite");
  const erroSoma = el("span", undefined, "erro");
  erroSoma.id = "a-soma-erro";
  erroSoma.hidden = true;
  erroSoma.setAttribute("role", "alert");
  const geral = el("div", undefined, "faixa");
  geral.setAttribute("role", "alert");
  geral.hidden = true;

  function atualizarSoma() {
    let total = 0;
    for (const e of entradas.values()) {
      const v = e.value.trim();
      if (INTEIRO.test(v) && Number(v) <= 100) total += Number(v);
    }
    soma.textContent = `Soma: ${total}%`;
    soma.classList.toggle("alerta", total !== 100);
  }

  function limpar() {
    for (const [codigo, e] of entradas) {
      e.removeAttribute("aria-invalid");
      e.removeAttribute("aria-describedby");
      erros.get(codigo).hidden = true;
      erros.get(codigo).textContent = "";
    }
    for (const x of [erroSoma, geral]) {
      x.hidden = true;
      x.textContent = "";
    }
    soma.removeAttribute("aria-describedby");
  }

  function marcar(lista) {
    limpar();
    const soltos = [];
    let primeiro = null;
    for (const { campo, mensagem } of lista) {
      if (campo === "alocacao") {
        erroSoma.textContent = mensagem;
        erroSoma.hidden = false;
        soma.setAttribute("aria-describedby", erroSoma.id);
        continue;
      }
      const e = entradas.get(campo);
      if (!e) {
        soltos.push(mensagem);
        continue;
      }
      erros.get(campo).textContent = mensagem;
      erros.get(campo).hidden = false;
      e.setAttribute("aria-invalid", "true");
      e.setAttribute("aria-describedby", erros.get(campo).id);
      primeiro ??= e;
    }
    if (soltos.length) {
      geral.textContent = soltos.join(" ");
      geral.hidden = false;
    }
    primeiro?.focus();
  }

  const sugerir = el("button", "Usar a sugestão do perfil", "btn ghost");
  sugerir.type = "button";
  sugerir.addEventListener("click", () => {
    for (const e of entradas.values()) e.value = "0";
    for (const a of perfil.alocacaoSugerida) entradas.get(a.classe).value = String(a.percentual);
    limpar();
    atualizarSoma();
  });

  const salvar = el("button", "Salvar", "btn");
  salvar.type = "button";
  salvar.addEventListener("click", async () => {
    const valores = [];
    const invalidos = [];
    for (const [codigo, e] of entradas) {
      const v = e.value.trim();
      if (INTEIRO.test(v) && Number(v) <= 100) valores.push({ classe: codigo, percentual: Number(v) });
      else invalidos.push({ campo: codigo, mensagem: MSG_CAMPO });
    }
    if (invalidos.length) {
      marcar(invalidos);
      return;
    }
    salvar.disabled = true;
    try {
      const retorno = await api.alocacao.salvar(valores);
      if (!vigente()) return;
      if (retorno.length > 0) {
        marcar(retorno);
        salvar.disabled = false;
        return;
      }
      window.location.hash = "#/perfil";
    } catch (e) {
      console.error("Falha ao salvar a alocação:", e);
      if (!vigente()) return;
      marcar([{ campo: "geral", mensagem: "Não foi possível salvar a alocação." }]);
      salvar.disabled = false;
    }
  });

  const cancelar = el("button", "Cancelar", "btn ghost");
  cancelar.type = "button";
  cancelar.addEventListener("click", () => {
    window.location.hash = "#/perfil";
  });

  const acoes = el("div", undefined, "row acoes-resultado");
  if (perfil) acoes.append(sugerir);
  acoes.append(salvar, cancelar);
  atualizarSoma();
  card.append(grade, soma, erroSoma, geral, acoes);
  return card;
}
