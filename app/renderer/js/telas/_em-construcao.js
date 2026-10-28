// Corpo comum das telas provisórias da fase 0.
export function emConstrucao(el, titulo) {
  const h1 = document.createElement("h1");
  h1.textContent = titulo;
  const p = document.createElement("p");
  p.className = "sub";
  p.textContent = "Em construção";
  el.replaceChildren(h1, p);
}
