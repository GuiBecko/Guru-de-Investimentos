# Fase 4: Importação da B3 — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O usuário escolhe a planilha de posição exportada da Área do Investidor da B3. O app compara a planilha, linha a linha, com o que já está cadastrado e mostra cada linha como novo, atualizar, divergente, igual ou fora da B3. O usuário decide o que fazer com cada uma, completa na própria tela o que a planilha não traz e aplica tudo de uma vez. Nada é apagado sem pedido explícito.

**Architecture:**
- No núcleo:
  - um leitor de XLSX mínimo, com o `zip` que já existe mais `quick-xml`;
  - um leitor da posição da B3, que lê as abas e as colunas pelo nome;
  - um comparador que casa as linhas da planilha com a carteira;
  - um aplicador transacional, que grava tudo ou nada.
- O main do Electron abre o diálogo de arquivo, lê os bytes e chama o addon. O caminho do arquivo nunca vai para a interface.
- O addon guarda a última comparação com um token. A aplicação se refere a esse token e às chaves das linhas.

**Tech Stack:** o mesmo das fases anteriores, mais `quick-xml = "0.42.0"` no `guru-core`. Ele já está no `Cargo.toml` e foi baixado no commit do plano.

**Spec:**
- [`docs/mvp.md`](../../mvp.md): tela 3 e a decisão "Importação".
- Protótipo: `docs/prototipo/index.html:510-560` (tela 3).

## Global Constraints

- Tudo das fases anteriores continua valendo:
  - segurança da janela e CSP;
  - nada de `innerHTML` com dados nem `style=`;
  - `vigente()` depois de cada await;
  - pt-BR;
  - códigos de erro em SCREAMING_SNAKE_CASE;
  - preload sem lógica;
  - IPC com checagem de origem;
  - dinheiro em centavos `i64` e taxas em `Decimal`;
  - e2e com a janela escondida.
- **Formato real do arquivo**, conferido num arquivo exportado em 30/10/2026:
  - É um `.xlsx` com uma aba por tipo de ativo. A primeira linha é o cabeçalho e as linhas seguintes são os ativos.
  - Depois dos ativos vem uma linha vazia, uma linha com "Total" na coluna de valor e uma linha com o número do total.
  - As datas são texto `dd/mm/aaaa`; números são células numéricas; campo vazio é o texto `-`.
  - Cabeçalhos exatos das abas vistas no arquivo real:
    - **BDR:** `Produto, Instituição, Conta, Código de Negociação, Código ISIN / Distribuição, Tipo, Escriturador, Quantidade, Quantidade Disponível, Quantidade Indisponível, Motivo, Preço de Fechamento, Valor Atualizado`
    - **Renda Fixa:** `Produto, Instituição, Emissor, Código, Indexador, Tipo de regime, Data de Emissão, Vencimento, Quantidade, Quantidade Disponível, Quantidade Indisponível, Motivo, Contraparte, Preço Atualizado MTM, Valor Atualizado MTM, Preço Atualizado CURVA, Valor Atualizado CURVA, Preço Atualizado FECHAMENTO, Valor Atualizado FECHAMENTO`
  - Formatos vistos:
    - `Produto` da renda fixa: `"CDB - BANCO X S/A"`, com o tipo antes do hífen;
    - `Produto` da renda variável: `"AAPL34 - NOME DA EMPRESA"`;
    - `Indexador`: `DI` e `PREFIXADO`.
- **Abas pelo layout público**, sem arquivo real conferido. As colunas são lidas pelo nome e as amostras são inventadas:
  - "Acoes" e "Ações" são a mesma aba;
  - "ETF";
  - "Fundo de Investimento", com os FIIs;
  - "Tesouro Direto", com as colunas `Produto, Instituição, Código ISIN, Indexador, Vencimento, Quantidade, …, Valor Aplicado, Valor bruto, Valor líquido, Valor Atualizado`.
  - Aba com outro nome, ou sem uma coluna obrigatória, vira aviso: "Aba <nome> não reconhecida." ou "Aba <nome>: falta a coluna <coluna>.". As outras abas seguem.
- **Amostras de teste:** `crates/guru-core/tests/fixtures/b3/`. Todas são fictícias, com a mesma estrutura do arquivo real:
  - `posicao-bdr-rf.xlsx`: as duas abas do arquivo real;
  - `posicao-completa.xlsx`: Acoes, BDR, ETF, Fundo de Investimento, Renda Fixa e Tesouro Direto;
  - `posicao-aba-desconhecida.xlsx`: uma aba desconhecida e uma Renda Fixa sem `Vencimento`.
  - Os dados pessoais do usuário nunca entram no repositório.
- **Limites do arquivo:** até 20 MB lido do disco; cada parte do ZIP com até 50 MB descompactados; até 50 abas e 100 mil linhas. Acima disso → `ARQUIVO_INVALIDO`, com "O arquivo é grande demais para uma planilha de posição.". Arquivo que não é XLSX → `ARQUIVO_INVALIDO`, com "Este arquivo não é uma planilha do Excel (.xlsx).".
- **Nada é apagado sozinho.** "Fora da B3" vem com "Manter" escolhido. "Remover" só acontece se o usuário escolher na linha.
- **A planilha de posição não traz valor aplicado, taxa contratada nem preço médio.** Um investimento novo só é gravado depois que o usuário completa, na própria tela, os campos que a validação da fase 1 exige. O preço médio já lançado nunca é sobrescrito.
- Trabalho na branch `fase-4-importacao-b3`. Commits em português terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Regras de comparação (exatas)

| Linha da planilha | Casa com | Igual | Diferente | Sem par |
|---|---|---|---|---|
| Renda variável (Acoes→`rv-acao`, BDR→`rv-bdr`, ETF→`rv-etf`, Fundo de Investimento→`rv-fii`) | investimentos `rv-*` com o mesmo ticker, sem diferenciar maiúsculas | soma das quantidades = `Quantidade` | um par importado antes (`codigo_b3` preenchido) → **atualizar** a quantidade; um par lançado à mão → **divergente**; dois ou mais pares → **divergente**, só com "Manter" | **novo** |
| Renda fixa | primeiro `codigo_b3 = Código`; senão `rf-*` com o mesmo vencimento, o mesmo emissor normalizado e, se a planilha permitir saber, o mesmo tipo | um par → **igual**, e ao aplicar grava `codigo_b3` | — (o valor da curva da B3 não é comparado com o valor calculado pelo app: só aparece como informação) | dois ou mais candidatos → **divergente**, com "Vincular a…" cada um ou "Adicionar como novo"; nenhum → **novo** |
| Tesouro Direto | `td-*` com o mesmo tipo e vencimento | soma dos aplicados igual a `Valor Aplicado`, com tolerância de 1 centavo | importado → **atualizar** o valor aplicado; à mão → **divergente** | **novo** |
| — | investimentos do app sem par na planilha | | | **fora da B3** |

- **Emissor normalizado:** sem acentos e em maiúsculas; tira pontuação e os sufixos `S/A`, `S.A.`, `SA` e `LTDA`; junta espaços.
- **Tipo da renda fixa:** vem do prefixo do `Produto`, antes de ` - `:
  - `CDB`→`rf-cdb`, `LCI`→`rf-lci`, `LCA`→`rf-lca`, `LC`→`rf-lc`, `DEB` ou `DEBENTURE`→`rf-deb`, `CRI`→`rf-cri`, `CRA`→`rf-cra`;
  - outro prefixo → tipo desconhecido, que o usuário escolhe.
- **Indexador:**
  - `DI` ou `CDI` → pós-fixado + benchmark CDI;
  - `SELIC` → pós-fixado + benchmark Selic;
  - `PREFIXADO` ou `PRE` → prefixado;
  - `IPCA` → IPCA.
- **Título do Tesouro:** vem do `Produto` sem o ano final (`"Tesouro Selic 2029"` → `td-selic`). Juros Semestrais, Renda+, Educa+ e IGPM+ não estão no catálogo: viram aviso "Título <produto> ainda não é suportado." e ficam fora da comparação.
- **Quantidade de renda variável** com fração vira aviso e fica fora da comparação.

## Review Focus

1. **Importar a mesma planilha duas vezes:**
   - Esperado: a segunda vez mostra tudo como igual e não duplica nada.
   - Testes na Tarefa 4 (núcleo) e na Tarefa 6 (e2e).
2. **Planilha de outro formato:** PDF renomeado, CSV, XLSX de outra coisa ou uma aba que mudou de colunas.
   - Esperado: uma mensagem clara, sem travar e sem gravar nada.
   - Testes nas Tarefas 1 e 2.
3. **Aplicar com um novo incompleto:**
   - Esperado: nada é gravado e a linha mostra o erro de cada campo, no mesmo padrão do formulário da fase 1.
   - Testes nas Tarefas 4 e 6.
4. **Mesmo ticker em duas corretoras, ou duas compras lançadas à mão:** as quantidades são somadas e o "Atualizar" não escolhe sozinho em qual compra mexer. Teste na Tarefa 3.
5. **Escolher o arquivo e cancelar o diálogo:** a tela volta ao estado inicial, sem erro. Teste na Tarefa 6, pelo caminho de teste do diálogo.

---

### Tarefa 1: Leitor de XLSX

**Files:**
- Create: `crates/guru-core/src/importacao/mod.rs`, `crates/guru-core/src/importacao/xlsx.rs`
- Modify: `crates/guru-core/src/lib.rs`, `crates/guru-core/src/error.rs` (`CoreError::ArquivoInvalido(String)` → `ARQUIVO_INVALIDO`)
- Test: unidade em `xlsx.rs`, lendo `tests/fixtures/b3/*.xlsx` com `include_bytes!`

**Interfaces:**
- Produces:
  - `pub enum Celula { Vazia, Texto(String), Numero(String) }`. `Numero` guarda o texto bruto da célula, para virar `Decimal` sem passar por `f64`.
  - `pub struct Aba { pub nome: String, pub linhas: Vec<Vec<Celula>> }`. As colunas vêm na posição da referência (`A`, `B`, …, `AA`), e as lacunas são preenchidas com `Vazia`.
  - `pub fn ler_xlsx(bytes: &[u8]) -> Result<Vec<Aba>, CoreError>`. Lê, na ordem do `workbook.xml`, as abas apontadas pelo `workbook.xml.rels`. Strings compartilhadas, `inlineStr` e números entram; fórmulas valem pelo valor guardado em `<v>`.

- [ ] **Step 1: Testes que falham:**
  - `posicao-bdr-rf.xlsx` dá duas abas, "BDR" e "Renda Fixa". A primeira linha do BDR é o cabeçalho exato das Global Constraints. A `Quantidade` da linha 2 é `Numero("10")` e o `Motivo` é `Texto("-")`.
  - A linha vazia vem com uma célula `Vazia`, ou com uma `Texto("")` normalizada para `Vazia`.
  - Bytes que não são ZIP, e um ZIP sem `xl/workbook.xml`, dão `ARQUIVO_INVALIDO` com a mensagem exata.
  - Um ZIP com uma parte que passa de 50 MB descompactados também dá `ARQUIVO_INVALIDO`. Monte-o no teste com o `zip` em modo escrita: comprimir zeros é barato.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar** com `zip` + `quick-xml` (leitor de eventos). Conte os bytes lidos de cada parte e pare ao passar do limite.
- [ ] **Step 4: Rodar e ver passar:** `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`.
- [ ] **Step 5: Commit:** "Leitor de planilhas XLSX".

---

### Tarefa 2: Leitor da posição da B3

**Files:**
- Create: `crates/guru-core/src/importacao/posicao.rs`
- Test: unidade em `posicao.rs`, com as amostras

**Interfaces:**
- Consumes: `ler_xlsx`, `Aba` e `Celula` (Tarefa 1).
- Produces:
  - `pub enum AbaRv { Acao, Bdr, Etf, Fii }`, com o tipo do catálogo (`rv-acao`, …).
  - `pub enum LinhaB3`:
    - `RendaVariavel { aba: AbaRv, ticker: String, produto: String, instituicao: String, quantidade: i64, preco_fechamento: Option<Decimal>, valor_atualizado: Option<Decimal> }`
    - `RendaFixa { produto: String, tipo: Option<String>, instituicao: String, emissor: String, codigo: String, indexador: Option<String>, benchmark: Option<String>, emissao: Option<NaiveDate>, vencimento: NaiveDate, quantidade: Decimal, valor_curva: Option<Decimal> }`
    - `Tesouro { produto: String, tipo: String, isin: Option<String>, instituicao: String, vencimento: NaiveDate, valor_aplicado: Option<Decimal>, valor_atualizado: Option<Decimal> }`
  - `pub struct Posicao { pub linhas: Vec<LinhaB3>, pub avisos: Vec<String>, pub linhas_lidas: usize }`
  - `pub fn ler_posicao(bytes: &[u8]) -> Result<Posicao, CoreError>`. A linha termina no primeiro `Produto` vazio, de modo que as linhas de total são ignoradas. O texto `-` vale nulo. Arquivo sem nenhuma aba reconhecida → `ARQUIVO_INVALIDO`, com "Não encontrei nenhuma aba da posição da B3 neste arquivo.".
  - Tabela, regras de indexador, prefixo e título: seção "Regras de comparação".
  - `#[ignore]` `posicao_real`: lê o caminho em `GURU_POSICAO_REAL` e imprime só a contagem de linhas por variante e de avisos.

- [ ] **Step 1: Testes que falham:**
  - `posicao-bdr-rf.xlsx` dá 3 linhas: BDR `AAPL34` com quantidade 10; CDB DI → `rf-cdb`, pós + CDI, vencimento 2029-03-15, código `CDB224ABCDE`; CDB PREFIXADO → `rf-cdb`, pré.
  - `posicao-completa.xlsx` lê as seis abas:
    - Ações: ITUB4 com 150, B3SA3 com 200;
    - ETF: BOVA11 com 30;
    - FII: HGLG11 com 12;
    - Tesouro: Selic 2029 com aplicado 8000,00 e IPCA+ 2035.
  - `posicao-aba-desconhecida.xlsx` dá os avisos "Aba Aluguel de Ações não reconhecida." e "Aba Renda Fixa: falta a coluna Vencimento.", com zero linhas, e por isso `ARQUIVO_INVALIDO`.
  - Os nomes de aba "Ações" e "Acoes" são reconhecidos.
  - Um título "Tesouro IPCA+ com Juros Semestrais 2040" vira aviso. Monte esse caso com uma aba sintética de `Celula`; para isso, exponha `ler_posicao_abas(&[Aba])`.
  - Uma quantidade `10.5` em ações vira aviso.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** (workspace, clippy all-targets, fmt).
- [ ] **Step 5: Commit:** "Leitor da posição da Área do Investidor B3".

---

### Tarefa 3: Comparação com a carteira (migração 0005)

**Files:**
- Create: `crates/guru-core/src/migrations/0005_importacao.sql`, `crates/guru-core/src/importacao/comparacao.rs`
- Modify:
  - `crates/guru-core/src/db.rs`, com `SCHEMA_VERSION` = 5;
  - as asserções literais da versão (`grep -rn "schema" crates/guru-node/test app/test/e2e`, mais os `migra_banco_da_versao_*`);
  - `crates/guru-core/src/investimento/repositorio.rs`: `Investimento` ganha `codigo_b3: Option<String>`.
- Test: `crates/guru-core/tests/importacao.rs`

**Interfaces:**
- Consumes: `Posicao` e `LinhaB3` (Tarefa 2), `Db::listar_investimentos`, `validar` (`investimento/validacao.rs`).
- Produces:
  - Migração:
    - `ALTER TABLE investimento ADD COLUMN codigo_b3 TEXT;`
    - `CREATE TABLE importacao (id INTEGER PRIMARY KEY, data TEXT NOT NULL, arquivo TEXT NOT NULL, linhas INTEGER NOT NULL, aplicadas INTEGER NOT NULL);`
    - Confira o nome real da tabela de investimentos em `0001`.
  - `pub enum Estado { Novo, Atualizar, Divergente, Igual, ForaDaB3 }`, com os códigos `novo`, `atualizar`, `divergente`, `igual`, `fora_da_b3` e os rótulos "novo", "atualizar", "divergente", "igual", "fora da B3".
  - `pub enum Acao { Adicionar, Ignorar, UsarB3, Manter, Vincular(i64), Remover }`, com os códigos `adicionar`, `ignorar`, `usar_b3`, `manter`, `vincular`, `remover`.
  - `pub struct Item { pub chave: String, pub estado: Estado, pub ativo: String /* ticker, "Tesouro Selic 2029" ou "CDB Banco X" */, pub instituicao: String, pub no_app: Option<String> /* texto pronto: "120 ações", "R$ 8.000,00" */, pub na_b3: Option<String>, pub investimentos: Vec<i64>, pub acoes: Vec<Acao>, pub acao_padrao: Acao, pub sugestao: Option<NovoInvestimento>, pub faltando: Vec<ErroCampo> }`
  - `chave` é estável dentro da comparação: `b3:<n>` para linhas da planilha e `app:<id>` para os itens fora da B3.
  - `sugestao` vem só para Novo e para "Adicionar como novo". É o `NovoInvestimento` pré-preenchido com o que a planilha traz:
    - tipo, instituição, emissor ou ticker, indexador, benchmark, vencimento e quantidade;
    - na renda fixa, `data_aplicacao` = data de emissão;
    - no Tesouro, valor aplicado = `Valor Aplicado`.
  - `faltando` = os erros de `validar(sugestao)`, que são exatamente os campos que o usuário precisa completar.
  - `acao_padrao`:
    - Novo → Adicionar;
    - Atualizar → UsarB3;
    - Divergente → Manter;
    - Igual → Manter;
    - Fora da B3 → Manter.
  - `pub struct Comparacao { pub itens: Vec<Item>, pub avisos: Vec<String>, pub linhas_lidas: usize }`, com os itens na ordem novo, atualizar, divergente, fora da B3, igual.
  - `pub fn comparar(db: &Db, posicao: &Posicao, hoje: NaiveDate) -> Result<Comparacao, CoreError>`, seguindo a seção "Regras de comparação".

- [ ] **Step 1: Testes que falham** (carteira montada no teste e `posicao-completa.xlsx`):
  - Carteira vazia: todas as linhas viram Novo.
    - O ITUB4 novo tem `faltando` com `precoMedioCentavos` e `dataAplicacao`, e a sugestão vem com a quantidade 150.
    - O CDB novo tem `faltando` com `valorAplicadoCentavos` e `taxa`.
  - ITUB4 lançado à mão com 120 → Divergente, com `no_app` "120 ações" e `na_b3` "150 ações". Com `codigo_b3` preenchido → Atualizar.
  - Dois ITUB4 à mão somando 150 → Igual. Somando 140 → Divergente, só com Manter (Review Focus 4).
  - Um CDB à mão do "Banco Fictício SA" com vencimento 2029-03-15 → Igual com o CDB da planilha, pela normalização do emissor.
  - Dois candidatos → Divergente, com `Vincular(id)` para cada um mais Adicionar.
  - Tesouro Selic 2029 à mão com aplicado R$ 8.000,00 → Igual; com R$ 7.000,00 → Divergente.
  - Um fundo e uma cripto do app viram Fora da B3, com padrão Manter.
  - `migra_banco_da_versao_4` mantém os dados.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** (workspace, clippy, fmt; addon build + test; e2e `-g "abre com título"`).
- [ ] **Step 5: Commit:** "Comparação da planilha da B3 com a carteira".

---

### Tarefa 4: Aplicar a importação

**Files:**
- Create: `crates/guru-core/src/importacao/aplicacao.rs`
- Modify: `crates/guru-core/src/investimento/repositorio.rs` (gravação dentro de uma transação, se precisar)
- Test: `crates/guru-core/tests/importacao.rs`

**Interfaces:**
- Consumes: `Comparacao`, `Item` e `Acao` (Tarefa 3), `criar_investimento`, `atualizar_investimento`, `apagar_investimento`.
- Produces:
  - `pub struct Decisao { pub chave: String, pub acao: Acao, pub dados: Option<NovoInvestimento> /* para Adicionar: a sugestão completada pelo usuário */ }`
  - `pub struct ResultadoImportacao { pub aplicadas: usize, pub erros: Vec<(String /* chave */, Vec<ErroCampo>)> }`
  - `pub fn aplicar_importacao(db: &Db, comparacao: &Comparacao, decisoes: &[Decisao], arquivo: &str, hoje: NaiveDate) -> Result<ResultadoImportacao, CoreError>`
  - Uma transação só. Se qualquer `Adicionar` falhar na validação, nada é gravado e `erros` traz os campos de cada chave (Review Focus 3).
  - Ação que não está em `item.acoes`, ou chave desconhecida → `CoreError` com o código `DECISAO_INVALIDA`.
  - Efeitos:
    - Adicionar: cria com `codigo_b3`. Renda variável usa o ticker, renda fixa o `Código`, Tesouro o ISIN ou `<tipo>:<vencimento>`.
    - UsarB3 na renda variável: muda a quantidade, mantém o preço médio e recalcula o aplicado como quantidade × preço médio, com `checked_mul`.
    - UsarB3 no Tesouro: muda o valor aplicado.
    - Vincular(id) e Igual na renda fixa: gravam `codigo_b3`.
    - Remover: apaga.
    - Manter e Ignorar: nada.
  - Grava uma linha em `importacao`.
  - `pub fn ultima_importacao(db: &Db) -> Result<Option<String /* YYYY-MM-DD */>, CoreError>`

- [ ] **Step 1: Testes que falham:**
  - Adicionar os seis ativos da `posicao-completa.xlsx`, com os campos completados, e comparar de novo: tudo Igual e nenhuma duplicata (Review Focus 1).
  - Um Adicionar sem preço médio: `erros` traz `precoMedioCentavos` e o banco continua sem nenhum investimento novo, nem mesmo os outros válidos.
  - UsarB3 no ITUB4 importado com preço médio R$ 31,42: a quantidade vai de 120 para 150 e o aplicado fica 150 × 3142.
  - Fora da B3 com Manter continua; com Remover some.
  - `Vincular` grava `codigo_b3`, e a próxima comparação casa direto.
  - `ultima_importacao` devolve a data.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar.**
- [ ] **Step 5: Commit:** "Aplicação da importação da B3".

---

### Tarefa 5: Addon, diálogo de arquivo e ponte

**Files:**
- Create: `crates/guru-node/src/importacao.rs`, `crates/guru-node/test/importacao.test.mjs`
- Modify: `crates/guru-node/src/lib.rs`, `app/main.js`, `app/preload.js`, `app/renderer/js/api.js`, `app/test/unit/api.test.mjs`, `app/test/e2e/abrir.spec.js` (superfície)

**Interfaces:**
- Produces (addon):
  - `core.compararImportacao(bytes: Buffer, arquivo: string): ComparacaoJs`.
    - `ComparacaoJs = { token, arquivo, linhasLidas, avisos, itens: [{ chave, estado, estadoRotulo, ativo, instituicao, noApp, naB3, investimentos, acoes, acaoPadrao, sugestao, faltando: [{campo, mensagem}] }] }`.
    - `sugestao` usa os mesmos nomes de campo da entrada do formulário de investimento (`api.investimentos.criar`).
    - Guarda a comparação no `Core` com um token novo e descarta a anterior.
  - `core.aplicarImportacao(token, decisoes: [{chave, acao, idVinculo?, dados?}]): { aplicadas, erros: [{chave, campos: [{campo, mensagem}]}] }`. Token que não é o da última comparação → `IMPORTACAO_EXPIRADA`.
  - `core.ultimaImportacao(): string | null`
  - As datas são as reais (validação e gravação), não a data de mercado.
- Produces (main/IPC):
  - `importacao:escolher`:
    - o main abre `dialog.showOpenDialog` com o filtro `{ name: "Planilha do Excel", extensions: ["xlsx"] }`;
    - lê o arquivo com o limite de 20 MB;
    - chama `compararImportacao(bytes, basename)` e devolve a comparação, ou `{ cancelado: true }`.
    - Com `GURU_TESTE_ARQUIVO_B3=<caminho>`, só quando `!app.isPackaged`, o main pula o diálogo e usa esse caminho. Com o valor `cancelar`, devolve `{ cancelado: true }`.
  - `importacao:aplicar(token, decisoes)` e `importacao:ultima()`.
  - `api.importacao = { escolher, aplicar, ultima }`, e a superfície ganha `"importacao"`.

- [ ] **Step 1: Testes que falham:**
  - Addon:
    - comparar `posicao-completa.xlsx` numa carteira vazia dá só Novos, com `faltando`;
    - aplicar com dados completos dá `aplicadas` igual ao número de itens;
    - um token velho dá `IMPORTACAO_EXPIRADA`;
    - bytes inválidos dão `ARQUIVO_INVALIDO`.
  - Unitário do `api.js`.
  - e2e: a superfície da ponte; e `api.importacao.escolher()` com `GURU_TESTE_ARQUIVO_B3=cancelar` dá `{ cancelado: true }`.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** (Rust, addon build + test, unitários, `npx playwright test abrir`).
- [ ] **Step 5: Commit:** "Importação da B3 no addon e na ponte".

---

### Tarefa 6: Tela "Importar da B3"

**Files:**
- Create: `app/renderer/js/telas/importacao.js`, `app/test/e2e/importacao.spec.js`
- Modify:
  - `app/renderer/js/router.js` (`#/investimentos/importar`, antes da rota `#/investimentos/<id>`);
  - `app/renderer/js/telas/investimentos.js` (botão "Importar da B3" ao lado de "Novo investimento");
  - `app/renderer/index.html` e `app/renderer/js/mercado.js`, ou um módulo pequeno, para o rodapé do menu "Última importação B3: dd/mm/aaaa", só quando houver;
  - `app/renderer/css/telas.css`.

Comportamento (protótipo, tela 3):
- **Abertura da tela:** título "Importar da B3", um texto curto "Exporte a posição em investidor.b3.com.br → Extratos → Posição → Baixar Excel." e o botão "Escolher planilha".
- **Depois da escolha:**
  - um card com o nome do arquivo, "Área do Investidor B3 · N linhas lidas" e as marcas por estado ("4 novos", "1 atualizar", …);
  - os avisos, se houver;
  - a tabela com as colunas "Situação", "Ativo", "Corretora", "No app hoje", "Na planilha B3" e "O que fazer". "O que fazer" é um `<select>` com as ações do item, começando no padrão; Vincular mostra "Vincular a <nome do investimento>".
  - Os iguais ficam recolhidos numa linha "N iguais" que abre a lista.
- **Novo com "Adicionar":** a linha abre embaixo, no mesmo padrão `.field` do formulário, só os campos de `faltando` (rótulos do formulário da fase 1) e um resumo do que veio da planilha. Erros aparecem no campo, como no formulário.
- **Valores e datas:** campos de texto em reais e datas no padrão da fase 1, com `paraCentavos`/`paraTaxa`. Nada de `type="number"`.
- **Rodapé:** "Fundos e previdência normalmente não aparecem na B3. Por padrão, ficam como estão.", mais "Cancelar" (volta para `#/investimentos`) e "Aplicar N alterações". N conta as decisões que mudam algo; Manter e Ignorar não contam, e o Igual que só grava o vínculo também não.
- **Aplicar:**
  - Com erros, as linhas mostram os campos e nada é gravado.
  - Com sucesso, vai para `#/investimentos` com o aviso "N alterações aplicadas." e dispara a atualização de mercado em segundo plano (a função da fase 3a).
- **Escolher de novo** troca a comparação. **Cancelar o diálogo** deixa a tela como estava (Review Focus 5).

- [ ] **Step 1: Testes e2e que falham**, com `GURU_TESTE_ARQUIVO_B3` apontando para as amostras:
  - Carteira vazia com `posicao-completa.xlsx`:
    - mostra "9 novos" e a tabela;
    - completar o preço médio e a data do ITUB4 e marcar os outros como Ignorar dá "Aplicar 1 alteração";
    - aplicar leva à lista com o ITUB4 e o aviso.
  - Importar a mesma planilha de novo mostra o ITUB4 como igual (Review Focus 1).
  - Um Adicionar incompleto mostra o erro no campo e não grava nada (Review Focus 3).
  - O fundo cadastrado aparece como "fora da B3" com Manter.
  - `posicao-aba-desconhecida.xlsx` mostra a mensagem do `ARQUIVO_INVALIDO`.
  - `cancelar` mantém a tela inicial.
  - O rodapé do menu mostra "Última importação B3:" depois de aplicar.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar:** a suíte inteira (Rust, addon, unitários e e2e) uma vez.
- [ ] **Step 5: Commit:** "Tela de importação da B3".

---

### Tarefa 7: Documentação

**Files:**
- Modify:
  - `docs/mvp.md`, decisão "Importação":
    > "Planilha de posição da Área do Investidor B3 (.xlsx), lida pelo nome das abas e das colunas. Casa por ticker (renda variável), código ou emissor + vencimento (renda fixa) e título + vencimento (Tesouro). Valor aplicado, taxa e preço médio não vêm na posição: o usuário completa na tela antes de gravar. Nada é apagado sem escolha explícita."
  - `docs/arquitetura.md`: o diálogo de arquivo no main, a planilha que nunca sai do computador, e `GURU_TESTE_ARQUIVO_B3` no modo de teste;
  - `docs/roadmap.md`: o link deste plano;
  - `README.md`: o status.

- [ ] **Step 1: Editar os arquivos.**
- [ ] **Step 2: Commit:** "Documentação da importação da B3".
