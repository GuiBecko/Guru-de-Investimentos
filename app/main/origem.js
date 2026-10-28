// Só a página principal do app pode chamar os canais IPC. Compara protocolo `file:` e
// caminho, ignorando `?` e `#`; qualquer outra coisa (ou URL inválida) é recusada.
function origemPermitida(urlDoQuadro, urlEsperada) {
  try {
    const a = new URL(urlDoQuadro);
    const b = new URL(urlEsperada);
    return a.protocol === "file:" && b.protocol === "file:" && a.pathname === b.pathname;
  } catch {
    return false;
  }
}

module.exports = { origemPermitida };
