# Fase 3a: Dados de mercado e valor atual — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O app busca, nas fontes oficiais, o CDI, a Selic, o IPCA, os preços do Tesouro Direto e as cotações de fechamento da B3, guarda tudo no banco local e calcula o valor atual bruto de cada investimento. A lista, o detalhe da classe, a carteira e as metas passam a mostrar o valor atual e a rentabilidade, com a data dos dados e um botão para atualizar.

**Architecture:** Leitores puros no `guru-core` para cada formato (JSON do SGS, CSV do Tesouro, TXT/ZIP da B3), um atualizador que baixa só o que falta e grava em tabelas de mercado (migração 0004), e um módulo de valorização que calcula o valor de cada investimento numa data a partir dessas tabelas. A rede fica atrás de uma interface `Baixador`, para os testes usarem arquivos reais gravados em `tests/fixtures/` e nunca acessarem a internet. O addon roda a atualização fora da thread principal (`AsyncTask`).

**Tech Stack:** o mesmo das fases anteriores, mais `serde`/`serde_json`, `ureq` (HTTP com rustls) e `zip` (só deflate) no `guru-core` — já adicionados ao `Cargo.toml` e baixados no commit do plano.

**Spec:** [`docs/mvp.md`](../../mvp.md) (decisões "Dados de mercado" e "Cotações"), [`docs/arquitetura.md`](../../arquitetura.md) (seção Rede). Protótipo: tabela do detalhe da classe (`docs/prototipo/index.html:783-794`).

## Global Constraints

- Tudo das fases anteriores continua valendo (segurança da janela, CSP, pt-BR, códigos de erro, preload sem lógica, `api.js`, `vigente()`, sem `innerHTML`/`style=`, IPC com checagem de origem, dinheiro em centavos `i64`, taxas em `Decimal`, nada de `type="number"` para valores digitados).
- **Só estes destinos de rede**, numa lista fixa no núcleo; qualquer outra URL é recusada antes da requisição:
  - `https://api.bcb.gov.br/dados/serie/bcdata.sgs.{serie}/dados?formato=json&dataInicial=dd/MM/aaaa&dataFinal=dd/MM/aaaa` — séries `12` (CDI, % ao dia), `11` (Selic, % ao dia), `433` (IPCA, % ao mês). No máximo 10 anos por consulta.
  - `https://www.tesourotransparente.gov.br/ckan/dataset/df56aa42-484a-4a59-8184-7676580c81e3/resource/796d2059-14e9-44e3-80c9-2d9e30b405c1/download/PrecoTaxaTesouroDireto.csv` — histórico completo (~15 MB, `;`, vírgula decimal, datas `dd/MM/aaaa`).
  - `https://bvmf.bmfbovespa.com.br/InstDados/SerHist/COTAHIST_D{ddMMaaaa}.ZIP` — um pregão por arquivo (~600 KB). Dia sem pregão responde erro ou arquivo inválido.
- O SGS às vezes responde HTML ou XML de erro com status 200 (exemplo real em `tests/fixtures/sgs-erro.html`): isso é falha da fonte, não "série vazia". Uma nova tentativa depois de 5 s; persistindo, a fonte fica com erro e o resto segue.
- Timeout de 60 s por requisição; `User-Agent: GuruDeInvestimentos/<versão>`. Nenhuma requisição sai da interface; o núcleo nunca manda dados do usuário para fora (só datas e códigos de série nas URLs).
- Os testes nunca acessam a rede: usam os arquivos de `crates/guru-core/tests/fixtures/` (dados reais de 28/10/2026) e um `Baixador` falso.
- **Valor atual é bruto** (antes do IR e de taxas) e sempre vem com a data dos dados usados e a origem. A interface escreve "valor bruto, antes do IR".
- Trabalho na branch `fase-3-dados-de-mercado`. Commits em português terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Os e2e rodam com a janela escondida e **sem rede**: o modo de teste usa `GURU_MERCADO_FIXTURES=<pasta>` (só com `!app.isPackaged`) para o atualizador ler os arquivos da pasta em vez de baixar.

## Regras de valorização (exatas)

Datas de pregão/dias úteis = as datas presentes na série diária do CDI (SGS 12). Fator diário de uma taxa `t` em % ao dia = `1 + t/100`. Toda conta em `Decimal`; o resultado em centavos é arredondado para baixo.

| Tipo | Valor atual na data D |
|---|---|
| Renda fixa `pos` + benchmark `cdi` | `aplicado × Π (1 + cdi_d/100 × taxa/100)` para cada dia útil `d` com `dataAplicacao < d ≤ D` |
| Renda fixa `pos` + benchmark `selic` | o mesmo com a série Selic |
| Renda fixa `pre` | `aplicado × (1 + taxa/100)^(du/252)`, `du` = dias úteis com `dataAplicacao < d ≤ D` |
| Renda fixa `ipca` | `aplicado × Π (1 + ipca_m/100) × (1 + taxa/100)^(du/252)`, com `ipca_m` de cada mês **inteiro** entre o mês seguinte ao da aplicação e o último mês publicado até D |
| Tesouro (`td-*`) | `quantidade × PU Base Manhã em D`, com `quantidade = aplicado / PU Compra Manhã na data da aplicação` (Decimal, sem arredondar), título casado por tipo e vencimento; sem PU na data exata, usa o último PU anterior |
| Renda variável | `quantidade × PREULT` do último pregão disponível ≤ D (código de negociação = ticker, mercado à vista `010`) |
| `fundo`, `prev`, `cripto` | sem fonte: valor atual = aplicado, origem `sem_fonte` |
| Antes da data da aplicação ou sem dados suficientes | valor atual = aplicado, origem `sem_dados` |

A potência fracionária `(1+t)^(du/252)` é calculada com `Decimal` por `exp(ln(1+t) × du/252)` com 12 casas (rust_decimal com a feature `maths`); a tolerância dos testes é 1 centavo.

Rentabilidade = `(atual − aplicado) / aplicado` em décimos de ponto percentual, arredondada meio para cima (`+17,1%` → `171`).

Origens: `calculo` (renda fixa privada), `tesouro`, `cotacao`, `sem_fonte`, `sem_dados`.

## Review Focus

1. **SGS respondendo HTML/XML, B3 sem pregão no dia, Tesouro fora do ar** — cada fonte falha sozinha, com mensagem, sem apagar dados já guardados e sem travar a tela. Teste na Tarefa 3 e na Tarefa 6.
2. **Ticker sem cotação** (código errado, ativo deslistado, compra de hoje antes do primeiro pregão) mostra o valor aplicado com a marca "sem cotação", não zero. Teste na Tarefa 4.
3. **Investimento aplicado num feriado ou fim de semana** e **aplicação de hoje** calculam sem erro (nenhum dia útil = fator 1). Teste na Tarefa 4.
4. **Atualizar duas vezes seguidas** não duplica linhas nem baixa de novo o que já existe. Teste na Tarefa 3.
5. **Primeira atualização com carteira antiga** (aplicação de 2014) busca o CDI em janelas de 10 anos. Teste na Tarefa 3.

---

### Tarefa 1: Leitores dos três formatos

**Files:**
- Create: `crates/guru-core/src/mercado/mod.rs`, `mercado/sgs.rs`, `mercado/tesouro.rs`, `mercado/cotahist.rs`
- Modify: `crates/guru-core/src/lib.rs`
- Test: unidade nos três arquivos, lendo `crates/guru-core/tests/fixtures/*` com `include_str!`/`include_bytes!`

**Interfaces:**
- Produces:
  - `pub struct PontoDiario { pub data: NaiveDate, pub valor: Decimal }`; `pub fn ler_sgs(corpo: &[u8]) -> Result<Vec<PontoDiario>, ErroFonte>` — JSON `[{"data":"dd/MM/aaaa","valor":"0.051660"}]`; corpo que não começa com `[` (após espaços) → `ErroFonte::Formato("resposta inesperada do Banco Central")`; lista vazia é válida.
  - `pub struct PrecoTesouro { pub tipo: String /* "Tesouro Selic" … */, pub vencimento: NaiveDate, pub data: NaiveDate, pub taxa_compra: Decimal, pub pu_compra: Decimal, pub pu_base: Decimal }`; `pub fn ler_tesouro(csv: &[u8]) -> Result<Vec<PrecoTesouro>, ErroFonte>` — cabeçalho exato `Tipo Titulo;Data Vencimento;Data Base;Taxa Compra Manha;Taxa Venda Manha;PU Compra Manha;PU Venda Manha;PU Base Manha`; linhas com campo vazio são puladas; cabeçalho diferente → `ErroFonte::Formato`.
  - `pub struct Cotacao { pub ticker: String, pub data: NaiveDate, pub fechamento_centavos: i64 }`; `pub fn ler_cotahist_txt(txt: &[u8]) -> Result<Vec<Cotacao>, ErroFonte>` (só registros `01` com TPMERC `010`; posições 1-based do layout da B3: TIPREG 1-2, DATA 3-10, CODNEG 13-24 com `trim`, TPMERC 25-27, PREULT 109-121 com 2 casas implícitas) e `pub fn ler_cotahist_zip(zip: &[u8]) -> Result<Vec<Cotacao>, ErroFonte>` (primeiro arquivo `.TXT` do ZIP).
  - `pub enum ErroFonte { Rede(String), Status(u16), Formato(String) }` com `Display` em pt-BR.

- [ ] **Step 1: Testes que falham** — SGS: `sgs-12-cdi.json` tem o primeiro ponto `01/09/2026` = `0.051660`; `sgs-433-ipca.json` lê `01/09/2026` = `0.82` (confira no arquivo); `sgs-erro.html` → `Formato`. Tesouro: `tesouro-amostra.csv` lê `Tesouro Selic` `01/03/2029` em `28/10/2026` com `pu_base = 20151.28`; cabeçalho trocado → `Formato`. COTAHIST: o TXT tem exatamente 6 cotações (ITUB4, BOVA11, HGLG11, B3SA3, M1TA34, XFIX11 — as linhas `020` e `030` são ignoradas), `ITUB4` em `28/10/2026` com fechamento lido de PREULT (confira no arquivo), e o ZIP dá o mesmo resultado que o TXT; bytes que não são ZIP → `Formato`.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar** com `serde_json` (struct com `#[derive(Deserialize)]`), leitura manual do CSV (é simples, sem aspas) e fatias de bytes no COTAHIST (o arquivo é ISO-8859-1; só os campos ASCII são usados).
- [ ] **Step 4: Rodar e ver passar** — `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`.
- [ ] **Step 5: Commit** — "Leitores do SGS, do Tesouro e do COTAHIST".

---

### Tarefa 2: Tabelas de mercado (migração 0004)

**Files:**
- Create: `crates/guru-core/src/migrations/0004_mercado.sql`, `crates/guru-core/src/mercado/repositorio.rs`
- Modify: `crates/guru-core/src/db.rs` (`SCHEMA_VERSION` = 4), as duas asserções literais da versão (addon e e2e) para 4
- Test: `crates/guru-core/tests/mercado.rs`

**Interfaces:**
- Consumes: `PontoDiario`, `PrecoTesouro`, `Cotacao` (Tarefa 1).
- Produces:
  - Tabelas: `indice_diario (serie TEXT, data TEXT, valor TEXT, PRIMARY KEY (serie, data))` (séries `cdi`, `selic`); `ipca_mensal (mes TEXT PRIMARY KEY /* YYYY-MM */, valor TEXT)`; `tesouro_preco (tipo TEXT, vencimento TEXT, data TEXT, taxa_compra TEXT, pu_compra TEXT, pu_base TEXT, PRIMARY KEY (tipo, vencimento, data))`; `cotacao (ticker TEXT, data TEXT, fechamento_centavos INTEGER NOT NULL CHECK (fechamento_centavos > 0), PRIMARY KEY (ticker, data))`; `fonte_status (fonte TEXT PRIMARY KEY, ultima_data TEXT, atualizado_em TEXT, erro TEXT)` (fontes `cdi`, `selic`, `ipca`, `tesouro`, `b3`).
  - `Db::gravar_indice(&self, serie: &str, pontos: &[PontoDiario]) -> Result<usize, CoreError>` (upsert; devolve quantos eram novos), `Db::gravar_ipca`, `Db::gravar_tesouro`, `Db::gravar_cotacoes` — todos `INSERT … ON CONFLICT DO UPDATE` numa transação.
  - Leitura: `Db::indice_entre(serie, de, ate) -> Vec<PontoDiario>`, `Db::ultima_data_indice(serie) -> Option<NaiveDate>`, `Db::ipca_meses(de_mes, ate_mes) -> Vec<(String, Decimal)>`, `Db::preco_tesouro_em(tipo, vencimento, data) -> Option<PrecoTesouro>` (último ≤ data), `Db::cotacao_em(ticker, data) -> Option<Cotacao>` (último ≤ data), `Db::datas_cotacao() -> Vec<NaiveDate>`.
  - `pub struct StatusFonte { pub fonte: String, pub ultima_data: Option<String>, pub atualizado_em: Option<String>, pub erro: Option<String> }`; `Db::registrar_status(&StatusFonte)`, `Db::status_fontes() -> Vec<StatusFonte>` (as cinco, na ordem acima, mesmo nunca atualizadas).

- [ ] **Step 1: Testes que falham** — gravar o CDI da fixture duas vezes devolve N e depois 0 novos (Review Focus 4); `cotacao_em("ITUB4", 2026-10-30)` devolve a de 28/10; `cotacao_em("XXXX3", …)` é `None`; `preco_tesouro_em` sem data exata devolve o anterior; `status_fontes` num banco novo devolve as cinco com tudo `None`; `migra_banco_da_versao_3` mantém investimentos, metas e perfil.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** (workspace, clippy all-targets, fmt; addon build+test; e2e só `-g "abre com título"`).
- [ ] **Step 5: Commit** — "Tabelas de dados de mercado".

---

### Tarefa 3: Atualizador

**Files:**
- Create: `crates/guru-core/src/mercado/rede.rs` (`Baixador`, implementação `ureq`, lista de destinos), `crates/guru-core/src/mercado/atualizador.rs`
- Test: unidade em `rede.rs` (lista de destinos); `crates/guru-core/tests/atualizador.rs` com um `Baixador` falso que serve as fixtures

**Interfaces:**
- Consumes: Tarefas 1 e 2; `Db::listar_investimentos`.
- Produces:
  - `pub trait Baixador { fn baixar(&self, url: &str) -> Result<Vec<u8>, ErroFonte>; }`; `pub struct BaixadorHttp` (ureq, timeout 60 s, User-Agent, recusa URL fora da lista com `ErroFonte::Rede("destino não permitido")`); `pub struct BaixadorPasta { pub pasta: PathBuf }` (modo de teste: mapeia cada URL para um arquivo da pasta — SGS → `sgs-<serie>.json`, Tesouro → `tesouro.csv`, B3 → `COTAHIST_D<ddMMaaaa>.ZIP`; arquivo ausente → `ErroFonte::Status(404)`).
  - `pub struct ResultadoAtualizacao { pub fontes: Vec<StatusFonte>, pub novos: usize }`
  - `pub fn atualizar(db: &Db, baixador: &dyn Baixador, hoje: NaiveDate, espera: &dyn Fn(Duration)) -> Result<ResultadoAtualizacao, CoreError>` — erros de fonte não viram `Err`: ficam no `StatusFonte.erro` da fonte; `Err` só para erro do banco. `espera` é injetada para o teste não dormir 5 s.

Regras do atualizador:
- **Início**: a data da aplicação mais antiga da carteira (ou `hoje − 13 meses` sem investimentos, ou o que for mais antigo dos dois).
- **CDI e Selic**: de `ultima_data_indice + 1 dia` (ou do início) até `hoje`, em janelas de no máximo 10 anos (Review Focus 5); nada a buscar se a última data já é `hoje`.
- **IPCA**: do mês seguinte ao último guardado (ou do início) até `hoje`.
- **Tesouro**: baixa o CSV se a carteira tem algum `td-*` e o status do Tesouro não foi atualizado hoje; grava só os títulos (tipo + vencimento) presentes na carteira, do início até hoje.
- **B3**: se a carteira tem renda variável, tenta os pregões que faltam entre estas datas-alvo: `hoje` e o último dia de cada um dos 12 meses anteriores (para o histórico da fase 3b); para cada data-alvo sem cotação guardada, tenta a data e volta um dia por vez até 7 tentativas até achar um arquivo válido. Grava só os tickers da carteira mais `BOVA11` e `XFIX11` (usados como referência na fase 3b).
- Uma nova tentativa depois de 5 s para cada falha de rede, status ≠ 200 ou formato inválido; a segunda falha registra `erro` com a mensagem em pt-BR (ex.: "O Banco Central não respondeu. Tente de novo mais tarde.") e mantém `ultima_data` anterior.
- `ultima_data` de cada fonte = a data mais recente guardada; `atualizado_em` = agora (UTC, RFC 3339) quando a fonte terminou sem erro.

- [ ] **Step 1: Testes que falham** — com o `BaixadorPasta` das fixtures e uma carteira com um CDB, um Tesouro Selic 2029 e ITUB4: depois de `atualizar`, `status_fontes` sem erros e `ultima_data` do CDI = 28/10/2026; rodar de novo não chama o baixador para o CDI (o falso conta chamadas) e `novos == 0`; baixador falso que devolve `sgs-erro.html` duas vezes → `erro` preenchido no CDI, Selic e B3 seguem; `espera` chamada uma vez por fonte com falha; aplicação em 2014 → as URLs do CDI pedidas são duas janelas que não passam de 10 anos; URL fora da lista recusada pelo `BaixadorHttp` sem rede (teste de unidade da validação, sem chamar a rede); B3: data-alvo num sábado volta para a sexta (o falso só tem o arquivo de 28/10/2026 — use `hoje = 2026-10-31` e confira que pediu 31, 30, 29 e 28).
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar.**
- [ ] **Step 5: Commit** — "Atualizador de dados de mercado".

---

### Tarefa 4: Valorização

**Files:**
- Modify: `crates/guru-core/Cargo.toml` (`rust_decimal` com a feature `maths`)
- Create: `crates/guru-core/src/mercado/valorizacao.rs`
- Modify: `crates/guru-core/src/investimento/repositorio.rs` (`Investimento` ganha os campos de valor atual), `crates/guru-core/src/metas/repositorio.rs` (acumulado pelo valor atual), resumo da carteira
- Test: unidade em `valorizacao.rs`; `crates/guru-core/tests/valorizacao.rs`

**Interfaces:**
- Consumes: leituras da Tarefa 2.
- Produces:
  - `pub enum Origem { Calculo, Tesouro, Cotacao, SemFonte, SemDados }` (códigos `calculo`, `tesouro`, `cotacao`, `sem_fonte`, `sem_dados`) com rótulos "calculado pelo índice", "preço do Tesouro", "cotação da B3", "sem fonte de preço", "sem dados ainda".
  - `pub struct Valor { pub atual_centavos: i64, pub rentabilidade_decimos: i64, pub origem: Origem, pub data_referencia: Option<NaiveDate> }`
  - `pub fn valorizar(db: &Db, inv: &Investimento, data: NaiveDate) -> Result<Valor, CoreError>` — regras da seção "Regras de valorização".
  - `Investimento` ganha `valor_atual_centavos`, `rentabilidade_decimos`, `origem_valor`, `data_valor` (preenchidos em `listar_investimentos`/`investimento` com `data = hoje`; `listar_investimentos` passa a receber `hoje`).
  - **Decisão desta fase**: o resumo da carteira (total, percentuais e diferença contra o alvo) e o acumulado das metas passam a usar o **valor atual**; o resumo ganha também `total_aplicado_centavos` ao lado de `total_atual_centavos`.

- [ ] **Step 1: Testes que falham** (com as fixtures gravadas no banco):
  - CDB 100% CDI de R$ 10.000,00 aplicado em 31/08/2026: valor em 28/10/2026 = `1.000.000 × Π(1 + cdi_d/100)` dos dias da fixture (calcule o esperado no teste a partir do arquivo, não à mão), tolerância 1 centavo.
  - 110% do CDI rende mais que 100%; aplicação em 28/10/2026 (hoje) → valor = aplicado, rentabilidade 0; aplicação num sábado → conta a partir do dia útil seguinte (Review Focus 3).
  - Prefixado 12% com 252 dias úteis → `aplicado × 1,12` (monte uma série sintética de 252 datas no teste).
  - IPCA+ 6%: só meses inteiros do IPCA entram (aplicação em 15/03/2025 → a partir de abril/2025).
  - Tesouro Selic 2029 com a amostra: quantidade pela data de compra e PU base de 28/10/2026.
  - ITUB4 150 × fechamento da fixture; `XXXX3` → origem `sem_dados` e valor = aplicado (Review Focus 2); `cripto` → `sem_fonte`.
  - rentabilidade +17,1% → `171`; prejuízo → negativo.
  - resumo e metas usam o valor atual; sem dados de mercado, continuam batendo com o aplicado (os testes antigos de resumo e metas seguem passando).
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.** Carregue as séries uma vez por chamada de `listar_investimentos`, não uma vez por investimento.
- [ ] **Step 4: Rodar e ver passar** (workspace, clippy all-targets, fmt; addon).
- [ ] **Step 5: Commit** — "Valor atual dos investimentos pelas fontes oficiais".

---

### Tarefa 5: Addon e IPC de mercado

**Files:**
- Create: `crates/guru-node/src/mercado.rs`
- Modify: `crates/guru-node/src/{lib,investimentos,metas}.rs`, `app/main.js`, `app/preload.js`, `app/renderer/js/api.js`
- Test: `crates/guru-node/test/mercado.test.mjs`, `app/test/unit/api.test.mjs`, `app/test/e2e/abrir.spec.js` (superfície)

**Interfaces:**
- Produces (addon):
  - `core.atualizarMercado(): Promise<{ fontes: StatusFonteJs[], novos: number }>` — `AsyncTask` (não bloqueia o main); usa `BaixadorHttp`, ou `BaixadorPasta` quando o addon recebe `Core.open(dataDir, { pastaFixtures })` com a pasta (o main só passa a pasta com `GURU_MERCADO_FIXTURES` e `!app.isPackaged`).
  - `core.statusMercado(): StatusFonteJs[]` com `{ fonte, rotulo /* "CDI", "Selic", "IPCA", "Tesouro Direto", "Cotações da B3" */, ultimaData?, atualizadoEm?, erro? }`.
  - `Investimento` JS ganha `valorAtualCentavos`, `rentabilidadeDecimos`, `origemValor`, `origemValorRotulo`, `dataValor?`; resumo ganha `totalAtualCentavos`; metas usam o valor atual (sem mudar nomes de campo).
  - A `Db` atrás do `Mutex` não pode ficar travada durante o download: o atualizador baixa sem o lock e só o pega para gravar (divida `atualizar` em "planejar e baixar" e "gravar", ou abra uma segunda conexão `Db::open` para a tarefa — escolha e documente).
- Produces (IPC/api): `api.mercado = { atualizar, status }` (canais `mercado:atualizar`, `mercado:status`); superfície `["app", "investimentos", "carteira", "perfil", "alocacao", "objetivo", "metas", "mercado"]`.

- [ ] **Step 1: Testes que falham** — addon com `pastaFixtures` = `crates/guru-core/tests/fixtures`: `atualizarMercado()` resolve com 5 fontes; um CDB lido depois tem `origemValor === "calculo"`; `statusMercado()` tem os rótulos exatos; e2e: superfície e `api.mercado.status()`.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** (Rust, addon, unitários, `npx playwright test abrir`).
- [ ] **Step 5: Commit** — "Atualização de mercado no addon e na ponte".

---

### Tarefa 6: Interface — atualização, valor atual e rentabilidade

**Files:**
- Create: `app/renderer/js/mercado.js` (estado da última atualização e o indicador do menu)
- Modify: `app/renderer/index.html` (indicador no rodapé do menu), `app/renderer/js/app.js`, `telas/investimentos.js`, `telas/carteira.js`, `telas/perfil/metas.js` (só textos, se preciso), `css/telas.css`, `app/playwright.config.js` (`GURU_MERCADO_FIXTURES` apontando para `crates/guru-core/tests/fixtures`)
- Test: `app/test/e2e/mercado.spec.js`; ajustes em `investimentos.spec.js`, `carteira.spec.js`

Comportamento:
- Ao abrir o app, se alguma fonte não foi atualizada hoje, roda `api.mercado.atualizar()` em segundo plano (uma vez por abertura). No rodapé do menu, acima das versões: "Dados de mercado de dd/mm" (a menor `ultimaData` entre as fontes com dados; "Dados de mercado: ainda não baixados" sem nenhuma), um botão "Atualizar" (desabilitado e com o texto "Atualizando…" durante a atualização) e, se alguma fonte tem erro, a marca "com falhas" que mostra a lista de fontes e erros num `<dialog>` ao clicar. Ao terminar, a tela atual é redesenhada (re-dispara a rota).
- Lista de investimentos e detalhe da classe: colunas do protótipo "Aplicado", "Atual" e "Rentab." (`+17,1%` com classe `up`/`down`, `0,0%` neutro). Origem `sem_fonte`/`sem_dados` mostra o valor aplicado em "Atual" com a marca "sem cotação" e "—" em "Rentab.". Rodapé da tabela: total aplicado e total atual.
- Carteira: "Patrimônio" (total atual) em destaque e "Total aplicado" ao lado; cards por classe com o valor atual; percentuais e alvo já vêm do núcleo pelo valor atual. A nota "Valores aplicados. Cotações e rentabilidade chegam numa próxima versão." vira "Valores brutos, antes do IR. Dados de mercado de dd/mm."
- Metas: nenhuma mudança visual além de o acumulado já vir pelo valor atual.

- [ ] **Step 1: Testes e2e que falham** — com as fixtures: ao abrir, o rodapé chega a "Dados de mercado de 28/10" (o menor entre as fontes é 28/10 para CDI/Selic/B3/Tesouro e 01/09 para o IPCA — **decida no teste**: o IPCA é mensal, então ele fica fora do "menor"; documente no código); lista com ITUB4 mostra o atual pela fixture e a rentabilidade com sinal; um ticker inexistente mostra "sem cotação" e "—"; carteira mostra "Patrimônio"; pasta de fixtures sem o arquivo do CDI (copie a pasta sem ele num temporário) → marca "com falhas" e o diálogo lista "CDI" com a mensagem; "Atualizar" desabilita durante a atualização.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** — suíte inteira (Rust, addon, unitários e e2e) uma vez.
- [ ] **Step 5: Commit** — "Valor atual, rentabilidade e atualização de mercado na interface".

---

### Tarefa 7: Documentação

**Files:**
- Modify: `docs/arquitetura.md` (seção Rede: trocar a linha do IBGE por "IPCA pela série 433 do SGS (dado do IBGE publicado pelo Banco Central)"; descrever o modo de fixtures; tabelas de mercado em Dados), `docs/mvp.md` (Decisões: "Valor atual" — "Bruto, antes do IR, calculado pelos índices oficiais (renda fixa), pelo preço do Tesouro e pela cotação de fechamento da B3. Fundos, previdência e cripto ficam pelo valor aplicado até haver fonte."; "Ibovespa e IFIX" — "Comparação pelos ETFs BOVA11 e XFIX11, cujas cotações vêm do mesmo arquivo da B3."), `docs/roadmap.md` (fase 3 dividida em 3a e 3b, link deste plano)

- [ ] **Step 1: Editar os três arquivos.**
- [ ] **Step 2: Commit** — "Documentação dos dados de mercado".
