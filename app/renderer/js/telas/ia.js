import { api } from "../api.js";
import { el } from "./_tabela.js";

const LIMITE_INSTRUCOES = 4000;
const FUTUROS = ["OpenAI", "Google Gemini", "Modelo local (Ollama)"];
const PRIVACIDADE = [
  ["apenasPercentuais", "Apenas percentuais"],
  ["nomesInstituicoes", "Nome das instituições"],
  ["buscaWeb", "Buscar na web"],
];

function linhaUso(uso) {
  const custo = (uso.custoCentesimosDolar / 100).toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  const volume = uso.tokens >= 1000 ? `${numero(Math.round(uso.tokens / 1000))} mil` : numero(uso.tokens);
  return `Uso este mês: ~${volume} tokens (≈ US$ ${custo}), sem contar buscas na web.`;
}

// Contador de chamadas de atualizar(): só a mais recente pode redesenhar a tela.
let sequencia = 0;

const numero = (n) => n.toLocaleString("pt-BR");

function chave(marcada, aoMudar, id) {
  const input = el("input");
  input.type = "checkbox";
  input.setAttribute("role", "switch");
  input.id = id;
  input.checked = marcada;
  input.addEventListener("change", () => aoMudar(input.checked, input));
  return input;
}

// Faixa de erro com role=alert, usada no lugar do campo quando o erro não é de um campo só.
function faixaErro() {
  const e = el("span", undefined, "erro");
  e.setAttribute("role", "alert");
  e.hidden = true;
  return e;
}

function mostrarErro(erro, input, mensagem) {
  erro.textContent = mensagem ?? "";
  erro.hidden = !mensagem;
  if (!input) return;
  if (mensagem) {
    input.setAttribute("aria-invalid", "true");
    input.setAttribute("aria-describedby", erro.id);
  } else {
    input.removeAttribute("aria-invalid");
    input.removeAttribute("aria-describedby");
  }
}

const juntar = (erros) => erros.map((e) => e.mensagem).join(" ");

export async function render(raiz, _parametros, vigente) {
  // Texto digitado e ainda não salvo, que um redesenho não pode perder.
  const editando = new Map();
  const rascunho = { chave: "", instrucoes: null };
  // Valores atuais da configuração: cada controle muda aqui na hora e toda gravação envia estes valores.
  let estado = null;
  let fila = Promise.resolve();
  const doConfig = (c) => ({ modelo: c.modelo, instrucoes: c.instrucoes, apenasPercentuais: c.apenasPercentuais, nomesInstituicoes: c.nomesInstituicoes, buscaWeb: c.buscaWeb });

  // Grava a configuração em fila, sempre com o `estado` do momento do envio.
  function salvarConfig(mudancas, aoErro, aoOk) {
    const anterior = Object.fromEntries(Object.keys(mudancas).map((k) => [k, estado[k]]));
    Object.assign(estado, mudancas);
    const tarefa = fila.then(async () => {
      try {
        const erros = await api.ia.salvarConfig({ ...estado });
        if (!vigente()) return;
        if (erros?.length) {
          Object.assign(estado, anterior);
          return aoErro(juntar(erros));
        }
        aoOk?.();
        await atualizar();
      } catch (e) {
        console.error("Falha ao salvar a configuração da IA:", e);
        if (!vigente()) return;
        Object.assign(estado, anterior);
        aoErro("Não foi possível salvar. Tente de novo.");
      }
    });
    fila = tarefa;
    return tarefa;
  }

  async function carregar() {
    const [config, contexto] = await Promise.all([api.ia.config(), api.ia.contexto()]);
    return { config, contexto };
  }

  async function atualizar() {
    const meu = ++sequencia;
    try {
      const dados = await carregar();
      if (!vigente() || meu !== sequencia) return;
      estado ??= doConfig(dados.config);
      desenhar(dados);
    } catch (e) {
      console.error("Falha ao carregar a configuração da IA:", e);
      if (!vigente()) return;
      const p = el("p", "Não foi possível abrir a configuração da IA.");
      p.setAttribute("role", "alert");
      raiz.replaceChildren(el("h1", "Configurar IA"), p);
    }
  }

  // Roda uma gravação; erros de campo voltam para `aoErro`, o resto vira mensagem geral.
  async function gravar(acao, aoErro, aoOk) {
    try {
      const erros = await acao();
      if (!vigente()) return;
      if (erros?.length) return aoErro(juntar(erros));
      aoOk?.();
      await atualizar();
    } catch (e) {
      console.error("Falha ao salvar a configuração da IA:", e);
      if (!vigente()) return;
      aoErro("Não foi possível salvar. Tente de novo.");
    }
  }

  function provedores({ config }) {
    const col = el("div", undefined, "coluna");
    const modelo = config.modelos.find((m) => m.codigo === config.modelo);
    const claude = el("div", undefined, "opt sel");
    const topo = el("div", undefined, "row");
    topo.append(el("b", "Claude (Anthropic)"));
    if (config.chave) topo.append(el("span", "ativo", "pill"));
    claude.append(topo, el("small", `Modelo: ${modelo?.rotulo ?? config.modelo}${config.chave ? ` · chave ${config.chave}` : ""}`));
    col.append(claude);
    for (const nome of FUTUROS) {
      const c = el("div", undefined, "opt");
      const l = el("div", undefined, "row");
      l.append(el("b", nome), el("span", "V2", "pill mute"));
      c.append(l);
      col.append(c);
    }
    col.append(formularioChave(config), seletorModelo(config), el("small", linhaUso(config.uso)), privacidade(config));
    return col;
  }

  function formularioChave(config) {
    const form = el("form", undefined, "card form");
    form.noValidate = true;
    const campo = el("div", undefined, "field");
    const input = el("input");
    input.type = "password";
    input.id = "ia-chave";
    input.autocomplete = "off";
    input.value = rascunho.chave;
    input.addEventListener("input", () => {
      rascunho.chave = input.value;
    });
    input.spellcheck = false;
    const label = el("label", "Chave da API da Anthropic");
    label.htmlFor = input.id;
    const erro = faixaErro();
    erro.id = "e-ia-chave";
    campo.append(label, input, erro);
    const salvar = el("button", "Salvar chave", "btn");
    salvar.type = "submit";
    const acoes = el("div", undefined, "row acoes-chave");
    acoes.append(salvar);
    if (config.chave) {
      const remover = el("button", "Remover chave", "btn ghost");
      remover.type = "button";
      remover.addEventListener("click", async () => {
        remover.disabled = true;
        try {
          await api.ia.removerChave();
          if (!vigente()) return;
          await atualizar();
        } catch (e) {
          console.error("Falha ao remover a chave:", e);
          if (!vigente()) return;
          remover.disabled = false;
          mostrarErro(erro, input, "Não foi possível remover a chave.");
        }
      });
      acoes.append(remover);
    }
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      if (salvar.disabled) return;
      mostrarErro(erro, input, null);
      salvar.disabled = true;
      await gravar(() => api.ia.salvarChave(input.value), (m) => mostrarErro(erro, input, m), () => {
        rascunho.chave = "";
      });
      if (vigente()) salvar.disabled = false;
    });
    form.append(campo, acoes, el("small", "A chave fica guardada só neste computador, no cofre de senhas do sistema."));
    return form;
  }

  function seletorModelo(config) {
    const campo = el("div", undefined, "field");
    const select = el("select");
    select.id = "ia-modelo";
    for (const m of config.modelos) {
      const o = el("option", m.rotulo);
      o.value = m.codigo;
      select.append(o);
    }
    select.value = estado.modelo;
    const label = el("label", "Modelo");
    label.htmlFor = select.id;
    const erro = faixaErro();
    erro.id = "e-ia-modelo";
    select.addEventListener("change", () => salvarConfig({ modelo: select.value }, (m) => {
      select.value = estado.modelo;
      mostrarErro(erro, select, m);
    }));
    campo.append(label, select, erro);
    return campo;
  }

  function privacidade(config) {
    const card = el("div", undefined, "card");
    card.append(el("b", "Privacidade"));
    const erro = faixaErro();
    erro.id = "e-ia-privacidade";
    PRIVACIDADE.forEach(([nome, rotulo]) => {
      const linha = el("label", undefined, "row");
      const input = chave(estado[nome], (ligada, i) => salvarConfig({ [nome]: ligada }, (m) => {
        i.checked = !ligada;
        mostrarErro(erro, undefined, m);
      }), `ia-priv-${nome}`);
      linha.append(el("span", rotulo), input);
      card.append(linha);
    });
    card.append(erro);
    return card;
  }

  function instrucoes({ config }) {
    const card = el("div", undefined, "card");
    const topo = el("div", undefined, "row");
    topo.append(el("b", "Instruções para a sua IA"), el("small", "vai em toda conversa"));
    const area = el("textarea");
    area.id = "ia-instrucoes";
    area.rows = 4;
    area.value = rascunho.instrucoes ?? estado.instrucoes;
    const label = el("label", "Instruções para a sua IA", "so-leitor");
    label.htmlFor = area.id;
    const contador = el("small", undefined, "contador");
    const contar = () => {
      contador.textContent = `${numero([...area.value].length)}/${numero(LIMITE_INSTRUCOES)}`;
    };
    contar();
    area.addEventListener("input", () => {
      rascunho.instrucoes = area.value === estado.instrucoes ? null : area.value;
      contar();
    });
    const erro = faixaErro();
    erro.id = "e-ia-instrucoes";
    const salvar = el("button", "Salvar instruções", "btn");
    salvar.type = "button";
    salvar.addEventListener("click", async () => {
      mostrarErro(erro, area, null);
      salvar.disabled = true;
      await salvarConfig({ instrucoes: area.value }, (m) => mostrarErro(erro, area, m), () => {
        rascunho.instrucoes = null;
      });
      if (vigente()) salvar.disabled = false;
    });
    card.append(topo, label, area, contador, erro, salvar);
    return card;
  }

  function bloco(p, indice) {
    const b = el("div", undefined, "ctx-block");
    b.dataset.testid = `parte-${p.parte}`;
    const cab = el("div", undefined, "hd");
    const titulo = el("span");
    titulo.append(el("b", `${indice + 1} · ${p.rotulo}`), document.createTextNode(" "), el("small", p.resumo));
    if (p.editado) titulo.append(document.createTextNode(" "), el("span", "editado por você", "pill"));
    const lado = el("span", undefined, "row");
    const erro = faixaErro();
    erro.id = `e-ia-parte-${p.parte}`;
    const salvarParte = (ligada, texto, aoOk) => gravar(() => api.ia.salvarParte(p.parte, ligada, texto), (m) => mostrarErro(erro, undefined, m), aoOk);
    if (p.parte !== "instrucoes") {
      if (p.editado) {
        const voltar = el("button", "voltar ao automático", "link");
        voltar.type = "button";
        voltar.addEventListener("click", () => {
          salvarParte(p.ligada, null, () => editando.delete(p.parte));
        });
        lado.append(voltar);
      }
      const editar = el("button", "editar", "link");
      editar.type = "button";
      editar.addEventListener("click", () => {
        editando.set(p.parte, p.texto);
        atualizar();
      });
      lado.append(editar);
    }
    const input = chave(p.ligada, (ligada, i) => {
      salvarParte(ligada, p.editado ? p.texto : null).then(() => {
        if (vigente() && !erro.hidden) i.checked = !ligada;
      });
    }, `ia-parte-${p.parte}`);
    input.setAttribute("aria-label", p.rotulo);
    lado.append(input);
    cab.append(titulo, lado);
    b.append(cab);
    if (editando.has(p.parte)) {
      const area = el("textarea");
      area.rows = 6;
      area.value = editando.get(p.parte);
      area.addEventListener("input", () => editando.set(p.parte, area.value));
      area.setAttribute("aria-label", `Texto de ${p.rotulo}`);
      const salvar = el("button", "Salvar", "btn sm");
      const cancelar = el("button", "Cancelar", "btn ghost sm");
      salvar.type = cancelar.type = "button";
      salvar.addEventListener("click", () => {
        salvarParte(p.ligada, area.value, () => editando.delete(p.parte));
      });
      cancelar.addEventListener("click", () => {
        editando.delete(p.parte);
        atualizar();
      });
      const acoes = el("div", undefined, "row");
      acoes.append(cancelar, salvar);
      const bd = el("div", undefined, "bd");
      bd.append(area, acoes);
      b.append(bd);
    } else if (p.texto) {
      b.append(el("p", p.texto, "bd texto-parte"));
    }
    b.append(erro);
    return b;
  }

  function contexto({ contexto: c }) {
    const card = el("div", undefined, "card");
    const topo = el("div", undefined, "row");
    const total = el("small", `≈ ${numero(c.tokens)} tokens`, "mono");
    total.dataset.testid = "ctx-tokens";
    topo.append(el("b", "Contexto enviado à IA"), total);
    card.append(topo, ...c.partes.map(bloco));
    return card;
  }

  function desenhar(dados) {
    const grade = el("div", undefined, "ia-grade");
    const direita = el("div", undefined, "coluna");
    direita.append(instrucoes(dados), contexto(dados));
    grade.append(provedores(dados), direita);
    raiz.replaceChildren(el("h1", "Configurar IA"), el("h2", "Sua IA", "titulo-campo"), grade);
  }

  await atualizar();
}
