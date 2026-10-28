import * as carteira from "./telas/carteira.js";
import * as aporte from "./telas/aporte.js";
import * as assistente from "./telas/assistente.js";
import * as investimentos from "./telas/investimentos.js";
import * as perfil from "./telas/perfil.js";
import * as ia from "./telas/ia.js";

export const TELAS = [
  { id: "carteira", titulo: "Carteira" },
  { id: "aporte", titulo: "Próximo aporte" },
  { id: "assistente", titulo: "Assistente" },
  { id: "investimentos", titulo: "Investimentos" },
  { id: "perfil", titulo: "Perfil e objetivo" },
  { id: "ia", titulo: "Configurar IA" },
];

const MODULOS = { carteira, aporte, assistente, investimentos, perfil, ia };

export function resolver(hash) {
  const [id, ...parametros] = String(hash).replace(/^#\/?/, "").split("/");
  const tela = TELAS.find((t) => t.id === id);
  return tela ? { tela, parametros } : { tela: TELAS[0], parametros: [] };
}

export function iniciar(raiz) {
  const menu = raiz.querySelector("nav");
  const conteudo = raiz.querySelector("main");

  menu.replaceChildren(
    ...TELAS.map((t) => {
      const a = document.createElement("a");
      a.href = `#/${t.id}`;
      a.textContent = t.titulo;
      return a;
    }),
  );

  function mostrar() {
    const { tela, parametros } = resolver(window.location.hash);
    for (const a of menu.querySelectorAll("a")) {
      a.classList.toggle("on", a.getAttribute("href") === `#/${tela.id}`);
    }
    try {
      MODULOS[tela.id].render(conteudo, parametros);
    } catch (e) {
      console.error(`Falha ao abrir a tela ${tela.id}:`, e);
      const p = document.createElement("p");
      p.textContent = "Não foi possível abrir esta tela.";
      conteudo.replaceChildren(p);
    }
  }

  window.addEventListener("hashchange", mostrar);
  mostrar();
}
