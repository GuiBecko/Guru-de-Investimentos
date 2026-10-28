const MENSAGENS = {
  SCHEMA_TOO_NEW:
    "Seus dados foram salvos por uma versão mais nova do app. Atualize o Guru de Investimentos para continuar.",
  DB_INVALID: "O arquivo de dados está danificado. Nada foi apagado.",
};
const PADRAO = "O núcleo do app não carregou. Reinstale o Guru de Investimentos.";

const codigo = new URLSearchParams(location.search).get("code");
document.getElementById("motivo").textContent = MENSAGENS[codigo] || PADRAO;
