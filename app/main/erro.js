// Normaliza um erro vindo do núcleo para `{ code, message }` seguro de atravessar a ponte.
// Só passa `code` em SCREAMING_SNAKE_CASE; qualquer outra coisa vira ERRO_DESCONHECIDO.
const FORMATO_CODE = /^[A-Z][A-Z0-9_]*$/;

function normalizarErro(e) {
  const code = e && e.code;
  if (typeof code === "string" && FORMATO_CODE.test(code)) {
    return { code, message: e.message, original: false };
  }
  return { code: "ERRO_DESCONHECIDO", message: "Ocorreu um erro inesperado.", original: true };
}

module.exports = { normalizarErro };
