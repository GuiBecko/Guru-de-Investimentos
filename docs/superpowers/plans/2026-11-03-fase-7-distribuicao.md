# Fase 7: Distribuição — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Quem não programa consegue instalar o Guru de Investimentos no Windows, no macOS e no Linux a partir da página de versões do GitHub, com os passos da primeira abertura de um app sem assinatura explicados no README. O app avisa quando sai uma versão nova e, no Windows e no AppImage, se atualiza com um clique; nunca baixa nada sem o usuário pedir. Todos os avisos legais ficam num lugar só, a tela "Sobre e avisos" e o `docs/avisos.md`, coerentes com a decisão "Regulatório" do MVP, junto com os avisos de terceiros. A revisão por um advogado continua pendente e isso fica escrito.

**Architecture:**
- **Empacotamento:** electron-builder no `app/`, com a configuração em `app/electron-builder.yml`.
  - O addon napi é construído no próprio runner de cada sistema e arquitetura, entra no pacote pelo `node_modules` e fica fora do asar (`asarUnpack`).
  - O `main.js` continua com `require("@guru/core-node")`: o Electron redireciona o `.node` para o `app.asar.unpacked`.
- **Smoke do pacote:** a opção `--guru-smoke` abre o núcleo numa pasta temporária, confere o conteúdo do pacote e sai com código 0, sem criar janela e sem rede. Vale também no app empacotado, porque é o próprio pacote que precisa ser provado.
- **Atualização:** `app/main/atualizacao.js`, no processo principal, com as dependências injetadas.
  - A procura consulta a última release publicada pela API do GitHub.
  - A instalação usa o electron-updater só no Windows (NSIS) e no AppImage. No macOS e no `.deb`, o app mostra a versão nova com o link da página dela.
  - É a única rede fora do `guru-core`, documentada em `arquitetura.md`.
- **Release:** um workflow novo, disparado por tag `v*`, constrói os instaladores nos quatro runners e cria a release em rascunho. Quem publica é uma pessoa.
- **Avisos:** `app/renderer/js/avisos.js` guarda todos os textos legais da interface, e um teste confere que cada um está igual em `docs/avisos.md`. Um script gera `app/avisos-terceiros.txt` das dependências Rust e npm que vão no pacote e recusa licença fora da lista.

**Tech Stack:** o mesmo das fases anteriores, mais `electron-builder` 26.17.0 (desenvolvimento) e `electron-updater` 6.8.10 (execução). O Electron fica fixo em 45.1.0.

**Spec:**
- [`docs/roadmap.md`](../../roadmap.md): fase 7, "Instaladores para Windows, macOS e Linux, atualização automática e revisão jurídica dos textos."
- [`docs/mvp.md`](../../mvp.md): as decisões "Distribuição" (programa instalado, sem servidor) e "Regulatório" (a IA apresenta opções, prós e contras e fontes; a decisão é do usuário; revisão jurídica antes do lançamento).
- [`docs/arquitetura.md`](../../arquitetura.md): "Rede" (só destinos listados), "Segurança da janela" e "Testes".
- Os textos legais que já existem: o rodapé do Próximo aporte (`telas/aporte.js`), o do perfil (`telas/perfil/_comum.js`), o do chat (`telas/assistente/chat.js`) e o da coluna de insights (`telas/carteira/insights.js`).
- A regra do usuário: os testes nunca mostram janela na tela e nunca usam a rede.

## Decisões desta fase

- **electron-builder, com versões fixas:** gera NSIS, DMG, AppImage e `.deb` com uma configuração só e escreve o `app-update.yml` de que o electron-updater precisa.
  - `electron-builder` 26.17.0 e `electron-updater` 6.8.10 são do mesmo dia (26/09/2026) e têm mais de um mês, em vez das versões de outubro, ainda recentes.
  - O Electron sai de `^45.1.0` para `45.1.0` exato, para a release ser reproduzível.
- **Um instalador por sistema e arquitetura:**
  - Windows x64 (NSIS);
  - macOS arm64 e macOS x64, em dois `.dmg` separados;
  - Linux x64 (AppImage e `.deb`).
  - **Sem DMG universal:** ele exigiria juntar dois `.node` com `lipo` num runner só. Com um DMG por arquitetura, cada um é construído e passa pelo smoke na máquina da própria arquitetura (`macos-latest` é arm64 e `macos-15-intel` é x64).
  - Windows arm64, Linux arm64, rpm, snap e flatpak ficam fora desta fase.
- **A arquitetura vem do runner:** a configuração não lista arquiteturas, e o workflow passa `--x64` ou `--arm64` igual à do runner. Um pacote com o `.node` de outra arquitetura falha no smoke, porque o addon não carrega.
- **O addon entra pelo `node_modules`:** a dependência `file:` continua sendo um link, e o electron-builder copia o conteúdo real.
  - O filtro de `files` tira do pacote o código Rust e os testes do addon, e o smoke confere que eles não estão lá.
  - Se o link não for copiado, a troca é `npm ci --install-links` nos passos de empacotamento, sem mudar o `main.js`.
- **Sem assinatura, nada falso:** o projeto não tem certificado.
  - Windows sem assinatura. O NSIS do electron-updater confere o `sha512` do `latest.yml`; sem `publisherName`, não confere o editor.
  - macOS com a assinatura ad hoc que o electron-builder aplica sem identidade (o Apple Silicon não roda binário sem nenhuma assinatura) e sem notarização. O Gatekeeper trata o app como de desenvolvedor não identificado.
  - O README explica a primeira abertura no SmartScreen e no Gatekeeper, e a release traz `SHA256SUMS.txt`.
  - O workflow passa os segredos de assinatura só quando eles existem. Sem eles, `CSC_IDENTITY_AUTO_DISCOVERY=false`. Ligar a assinatura depois é cadastrar os segredos e trocar `notarize: false` por `true`.
- **Atualização: procura ao abrir, ligada e visível; baixar só por clique:**
  - **Procura ao abrir:** o app já consulta as fontes oficiais de mercado ao abrir, e a procura não leva nenhum dado do usuário além do IP e da versão. Correção de segurança precisa chegar a quem não acompanha o GitHub.
  - **Ajuste visível:** "Procurar atualizações ao abrir o app" fica em "Sobre e avisos", com "Procurar agora" ao lado, e desligar vale a partir da próxima abertura.
  - **Nunca sozinho:** nada é baixado nem instalado sem o clique em "Baixar e instalar", e só "Reiniciar e atualizar" reinicia o app (`autoDownload = false`, `autoInstallOnAppQuit = false`).
- **Onde instala e onde só avisa:**
  - **Windows (NSIS) e AppImage:** o electron-updater baixa e instala.
  - **macOS:** o Squirrel.Mac recusa app sem assinatura de verdade.
  - **`.deb`:** a instalação pediria senha de administrador (`pkexec`) e ficaria à mercê do gerenciador de pacotes.
  - **No macOS e no `.deb`,** a faixa mostra a versão nova e o link "Ver no GitHub".
  - **Um app de Linux fora do AppImage** é tratado como `.deb`.
- **A procura é nossa, a instalação é do electron-updater:** uma chamada só à API do GitHub (`releases/latest`) serve aos quatro formatos e ignora rascunhos e pré-lançamentos. O electron-updater só entra depois do clique e confere a mesma versão no `latest.yml`. Com isso, os dois `latest-mac.yml` dos dois runners do macOS não são necessários e não vão para a release.
- **O ajuste fica num JSON do processo principal:** `atualizacao.json` na pasta de dados guarda `{ "procurarAoAbrir": true|false }`. É assunto da casca Electron, não do domínio, e não justifica uma migração do banco.
- **Smoke por opção, não por variável:** o `GURU_JANELA_OCULTA` só vale fora do pacote, de propósito. O pacote é provado com `--guru-smoke`, que não cria janela nenhuma, usa uma pasta temporária com o cofre em memória e não chama a rede (a atualização de mercado é disparada pela tela, que não abre). Quem rodar `--guru-smoke` por engano só vê o processo terminar.
- **CI: provar o pacote em todo PR, instaladores só na `main`:**
  - **Em todo PR, nos três sistemas:** `electron-builder --dir` e o smoke na pasta desempacotada (cerca de 1 a 2 minutos).
  - **No push para a `main`:** os instaladores completos, como artefatos de 7 dias.
  - **O macOS x64** só roda na release, para não ocupar um runner a mais em cada PR.
- **Release só em rascunho:** a tag `v*` constrói tudo, confere que a tag é `v` + a versão do `app/package.json` e cria a release com `--draft`. Nenhum passo publica.
  - O electron-builder roda sempre com `--publish never`, e só o último job tem `contents: write`.
  - Depois do merge, quem implementa empurra a tag `v0.9.0` para provar o workflow. O rascunho fica para uma pessoa revisar e publicar.
- **Versão:** o `app/package.json` é a fonte. A primeira release é a `0.9.0`, porque a `1.0.0` fica para depois da revisão jurídica.
  - Os dois `Cargo.toml` e o `crates/guru-node/package.json` acompanham o mesmo número, para o "Núcleo" do rodapé e o `User-Agent` não mostrarem outra versão.
  - Um teste confere que os números batem.
- **Ícone:** um PNG de 1024 × 1024, gerado por um script sem dependência (`node:zlib`): o verde `#04321F` com um "G" em pixel creme, na identidade do protótipo. O PNG fica no repositório.
- **Fusíveis do Electron:** `RunAsNode`, `NODE_OPTIONS` e os argumentos de inspeção desligados, e `OnlyLoadAppFromAsar` ligado, para fechar o atalho de rodar o executável como Node. A integridade do asar fica de fora, porque no macOS ela depende de assinatura.
- **"Revisão jurídica dos textos" sem fingir revisão:** uma IA não faz revisão jurídica. Esta fase faz o que é possível sem advogado:
  - consolida os textos e os alinha à decisão "Regulatório";
  - acrescenta o que faltava: CVM, erro da IA, limites das fontes, privacidade, licença e ausência de garantia;
  - gera os avisos de terceiros com a checagem de licenças;
  - registra no app, no README, no roadmap e no MVP que a revisão por advogado é pendente antes da 1.0.
  - Não há tela de "Termos de uso" para aceitar: sem texto revisado, um aceite daria uma segurança falsa.
- **Rodapés:** o do chat e o da coluna de insights ganham "Pode conter erros.". O do Próximo aporte e o do perfil ficam como estão. Os quatro passam a vir de `avisos.js`.
- **Desinstalar não apaga os dados:** `deleteAppDataOnUninstall: false`, e o README diz onde a pasta fica.

## Global Constraints

- **Tudo das fases anteriores continua valendo:**
  - CSP;
  - nada de `innerHTML` com dados nem `style=`;
  - `vigente()` depois de cada await;
  - pt-BR com acentos e erros em SCREAMING_SNAKE_CASE;
  - preload sem lógica e IPC com checagem de origem;
  - as variáveis `GURU_*` só fora do app empacotado;
  - os testes nunca usam a rede;
  - e2e com a janela escondida via `lancar()`.
- **Commits** em português, sem `Co-Authored-By` e sem nenhuma menção ao Claude.
- **Trabalho** na branch `fase-7-distribuicao`, criada a partir da `main` depois do merge da 6b (PR #9).
- **Versões exatas:**
  - `electron` `45.1.0`, `electron-builder` `26.17.0` e `@playwright/test` como está, em `devDependencies`;
  - `electron-updater` `6.8.10` em `dependencies`;
  - `version` `0.9.0` em `app/package.json`, `crates/guru-node/package.json`, `crates/guru-core/Cargo.toml` e `crates/guru-node/Cargo.toml`.
- **`app/package.json`** ganha:
  - `"description": "Carteira de investimentos com IA, no seu computador."`;
  - `"author": { "name": "Guilherme Becker", "email": "GuiBecko@users.noreply.github.com" }`, porque o `.deb` exige um e-mail de mantenedor e o e-mail pessoal não vai para o pacote;
  - `"homepage": "https://github.com/GuiBecko/Guru-de-Investimentos"` e `"license": "MIT"`;
  - os scripts:
    - `"dist": "electron-builder --publish never"`;
    - `"dist:dir": "electron-builder --dir --publish never"`;
    - `"avisos": "node scripts/avisos-terceiros.mjs"`;
    - `"avisos:conferir": "node scripts/avisos-terceiros.mjs --conferir"`.
- **`app/electron-builder.yml`:**
  ```yaml
  appId: io.github.guibecko.guru-de-investimentos
  productName: Guru de Investimentos
  artifactName: Guru-de-Investimentos-${version}-${os}-${arch}.${ext}
  directories: { output: dist, buildResources: build }
  npmRebuild: false
  asar: true
  files:
    - main.js
    - preload.js
    - main/**
    - renderer/**
    - avisos-terceiros.txt
    - package.json
    - "!node_modules/@guru/core-node/{src,test,node_modules,Cargo.toml,build.rs,package-lock.json}"
  asarUnpack: ["node_modules/@guru/core-node/*.node"]
  electronFuses:
    runAsNode: false
    enableNodeOptionsEnvironmentVariable: false
    enableNodeCliInspectArguments: false
    onlyLoadAppFromAsar: true
  publish: { provider: github, owner: GuiBecko, repo: Guru-de-Investimentos, releaseType: release }
  win: { target: nsis }
  nsis: { oneClick: true, perMachine: false, deleteAppDataOnUninstall: false }
  mac: { target: dmg, category: public.app-category.finance, notarize: false }
  linux: { target: [AppImage, deb], category: Office, synopsis: Carteira de investimentos com IA }
  deb: { maintainer: "Guilherme Becker <GuiBecko@users.noreply.github.com>" }
  ```
  - `perMachine: false` instala na pasta do usuário, e a atualização não pede administrador.
  - `.gitignore` ganha `app/dist/`.
- **Smoke (`--guru-smoke`):**
  - É tratado no topo do `main.js`, antes do `requestSingleInstanceLock`.
  - **Pasta:** `app.setPath("userData", mkdtemp(<tmp>/guru-smoke-))`, apagada no fim.
  - **Sem janela:** `app.dock?.hide()`.
  - **Núcleo:** `Core.open(pasta, { cofreMemoria: true })` e `info()`.
  - **Arquivos do app:** `renderer/index.html` e `avisos-terceiros.txt` existem.
  - **Empacotado, confere também:**
    - em `<resourcesPath>/app.asar.unpacked/node_modules/@guru/core-node/` há um `.node` cujo nome contém `<process.platform>-<process.arch>`;
    - `node_modules/@guru/core-node/src` e `test/` não existem dentro do asar.
  - **Sucesso:** imprime `GURU_SMOKE_OK app=<versão> nucleo=<coreVersion> esquema=<schemaVersion>` e sai com 0.
  - **Falha:** imprime `GURU_SMOKE_FALHOU <CODE>: <mensagem>` e sai com 1. Os códigos são `ADDON_INDISPONIVEL`, `PACOTE_INCOMPLETO` e `VERSAO_DIVERGENTE`, este quando `coreVersion` não é a versão do app.
  - **Prazo:** 30 s sem terminar: sai com 2.
  - **Nunca:** cria `BrowserWindow`, registra a procura de atualização ou chama a rede.
- **Atualização, destinos (processo principal):**
  - **Procura:** `GET https://api.github.com/repos/GuiBecko/Guru-de-Investimentos/releases/latest`.
    - Cabeçalhos: `Accept: application/vnd.github+json` e `User-Agent: GuruDeInvestimentos/<versão>`.
    - Prazo de 30 s; resposta acima de 1 MB é erro.
  - **Depois do clique** em "Baixar e instalar" (só NSIS e AppImage), o electron-updater usa:
    - `https://github.com/GuiBecko/Guru-de-Investimentos/releases.atom`;
    - `…/releases/download/<tag>/latest.yml` ou `latest-linux.yml`;
    - o instalador, com o redirecionamento do GitHub para `*.githubusercontent.com`.
  - Fora do pacote, sem `GURU_TESTE_ATUALIZACAO`, nada é consultado.
- **Leitura da release:**
  - A tag precisa casar `^v(\d+)\.(\d+)\.(\d+)$`; outra tag é erro.
  - A página é o `html_url` só quando começa com `https://github.com/GuiBecko/Guru-de-Investimentos/releases/`. Se não, é `https://github.com/GuiBecko/Guru-de-Investimentos/releases/latest`.
  - Só uma versão maior que a atual é "disponível".
- **Estado da atualização** (`atualizacao:estado`):
  ```js
  { estado: "indisponivel" | "parado" | "procurando" | "atualizado" | "disponivel" | "baixando" | "baixada" | "erro",
    versaoAtual, versaoNova /* string|null */, pagina /* https|null */, instalavel /* boolean */,
    formato: "nsis" | "appimage" | "dmg" | "deb" | "dev", procurarAoAbrir, progresso /* 0..100|null */,
    erro /* { code, message }|null */ }
  ```
- **Erros novos:**

  | Situação | Código | Mensagem |
  |---|---|---|
  | procurar ou baixar fora do app instalado | `ATUALIZACAO_INDISPONIVEL` | "A procura de atualizações só funciona no app instalado." |
  | baixar no macOS ou no `.deb` | `ATUALIZACAO_NAO_INSTALAVEL` | "Neste sistema o app não se atualiza sozinho: baixe o instalador novo na página da versão." |
  | rede, tag inválida ou `latest.yml` com outra versão | `ATUALIZACAO_FALHOU` | "Não foi possível procurar atualizações. Tente de novo mais tarde." |

- **Textos fixos da atualização:**
  - **Ajuste:** "Procurar atualizações ao abrir o app", com a dica "Consulta a página de versões do projeto no GitHub. Nada é baixado sem o seu clique.".
  - **Botões:** "Procurar agora", "Baixar e instalar", "Reiniciar e atualizar"; link "Ver no GitHub".
  - **Estados:**
    - "Procurando atualizações…";
    - "Você está na versão mais recente (0.9.0).";
    - "Nova versão disponível: 0.9.1.";
    - "Baixando a versão 0.9.1… 50%";
    - "Versão 0.9.1 baixada. Reinicie para atualizar.".
  - A faixa da barra lateral (`data-testid="atualizacao"`) aparece só em `disponivel`, `baixando` e `baixada`.
- **Modo de teste da atualização:** `GURU_TESTE_ATUALIZACAO=<arquivo.json>`, só fora do pacote.
  - O arquivo tem o formato `{ "formato": "nsis"|"appimage"|"dmg"|"deb", "ultima": { "tag", "html_url" } | null, "erro": true|false }`.
  - A procura lê o arquivo no lugar da API.
  - O instalador falso avança o progresso em 0, 50 e 100 e, em "Reiniciar e atualizar", só muda o estado para `baixada` com `reiniciando: true`, sem fechar o app.
- **Textos legais** (`app/renderer/js/avisos.js`, iguais em `docs/avisos.md`):
  - **`RECOMENDACAO`:**
    > O Guru de Investimentos é um programa de código aberto para organizar a sua carteira e conversar sobre ela com uma IA. Ele não faz recomendação individual de investimento nem presta consultoria, análise ou gestão de carteira. O app e os autores não são registrados na CVM como analistas de valores mobiliários, consultores ou administradores de carteira. As decisões de investimento são suas, e a responsabilidade por elas também.
  - **`IA`:**
    > As respostas, os insights e as notícias são gerados por IA (Claude, da Anthropic) e podem conter erros, omissões ou informações desatualizadas. A IA apresenta opções, prós e contras e fontes; confira as fontes antes de decidir. O perfil de investidor é um questionário próprio baseado nas dimensões da ANBIMA e não substitui a análise de perfil (suitability) feita pela sua instituição.
  - **`MERCADO`:**
    > Selic, CDI e IPCA vêm do Banco Central (SGS), as taxas e os preços do Tesouro Direto vêm do Tesouro Transparente e as cotações vêm do arquivo diário da B3, com o preço de fechamento do último pregão. Não há preço em tempo real. Fundos, previdência e cripto contam pelo valor aplicado. Os valores são brutos, antes do IR, e podem diferir do extrato da sua instituição. Rentabilidade passada não garante rentabilidade futura.
  - **`PRIVACIDADE`:**
    > A carteira, o perfil, as metas e as conversas ficam só neste computador, e a chave da IA fica no cofre do sistema. Não há servidor do Guru. Quando você usa a IA, o app envia à Anthropic, com a sua chave, a sua pergunta e só as partes do contexto que você deixou ligadas em Configurar IA; a Anthropic trata esses dados pelos termos dela. As fontes de mercado recebem só datas e códigos. Com “Procurar atualizações ao abrir o app” ligada, o app consulta o GitHub ao abrir.
  - **`LICENCA`:**
    > Software livre sob a licença MIT, fornecido “no estado em que se encontra”, sem garantia de qualquer tipo. Os componentes de terceiros e as licenças deles estão em “Avisos de terceiros”.
  - **`REVISAO_JURIDICA`:**
    > Versão de pré-lançamento: estes avisos ainda vão passar por revisão jurídica antes da versão 1.0.
  - **Os quatro rodapés:**
    - **`RODAPE_CHAT`:**
      > Conteúdo educativo gerado por IA com os seus dados e fontes da web. Pode conter erros. Não é recomendação individual de investimento. Confira as fontes.
    - **`RODAPE_INSIGHTS`:**
      > Gerado por IA com os seus dados. Pode conter erros. Não é recomendação individual de investimento.
    - **`RODAPE_APORTE` e `RODAPE_PERFIL`:** os textos atuais, sem mudança.
- **Avisos de terceiros** (`app/avisos-terceiros.txt`, gerado e no repositório):
  - **Cabeçalho:** o `LICENSE` do projeto e o `renderer/fonts/OFL.txt` (IBM Plex).
  - **Uma seção por pacote, em ordem de nome e versão:** `== <nome> <versão> (<licença SPDX>) ==` e o texto dos arquivos `LICENSE*`/`COPYING*` da pasta dele.
    - Pacote sem arquivo: "Texto da licença em https://spdx.org/licenses/<id>.html".
  - **Rust:** o grafo de `cargo metadata --format-version 1 --locked` a partir do `guru-node`, só dependências normais (sem `build` e `dev`), de todas as plataformas.
  - **npm:** `npm ls --omit=dev --all --json` no `app/`, sem o `@guru/core-node`.
  - **Rodapé:** "O Electron e o Chromium trazem as próprias licenças nos arquivos LICENSE e LICENSES.chromium.html da pasta de instalação.".
  - **Licenças aceitas:** MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD, CC0-1.0, BSL-1.0, Unicode-3.0, Unicode-DFS-2016, MPL-2.0 e OFL-1.1.
    - Numa expressão com `OR`, basta uma aceita; com `AND`, todas.
    - Fora da lista, o script sai com 1 e o nome do pacote.
    - `--conferir` sai com 1 e "avisos-terceiros.txt desatualizado: rode npm run avisos." quando o arquivo gerado difere do gravado.
- **Workflows:**
  - **Empacotamento:** toda chamada ao electron-builder leva `--publish never`.
  - **`ci.yml`:**
    - **Em todo evento, nos três sistemas:** depois do e2e, `npm run dist:dir` e o smoke na pasta desempacotada:
      - `dist/linux-unpacked/guru-de-investimentos`, com `xvfb-run -a`;
      - `dist/win-unpacked/Guru de Investimentos.exe`;
      - `dist/mac-arm64/Guru de Investimentos.app/Contents/MacOS/Guru de Investimentos`.
    - **Só em `push` para a `main`:** `npm run dist -- --<arch>` e `actions/upload-artifact` com `instaladores-<os>`, `retention-days: 7`.
    - **No Ubuntu:** `npm run avisos:conferir`.
    - O `timeout-minutes` do job do app vai de 20 para 30.
  - **`release.yml`:**
    - **Disparo e permissões:** `on: push: tags: ["v*"]`, `permissions: contents: read` no topo.
    - **Jobs:**
      - `conferir` (Ubuntu): a tag é `v` + a versão do `app/package.json`;
      - `instaladores`, na matriz `ubuntu-latest`/x64, `windows-latest`/x64, `macos-latest`/arm64 e `macos-15-intel`/x64: o addon, os testes do addon e os unitários do app, `npm run dist -- --<arch>`, o smoke na pasta desempacotada e o upload;
      - `rascunho`: o único com `contents: write`.
    - **O `rascunho`:**
      - gera `SHA256SUMS.txt` dos instaladores;
      - roda `gh release create "$GITHUB_REF_NAME" --draft --verify-tag --title "Guru de Investimentos <versão>" --notes-file .github/notas-da-versao.md --generate-notes`;
      - anexa os `.exe`, `.dmg`, `.AppImage`, `.deb` e `*.blockmap`, mais `latest.yml`, `latest-linux.yml` e `SHA256SUMS.txt`;
      - falha se a release já existe.
  - **Segredos:**
    - nos passos de empacotamento: `CSC_LINK`, `CSC_KEY_PASSWORD`, `WIN_CSC_LINK`, `WIN_CSC_KEY_PASSWORD`, `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD` e `APPLE_TEAM_ID`, de `secrets.*`;
    - `CSC_IDENTITY_AUTO_DISCOVERY: ${{ secrets.CSC_LINK != '' }}`;
    - nenhum segredo no `ci.yml`.

## Review Focus

1. **Nada baixa sem clique, e só NSIS e AppImage instalam:** procurar, abrir o app e achar uma versão nova nunca chamam `baixar` do instalador. No `dmg` e no `deb`, `baixar()` recusa sem tocar nele. Teste (Task 3, `procurar_nao_baixa` e `nao_instala_em_dmg_e_deb`; o e2e com `nova-deb.json` mostra "Ver no GitHub" e nenhum "Baixar e instalar").
2. **Link e versão só do repositório:** uma `html_url` de outro lugar vira a página fixa, e uma tag fora de `vX.Y.Z` vira `ATUALIZACAO_FALHOU`. Teste (Task 3, `pagina_fora_do_repositorio_vira_a_padrao` e `tag_invalida_e_erro`).
3. **Sem rede e sem janela nos testes, inclusive no smoke:** fora do pacote e sem a variável de teste, a procura não chama `buscarUltima`. O smoke usa a pasta temporária com o cofre em memória e não recebe nenhuma fábrica de janela. Teste (Task 3, `dev_sem_teste_nao_consulta`; Task 1, `smoke_usa_pasta_temporaria_e_cofre_em_memoria`, e o passo do pacote real sob `xvfb-run`).
4. **Release nunca publica sozinha:** o `release.yml` só dispara por tag `v*` e cria a release com `--draft`. Todo electron-builder leva `--publish never`, e `contents: write` aparece uma vez só. Teste (Task 4, `workflows.test.js`).
5. **Avisos numa fonte só e iguais aos docs:** todo texto de `AVISOS` aparece igual em `docs/avisos.md`, e nenhum outro arquivo do `renderer/` escreve "recomendação". Teste (Task 2, `avisos.test.mjs`).

---

### Task 1: Versão única, empacotamento e smoke

**Files:**
- Create:
  - `app/electron-builder.yml`;
  - `app/main/smoke.js`;
  - `app/scripts/icone.mjs` e `app/build/icon.png`;
  - `app/test/unit/smoke.test.js` e `app/test/unit/versao.test.js`.
- Modify:
  - `app/package.json` e `app/package-lock.json` (versões, metadados, scripts, `electron-builder`);
  - `app/main.js` (o ramo `--guru-smoke`; `app:info` ganha `versaoApp`);
  - `app/renderer/index.html` e `app/renderer/js/app.js` ("Versão <versaoApp>" no rodapé, `data-testid="app-version"`);
  - `crates/guru-core/Cargo.toml`, `crates/guru-node/Cargo.toml`, `Cargo.lock` e `crates/guru-node/package.json` (0.9.0);
  - `.gitignore` (`app/dist/`);
  - `app/test/e2e/abrir.spec.js` (o rodapé mostra a versão).

**Interfaces:**
- Consumes: `Core.open`, `info()` (`coreVersion`, `schemaVersion`) e o carregador gerado pelo napi.
- **Produces:**
  ```js
  // main/smoke.js (CommonJS, sem require("electron"))
  /** Devolve o código de saída (0 ou 1) e escreve uma linha em `log`. */
  async function executarSmoke({ pasta, versaoApp, empacotado, abrirCore /* (pasta, opcoes) -> Core */,
                                 existe /* (caminho) -> boolean */, listar /* (pasta) -> string[] */,
                                 dirApp, dirRecursos, plataforma, arquitetura, log }) -> Promise<0 | 1>;
  module.exports = { executarSmoke };
  ```
  O `main.js` monta as dependências reais, cuida do prazo de 30 s, apaga a pasta e chama `app.exit(codigo)`.
- **`app:info`** devolve `{ ...nucleo().info(), versaoApp: app.getVersion() }`.

- [ ] **Step 1: Testes que falham:**
  - **`smoke.test.js`**, com fakes:
    - `smoke_ok`: imprime `GURU_SMOKE_OK app=0.9.0 nucleo=0.9.0 esquema=7` e devolve 0;
    - `smoke_usa_pasta_temporaria_e_cofre_em_memoria`: `abrirCore` é chamado uma vez, com a `pasta` recebida e exatamente `{ cofreMemoria: true }`;
    - `addon_que_nao_carrega`: dá 1 com `GURU_SMOKE_FALHOU ADDON_INDISPONIVEL`;
    - `pacote_com_codigo_do_addon`: empacotado, com `src` presente, dá 1 com `PACOTE_INCOMPLETO`;
    - `pacote_sem_node_da_arquitetura`: só um `.node` de `darwin-x64` num `darwin-arm64` dá `PACOTE_INCOMPLETO`;
    - `nucleo_com_outra_versao`: dá `VERSAO_DIVERGENTE`;
    - `fora_do_pacote_nao_confere_o_asar`: `listar` não é chamado.
  - **`versao.test.js`:**
    - as quatro versões e as entradas `guru-core` e `guru-node` do `Cargo.lock` são iguais ao `app/package.json`;
    - ela casa `^\d+\.\d+\.\d+$`;
    - `electron`, `electron-builder` e `electron-updater` (este a partir da Task 3) não começam com `^` nem `~`.
  - **`abrir.spec.js`:** `app-version` mostra "0.9.0".
- [ ] **Step 2:** `npm test` e `npx playwright test abrir.spec.js`. Esperado: FAIL.
- [ ] **Step 3:** implementar.
  - O `icone.mjs` escreve o PNG com `node:zlib`, sem dependência.
  - `npm install --save-dev --save-exact electron@45.1.0 electron-builder@26.17.0` no `app/`.
- [ ] **Step 4: Verificar:**
  - `npm test` e `npx playwright test`;
  - **Pacote real:**
    - `npm run dist:dir`;
    - `xvfb-run -a dist/linux-unpacked/guru-de-investimentos --guru-smoke`, que precisa imprimir `GURU_SMOKE_OK` e sair com 0;
    - `ls dist/linux-unpacked/resources/app.asar.unpacked/node_modules/@guru/core-node/` mostra só o `.node`;
  - **Sem janela na tela**, porque o smoke não cria nenhuma.
- [ ] **Step 5: Commit** com a mensagem "Distribuição: versão 0.9.0, empacotamento com electron-builder e smoke do pacote".

### Task 2: Avisos numa fonte só, tela "Sobre e avisos" e avisos de terceiros

**Files:**
- Create:
  - `docs/avisos.md`, com:
    - a introdução: "Textos legais do app. A fonte no código é `app/renderer/js/avisos.js`, e um teste confere os dois. Revisão jurídica: pendente.";
    - uma seção por constante, com o texto em citação;
    - a seção "Onde aparece".
  - `app/renderer/js/avisos.js` e `app/renderer/js/telas/sobre.js`;
  - `app/scripts/avisos-terceiros.mjs` e `app/avisos-terceiros.txt`;
  - `app/test/unit/avisos.test.mjs` e `app/test/unit/avisos-terceiros.test.mjs`;
  - `app/test/e2e/sobre.spec.js`.
- Modify:
  - `app/renderer/js/router.js` (a tela `sobre`, "Sobre e avisos", última do menu);
  - `app/renderer/js/telas/aporte.js`, `telas/perfil/_comum.js`, `telas/assistente/chat.js` e `telas/carteira/insights.js` (importam o rodapé de `avisos.js`);
  - `app/main.js`, `app/preload.js` e `app/renderer/js/api.js` (`app:avisosTerceiros`);
  - `app/renderer/css/telas.css` (só o que faltar: cartões de aviso e o `<pre>` com rolagem);
  - os testes:
    - `app/test/unit/router.test.mjs`, `app/test/unit/api.test.mjs`;
    - `app/test/e2e/navegar.spec.js`, `app/test/e2e/abrir.spec.js`;
    - `app/test/e2e/assistente.spec.js` e `app/test/e2e/insights.spec.js` (o "Pode conter erros.").

**Interfaces:**
- Consumes: Task 1 (`versaoApp` no `app:info`).
- **Produces:**
  ```js
  // renderer/js/avisos.js
  export const AVISOS = Object.freeze({ RECOMENDACAO, IA, MERCADO, PRIVACIDADE, LICENCA, REVISAO_JURIDICA,
                                        RODAPE_CHAT, RODAPE_INSIGHTS, RODAPE_APORTE, RODAPE_PERFIL });
  // renderer/js/telas/sobre.js
  export function render(raiz, parametros, vigente);
  // scripts/avisos-terceiros.mjs (exporta o que é puro, para o teste)
  export function licencaAceita(expressaoSpdx) -> boolean;
  export function montarTexto({ projeto, fontes, rust: [{ nome, versao, licenca, textos }], npm: [...] }) -> string;
  ```
- **Canal:** `app:avisosTerceiros`, que devolve o texto de `path.join(__dirname, "avisos-terceiros.txt")`, e `api.app.avisosTerceiros()`.
- **Tela "Sobre e avisos"** (`#/sobre`), de cima para baixo:
  - **"Guru de Investimentos":**
    - "Versão <versaoApp> · Núcleo <coreVersion>";
    - o link do repositório;
    - "Licença MIT".
  - **"Atualizações":** um lugar reservado (`data-testid="sobre-atualizacoes"`), preenchido na Task 3.
  - **Os avisos, um cartão para cada texto:**
    - "Não é recomendação" (`RECOMENDACAO`);
    - "Inteligência artificial" (`IA`);
    - "Dados de mercado" (`MERCADO`);
    - "Privacidade" (`PRIVACIDADE`);
    - "Licença e garantia" (`LICENCA`);
    - e `REVISAO_JURIDICA` em destaque (`faixa`).
  - **"Avisos de terceiros":** o botão "Ver avisos de terceiros" carrega o texto num `<pre>` (`textContent`), só no clique.

- [ ] **Step 1: Testes que falham:**
  - **`avisos.test.mjs`:**
    - cada valor de `AVISOS` aparece numa linha `> <texto>` de `docs/avisos.md`, igual;
    - nenhum `.js` de `app/renderer/` fora de `avisos.js` contém "recomendação";
    - `RECOMENDACAO` contém "CVM", e `IA` contém "podem conter erros";
    - `PRIVACIDADE` contém "Anthropic" e "GitHub".
  - **`avisos-terceiros.test.mjs`:**
    - `licencaAceita` aceita "MIT OR Apache-2.0" e "Apache-2.0 WITH LLVM-exception", e recusa "GPL-3.0" e "MIT AND GPL-3.0";
    - `montarTexto` ordena por nome e versão e usa o texto do SPDX para um pacote sem arquivo.
  - **`sobre.spec.js`:**
    - o menu tem "Sobre e avisos", e a tela mostra a versão e os seis textos, iguais aos de `AVISOS`;
    - "Ver avisos de terceiros" mostra um texto com "serde" e "electron-updater" (este depois da Task 3; até lá, só "serde" e "OFL");
    - nenhum erro de console ou CSP.
  - Os e2e do chat e da coluna de insights esperam os rodapés novos.
- [ ] **Step 2:** `npm test` e `npx playwright test sobre.spec.js navegar.spec.js`. Esperado: FAIL.
- [ ] **Step 3:** implementar.
  - O script roda `cargo metadata` e `npm ls` com `execFileSync`, sem shell.
  - Gravar `avisos-terceiros.txt` com `npm run avisos`.
- [ ] **Step 4:** `npm test`, `npm run avisos:conferir` e `npx playwright test`. Esperado: verde e sem janela na tela.
- [ ] **Step 5: Commit** com a mensagem "Avisos: textos legais numa fonte só, tela Sobre e avisos e avisos de terceiros".

### Task 3: Atualização automática

**Files:**
- Create:
  - `app/main/atualizacao.js` (lógica pura, com as dependências injetadas);
  - `app/main/atualizacao-electron.js` (as peças reais: `net.fetch` e `autoUpdater`);
  - `app/renderer/js/atualizacao.js` (a faixa da barra lateral e a seção da tela Sobre);
  - `app/test/unit/atualizacao.test.js`;
  - `app/test/e2e/atualizacao.spec.js` e `app/test/e2e/fixtures/atualizacao/{nova-nsis,nova-deb,igual,erro}.json`.
- Modify:
  - `app/package.json` e `app/package-lock.json` (`electron-updater` 6.8.10 exato);
  - `app/main.js` (cria a atualização depois do `whenReady`, fora do smoke; os canais novos; a procura ao abrir depois do `did-finish-load`);
  - `app/preload.js`, `app/renderer/js/api.js`, `app/renderer/index.html` (`<div data-testid="atualizacao" hidden>` na barra lateral) e `app/renderer/js/app.js`;
  - `app/renderer/js/telas/sobre.js` (a seção "Atualizações") e `app/renderer/css/telas.css`;
  - `app/avisos-terceiros.txt` (regerado);
  - `app/test/e2e/abrir.spec.js` (`window.guru.atualizacao`) e `app/test/unit/api.test.mjs`.

**Interfaces:**
- Consumes: Task 1 (versão, `publish` do electron-builder) e Task 2 (a tela Sobre).
- **Produces:**
  ```js
  // main/atualizacao.js (CommonJS, sem require("electron"))
  function compararVersoes(a, b) -> -1 | 0 | 1;            // "0.9.0" < "0.10.0"
  function lerRelease(json, versaoAtual) -> { versao, pagina } // lança ATUALIZACAO_FALHOU com tag inválida
  function formatoInstalado({ empacotado, plataforma, env }) -> "nsis" | "appimage" | "dmg" | "deb" | "dev";
  // win32 → nsis; darwin → dmg; linux com env.APPIMAGE → appimage, sem ele → deb; fora do pacote → dev
  function criarAtualizacao({ versaoAtual, formato, arquivoConfig, buscarUltima /* () -> Promise<json> */,
                              criarInstalador /* () -> { procurar(), baixar(aoProgresso), instalar() } */ })
    -> { estado(), procurar(), salvarConfig({ procurarAoAbrir }), baixar(), reiniciar(), aoAbrir() };
  module.exports = { compararVersoes, lerRelease, formatoInstalado, criarAtualizacao };
  ```
  - **`aoAbrir()`:** procura uma vez só se `procurarAoAbrir` e `formato !== "dev"`. Falha na procura ao abrir vira o estado `erro`, sem faixa.
  - **`baixar()`:**
    - só depois de `disponivel` e com `instalavel`;
    - chama `instalador.procurar()` e exige a mesma `versaoNova`;
    - depois `baixar`, que leva a `baixando` e `baixada`.
  - **`reiniciar()`:** só em `baixada`, chama `instalar()` (`quitAndInstall(false, true)`).
  - **`procurar()`:** não aceita uma segunda chamada enquanto `procurando` ou `baixando`; devolve o estado atual.
  - **Sem arquivo de configuração**, `procurarAoAbrir` é `true`. Um arquivo ilegível também, e ele é regravado no próximo `salvarConfig`.
  - **No `atualizacao-electron.js`, o `autoUpdater`** é configurado com:
    - `autoDownload = false` e `autoInstallOnAppQuit = false`;
    - `allowPrerelease = false` e `allowDowngrade = false`;
    - `logger = null`, para não gravar log com caminhos.
- **Canais e API:**
  - os canais `atualizacao:estado`, `atualizacao:procurar`, `atualizacao:salvarConfig`, `atualizacao:baixar` e `atualizacao:reiniciar`;
  - `api.atualizacao` com `estado`, `procurar`, `salvarConfig`, `baixar` e `reiniciar`.
- **Tela:**
  - **A faixa:** "Nova versão disponível: X.", com "Baixar e instalar" quando instalável ou "Ver no GitHub" quando não. Durante o download, o texto de baixando; no fim, o texto de baixada com "Reiniciar e atualizar".
  - **A seção "Atualizações" da tela Sobre:** o `checkbox` do ajuste com a dica, "Procurar agora", a linha do estado e os mesmos botões.
  - **Acompanhamento:** a tela lê o estado ao abrir e a cada 1 s enquanto ele é `procurando` ou `baixando`, e para quando a tela sai de cena (`vigente`).

- [ ] **Step 1: Testes que falham:**
  - **`atualizacao.test.js`**, com fakes que contam chamadas e um arquivo de configuração numa pasta temporária:
    - `compara_versoes`: inclui "0.9.0" < "0.10.0" e igualdade;
    - `procurar_nao_baixa`: com a release `v0.9.1`, o estado é `disponivel` com `versaoNova` "0.9.1", e `baixar` do instalador fica em 0 chamadas (`procurar` do instalador também);
    - `abrir_respeita_o_ajuste`:
      - com `procurarAoAbrir` falso, `aoAbrir()` não chama `buscarUltima`;
      - com verdadeiro, chama uma vez;
      - uma segunda `aoAbrir()` não chama de novo;
    - `nao_instala_em_dmg_e_deb`: `instalavel` é falso, `baixar()` recusa com `ATUALIZACAO_NAO_INSTALAVEL`, e `criarInstalador` não é chamado;
    - `nsis_baixa_e_reinicia_so_por_clique`:
      - `baixar()` passa por `baixando` com o progresso e chega a `baixada`;
      - `instalar` só é chamado em `reiniciar()`;
      - `reiniciar()` antes de `baixada` recusa;
    - `latest_yml_com_outra_versao`: dá `ATUALIZACAO_FALHOU`, sem chamar `baixar`;
    - `pagina_fora_do_repositorio_vira_a_padrao`;
    - `tag_invalida_e_erro`: "v1.0.0-beta", "1.0.0" e "latest";
    - `versao_igual_ou_menor_e_atualizado`;
    - `dev_sem_teste_nao_consulta`: `formato` "dev" dá `indisponivel`, `procurar()` recusa com `ATUALIZACAO_INDISPONIVEL`, e `buscarUltima` fica em 0;
    - `formato_instalado`: os cinco casos;
    - `config_padrao_ligada_e_arquivo_ilegivel`.
  - **`atualizacao.spec.js`**, com `lancar(dir, { GURU_TESTE_ATUALIZACAO })`:
    - **`nova-nsis.json`:**
      - ao abrir, a faixa mostra "Nova versão disponível: 9.9.0.";
      - "Baixar e instalar" chega a "Versão 9.9.0 baixada. Reinicie para atualizar.";
      - "Reiniciar e atualizar" não fecha o app no modo de teste;
    - **`nova-deb.json`:** a faixa tem "Ver no GitHub", com o `href` da página do fixture, e nenhum "Baixar e instalar";
    - **`igual.json`:** sem faixa; em Sobre, "Você está na versão mais recente (0.9.0).";
    - **`erro.json`:** "Procurar agora" mostra a mensagem de `ATUALIZACAO_FALHOU` (`role=alert`);
    - **o ajuste:** desligar e relançar com a mesma pasta não procura ao abrir (sem faixa com `nova-nsis.json`), e "Procurar agora" ainda procura;
    - **sem a variável:** Sobre mostra "A procura de atualizações só funciona no app instalado.", sem botão "Procurar agora";
    - nenhum erro de console ou CSP.
- [ ] **Step 2:** `npm test` e `npx playwright test atualizacao.spec.js abrir.spec.js`. Esperado: FAIL.
- [ ] **Step 3:** implementar.
  - `npm install --save-exact electron-updater@6.8.10`, e depois `npm run avisos`.
- [ ] **Step 4:** `npm test`, `npm run avisos:conferir`, `npx playwright test`, e `npm run dist:dir` com o smoke da Task 1. Esperado: verde, `GURU_SMOKE_OK` e sem janela na tela.
- [ ] **Step 5: Commit** com a mensagem "Atualização: procura no GitHub ao abrir e instalação por clique no Windows e no AppImage".

### Task 4: CI, release em rascunho e documentação

**Files:**
- Create:
  - `.github/workflows/release.yml`;
  - `.github/notas-da-versao.md`, com:
    - o aviso de build sem assinatura e o link para "Instalar" no README;
    - a tabela "qual arquivo baixar";
    - a conferência pelo `SHA256SUMS.txt`;
    - o `REVISAO_JURIDICA`;
  - `app/test/unit/workflows.test.js`.
- Modify:
  - `.github/workflows/ci.yml`;
  - `README.md`;
  - `docs/roadmap.md`, `docs/mvp.md` e `docs/arquitetura.md`.

**Interfaces:**
- Consumes: Tasks 1 a 3 (`dist`, `dist:dir`, `--guru-smoke`, `avisos:conferir`).
- **Smoke no workflow:** um passo por sistema, com o caminho das Global Constraints. O passo falha se a saída não tiver `GURU_SMOKE_OK` ou se o código não for 0. No Linux, depois do passo do `sysctl` que já existe.
- **README, seção "Instalar"** (antes de "Rodando localmente"):
  - **Qual arquivo:** `.exe` no Windows; `mac-arm64.dmg` no Mac com Apple Silicon e `mac-x64.dmg` no Mac com Intel; `.deb` no Ubuntu e no Debian; `.AppImage` nos outros Linux.
  - **Windows:** "O Windows protegeu o computador" aparece porque o app não tem assinatura: "Mais informações" → "Executar assim mesmo".
  - **macOS:** arrastar para Aplicativos. Na primeira abertura bloqueada, Ajustes do Sistema → Privacidade e Segurança → "Abrir Mesmo Assim". Se o aviso disser que o app está danificado, `xattr -dr com.apple.quarantine "/Applications/Guru de Investimentos.app"`.
  - **Linux:** `sudo apt install ./<arquivo>.deb`, ou o AppImage com `chmod +x`. No Ubuntu 24.04, o AppImage esbarra na restrição de namespaces do AppArmor: use o `.deb`. O app nunca é distribuído com `--no-sandbox`.
  - **Atualizações:** o que cada sistema faz, e onde ligar e desligar.
  - **Os dados:** `%APPDATA%\Guru de Investimentos`, `~/Library/Application Support/Guru de Investimentos` e `~/.config/Guru de Investimentos`. Desinstalar não os apaga.
  - **Conferir o download:** `sha256sum` ou `Get-FileHash`, contra o `SHA256SUMS.txt`.

- [ ] **Step 1: Testes que falham** (`workflows.test.js`, lendo os `.yml` como texto, sem dependência):
  - `release_so_por_tag`: o `release.yml` tem `tags:` com `"v*"` e não tem `branches:`, `pull_request` nem `workflow_dispatch`;
  - `release_em_rascunho`: a única linha `gh release create` tem `--draft` e `--verify-tag`. Nenhum arquivo tem `gh release edit` nem `--draft=false`;
  - `nunca_publica_pelo_builder`: toda linha com `electron-builder`, `npm run dist` ou `npm run dist:dir` nos dois workflows termina com `--publish never` ou chama um script que o tem. Nenhum tem `--publish always` nem `onTagOrDraft`;
  - `escrita_so_no_rascunho`: `contents: write` aparece uma vez no `release.yml` e nenhuma no `ci.yml`;
  - `ci_sem_segredos`: o `ci.yml` não contém `secrets.`;
  - `smoke_nos_dois`: os dois workflows têm `--guru-smoke` e conferem `GURU_SMOKE_OK`;
  - `quatro_runners_na_release`: `ubuntu-latest`, `windows-latest`, `macos-latest` e `macos-15-intel`.
- [ ] **Step 2:** `npm test`. Esperado: FAIL.
- [ ] **Step 3:** implementar os workflows e os docs:
  - **`docs/roadmap.md`:**
    - o link do plano da fase 7;
    - abaixo da tabela: "A revisão jurídica dos textos por um advogado é pendente e condição para a versão 1.0. A fase 7 consolidou os avisos em [`avisos.md`](avisos.md)."
  - **`docs/mvp.md`:**
    - a decisão "Distribuição" ganha os instaladores, sem assinatura por enquanto, e a atualização: procura ao abrir, com o ajuste, e instalação por clique só no Windows e no AppImage;
    - a "Regulatório" ganha o link para `avisos.md` e "revisão por advogado pendente antes da 1.0".
  - **`docs/arquitetura.md`:**
    - **Stack:** a linha "Empacotamento: electron-builder 26.17.0; atualização: electron-updater 6.8.10".
    - **"Rede":**
      - a frase de abertura passa a dizer que só o `guru-core` acessa a rede, com uma exceção: a procura e o download de atualizações, no processo principal;
      - as linhas `api.github.com/repos/GuiBecko/Guru-de-Investimentos/releases/latest` (GET, ao abrir se ligado e por "Procurar agora", fase 7) e `github.com/GuiBecko/Guru-de-Investimentos/releases…` e `*.githubusercontent.com` (só depois do clique em "Baixar e instalar", fase 7);
      - o que cada uma leva: só a versão no `User-Agent`.
    - **Seção nova, "Distribuição e atualização":**
      - os formatos e as arquiteturas;
      - o `asarUnpack` do addon;
      - os fusíveis;
      - a assinatura pendente;
      - o `atualizacao.json`.
    - **"Testes":** o smoke do pacote e a geração dos instaladores na CI.
    - **Modo de teste:** o `GURU_TESTE_ATUALIZACAO`.
  - **`README.md`:**
    - o status: "fases 0 a 7 concluídas; os textos legais aguardam revisão jurídica antes da 1.0";
    - a seção "Instalar";
    - em "Documentação", o link para `docs/avisos.md`.
- [ ] **Step 4:** a suíte inteira:
  - `cargo test`, clippy e fmt;
  - os testes do addon;
  - `npm test` e `npm run avisos:conferir`;
  - `npx playwright test`;
  - `npm run dist` local no Linux, e o smoke no AppImage com `APPIMAGE_EXTRACT_AND_RUN=1 xvfb-run -a dist/*.AppImage --guru-smoke`;
  - no PR, o CI verde nos três sistemas, com o smoke.
  - Esperado: tudo verde e sem janela na tela.
- [ ] **Step 5: Commit** com a mensagem "Distribuição: instaladores na CI, release em rascunho por tag e documentação da fase 7".
- [ ] **Step 6: Depois do merge:**
  - `git tag v0.9.0` na `main` e `git push origin v0.9.0`;
  - conferir que o `release.yml` criou o rascunho com os 5 instaladores, os 2 `latest*.yml`, os `.blockmap` e o `SHA256SUMS.txt`;
  - não publicar: a publicação é de uma pessoa, depois de baixar e abrir os instaladores.
