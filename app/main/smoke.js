// Smoke do pacote (`--guru-smoke`): abre o núcleo numa pasta temporária, com o cofre em memória,
// confere o conteúdo do pacote e devolve o código de saída. Sem Electron, sem janela e sem rede:
// o main.js monta as dependências reais, cuida do prazo, apaga a pasta e encerra o app.
const path = require("node:path");

const ARQUIVOS_DO_APP = [path.join("renderer", "index.html"), "avisos-terceiros.txt"];
// Código e testes do addon não vão para o pacote (filtro `files` do electron-builder.yml).
const FORA_DO_PACOTE = ["src", "test"];

class FalhaSmoke extends Error {
  constructor(code, message) {
    super(message);
    this.code = code;
  }
}

function conferirPacote({ existe, listar, dirApp, dirRecursos, plataforma, arquitetura }) {
  const addonDesempacotado = path.join(dirRecursos, "app.asar.unpacked", "node_modules", "@guru", "core-node");
  let nomes;
  try {
    nomes = listar(addonDesempacotado);
  } catch {
    nomes = [];
  }
  const alvo = `${plataforma}-${arquitetura}`;
  if (!nomes.some((nome) => nome.endsWith(".node") && nome.includes(alvo))) {
    throw new FalhaSmoke("PACOTE_INCOMPLETO", `Nenhum .node de ${alvo} fora do asar.`);
  }
  const addonNoAsar = path.join(dirApp, "node_modules", "@guru", "core-node");
  for (const pasta of FORA_DO_PACOTE) {
    if (existe(path.join(addonNoAsar, pasta))) {
      throw new FalhaSmoke("PACOTE_INCOMPLETO", `O pacote leva a pasta ${pasta} do addon.`);
    }
  }
}

/** Devolve o código de saída (0 ou 1) e escreve uma linha em `log`. */
async function executarSmoke({
  pasta,
  versaoApp,
  empacotado,
  abrirCore,
  existe,
  listar,
  dirApp,
  dirRecursos,
  plataforma,
  arquitetura,
  log,
}) {
  try {
    let info;
    try {
      info = abrirCore(pasta, { cofreMemoria: true }).info();
    } catch (e) {
      throw new FalhaSmoke("ADDON_INDISPONIVEL", (e && e.message) || "O núcleo não abriu.");
    }
    for (const arquivo of ARQUIVOS_DO_APP) {
      if (!existe(path.join(dirApp, arquivo))) {
        throw new FalhaSmoke("PACOTE_INCOMPLETO", `Falta ${arquivo.split(path.sep).join("/")} no app.`);
      }
    }
    if (empacotado) conferirPacote({ existe, listar, dirApp, dirRecursos, plataforma, arquitetura });
    if (info.coreVersion !== versaoApp) {
      throw new FalhaSmoke("VERSAO_DIVERGENTE", `Núcleo ${info.coreVersion}, app ${versaoApp}.`);
    }
    log(`GURU_SMOKE_OK app=${versaoApp} nucleo=${info.coreVersion} esquema=${info.schemaVersion}`);
    return 0;
  } catch (e) {
    const code = e instanceof FalhaSmoke ? e.code : "ADDON_INDISPONIVEL";
    log(`GURU_SMOKE_FALHOU ${code}: ${(e && e.message) || "falha inesperada"}`);
    return 1;
  }
}

module.exports = { executarSmoke };
