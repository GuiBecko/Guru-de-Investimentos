# Fase 1: Carteira manual — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O usuário lança, edita e apaga investimentos num formulário de campos fechados que se adapta ao tipo (IR e garantia preenchidos pelo tipo, indexador forçado no Tesouro, campos de renda variável), vê a lista de investimentos, a carteira por classe com totais aplicados e o detalhe de cada classe em tabela. Tudo gravado no SQLite e mantido entre aberturas do app.

**Architecture:** As regras de cada tipo (catálogo), a validação e a gravação ficam no `guru-core`. O addon expõe o catálogo e o CRUD. O main ganha um canal IPC por operação, todos com checagem de origem. A interface monta o formulário a partir do catálogo, então nenhuma regra de tipo é duplicada em JS. Erros de validação são um valor de retorno (lista de campos com mensagem), não uma exceção.

**Tech Stack:** o mesmo da fase 0, mais `rust_decimal` (taxas) e `chrono` (datas) no `guru-core`.

**Spec:** [`docs/mvp.md`](../../mvp.md) (tela 4 e tela 6), [`docs/arquitetura.md`](../../arquitetura.md) (seção Dados). Visual e comportamento do formulário: [`docs/prototipo/index.html`](../../prototipo/index.html), telas `#manual` e `#classe`.

## Global Constraints

- Tudo da fase 0 continua valendo: segurança da janela, CSP exata, pt-BR com acentos, códigos de erro em `SCREAMING_SNAKE_CASE`, preload sem lógica, a interface chama sempre por `renderer/js/api.js`.
- **Dinheiro em centavos, `i64`.** Nunca `f64` para reais, nem no JS: a interface converte o texto digitado em centavos inteiros antes de enviar.
- **Taxas em `rust_decimal::Decimal`**, trafegando como texto com ponto (`"102.5"`).
- **Datas como texto `YYYY-MM-DD`** no banco, na ponte e no IPC; `chrono::NaiveDate` no Rust.
- Campos fechados: tipo, instituição (lista + "Outra"), indexador, benchmark, liquidez e IR são `select`; datas são `input type="date"`; valores em reais com máscara. Texto livre só em emissor/nome, ticker, nome de instituição "Outra" e observação.
- Todo canal IPC novo passa por `tratar` e pela checagem de origem da Tarefa 4.
- Trabalho na branch `fase-1-carteira-manual`. Commits em português terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Os e2e abrem janelas na tela do usuário: enquanto itera, rode só os testes focados (`-g`); a suíte inteira uma vez antes do commit.

## Catálogo de tipos (valores exatos)

| código | rótulo | grupo | classe | forma | indexador forçado | liquidez forçada | IR padrão | IR editável | garantia |
|---|---|---|---|---|---|---|---|---|---|
| `rf-cdb` | CDB | Renda fixa | `rf` | `renda_fixa` | — | — | `regressiva` | não | `fgc` |
| `rf-lci` | LCI | Renda fixa | `rf` | `renda_fixa` | — | — | `isento` | não | `fgc` |
| `rf-lca` | LCA | Renda fixa | `rf` | `renda_fixa` | — | — | `isento` | não | `fgc` |
| `rf-lc` | LC | Renda fixa | `rf` | `renda_fixa` | — | — | `regressiva` | não | `fgc` |
| `rf-deb` | Debênture | Renda fixa | `rf` | `renda_fixa` | — | — | `regressiva` | **sim** | `nenhuma` |
| `rf-cri` | CRI | Renda fixa | `rf` | `renda_fixa` | — | — | `isento` | não | `nenhuma` |
| `rf-cra` | CRA | Renda fixa | `rf` | `renda_fixa` | — | — | `isento` | não | `nenhuma` |
| `td-selic` | Tesouro Selic | Renda fixa | `rf` | `renda_fixa` | `selic` | `d1` | `regressiva` | não | `tesouro` |
| `td-pre` | Tesouro Prefixado | Renda fixa | `rf` | `renda_fixa` | `pre` | `d1` | `regressiva` | não | `tesouro` |
| `td-ipca` | Tesouro IPCA+ | Renda fixa | `rf` | `renda_fixa` | `ipca` | `d1` | `regressiva` | não | `tesouro` |
| `rv-acao` | Ação | Renda variável | `acoes` | `renda_variavel` | — | — | — | — | — |
| `rv-fii` | Fundo imobiliário (FII) | Renda variável | `fii` | `renda_variavel` | — | — | — | — | — |
| `rv-etf` | ETF | Renda variável | `acoes` | `renda_variavel` | — | — | — | — | — |
| `rv-bdr` | BDR | Renda variável | `ext` | `renda_variavel` | — | — | — | — | — |
| `fundo` | Fundo de investimento | Outros | `outros` | `valor` | — | — | — | — | — |
| `prev` | Previdência (PGBL/VGBL) | Outros | `outros` | `valor` | — | — | — | — | — |
| `cripto` | Criptoativo | Outros | `outros` | `valor` | — | — | — | — | — |

Rótulos dos códigos:

- Classes (nesta ordem): `rf` Renda fixa, `acoes` Ações, `fii` FIIs, `ext` Exterior, `outros` Outros.
- Indexador: `pos` Pós-fixado, `pre` Prefixado, `ipca` Híbrido (IPCA + taxa), `selic` Selic + taxa. Os três primeiros são as opções do `select` para renda fixa privada; `selic` só existe forçado no Tesouro Selic.
- Benchmark (só com `pos`): `cdi` CDI, `selic` Selic.
- Liquidez: `diaria` Diária, `d1` D+1, `d30` D+30, `vencimento` No vencimento, `carencia` Carência até uma data.
- IR: `regressiva` Tabela regressiva, `isento` Isento. Garantia: `fgc` FGC, `tesouro` Tesouro Nacional, `nenhuma` Sem garantia.
- Instituições sugeridas (nesta ordem): Banco Inter, XP Investimentos, BTG Pactual, Rico, Nubank, Banco do Brasil, Itaú, Bradesco, Santander, Caixa Econômica Federal, C6 Bank. A interface acrescenta "Outra…", que abre um campo de texto.

Textos calculados pelo núcleo (formato pt-BR: vírgula decimal, sem zeros à direita, `102.50` → `102,5`):

- `nome`: renda fixa privada `"{rótulo} {emissor}"` (`CDB Banco Inter`); Tesouro `"{rótulo} {ano do vencimento}"` (`Tesouro Selic 2029`); renda variável o ticker (`ITUB4`); forma `valor` o emissor (que a interface chama de "Nome").
- `rendimento`: `pos` → `102% do CDI`; `pre` → `12,5% ao ano`; `ipca` → `IPCA + 6,2% ao ano`; `selic` → `Selic + 0,09% ao ano`. Sem rendimento fora da renda fixa.

## Regras de validação

Cada erro é `{ campo, mensagem }`, com `campo` no nome camelCase do campo de entrada e mensagem curta em pt-BR dizendo o que corrigir. A validação junta **todos** os erros, não para no primeiro. `hoje` é parâmetro (testável).

| campo | regra | mensagem exata |
|---|---|---|
| `tipo` | código do catálogo | Escolha um tipo da lista. |
| `instituicao` | 1 a 60 caracteres após `trim` | Informe a corretora ou o banco. |
| `emissor` | obrigatório em `renda_fixa` (menos Tesouro) e `valor`; 1 a 80 caracteres | Informe o emissor. (em `valor`: Informe o nome.) |
| `ticker` | obrigatório em `renda_variavel`; após `trim` e maiúsculas, casa `^[A-Z]{4}[0-9]{1,2}$` | Use o código da B3, por exemplo ITUB4. |
| `indexador` | obrigatório em renda fixa; `pos`, `pre` ou `ipca`, ou o forçado do tipo | Escolha o indexador. |
| `taxa` | obrigatória em renda fixa; decimal; `pos`: > 0 e ≤ 300; `pre`, `ipca`: > 0 e ≤ 100; `selic`: ≥ 0 e ≤ 10 | Informe uma taxa válida. |
| `benchmark` | obrigatório se e só se `indexador = pos` | Escolha CDI ou Selic. |
| `dataAplicacao` | obrigatória; ≤ `hoje` | A data da aplicação não pode estar no futuro. (vazia: Informe a data da aplicação.) |
| `vencimento` | obrigatório no Tesouro e com liquidez `vencimento`; se presente, > `dataAplicacao` | O vencimento precisa ser depois da aplicação. (vazio: Informe o vencimento.) |
| `valorAplicadoCentavos` | obrigatório em `renda_fixa` e `valor`; > 0 | Informe o valor aplicado. |
| `quantidade` | obrigatória em `renda_variavel`; inteiro > 0 | Informe a quantidade. |
| `precoMedioCentavos` | obrigatório em `renda_variavel`; > 0; `quantidade × preço` sem estouro de `i64` | Informe o preço médio. |
| `liquidez` | obrigatória em `renda_fixa` e `valor`; no Tesouro, só a forçada | Escolha a liquidez. |
| `dataCarencia` | obrigatória se `liquidez = carencia`; > `dataAplicacao` e, havendo vencimento, ≤ `vencimento` | Informe uma data de carência entre a aplicação e o vencimento. |
| `ir` | obrigatório em renda fixa; igual ao padrão do tipo, a menos que o IR seja editável | O IR deste tipo é definido por lei. |
| `observacao` | opcional; até 200 caracteres | Use até 200 caracteres. |

Campos que não pertencem à forma do tipo (por exemplo `ticker` num CDB) são ignorados e gravados como `NULL`. A garantia não é entrada: vem do catálogo.

## Review Focus

1. **Valor digitado em formatos brasileiros** (`20.000,00`, `20000`, `R$ 1.234,5`, `0,01`) vira o centavo certo, e formatos ambíguos ou inválidos (`1,234,56`, `abc`, `-5`, `1.2.3`) são recusados com mensagem no campo, nunca viram 0 em silêncio. Teste na Tarefa 5.
2. **Trocar o tipo no meio do preenchimento** (CDB → Ação → LCI) não deixa valor fantasma de outra forma salvo no banco nem IR incoerente. Teste na Tarefa 1 (campos ignorados) e na Tarefa 6.
3. **Editar e apagar um investimento que não existe mais** (id antigo numa aba, banco mexido) dá `NAO_ENCONTRADO` com mensagem, sem travar a tela. Teste na Tarefa 2 e na Tarefa 6.
4. **Os dados continuam depois de fechar e abrir o app.** Teste na Tarefa 6.
5. **Taxa com muitas casas ou separador errado** (`102,5` digitado no campo numérico, `0,0905`) chega ao núcleo com o valor certo e volta formatada igual. Teste na Tarefa 1 e na Tarefa 6.

---

### Tarefa 1: Catálogo, tipos e validação no `guru-core`

**Files:**
- Modify: `crates/guru-core/Cargo.toml` (adicionar `rust_decimal` e `chrono`)
- Create: `crates/guru-core/src/investimento/mod.rs` (reexporta)
- Create: `crates/guru-core/src/investimento/catalogo.rs`
- Create: `crates/guru-core/src/investimento/modelo.rs`
- Create: `crates/guru-core/src/investimento/validacao.rs`
- Create: `crates/guru-core/src/investimento/texto.rs` (`nome` e `rendimento`)
- Modify: `crates/guru-core/src/lib.rs`
- Test: testes de unidade em cada arquivo (`#[cfg(test)]`)

**Interfaces:**
- Produces:
  - Enums com `fn codigo(&self) -> &'static str`, `fn rotulo(&self) -> &'static str` e `fn de_codigo(&str) -> Option<Self>`: `Classe`, `Forma`, `Indexador`, `Benchmark`, `Liquidez`, `Ir`, `Garantia`, com os códigos e rótulos da seção Catálogo.
  - `pub struct TipoInfo { pub codigo, pub rotulo, pub grupo: &'static str, pub classe: Classe, pub forma: Forma, pub indexador_forcado: Option<Indexador>, pub liquidez_forcada: Option<Liquidez>, pub ir_padrao: Option<Ir>, pub ir_editavel: bool, pub garantia: Option<Garantia> }`
  - `pub fn tipos() -> &'static [TipoInfo]` (na ordem da tabela), `pub fn tipo(codigo: &str) -> Option<&'static TipoInfo>`, `pub const INSTITUICOES: &[&str]`.
  - `pub struct NovoInvestimento { pub tipo: String, pub instituicao: String, pub emissor: Option<String>, pub ticker: Option<String>, pub indexador: Option<String>, pub taxa: Option<String>, pub benchmark: Option<String>, pub data_aplicacao: Option<String>, pub vencimento: Option<String>, pub valor_aplicado_centavos: Option<i64>, pub quantidade: Option<i64>, pub preco_medio_centavos: Option<i64>, pub liquidez: Option<String>, pub data_carencia: Option<String>, pub ir: Option<String>, pub observacao: Option<String> }` — a entrada como chega da interface, ainda não validada.
  - `pub struct ErroCampo { pub campo: String, pub mensagem: String }`
  - `pub fn validar(novo: &NovoInvestimento, hoje: NaiveDate) -> Result<InvestimentoValido, Vec<ErroCampo>>`, onde `InvestimentoValido` tem os campos já convertidos (`Decimal`, `NaiveDate`, enums, ticker em maiúsculas, textos com `trim`, campos fora da forma como `None`, indexador/liquidez forçados aplicados).
  - `pub fn nome(v: &InvestimentoValido) -> String` e `pub fn rendimento(v: &InvestimentoValido) -> Option<String>`.

- [ ] **Step 1: Escrever os testes que falham**

Testes mínimos (nomes e asserções):
- `catalogo_tem_os_17_tipos_na_ordem` — `tipos().len() == 17`, primeiro `rf-cdb`, último `cripto`.
- `lci_e_isenta_com_fgc_e_ir_nao_editavel`, `debenture_tem_ir_editavel_e_sem_garantia`, `tesouro_selic_forca_indexador_selic_e_liquidez_d1`, `bdr_e_da_classe_exterior`.
- `cdb_valido_passa` — CDB Banco Inter, `pos`, `"102"`, `cdi`, aplicação `2025-03-15`, vencimento `2028-03-15`, `2_000_000` centavos, `vencimento`, `regressiva`, com `hoje = 2026-10-28` → `Ok`.
- `junta_todos_os_erros` — entrada vazia com tipo `rf-cdb` → contém erros para `instituicao`, `emissor`, `indexador`, `taxa`, `dataAplicacao`, `valorAplicadoCentavos`, `liquidez`, `ir`.
- `lci_nao_aceita_ir_regressivo` → erro `ir` com "O IR deste tipo é definido por lei."; `debenture_aceita_isento` → `Ok`.
- `benchmark_so_com_pos` — `pre` com `benchmark: Some("cdi")` → erro `benchmark`; `pos` sem benchmark → erro `benchmark`.
- `data_futura_recusada`, `vencimento_antes_da_aplicacao_recusado`, `tesouro_sem_vencimento_recusado`.
- `ticker_normalizado_e_validado` — `" itub4 "` vira `ITUB4`; `"PETR"` → erro `ticker` com "Use o código da B3, por exemplo ITUB4."
- `campos_de_outra_forma_sao_descartados` — CDB válido com `ticker: Some("ITUB4")`, `quantidade: Some(10)` → `InvestimentoValido` com `ticker` e `quantidade` `None`.
- `estouro_de_quantidade_vezes_preco_recusado` — `quantidade = i64::MAX`, `preco_medio_centavos = 2` → erro `precoMedioCentavos`.
- `taxa_com_virgula_recusada` — `taxa: Some("102,5")` → erro `taxa` (a interface manda com ponto; vírgula aqui é bug de quem chamou).
- `nome_e_rendimento` — CDB → `CDB Banco Inter` / `102% do CDI`; `taxa "102.50"` → `102,5% do CDI`; Tesouro Selic venc. `2029-03-01` taxa `"0.0905"` → `Tesouro Selic 2029` / `Selic + 0,0905% ao ano`; `pre` `"12.5"` → `12,5% ao ano`; `ipca` `"6.2"` → `IPCA + 6,2% ao ano`; ação → `ITUB4` / `None`.

- [ ] **Step 2: Rodar e ver falhar** — `cargo test -p guru-core` → não compila.
- [ ] **Step 3: Implementar** catálogo como `static` array; validação como uma função por campo que empurra em `Vec<ErroCampo>`.
- [ ] **Step 4: Rodar e ver passar** — `cargo test -p guru-core && cargo clippy --workspace -- -D warnings && cargo fmt --check`.
- [ ] **Step 5: Commit** — "Catálogo de tipos e validação de investimentos no núcleo".

---

### Tarefa 2: Tabela e repositório de investimentos

**Files:**
- Create: `crates/guru-core/src/migrations/0002_investimentos.sql`
- Create: `crates/guru-core/src/investimento/repositorio.rs` (métodos em `impl Db`)
- Modify: `crates/guru-core/src/error.rs` (variante `NaoEncontrado(i64)`, code `NAO_ENCONTRADO`, mensagem "Este investimento não existe mais. Atualize a lista.")
- Modify: `crates/guru-core/src/db.rs` (registrar a migração; `SCHEMA_VERSION` passa a 2)
- Test: `crates/guru-core/tests/investimentos.rs`; ajustar `tests/db.rs` se ele fixa a versão 1

**Interfaces:**
- Consumes: `validar`, `InvestimentoValido`, `nome`, `rendimento`, `tipo`, enums (Tarefa 1).
- Produces:
  - `pub enum Gravacao { Ok(i64), Invalido(Vec<ErroCampo>) }`
  - `Db::criar_investimento(&self, novo: &NovoInvestimento, hoje: NaiveDate) -> Result<Gravacao, CoreError>`
  - `Db::atualizar_investimento(&self, id: i64, novo: &NovoInvestimento, hoje: NaiveDate) -> Result<Gravacao, CoreError>` (`NaoEncontrado` se o id não existe; `Gravacao::Ok(id)` se gravou)
  - `Db::apagar_investimento(&self, id: i64) -> Result<(), CoreError>` (`NaoEncontrado` se não existe)
  - `Db::listar_investimentos(&self) -> Result<Vec<Investimento>, CoreError>` ordenado por classe (ordem do catálogo), depois `nome`
  - `Db::investimento(&self, id: i64) -> Result<Investimento, CoreError>`
  - `Db::resumo_carteira(&self) -> Result<Resumo, CoreError>`
  - `pub struct Investimento { id, tipo: String, tipo_rotulo: String, classe: Classe, nome: String, instituicao, emissor: Option<String>, ticker: Option<String>, indexador: Option<Indexador>, taxa: Option<String>, benchmark: Option<Benchmark>, rendimento: Option<String>, data_aplicacao: String, vencimento: Option<String>, valor_aplicado_centavos: i64, quantidade: Option<i64>, preco_medio_centavos: Option<i64>, liquidez: Option<Liquidez>, data_carencia: Option<String>, ir: Option<Ir>, garantia: Option<Garantia>, observacao: Option<String> }` — em renda variável `valor_aplicado_centavos = quantidade × preço médio`; `taxa` como texto com ponto, sem zeros à direita.
  - `pub struct Resumo { pub total_aplicado_centavos: i64, pub classes: Vec<ResumoClasse> }`, `pub struct ResumoClasse { pub classe: Classe, pub total_aplicado_centavos: i64, pub quantidade: i64 }` — só classes com investimento, na ordem do catálogo.

Tabela `investimentos`: `id INTEGER PRIMARY KEY`, uma coluna por campo de `InvestimentoValido` (texto para códigos, decimais e datas; inteiro para centavos e quantidade), `criado_em TEXT NOT NULL`, `atualizado_em TEXT NOT NULL` (UTC, RFC 3339). `CHECK` no banco para `valor_aplicado_centavos > 0` quando não nulo, `quantidade > 0` e `preco_medio_centavos > 0` quando não nulos. Índice em `tipo`.

- [ ] **Step 1: Escrever os testes que falham**
  - `cria_e_lista` — cria um CDB e um ITUB4 (150 × 3142) → lista com 2; o CDB tem `nome = "CDB Banco Inter"`, `rendimento = Some("102% do CDI")`, `garantia = Some(Fgc)`; a ação tem `valor_aplicado_centavos = 471300`.
  - `invalido_nao_grava` — `Gravacao::Invalido` e a lista continua vazia.
  - `atualiza_mantem_id_e_muda_campos`, `atualiza_id_inexistente_da_nao_encontrado`, `apaga_e_some_da_lista`, `apaga_id_inexistente_da_nao_encontrado` (code `NAO_ENCONTRADO`).
  - `resumo_por_classe` — CDB 20.000,00 + Tesouro 18.000,00 + ITUB4 4.713,00 + HGLG11 10 × 160,00 → total `4_431_300`; classes `[rf 3_800_000 (2), acoes 471_300 (1), fii 160_000 (1)]`.
  - `dados_sobrevivem_a_reabrir` — cria, `drop(db)`, `Db::open` de novo → lista com 1.
  - `migra_banco_da_versao_1` — banco criado só com a 0001 (via `rusqlite` com `user_version = 1` e a tabela `app_meta`) abre e fica na versão 2.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar** com `rusqlite` e parâmetros nomeados; leitura monta `Investimento` com `nome`/`rendimento` calculados.
- [ ] **Step 4: Rodar e ver passar** (suíte do workspace, clippy, fmt).
- [ ] **Step 5: Commit** — "Tabela e repositório de investimentos".

---

### Tarefa 3: Catálogo e CRUD no addon

**Files:**
- Modify: `crates/guru-node/src/lib.rs` (ou dividir em `src/investimentos.rs` se passar de ~150 linhas)
- Test: `crates/guru-node/test/investimentos.test.mjs`

**Interfaces:**
- Consumes: tudo que a Tarefa 2 produz.
- Produces (JS, camelCase gerado pelo napi):
  - `core.catalogo(): Catalogo` — `{ tipos: TipoInfo[], classes: {codigo, rotulo}[], indexadores, benchmarks, liquidezes, irs, garantias: {codigo, rotulo}[], instituicoes: string[] }`; `TipoInfo` = `{ codigo, rotulo, grupo, classe, forma, indexadorForcado?, liquidezForcada?, irPadrao?, irEditavel, garantia? }` (códigos como texto).
  - `core.listarInvestimentos(): Investimento[]`, `core.investimento(id: number): Investimento`
  - `core.criarInvestimento(dados: NovoInvestimento): ResultadoGravacao`, `core.atualizarInvestimento(id: number, dados: NovoInvestimento): ResultadoGravacao` — `ResultadoGravacao = { id?: number, erros: { campo, mensagem }[] }`; `erros` vazio quando gravou.
  - `core.apagarInvestimento(id: number): void`
  - `core.resumoCarteira(): { totalAplicadoCentavos, classes: { classe, rotulo, totalAplicadoCentavos, quantidade }[] }`
  - Enums atravessam como o `codigo` em texto; `Investimento` ganha `classeRotulo`, `indexadorRotulo?`, `liquidezRotulo?`, `irRotulo?`, `garantiaRotulo?` para a interface não precisar de tabela de rótulos.
  - `hoje` vem de `chrono::Local::now().date_naive()` dentro do addon.
  - `NAO_ENCONTRADO` sai como erro JS com `code`, como os demais.

- [ ] **Step 1: Testes que falham** — `catalogo` tem 17 tipos e `rf-lci` com `irPadrao: "isento"`, `irEditavel: false`, `garantia: "fgc"`; cria CDB e lê de volta com `rendimento: "102% do CDI"` e `garantiaRotulo: "FGC"`; criar inválido devolve `erros` com `campo: "emissor"` e não grava; apagar id inexistente lança `{ code: "NAO_ENCONTRADO" }`; `resumoCarteira` com um CDB tem `classes[0].rotulo === "Renda fixa"`.
- [ ] **Step 2: Rodar e ver falhar** — `cd crates/guru-node && npm run build && npm test`.
- [ ] **Step 3: Implementar** structs `#[napi(object)]` de entrada e saída e conversões em funções pequenas.
- [ ] **Step 4: Rodar e ver passar** (inclui a suíte Rust, clippy e fmt).
- [ ] **Step 5: Commit** — "Catálogo e CRUD de investimentos no addon".

---

### Tarefa 4: Canais IPC, checagem de origem e variáveis de teste

**Files:**
- Create: `app/main/origem.js` (CommonJS)
- Modify: `app/main.js`, `app/preload.js`, `app/renderer/js/api.js`
- Test: `app/test/unit/origem.test.js`, `app/test/unit/api.test.mjs`, `app/test/e2e/abrir.spec.js`

**Interfaces:**
- Consumes: métodos do addon da Tarefa 3; `tratar` e `desembrulhar` da fase 0.
- Produces:
  - `origemPermitida(urlDoQuadro: string, urlEsperada: string): boolean` em `app/main/origem.js` — compara protocolo `file:` e caminho, ignorando `?` e `#`; falso para qualquer outro protocolo, caminho ou URL inválida.
  - `tratar(handler)` passa a recusar, antes do handler, eventos cujo `event.senderFrame.url` não seja `renderer/index.html`, com `code: "ORIGEM_RECUSADA"`. `app:info` também passa por isso.
  - Canais: `investimentos:catalogo`, `investimentos:listar`, `investimentos:ler` (id), `investimentos:criar` (dados), `investimentos:atualizar` (id, dados), `investimentos:apagar` (id), `carteira:resumo`. Cada um repassa ao método do addon de mesmo papel.
  - `window.guru` ganha `investimentos: { catalogo, listar, ler, criar, atualizar, apagar }` e `carteira: { resumo }`, cada um só um `ipcRenderer.invoke`.
  - `api` espelha a mesma forma (`api.investimentos.criar(dados)` etc.), sempre com `desembrulhar`.
  - `GURU_DATA_DIR`, `GURU_ADDON_PATH` e `GURU_TESTE_ERRO_INFO` só valem com `!app.isPackaged`.

- [ ] **Step 1: Testes que falham**
  - unitário `origemPermitida`: aceita `file:///x/renderer/index.html` e `file:///x/renderer/index.html#/investimentos`; recusa `file:///x/renderer/erro.html`, `https://exemplo.com/`, `file:///y/renderer/index.html`, `""` e `"não é url"`.
  - e2e de superfície atualizado: `Object.keys(window.guru)` = `["app", "investimentos", "carteira"]`; `Object.keys(window.guru.investimentos)` = `["catalogo", "listar", "ler", "criar", "atualizar", "apagar"]`.
  - e2e: `api.investimentos.catalogo()` pela página devolve 17 tipos; `api.investimentos.apagar(999)` rejeita com `code: "NAO_ENCONTRADO"`.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.** `tratar` recebe o `event` como primeiro argumento do `ipcMain.handle`; a URL esperada é `pathToFileURL(path.join(__dirname, "renderer", "index.html")).href`.
- [ ] **Step 4: Rodar e ver passar** — `cd app && npm test && npx playwright test`.
- [ ] **Step 5: Commit** — "Canais IPC de investimentos com checagem de origem".

---

### Tarefa 5: Roteador com parâmetros, formatação e lista de investimentos

**Files:**
- Modify: `app/renderer/js/router.js`, `app/test/unit/router.test.mjs`
- Create: `app/renderer/js/formato.js`
- Modify: `app/renderer/js/telas/investimentos.js`
- Create: `app/renderer/css/telas.css` (tabela, botões e diálogo, no estilo do protótipo: `.tbl`, `.btn`, `.btn.ghost`, cantos em pixel) e link em `index.html`
- Test: `app/test/unit/formato.test.mjs`, `app/test/e2e/investimentos.spec.js`

**Interfaces:**
- Consumes: `api.investimentos.listar`, `api.investimentos.apagar` (Tarefa 4).
- Produces:
  - `resolver(hash): { tela: Tela, parametros: string[] }` — `"#/investimentos/novo"` → tela `investimentos`, `["novo"]`; `"#/carteira/rf"` → `carteira`, `["rf"]`; vazio ou desconhecido → `carteira`, `[]`. O menu marca `.on` pela tela.
  - Cada tela passa a ter `render(el, parametros)` (as provisórias ignoram o segundo argumento).
  - `formato.js`:
    - `formatarReais(centavos: number): string` — `2000000` → `"R$ 20.000,00"`, `1` → `"R$ 0,01"`, `0` → `"R$ 0,00"`.
    - `paraCentavos(texto: string): number | null` — aceita `"20.000,00"`, `"20000"`, `"20000,5"` (→ 2000050), `"R$ 1.234,56"`, `"0,01"`, espaços nas pontas; devolve `null` para `""`, `"abc"`, `"-5"`, `"1,234,56"`, `"1.2.3"`, `"1.23"` (ponto só como milhar, em grupos de 3), mais de 2 casas decimais e valores acima de `Number.MAX_SAFE_INTEGER` centavos. Sem `parseFloat`: dígitos são lidos como inteiros.
    - `formatarData("2028-03-15")` → `"15/03/2028"`; `null`/`""` → `"—"`.
  - Tela Investimentos sem parâmetro: título "Investimentos", botão "Novo investimento" (`href="#/investimentos/novo"`), tabela com colunas Investimento, Tipo, Instituição, Rendimento, Aplicado, Vencimento, Liquidez e ações Editar (`#/investimentos/<id>`) e Apagar; linha de total "Total aplicado". Sem investimentos: "Nenhum investimento ainda." e o botão.
  - Apagar abre um `<dialog>` "Apagar {nome}?" com "Cancelar" e "Apagar"; confirmado, chama `api.investimentos.apagar` e redesenha. `NAO_ENCONTRADO` mostra a mensagem do erro numa faixa acima da tabela e redesenha.

- [ ] **Step 1: Testes que falham** — unitários de `resolver` (os três casos acima e os antigos adaptados) e de `formato.js` (cada exemplo acima é uma asserção); e2e: lista vazia mostra "Nenhum investimento ainda."; com um CDB criado pela `api` no teste, a linha mostra `CDB Banco Inter`, `102% do CDI`, `R$ 20.000,00`, `15/03/2028`; apagar → diálogo → "Apagar" → lista vazia; "Cancelar" mantém a linha.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.** Toda montagem de DOM com `createElement`/`textContent`, nada de `innerHTML` com dados.
- [ ] **Step 4: Rodar e ver passar.**
- [ ] **Step 5: Commit** — "Lista de investimentos com apagar e formatação em reais".

---

### Tarefa 6: Formulário de novo e editar investimento

**Files:**
- Create: `app/renderer/js/telas/formulario-investimento.js` (usado por `investimentos.js` quando `parametros[0]` é `novo` ou um id)
- Modify: `app/renderer/js/telas/investimentos.js`, `app/renderer/css/telas.css`
- Test: `app/test/e2e/formulario.spec.js`

**Interfaces:**
- Consumes: `api.investimentos.catalogo/ler/criar/atualizar` (Tarefa 4), `paraCentavos`, `formatarReais`, `formatarData` (Tarefa 5).
- Produces: a tela `#/investimentos/novo` e `#/investimentos/<id>`.

Comportamento (segue a tela `#manual` do protótipo, `docs/prototipo/index.html:557-640` e o script em `:1331-1370`):
- Título "Novo investimento" ou "Editar investimento". Seções "Identificação", "Rentabilidade e prazo" (renda fixa), "Posição" (renda variável), "Valores e condições".
- `select` de tipo agrupado por `grupo` (`optgroup`), montado do catálogo. Trocar o tipo mostra só as seções da `forma`, aplica indexador e liquidez forçados (com o `select` desabilitado), põe o IR padrão (desabilitado se não editável, com a marca "preenchido pelo tipo") e mostra a garantia do catálogo como texto, com a mesma marca.
- Rendimento: número (`input type="number"`, `step="0.0001"`) + benchmark (só em `pos`). Rótulo e sufixo mudam com o indexador como no protótipo ("Rendimento" `%`; "Taxa prefixada" `% a.a.`; "Taxa acima da inflação" `% a.a.`; "Taxa acima da Selic" `% a.a.`) e uma dica "Lê-se: …" igual ao `rendimento` do núcleo.
- Emissor vira "Nome" na forma `valor` e some no Tesouro. Instituição: `select` com as sugestões + "Outra…", que mostra um campo de texto.
- Liquidez "Carência até uma data" mostra o campo de data de carência.
- Valor aplicado e preço médio: texto com `inputmode="decimal"` e prefixo `R$`; ao sair do campo, reformata com `formatarReais`. `paraCentavos` devolvendo `null` mostra "Informe um valor em reais, por exemplo 1.234,56." no campo, sem enviar.
- Rodapé com o resumo do protótipo (`CDB Banco Inter · 102% do CDI · vence em 15/03/2028`) e botões "Salvar e adicionar outro" e "Salvar".
- Ao salvar: monta `NovoInvestimento` só com os campos da forma atual (`taxa` com ponto, centavos inteiros); `erros` voltando preenchidos marcam cada campo (`aria-invalid="true"` e a mensagem abaixo, ligada por `aria-describedby`) e focam o primeiro. Gravou: "Salvar" volta para `#/investimentos`; "Salvar e adicionar outro" limpa o formulário mantendo tipo e instituição, e mostra "Investimento salvo."
- Editar: carrega com `api.investimentos.ler(id)`; `NAO_ENCONTRADO` mostra a mensagem e um link "Voltar para a lista".

- [ ] **Step 1: Testes e2e que falham**
  - `lança um CDB e ele aparece na lista` — preenche, salva, vê a linha com `102% do CDI` e `R$ 20.000,00`.
  - `LCI preenche IR isento e FGC` — o `select` de IR fica `Isento` e desabilitado, e a garantia mostra "FGC".
  - `Tesouro IPCA+ força o indexador` — indexador "Híbrido (IPCA + taxa)" desabilitado; benchmark escondido; liquidez D+1 desabilitada.
  - `ação troca para ticker, quantidade e preço médio` — `itub4`, 150, `31,42` → lista mostra `ITUB4` e `R$ 4.713,00`.
  - `trocar CDB → Ação → LCI não grava campo fantasma` — preenche ticker na ação, troca para LCI, salva; ao editar, nenhum ticker aparece e o IR é Isento.
  - `erros aparecem no campo` — salvar vazio marca emissor e valor com as mensagens exatas da tabela de validação; `1,234,56` no valor mostra a mensagem de formato e não chama `criar`.
  - `taxa com casas decimais volta igual` — Tesouro Selic com `0,0905` digitado → lista mostra `Selic + 0,0905% ao ano`.
  - `edita e mantém o id` — muda a taxa de 102 para 105 e a lista mostra `105% do CDI`.
  - `editar id que não existe` — `#/investimentos/999` mostra "Este investimento não existe mais. Atualize a lista." e o link.
  - `os dados continuam depois de reabrir o app` — salva, fecha o Electron, abre de novo com o mesmo `GURU_DATA_DIR`, a linha está lá.
- [ ] **Step 2: Rodar e ver falhar** — `npx playwright test formulario`.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** — suíte inteira uma vez.
- [ ] **Step 5: Commit** — "Formulário de investimento que se adapta ao tipo".

---

### Tarefa 7: Carteira por classe, detalhe da classe e documentação

**Files:**
- Modify: `app/renderer/js/telas/carteira.js`, `app/renderer/css/telas.css`
- Modify: `docs/mvp.md` (decisão de classes)
- Test: `app/test/e2e/carteira.spec.js`

**Interfaces:**
- Consumes: `api.carteira.resumo`, `api.investimentos.listar`, `formatarReais`, `formatarData`.
- Produces: `#/carteira` e `#/carteira/<classe>`.

Comportamento:
- `#/carteira`: título "Carteira", "Total aplicado" com o valor; um card por classe do resumo (rótulo, total, "N investimentos" / "1 investimento", percentual do total com uma casa e vírgula, ex. `85,8%`), cada card é um link para `#/carteira/<classe>`. Sem investimentos: "Sua carteira está vazia." e um link "Lançar o primeiro investimento" para `#/investimentos/novo`. Uma linha discreta: "Valores aplicados. Cotações e rentabilidade chegam numa próxima versão."
- `#/carteira/<classe>`: migalha "Carteira ›" (link para `#/carteira`), título com o rótulo da classe, total da classe, e a tabela do protótipo (`#classe`) sem as colunas Atual e Rentab.: Investimento, Instituição, Rendimento, Aplicado, Vencimento, Liquidez. Classe desconhecida ou vazia: "Nenhum investimento nesta classe." e o link de volta.
- O menu continua marcando "Carteira".
- Docs: em `mvp.md`, nas Decisões, linha "Classes" — "Renda fixa (inclui Tesouro), Ações (inclui ETFs), FIIs, Exterior (BDRs) e Outros (fundos, previdência e cripto). A classe Caixa do protótipo fica para quando houver um tipo para ela."

- [ ] **Step 1: Testes e2e que falham** — carteira vazia mostra o texto e o link; com CDB 20.000,00 + Tesouro 18.000,00 + ITUB4 (150 × 31,42) criados pela `api`, mostra "Total aplicado" `R$ 42.713,00`, card Renda fixa `R$ 38.000,00` "2 investimentos" `89,0%`, card Ações `11,0%`; clicar Renda fixa mostra a tabela com as duas linhas e a migalha volta.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** — suíte inteira (Rust, addon, unitários e e2e) uma vez.
- [ ] **Step 5: Commit** — "Carteira por classe e detalhe da classe".
