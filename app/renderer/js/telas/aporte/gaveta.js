// Gaveta do Próximo aporte: o chat com a IA sobre uma classe da divisão, com o foco do aporte
// (valor, direção e meta) na primeira pergunta.
import { formatarReais } from "../../formato.js";
import { el } from "../_tabela.js";
import { criarChat } from "../assistente/chat.js";
import { painel } from "../assistente/painel.js";

// Direções do dinheiro do aporte: (código, rótulo), nesta ordem (as mesmas do núcleo).
export const DIRECOES = [
  ["renda_fixa", "Renda fixa"],
  ["acoes", "Ações"],
  ["fiis", "FIIs"],
  ["exterior", "Exterior"],
  ["novo", "Experimentar algo novo"],
  ["seguranca", "Mais segurança"],
];
export const MAX_DIRECAO_LIVRE = 300;

// Espaços e quebras de linha viram um espaço só, como o núcleo faz ao montar o contexto.
const numaLinha = (texto) => String(texto ?? "").split(/\s+/).filter(Boolean).join(" ");

// A linha do topo da gaveta: o que, além das partes de Configurar IA, vai para a IA.
export function resumoContexto({ valorCentavos, direcoes, direcaoLivre, metaNome }) {
  const partes = DIRECOES.filter(([c]) => direcoes.includes(c)).map(([, r]) => r);
  const livre = numaLinha(direcaoLivre);
  if (livre) partes.push(`"${livre}"`);
  return [
    `Contexto: aporte de ${formatarReais(valorCentavos)}`,
    `direção: ${partes.length ? partes.join(", ") : "livre"}`,
    `meta: ${metaNome || "nenhuma"}`,
    "mais as partes ligadas em Configurar IA",
  ].join(" · ");
}

const sugestoes = (rotulo) => [
  `Quais investimentos de ${rotulo} combinam com a direção que escolhi?`,
  "Compare duas opções para esse valor, com prós e contras.",
  "Quais custos, impostos e riscos devo olhar nesta classe?",
];

// `parte`: o cartão da divisão; `foco`: o que vai para `api.ia.perguntar`.
// Opcionais: `aoMudar({ conversa, emAndamento })` depois de cada redesenho e `aoNova()` para
// recomeçar com o foco atual. O evento "guru:foco-mudou" na gaveta mostra o botão "Nova conversa".
export function gaveta({ vigente, parte, foco, metaNome, aoFechar, aoMudar = () => {}, aoNova = () => {} }) {
  const caixa = el("section", undefined, "gaveta");
  caixa.dataset.testid = "gaveta";
  const titulo = el("h2", `${parte.rotulo} · ${formatarReais(parte.sugeridoCentavos)}`);
  titulo.id = `gaveta-${parte.classe}`;
  caixa.setAttribute("aria-labelledby", titulo.id);
  const fechar = el("button", "Fechar", "btn ghost");
  fechar.type = "button";
  fechar.addEventListener("click", aoFechar);
  const nova = el("button", "Nova conversa", "btn ghost");
  nova.type = "button";
  nova.hidden = true;
  nova.addEventListener("click", aoNova);
  const lado = el("div", undefined, "row");
  lado.append(nova, fechar);
  const nome = el("div", undefined, "row");
  nome.append(titulo, el("span", "Claude", "pill"));
  const topo = el("div", undefined, "row");
  topo.append(nome, lado);

  const registrar = el("a", "Registrar aporte", "link");
  registrar.href = "#/investimentos/novo";
  const ver = el("a", "Ver no Assistente", "link");
  ver.hidden = true;
  const links = el("div", undefined, "row links-gaveta");
  links.append(registrar, ver);
  const colPainel = el("div");

  let mudou = false;
  const chat = criarChat({
    vigente,
    sugestoes: sugestoes(parte.rotulo),
    placeholder: "Pergunte sobre esta classe…",
    foco,
    aoMudar: (estado) => {
      const id = estado.conversa?.id;
      ver.hidden = id === undefined;
      if (id !== undefined) ver.href = `#/assistente/${id}`;
      else ver.removeAttribute("href");
      nova.hidden = !mudou;
      nova.disabled = estado.emAndamento;
      const { web, internas } = chat.fontes();
      colPainel.replaceChildren(painel(web, internas));
      aoMudar(estado);
    },
  });

  caixa.addEventListener("guru:foco-mudou", () => {
    mudou = true;
    nova.hidden = false;
  });

  caixa.append(topo, el("small", resumoContexto({ ...foco, metaNome }), "resumo-contexto"), chat.elemento, links, colPainel);
  chat.mostrar(null);
  return caixa;
}
