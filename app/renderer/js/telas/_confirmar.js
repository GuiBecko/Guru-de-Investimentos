import { el } from "./_tabela.js";

// Confirmação dentro da página (um <dialog> modal), no lugar do confirm() nativo, que abriria
// uma janela do sistema. Resolve true só quando a pessoa clica em `acao`.
export function confirmar(pergunta, acao = "Apagar") {
  return new Promise((resolve) => {
    const dialogo = el("dialog");
    const titulo = el("h2", pergunta);
    const cancelar = el("button", "Cancelar", "btn ghost");
    const confirmarBotao = el("button", acao, "btn");
    cancelar.type = confirmarBotao.type = "button";
    const acoes = el("div", undefined, "acoes");
    acoes.append(cancelar, confirmarBotao);
    const caixa = el("div", undefined, "caixa");
    caixa.append(titulo, acoes);
    dialogo.append(caixa);
    let resposta = false;
    cancelar.addEventListener("click", () => dialogo.close());
    confirmarBotao.addEventListener("click", () => {
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
