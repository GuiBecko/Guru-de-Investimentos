import { api } from "../api.js";
import { el } from "./_tabela.js";

const LIMITE_INSTRUCOES = 4000;
const FUTUROS = ["OpenAI", "Google Gemini", "Modelo local (Ollama)"];
const PRIVACIDADE = [
  ["apenasPercentuais", "Apenas percentuais"],
  ["nomesInstituicoes", "Nome das instituições"],
  ["buscaWeb", "Buscar na web"],
];

const numero = (n) => n.toLocaleString("pt-BR");
const dolares = (centesimos) => (centesimos / 100).toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

function chave(rotulo, marcada, aoMudar, id) {
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
  const editando = new Set();

  async function carregar() {
    const [config, contexto] = await Promise.all([api.ia.config(), api.ia.contexto()]);
    return { config, contexto };
  }

  async function atualizar() {
    try {
      const dados = await carregar();
      if (!vigente()) return;
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
  async function gravar(acao, aoErro) {
    try {
      const erros = await acao();
      if (!vigente()) return;
      if (erros?.length) return aoErro(juntar(erros));
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
    col.append(formularioChave(config), seletorModelo(config), el("small", `Uso este mês: ~${numero(config.uso.tokens)} tokens (≈ US$ ${dolares(config.uso.custoCentesimosDolar)})`), privacidade(config));
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
      await gravar(() => api.ia.salvarChave(input.value), (m) => mostrarErro(erro, input, m));
      if (vigente()) salvar.disabled = false;
    });
    form.append(campo, acoes, el("small", "A chave fica guardada só neste computador, no cofre de senhas do sistema."));
    return form;
  }

  function entradaConfig(config, mudancas) {
    return {
      modelo: config.modelo,
      instrucoes: config.instrucoes,
      apenasPercentuais: config.apenasPercentuais,
      nomesInstituicoes: config.nomesInstituicoes,
      buscaWeb: config.buscaWeb,
      ...mudancas,
    };
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
    select.value = config.modelo;
    const label = el("label", "Modelo");
    label.htmlFor = select.id;
    const erro = faixaErro();
    erro.id = "e-ia-modelo";
    select.addEventListener("change", () => gravar(() => api.ia.salvarConfig(entradaConfig(config, { modelo: select.value })), (m) => mostrarErro(erro, select, m)));
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
      const input = chave(rotulo, config[nome], (ligada, i) => gravar(() => api.ia.salvarConfig(entradaConfig(config, { [nome]: ligada })), (m) => {
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
    area.value = config.instrucoes;
    const label = el("label", "Instruções para a sua IA", "so-leitor");
    label.htmlFor = area.id;
    const contador = el("small", undefined, "contador");
    const contar = () => {
      contador.textContent = `${numero([...area.value].length)}/${numero(LIMITE_INSTRUCOES)}`;
    };
    contar();
    area.addEventListener("input", contar);
    const erro = faixaErro();
    erro.id = "e-ia-instrucoes";
    const salvar = el("button", "Salvar instruções", "btn");
    salvar.type = "button";
    salvar.addEventListener("click", async () => {
      mostrarErro(erro, area, null);
      salvar.disabled = true;
      await gravar(() => api.ia.salvarConfig(entradaConfig(config, { instrucoes: area.value })), (m) => mostrarErro(erro, area, m));
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
    const salvarParte = (ligada, texto) => gravar(() => api.ia.salvarParte(p.parte, ligada, texto), (m) => mostrarErro(erro, undefined, m));
    if (p.parte !== "instrucoes") {
      if (p.editado) {
        const voltar = el("button", "voltar ao automático", "link");
        voltar.type = "button";
        voltar.addEventListener("click", () => {
          editando.delete(p.parte);
          salvarParte(p.ligada, null);
        });
        lado.append(voltar);
      }
      const editar = el("button", "editar", "link");
      editar.type = "button";
      editar.addEventListener("click", () => {
        editando.add(p.parte);
        atualizar();
      });
      lado.append(editar);
    }
    const input = chave(p.rotulo, p.ligada, (ligada, i) => {
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
      area.value = p.texto;
      area.setAttribute("aria-label", `Texto de ${p.rotulo}`);
      const salvar = el("button", "Salvar", "btn sm");
      const cancelar = el("button", "Cancelar", "btn ghost sm");
      salvar.type = cancelar.type = "button";
      salvar.addEventListener("click", () => {
        editando.delete(p.parte);
        salvarParte(p.ligada, area.value);
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
