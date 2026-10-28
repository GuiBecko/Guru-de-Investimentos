const MENSAGENS = {
  SCHEMA_TOO_NEW:
    "Seus dados foram salvos por uma versão mais nova do app. Atualize o Guru de Investimentos para continuar.",
  DB_INVALID: "O arquivo de dados está danificado. Nada foi apagado.",
  IO: "Não foi possível ler ou gravar a pasta de dados do Guru de Investimentos. Verifique as permissões e o espaço em disco.",
  SQLITE:
    "O banco de dados do Guru de Investimentos está ocupado ou com problema. Feche outras cópias do app e tente de novo. Nada foi apagado.",
};
const PADRAO = "O núcleo do app não carregou. Reinstale o Guru de Investimentos.";

const codigo = new URLSearchParams(location.search).get("code");
document.getElementById("motivo").textContent = Object.hasOwn(MENSAGENS, codigo)
  ? MENSAGENS[codigo]
  : PADRAO;
