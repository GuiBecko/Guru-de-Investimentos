import { api } from "../../api.js";
import { formatarPrazo, formatarReais, paraCentavos } from "../../formato.js";
import { el } from "../_tabela.js";
import { MENSAGEM_VALOR } from "../formulario-investimento/dados.js";

const TEXTO_VAZIO = "Metas ajudam a acompanhar objetivos com valor e prazo, como a entrada de um imóvel.";
const TONS = { no_caminho: "up", concluida: "up", atrasada: "down", vencida: "down", sem_aportes: "muted" };
const semPrefixo = (centavos) => formatarReais(centavos).replace(/^R\$ /, "");

function novaMeta() {
  const a = el("a", "+ Nova meta", "dashed");
  a.href = "#/perfil/metas/nova";
  return a;
}

function cartao(m) {
  const a = el("a", undefined, "card click");
  a.href = `#/perfil/metas/${m.id}`;
  const titulo = el("div", undefined, "row");
  titulo.append(el("b", m.nome), el("small", formatarPrazo(m.prazo)));
  const partes = [titulo];
  if (m.aporteNecessarioCentavos) partes.push(el("small", `aporte necessário ${formatarReais(m.aporteNecessarioCentavos)}/mês`));
  partes.push(el("span", `${formatarReais(m.acumuladoCentavos)} / ${formatarReais(m.valorAlvoCentavos)}`, "mono"));
  // Largura em passos de 5%: a CSP não aceita style=, então a barra usa uma classe por passo.
  const passo = Math.min(20, Math.floor(m.percentualDecimos / 50));
  const barra = el("div", undefined, `bar p5-${passo}`);
  barra.append(el("i"));
  a.append(...partes, barra, el("small", m.mensagem, TONS[m.situacao] ?? "muted"));
  return a;
}

// Coluna "Metas" do painel: um card por meta e, por último, o card "+ Nova meta".
export function coluna(metas) {
  const partes = [el("b", "Metas", "titulo-campo")];
  if (metas.length === 0) partes.push(el("p", TEXTO_VAZIO));
  partes.push(...metas.map(cartao), novaMeta());
  const c = el("div", undefined, "coluna");
  c.append(...partes);
  return c;
}

function confirmarApagar(nome) {
  return new Promise((resolve) => {
    const dialogo = el("dialog");
    const titulo = el("h2", `Apagar a meta ${nome}? Os investimentos vinculados continuam na carteira.`);
    const cancelar = el("button", "Cancelar", "btn ghost");
    const apagar = el("button", "Apagar meta", "btn");
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

function aviso(raiz, titulo, mensagem) {
  const p = el("p", mensagem);
  p.setAttribute("role", "alert");
  const voltar = el("a", "Voltar ao perfil", "btn ghost");
  voltar.href = "#/perfil";
  raiz.replaceChildren(el("h1", titulo), p, voltar);
}

// Formulário de meta: `alvo` é "nova" ou o id da meta.
export async function formulario(raiz, alvo, vigente) {
  const id = alvo === "nova" ? undefined : Number(alvo);
  const edicao = id !== undefined;
  const titulo = edicao ? "Editar meta" : "Nova meta";
  if (edicao && !Number.isSafeInteger(id)) return aviso(raiz, titulo, "Esta meta não existe mais.");

  let atual;
  if (edicao) {
    try {
      atual = await api.metas.ler(id);
    } catch (e) {
      if (!vigente()) return;
      if (e.code === "NAO_ENCONTRADO") return aviso(raiz, titulo, "Esta meta não existe mais.");
      console.error("Falha ao abrir a meta:", e);
      return aviso(raiz, titulo, "Não foi possível abrir a meta.");
    }
    if (!vigente()) return;
  }

  const form = el("form", undefined, "card form");
  form.noValidate = true;
  const geral = el("div", undefined, "faixa");
  geral.setAttribute("role", "alert");
  geral.hidden = true;

  const campos = {};
  function campo(nome, rotulo, tipo, classe) {
    const wrap = el("div", undefined, `field ${classe ?? ""}`.trim());
    const input = el("input");
    input.id = `m-${nome}`;
    input.type = tipo;
    const label = el("label", rotulo);
    label.htmlFor = input.id;
    const erro = el("span", undefined, "erro");
    erro.id = `e-${nome}`;
    erro.hidden = true;
    wrap.append(label, input, erro);
    campos[nome] = { input, erro };
    return wrap;
  }
  const grade = el("div", undefined, "grid4");
  grade.append(campo("nome", "Nome", "text", "span2"), campo("valorAlvoCentavos", "Valor da meta", "text"), campo("prazo", "Prazo", "month"));
  campos.nome.input.maxLength = 60;
  campos.valorAlvoCentavos.input.inputMode = "decimal";
  campos.valorAlvoCentavos.input.className = "num";
  if (atual) {
    campos.nome.input.value = atual.nome;
    campos.valorAlvoCentavos.input.value = semPrefixo(atual.valorAlvoCentavos);
    campos.prazo.input.value = atual.prazo;
  }

  const salvar = el("button", "Salvar", "btn");
  salvar.type = "submit";
  const cancelar = el("a", "Cancelar", "btn ghost");
  cancelar.href = "#/perfil";
  const acoes = el("span", undefined, "row");
  acoes.append(cancelar, salvar);
  const rodape = el("div", undefined, "row");
  const apagar = el("button", "Apagar meta", "btn ghost");
  apagar.type = "button";
  apagar.hidden = !edicao;
  rodape.append(apagar, acoes);
  form.append(geral, grade, rodape);

  function limpar() {
    geral.hidden = true;
    for (const { input, erro } of Object.values(campos)) {
      erro.hidden = true;
      erro.textContent = "";
      input.removeAttribute("aria-invalid");
      input.removeAttribute("aria-describedby");
    }
  }

  function marcar(erros) {
    const soltos = [];
    let primeiro = null;
    for (const { campo: nome, mensagem } of erros) {
      const c = campos[nome];
      if (!c) {
        soltos.push(mensagem);
        continue;
      }
      c.erro.textContent = mensagem;
      c.erro.hidden = false;
      c.input.setAttribute("aria-invalid", "true");
      c.input.setAttribute("aria-describedby", c.erro.id);
      primeiro ??= c.input;
    }
    if (soltos.length) {
      geral.textContent = soltos.join(" ");
      geral.hidden = false;
    }
    (primeiro ?? (soltos.length ? geral : null))?.focus?.();
  }

  let ocupado = false;
  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    if (ocupado) return;
    limpar();
    const textoValor = campos.valorAlvoCentavos.input.value;
    const centavos = paraCentavos(textoValor);
    if (centavos === null && textoValor.trim() !== "") {
      marcar([{ campo: "valorAlvoCentavos", mensagem: MENSAGEM_VALOR }]);
      return;
    }
    // Valor vazio vai como 0: o núcleo responde "Informe o valor da meta." junto dos demais erros.
    const dados = {
      nome: campos.nome.input.value,
      valorAlvoCentavos: centavos ?? 0,
      prazo: campos.prazo.input.value,
    };
    ocupado = salvar.disabled = true;
    try {
      const { erros = [] } = edicao ? await api.metas.atualizar(id, dados) : await api.metas.criar(dados);
      if (!vigente()) return;
      if (erros.length) marcar(erros);
      else window.location.hash = "#/perfil";
    } catch (err) {
      console.error("Falha ao salvar a meta:", err);
      if (!vigente()) return;
      marcar([{ campo: "geral", mensagem: err.code === "NAO_ENCONTRADO" ? err.message : "Não foi possível salvar a meta." }]);
    } finally {
      ocupado = salvar.disabled = false;
    }
  });

  apagar.addEventListener("click", async () => {
    if (!(await confirmarApagar(atual.nome)) || !vigente()) return;
    try {
      await api.metas.apagar(id);
      if (!vigente()) return;
      window.location.hash = "#/perfil";
    } catch (err) {
      console.error("Falha ao apagar a meta:", err);
      if (!vigente()) return;
      marcar([{ campo: "geral", mensagem: err.code === "NAO_ENCONTRADO" ? err.message : "Não foi possível apagar a meta." }]);
    }
  });

  const topo = el("div", undefined, "row");
  topo.append(el("h1", titulo));
  raiz.replaceChildren(topo, form);
}
