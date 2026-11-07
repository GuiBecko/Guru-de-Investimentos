# Fase 8: Importação do extrato da BTG — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O usuário escolhe o PDF "Extrato da Conta Investimento" da BTG. O app lê fundos, renda fixa, ações/BDRs e cripto, compara com a carteira na mesma tela de revisão da B3 e aplica numa transação só. Ao mesmo tempo, as duas importações passam a ter um investimento por ativo **e por corretora**, deixam de sugerir apagar o que não veio no arquivo, e fundos e cripto ganham o valor atual vindo do extrato.

**Architecture:**
- No núcleo (`crates/guru-core/src/importacao/`):
  - `pdf.rs`: extrai os caracteres posicionados com o crate `pdf-extract` e monta linhas físicas com células (`LinhaPdf`). Não sabe nada da BTG.
  - `btg.rs`: lê as `LinhaPdf` e devolve um `ExtratoBtg` tipado (data, ativos, avisos).
  - `comparacao.rs` fica neutro quanto à fonte (`Fonte::B3 | Fonte::Btg`); a B3 continua nele, e `comparacao_btg.rs` compara o `ExtratoBtg`, reaproveitando `Item`, `Estado`, `Acao` e os auxiliares.
  - `aplicacao.rs` ganha as operações de saldo e de completar campos vazios.
- Banco: migração `0008_btg.sql` com `investimentos.saldo_centavos`, `investimentos.saldo_data` e `importacao.fonte`.
- Valorização: `fundo` e `cripto` (e `prev`) usam o saldo do extrato quando `saldo_data <= data`, com a nova `Origem::Extrato`.
- O main do Electron abre o diálogo da fonte pedida; o addon ganha `compararImportacaoBtg`; a tela de importação recebe a fonte pela rota.

**Tech Stack:** o mesmo das fases anteriores, mais `pdf-extract = "0.10"` no `guru-core` (puxa `lopdf 0.38`, sem features padrão). `lopdf = "0.38"` entra como `dev-dependency` para o gerador de fixtures.

**Spec:** [`docs/superpowers/specs/2026-11-06-importacao-btg-design.md`](../specs/2026-11-06-importacao-btg-design.md). Plano da importação da B3, para o vocabulário existente: [`2026-10-30-fase-4-importacao-b3.md`](2026-10-30-fase-4-importacao-b3.md).

## Global Constraints

- Tudo das fases anteriores continua valendo: segurança da janela e CSP; nada de `innerHTML` com dados nem `style=`; `vigente()` depois de cada await; pt-BR; códigos de erro em SCREAMING_SNAKE_CASE; preload sem lógica; IPC com checagem de origem; dinheiro em centavos `i64` e taxas em `Decimal`; e2e com a janela escondida; o caminho do arquivo nunca vai para a interface.
- **Formato real do PDF**, conferido no extrato real de 06/11/2026 (13 páginas, A4 deitado 841,92 × 594,96 pt, produtor PyPDF2):
  - O texto é extraível. Os **dados** (nomes, códigos, números, datas) vêm em fontes CID com mapeamento correto.
  - Os **rótulos** (títulos de seção, cabeçalhos de coluna, "Período de … a …") vêm em fontes Type 3 cujo ToUnicode mapeia uma variante do `a` minúsculo e as ligaduras `fi`/`fl` para U+0000. O `pdf-extract` entrega esses glifos como `"\0"`. Exemplos reais: `"Rend\0 \0x\0 - Posição - CDB"`, `"Dat\0 Compr\0"`, `"Período de 01/10/26 \0 31/10/26"`. O `lopdf::extract_text` e o pdf.js têm o mesmo defeito; o `pdftotext` (poppler) não, mas não pode ser usado (dependência externa).
  - A fonte mono dos números informa largura errada ao `pdf-extract`: **o agrupamento em células não pode depender da largura do caractere**.
  - Datas com ano de dois dígitos (`21/10/31` = 2031-10-21). Números `1.234,56`. Campo vazio `-`. Tickers e nomes de fundo podem terminar em `*` (nota de rodapé).
  - Células quebram em mais de uma linha física, acima ou abaixo da linha dos números: `BANCO FICTICIO` / `S.A`, `CDB-` / `CDBA26FICT1`, `12,75%` / `a.a.`, `103,00% do` / `CDI`. Cabeçalhos quebram em até três linhas e trazem números de nota soltos (`1`, `2`, `3`) acima ou ao lado.
  - Cada página repete "Extrato da Conta / Período … / Investimento" no topo e "Disclaimers: …" e o número da página no rodapé.
- **Seções lidas** (títulos comparados pelo esqueleto, ver Task 1):
  - `Fundo de Investimento - Posição - Portfólio de fundos`: por fundo, uma linha-título `<NOME> - Classe CNPJ: <cnpj>*` e, logo abaixo, a linha `Data Referência | Saldo Líquido R$ <data> | Quantidade de Cotas | Cotação Atual R$ | Saldo Bruto R$ | Provisão de IR R$ | Provisão de IOF R$ | Saldo Líquido R$ | Variação Nominal R$`. Usa-se: data de referência e saldo bruto.
  - `Fundo de Investimento - Detalhamento - <NOME> - Classe CNPJ: <cnpj>*`: colunas `Data Compra | Quantidade de Cotas | Cotação Compra R$ | Valor de Compra Ajustado R$ | Saldo Bruto R$ | Provisão de IR R$ | Provisão de IOF R$ | Saldo Líquido R$`, uma linha por aplicação e uma linha `Total`. Usa-se: data de compra e valor de compra ajustado.
  - `Renda fixa - Detalhamento - <TIPO> | <EMISSOR>`: colunas `Ativo | Emissão | Vencimento | Aquisição | Liquidez | Dias de carência para liquidez | Data inicial de liquidez | Taxa Compra | Quantidade | Preço Compra R$ | Valor Compra R$ | Preço R$ | Saldo Bruto R$ | IR R$ | IOF R$ | Saldo Líquido R$`, uma linha por aquisição e uma linha de totais sem data.
  - `Renda variável - Posição - <Ações|BDR|FII|ETF>`: colunas `Código | Ação/Ativo | Qtde. | Preço Fechamento R$ | Preço Médio R$ | Saldo Bruto R$`, e uma linha `Total em …`.
  - `CriptoAtivos - Posição` (formato **não visto**): lida pelos nomes de coluna `Ativo` e `Saldo Bruto R$` ou `Valor Bruto R$` (obrigatórios), `Valor Aplicado R$` (opcional). Sem eles → aviso.
  - Ignoradas sem aviso: Sumário, Distribuição, Rentabilidade, `Renda fixa - Posição - …`, `Renda fixa - Posição Consolidada Por Emissor`, Conta corrente, Valores em trânsito, `CriptoAtivos - Movimentação`, Perfil de risco, Disclaimers, contatos.
  - Outra seção com prefixo `Fundo de Investimento - `, `Renda fixa - Detalhamento - `, `Renda variável - Posição - ` ou `CriptoAtivos - Posição` que o leitor não conhece (ex.: `Renda fixa - Detalhamento - Tesouro Direto | …`, `Renda variável - Posição - Opções`) → aviso `Seção "<título>" não reconhecida.` e segue.
- **Mapa de tipos:** renda fixa `CDB`→`rf-cdb`, `LCI`→`rf-lci`, `LCA`→`rf-lca`, `LC`→`rf-lc`, `Debênture`/`DEB`→`rf-deb`, `CRI`→`rf-cri`, `CRA`→`rf-cra`; renda variável `Ações`→`rv-acao`, `BDR`→`rv-bdr`, `FII`/`Fundos Imobiliários`→`rv-fii`, `ETF`→`rv-etf`; fundos → `fundo`; cripto → `cripto`.
- **Taxa:** `"12,75% a.a."` → indexador `pre`, taxa `12.75`; `"103,00% do CDI"` → `pos`, `103.00`, benchmark `cdi`; `"IPCA + 6,50%"` (com ou sem `a.a.`) → `ipca`, `6.50`; outro texto → indexador e taxa vazios (o item fica com `faltando`). **Liquidez:** `Não` → `vencimento`; `Sim` → `diaria`.
- **Identidade gravada em `investimentos.codigo_b3`** (o nome da coluna fica; ela passa a ser "código do ativo na fonte"): renda variável = ticker sem `*`; renda fixa = código sem o prefixo `<TIPO>-` (`CDB-CDBA26FICT1` → `CDBA26FICT1`); fundo = CNPJ só com dígitos (`11111111000111`); cripto = nome normalizado (`normalizar_emissor`).
- **Instituição** de tudo o que vem do PDF: `"BTG Pactual"` (o nome do catálogo). Comparação de instituições sempre por `chave_instituicao` (Task 5).
- **Erros:** arquivo que não é PDF, ou PDF que o `pdf-extract` não consegue ler (inclusive pânico dentro do crate) → `ARQUIVO_INVALIDO`, "Este arquivo não é um PDF válido."; PDF legível que não é o extrato → `ARQUIVO_INVALIDO`, "Este PDF não é um Extrato da Conta Investimento da BTG."; acima de 20 MB → `ARQUIVO_INVALIDO`, "O arquivo é grande demais para um extrato da BTG."; mais de 200 páginas → mesma mensagem.
- **Amostras de teste** em `crates/guru-core/tests/fixtures/btg/`: todas fictícias, geradas pelo exemplo da Task 2. **O PDF real do usuário nunca entra no repositório**, nem os valores dele (nome, CPF, conta, endereço, fundos, valores). A conferência com o PDF real é manual (Task 11).
- **Nada é apagado por importação.** Os estados ficam `novo`, `atualizar`, `divergente`, `igual`; `fora_da_b3` e a ação `remover` saem.
- Trabalho na branch `fase-8-importacao-btg` (já criada, com o design commitado). Commits em português terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Ao fim de cada task: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` e os testes tocados passam.

## Review Focus

- **Mesmo ativo, duas grafias de corretora** (B3 grava "BTG PACTUAL CTVM S/A", o PDF vira "BTG Pactual"): tem de casar, nunca virar dois investimentos. Teste na Task 5 (`chave_instituicao`) e na Task 7 (`btg_casa_ativo_vindo_da_b3`).
- **Tabela que continua na página seguinte** (muitos CDBs ou muitas ações): o cabeçalho de página e o rodapé no meio não podem cortar a seção nem virar linha de dado. Teste na Task 3 (`tabela_continua_na_pagina_seguinte`).
- **Linha de total e linha de rodapé coladas à última linha da tabela**: não podem somar valores na última aquisição. Teste na Task 3 (`total_nao_entra_na_ultima_linha`).
- **Carteira gravada antes desta fase, com a soma de várias corretoras num investimento só**: a B3 separada por corretora não pode apagar nem dividir sozinha; o item aparece como `atualizar`/`divergente`. Teste na Task 5 (`soma_antiga_vira_atualizar_da_corretora`).
- **PDF malformado ou de outro banco**: o `pdf-extract` tem `unwrap`s internos; um pânico não pode derrubar o app. Teste na Task 1 (`pdf_corrompido_e_arquivo_invalido`) e na Task 3 (`pdf_de_outro_banco`).

---

### Task 1: Extração de linhas e células do PDF

**Files:**
- Modify: `crates/guru-core/Cargo.toml` (`pdf-extract = "0.10"`)
- Create: `crates/guru-core/src/importacao/pdf.rs`
- Modify: `crates/guru-core/src/importacao/mod.rs`
- Modify: `app/avisos-terceiros.txt` (regerado)

**Interfaces:**
- Produces:
  - `pub struct Caractere { pub pagina: u32, pub x: f64, pub y: f64, pub tamanho: f64, pub texto: String }` (y cresce para cima, como no PDF; `tamanho` = tamanho efetivo da fonte em pontos).
  - `pub struct Celula { pub x: f64, pub texto: String }` (x do primeiro caractere; texto com `"\0"` preservado e espaços das pontas removidos).
  - `pub struct LinhaPdf { pub pagina: u32, pub y: f64, pub celulas: Vec<Celula> }`; método `pub fn texto(&self) -> String` (células unidas por `" "`).
  - `pub fn agrupar(caracteres: Vec<Caractere>) -> Vec<LinhaPdf>` — pura.
  - `pub fn extrair_linhas(bytes: &[u8]) -> Result<Vec<LinhaPdf>, CoreError>`.
  - `pub fn esqueleto(texto: &str) -> String`.
  - Constante `pub const LIMITE_PAGINAS: usize = 200`.

- [ ] **Step 1: Escrever os testes de `esqueleto` e `agrupar` em `pdf.rs` (`#[cfg(test)]`)**

```rust
#[test]
fn esqueleto_iguala_rotulo_quebrado_e_inteiro() {
    assert_eq!(esqueleto("Rend\0 \0x\0 - Posição - CDB"), esqueleto("Renda fixa - Posição - CDB"));
    assert_eq!(esqueleto("Dat\0 Compr\0"), esqueleto("Data Compra"));
    assert_eq!(esqueleto("Período de 01/10/26 \0 31/10/26"), esqueleto("Período de 01/10/26 a 31/10/26"));
    assert_ne!(esqueleto("Renda fixa - Posição - CDB"), esqueleto("Renda variável - Posição - BDR"));
}

#[test]
fn agrupar_junta_caracteres_proximos_mesmo_com_largura_errada() {
    // dígitos da fonte mono: 5 pt entre inícios, tamanho 7 → mesma célula
    let l = agrupar(chars(1, 434.0, 7.0, &[(472.0, "7"), (477.0, ","), (480.0, "0"), (493.0, "1"), (498.0, "."), (500.0, "0")]));
    assert_eq!(textos(&l[0]), ["7,0", "1.0"]); // 480→493 = 13 pt > 1,6 × 7 → célula nova
}

#[test]
fn agrupar_separa_linhas_por_y_e_ordena_de_cima_para_baixo() {
    let mut c = chars(1, 428.0, 7.0, &[(27.0, "C")]);
    c.extend(chars(1, 437.0, 7.0, &[(27.0, "C"), (31.5, "D"), (36.0, "B"), (40.0, "-")]));
    c.extend(chars(1, 434.0, 7.0, &[(88.0, "2")]));
    let l = agrupar(c);
    assert_eq!(l.iter().map(|x| x.y).collect::<Vec<_>>(), [437.0, 434.0, 428.0]);
}

#[test]
fn agrupar_separa_paginas() { /* mesmo y em páginas 1 e 2 → duas linhas, página 1 antes */ }

#[test]
fn pdf_corrompido_e_arquivo_invalido() {
    let e = extrair_linhas(b"%PDF-1.4\n isto nao e um pdf").unwrap_err();
    assert!(matches!(e, CoreError::ArquivoInvalido(ref m) if m == "Este arquivo não é um PDF válido."));
    assert!(extrair_linhas(b"PK\x03\x04 planilha").is_err());
}
```

`chars(pagina, y, tamanho, &[(x, texto)])` e `textos(&LinhaPdf) -> Vec<&str>` são auxiliares do próprio módulo de teste. Confirme o nome real da variante de erro de arquivo em `crates/guru-core/src/error.rs` (a B3 usa o código `ARQUIVO_INVALIDO`) e use-a.

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p guru-core --lib importacao::pdf`
Expected: FAIL (módulo e funções não existem).

- [ ] **Step 3: Implementar `pdf.rs`**

- `esqueleto`: minúsculas; remove `\0`; remove as sequências `fi` e `fl`; remove a letra `a`; mantém só `char::is_alphanumeric` (acentos ficam). Os dois lados da comparação passam pelo mesmo esqueleto, então funciona com o PDF quebrado e com um PDF corrigido.
- `agrupar`: ordena por página, depois y decrescente, depois x. Uma linha nova começa quando `|y − y_da_linha| > 0,3 × tamanho` (no real, linhas quebradas da mesma célula ficam a 3 pt uma da outra com fonte de ~7 pt, e o mesmo texto oscila até 1 pt). Dentro da linha, célula nova quando `x − x_do_caractere_anterior > 1,6 × tamanho` (distância entre **inícios**, nunca a largura). Linha sem nenhum caractere visível depois de `trim` é descartada.
- `extrair_linhas`: `lopdf::Document::load_mem` (via o `lopdf` reexportado ou o mesmo que o `pdf-extract` usa), recusa mais de `LIMITE_PAGINAS` páginas, roda `pdf_extract::output_doc` com um `OutputDev` que coleta `Caractere` (x = `trm.m31`, y = `trm.m32`, tamanho efetivo = `font_size × trm.m22.abs()`; confira na Task 11 contra o PDF real e ajuste se o tamanho vier em outra escala). Tudo dentro de `std::panic::catch_unwind`; erro ou pânico → "Este arquivo não é um PDF válido.".

- [ ] **Step 4: Rodar e ver passar**

Run: `cargo test -p guru-core --lib importacao::pdf`
Expected: PASS.

- [ ] **Step 5: Regerar os avisos de terceiros**

Run: `cd app && node scripts/avisos-terceiros.mjs && node scripts/avisos-terceiros.mjs --conferir`
Expected: sai com 0. Se alguma dependência nova tiver licença fora de `ACEITAS`, pare e reporte (não amplie a lista sem decisão).

- [ ] **Step 6: Commit** — `git commit -m "Importação BTG: extração de linhas e células do PDF"`

---

### Task 2: Gerador de extratos fictícios

**Files:**
- Modify: `crates/guru-core/Cargo.toml` (`[dev-dependencies] lopdf = "0.38"`)
- Create: `crates/guru-core/examples/gerar_extratos_btg.rs`
- Create: `crates/guru-core/tests/fixtures/btg/extrato-completo.pdf`, `extrato-secao-desconhecida.pdf`, `extrato-varias-paginas.pdf`, `outro-banco.pdf`
- Create: `crates/guru-core/tests/fixtures/btg/LEIAME.md` (uma linha por arquivo: o que contém; como regerar)

**Interfaces:**
- Produces: os quatro PDFs, que as Tasks 3, 9 e 10 usam.

- [ ] **Step 1: Escrever o exemplo**

Um `main` que escreve os quatro PDFs com `lopdf` (fonte Helvetica, `BT … Tf … Td (texto) Tj ET` por célula), página 841,92 × 594,96, y e x das células iguais aos do extrato real descritos em Global Constraints. Cada célula vira um `Tj` próprio; as células quebradas são `Tj`s em y diferentes, como no real. Para reproduzir o defeito dos rótulos, os títulos e cabeçalhos **omitem** o `a` minúsculo e o `fi` (escreva `"Rend  x  - Detalhamento - CDB | BANCO FICTICIO S.A"`) — o leitor tem de funcionar assim.

Conteúdo, todo fictício (titular "FULANO DE TAL", CPF `000.000.000-00`, conta `000000000`):
- `extrato-completo.pdf`, período `01/10/26 a 31/10/26`, 4 páginas de dados + 1 de disclaimers com `atendimentoinvestimentos@btgpactual.com`:
  - Fundos (posição e detalhamento): "FUNDO EXEMPLO RF LP" CNPJ `11.111.111/0001-11`, referência `30/10/26`, saldo bruto `1.050,00`, **dois lotes** (`02/01/26` `400,00` e `05/03/26` `600,00`); "FUNDO TESTE MULTIMERCADO" CNPJ `22.222.222/0001-22*`, referência `30/10/26`, saldo bruto `210,00`, um lote `10/02/26` `200,00`.
  - Renda fixa detalhamento: `CDB | BANCO FICTICIO S.A`, ativo `CDB-CDBA26FICT1` (quebrado em `CDB-`/`CDBA26FICT1`), emissão `15/01/26`, vencimento `15/01/29`, aquisição `15/01/26`, liquidez `Não`, taxa `12,50%`/`a.a.`, valor compra `5.000,00`, saldo bruto `5.400,00`; `LCA | BANCO EXEMPLO S/A`, ativo `LCA-LCA26EXEMP`, emissão e aquisição `01/02/26`, vencimento `01/02/28`, `95,00% do`/`CDI`, valor compra `2.000,00`, saldo bruto `2.120,00`.
  - Renda variável: Ações `ABCD3*` "EMPRESA EXEMPLO ON" qtde `10` preço médio `20,00`; BDR `XYZW34*` "EXEMPLO CORP DRN" qtde `3` preço médio `50,00`.
  - CriptoAtivos - Posição: colunas `Ativo | Qtde. | Valor Aplicado R$ | Saldo Bruto R$`, linha `BITCOIN (XBT)` `0,001` `300,00` `350,00`.
  - Conta corrente com saldo e Valores em trânsito com uma linha (devem ser ignorados).
- `extrato-secao-desconhecida.pdf`: o mesmo cabeçalho; uma seção `Renda fixa - Detalhamento - Tesouro Direto | TESOURO NACIONAL` e uma `Renda variável - Posição - Opções`, mais uma ação válida `ABCD3`.
- `extrato-varias-paginas.pdf`: 30 ações (`AAAA3` … ) que passam da página 1 para a 2, com o cabeçalho de página e o cabeçalho de colunas repetidos na página 2.
- `outro-banco.pdf`: uma página com "Extrato de Investimentos – Banco Qualquer" e uma tabela de ações; sem "Conta Investimento" nem `btgpactual`.

- [ ] **Step 2: Gerar e conferir**

Run: `cargo run -p guru-core --example gerar_extratos_btg -- crates/guru-core/tests/fixtures/btg && pdftotext -layout crates/guru-core/tests/fixtures/btg/extrato-completo.pdf - | head -60`
Expected: os quatro arquivos existem; o texto mostra as seções acima nas posições esperadas (o `pdftotext` aqui é só para conferência manual, não entra no app nem nos testes).

- [ ] **Step 3: Commit** — `git commit -m "Importação BTG: gerador e amostras fictícias de extrato"`

---

### Task 3: Leitor do extrato da BTG

**Files:**
- Create: `crates/guru-core/src/importacao/btg.rs`
- Modify: `crates/guru-core/src/importacao/mod.rs`, `crates/guru-core/src/lib.rs` (reexportar `ExtratoBtg`, `AtivoBtg`, `ler_extrato`, `ler_extrato_linhas`)
- Test: `crates/guru-core/tests/importacao_btg.rs` (novo) e `#[cfg(test)]` em `btg.rs`

**Interfaces:**
- Consumes: `LinhaPdf`, `Celula`, `extrair_linhas`, `esqueleto` (Task 1).
- Produces:

```rust
pub struct ExtratoBtg {
    pub data: NaiveDate,            // fim do período
    pub ativos: Vec<AtivoBtg>,
    pub avisos: Vec<String>,
    pub linhas_lidas: usize,        // = ativos.len() + linhas recusadas com aviso
}
pub enum AtivoBtg {
    Fundo { nome: String, cnpj: String, aplicado: Decimal, primeira_compra: NaiveDate, saldo_bruto: Option<Decimal>, referencia: Option<NaiveDate> },
    RendaFixa { tipo: &'static str, emissor: String, codigo: String, emissao: NaiveDate, vencimento: NaiveDate, primeira_aquisicao: NaiveDate,
                indexador: Option<&'static str>, taxa: Option<Decimal>, benchmark: Option<&'static str>, liquidez: &'static str, aplicado: Decimal, saldo_bruto: Decimal },
    RendaVariavel { tipo: &'static str, ticker: String, nome: String, quantidade: i64, preco_medio: Decimal },
    Cripto { nome: String, aplicado: Option<Decimal>, saldo_bruto: Decimal },
}
pub fn ler_extrato(bytes: &[u8]) -> Result<ExtratoBtg, CoreError>;           // extrair_linhas + ler_extrato_linhas
pub fn ler_extrato_linhas(linhas: &[LinhaPdf]) -> Result<ExtratoBtg, CoreError>;
```

`nome` de fundo sem o sufixo ` - Classe CNPJ: …` e sem `*`; `cnpj` só dígitos; `aplicado` de fundo e de renda fixa = soma dos lotes; `primeira_compra`/`primeira_aquisicao` = a menor data dos lotes; `emissor` como no título (`BANCO FICTICIO S.A`).

- [ ] **Step 1: Escrever os testes**

Em `tests/importacao_btg.rs`, com `include_bytes!("fixtures/btg/…")`:

```rust
#[test]
fn extrato_completo() {
    let e = ler_extrato(COMPLETO).unwrap();
    assert_eq!(e.data, d(2026, 10, 31));
    assert!(e.avisos.is_empty(), "{:?}", e.avisos);
    // Fundo com dois lotes: aplicado somado, primeira compra, saldo e referência da posição
    assert!(e.ativos.contains(&AtivoBtg::Fundo { nome: "FUNDO EXEMPLO RF LP".into(), cnpj: "11111111000111".into(),
        aplicado: dec!(1000.00), primeira_compra: d(2026, 1, 2), saldo_bruto: Some(dec!(1050.00)), referencia: Some(d(2026, 10, 30)) }));
    // CDB com código e taxa quebrados em duas linhas
    assert!(e.ativos.contains(&AtivoBtg::RendaFixa { tipo: "rf-cdb", emissor: "BANCO FICTICIO S.A".into(), codigo: "CDBA26FICT1".into(),
        emissao: d(2026, 1, 15), vencimento: d(2029, 1, 15), primeira_aquisicao: d(2026, 1, 15), indexador: Some("pre"), taxa: Some(dec!(12.50)),
        benchmark: None, liquidez: "vencimento", aplicado: dec!(5000.00), saldo_bruto: dec!(5400.00) }));
    // LCA pós-fixada em % do CDI
    assert!(matches!(achar_rf(&e, "LCA26EXEMP"), AtivoBtg::RendaFixa { tipo: "rf-lca", indexador: Some("pos"), benchmark: Some("cdi"), .. }));
    // Ticker sem asterisco
    assert!(e.ativos.contains(&AtivoBtg::RendaVariavel { tipo: "rv-acao", ticker: "ABCD3".into(), nome: "EMPRESA EXEMPLO ON".into(), quantidade: 10, preco_medio: dec!(20.00) }));
    assert!(matches!(achar_rv(&e, "XYZW34"), AtivoBtg::RendaVariavel { tipo: "rv-bdr", quantidade: 3, .. }));
    assert!(e.ativos.contains(&AtivoBtg::Cripto { nome: "BITCOIN (XBT)".into(), aplicado: Some(dec!(300.00)), saldo_bruto: dec!(350.00) }));
    assert_eq!(e.ativos.len(), 7); // conta corrente e valores em trânsito ficam de fora
}

#[test]
fn secao_desconhecida_vira_aviso() {
    let e = ler_extrato(DESCONHECIDA).unwrap();
    assert_eq!(e.ativos.len(), 1);
    assert!(e.avisos.iter().any(|a| a.contains("Tesouro Direto") && a.ends_with("não reconhecida.")));
    assert!(e.avisos.iter().any(|a| a.contains("Opções")));
}

#[test]
fn tabela_continua_na_pagina_seguinte() {
    let e = ler_extrato(VARIAS_PAGINAS).unwrap();
    assert_eq!(e.ativos.len(), 30);
    assert!(e.avisos.is_empty(), "{:?}", e.avisos);
}

#[test]
fn pdf_de_outro_banco() {
    let erro = ler_extrato(OUTRO_BANCO).unwrap_err();
    assert!(matches!(erro, CoreError::ArquivoInvalido(ref m) if m == "Este PDF não é um Extrato da Conta Investimento da BTG."));
}
```

Em `btg.rs` (`#[cfg(test)]`), com `LinhaPdf` montadas à mão (auxiliar `l(pagina, y, &[(x, "texto")])`) copiando as posições do extrato real:

- `total_nao_entra_na_ultima_linha`: detalhamento de CDB com a linha de totais 24 pt abaixo da aquisição → `aplicado` é o da aquisição, não o dobro.
- `taxas`: `"12,75% a.a."`, `"103,00% do CDI"`, `"IPCA + 6,50% a.a."`, `"IPCA + 6,50%"`, `"CDI + 1,00%"` (→ `None`/`None`, sem pânico).
- `ano_com_dois_digitos`: `"07/10/30"` → 2030-10-07.
- `quantidade_fracionaria_de_acao_vira_aviso`: `Qtde.` `"1,5"` → ativo não entra, aviso `"Ação <ticker>: quantidade fracionária não importada."`.
- `cripto_sem_coluna_de_saldo_vira_aviso`: seção `CriptoAtivos - Posição` só com `Ativo | Qtde.` → aviso `"Seção de criptoativos em formato não reconhecido."`.
- `rotulos_com_glifos_nulos`: o mesmo detalhamento de fundo, uma vez com rótulos inteiros e outra com `"\0"` no lugar dos `a` → mesmos ativos.

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p guru-core --test importacao_btg && cargo test -p guru-core --lib importacao::btg`
Expected: FAIL (não compila).

- [ ] **Step 3: Implementar `btg.rs`**

Algoritmo (o que os testes não determinam):
1. **É da BTG?** Esqueleto do texto da 1ª página contém `esqueleto("Conta Investimento")` **e** algum texto do documento contém `btgpactual`. Senão, o erro de "outro banco". A data é a segunda data `dd/mm/aa` da primeira linha cujo esqueleto começa com `esqueleto("Período de")`.
2. **Descartar ruído de página:** linhas cujo esqueleto começa com `extrato`, `periodo`/`período` (pelo esqueleto), igual a `investimento`, começa com `disclaimers`, ou que são só um número (número da página).
3. **Seções:** uma linha é título quando o esqueleto do seu texto começa com o esqueleto de um dos prefixos de Global Constraints. A seção vai até o próximo título (mesmo em outra página).
4. **Cabeçalho de colunas:** as linhas da seção entre o título e a primeira linha-âncora formam a faixa de cabeçalho. Descarte células que são só dígitos (notas). Junte células de linhas diferentes cujo x difere em até 4 pt numa coluna só (rótulo = textos de cima para baixo unidos por espaço). Identifique cada coluna esperada comparando esqueletos (igualdade ou prefixo). Coluna obrigatória ausente → aviso `Seção "<título>": falta a coluna <coluna>.` e a seção é pulada. Na página seguinte, o cabeçalho repetido (mesmos esqueletos) é pulado.
5. **Linhas-âncora** (uma por linha de dado): fundo detalhamento e renda fixa → a célula da primeira coluna de data (`Data Compra` / `Emissão`) é `dd/mm/aa`; fundo posição → a linha logo abaixo de um título `… - Classe CNPJ: …`; renda variável e cripto → a primeira célula não começa com `Total` e não é cabeçalho.
6. **Montar a linha de dado:** cada linha física da seção que não é âncora, título, cabeçalho ou `Total…` vai para a âncora mais próxima em y, **se** a distância for ≤ 1,2 × a altura da fonte da âncora (o maior `tamanho` dos caracteres); senão é descartada. Cada célula vai para a coluna de maior x inicial ≤ `x_da_célula + 3`. Textos da mesma coluna são unidos de cima para baixo (`"CDB-" + "CDBA26FICT1"` sem espaço quando o primeiro termina em `-`; com espaço nos demais casos).
7. **Fundos:** junte posição e detalhamento pelo CNPJ. Detalhamento sem posição → `saldo_bruto`/`referencia` `None`. Posição sem detalhamento → aviso `Fundo <nome>: sem detalhamento das aplicações.` e não entra.
8. Valores com `Decimal` a partir de `1.234,56`; `-` = vazio.

- [ ] **Step 4: Rodar e ver passar** — os dois comandos do Step 2. Expected: PASS.

- [ ] **Step 5: Commit** — `git commit -m "Importação BTG: leitor do extrato"`

---

### Task 4: Importação neutra quanto à fonte, sem "fora da B3"

**Files:**
- Create: `crates/guru-core/src/migrations/0008_btg.sql`
- Modify: `crates/guru-core/src/db.rs` (registrar a migração), `crates/guru-core/src/importacao/comparacao.rs`, `aplicacao.rs`, `mod.rs`, `crates/guru-core/src/lib.rs`, `crates/guru-core/tests/importacao.rs`, `crates/guru-core/tests/db.rs` (versão do esquema), `crates/guru-core/tests/atualizador.rs` (se usar os nomes renomeados)

**Interfaces:**
- Produces:
  - Migração:
    ```sql
    ALTER TABLE investimentos ADD COLUMN saldo_centavos INTEGER;
    ALTER TABLE investimentos ADD COLUMN saldo_data TEXT;
    ALTER TABLE importacao ADD COLUMN fonte TEXT NOT NULL DEFAULT 'b3';
    ```
  - `pub enum Fonte { B3, Btg }` com `codigo()` (`"b3"`, `"btg"`) e `rotulo()` (`"B3"`, `"BTG"`).
  - `Comparacao { pub fonte: Fonte, pub data_arquivo: Option<NaiveDate>, pub itens, pub avisos, pub linhas_lidas }` (`data_arquivo` = `None` na B3).
  - `Estado { Novo, Atualizar, Divergente, Igual }` (sai `ForaDaB3`).
  - `Acao { Adicionar, Ignorar, UsarArquivo, Manter, Vincular(i64) }` (sai `Remover`; `UsarB3` vira `UsarArquivo`, código `"usar_arquivo"`).
  - `Item.na_b3` → `Item.no_arquivo`; `Item.b3: DadosB3` → `Item.dados: DadosArquivo`:
    ```rust
    pub(crate) struct DadosArquivo {
        pub codigo: Option<String>,
        pub quantidade: Option<i64>,
        pub aplicado_centavos: Option<i64>,
        pub preco_medio_centavos: Option<i64>,   // usado na Task 8
        pub saldo: Option<(i64, NaiveDate)>,      // usado na Task 8
        pub completar: Option<Completar>,         // usado na Task 8
    }
    pub(crate) struct Completar { pub emissor: Option<String>, pub indexador: Option<String>, pub taxa: Option<String>, pub benchmark: Option<String> }
    ```
  - `aplicar_importacao` grava `importacao.fonte = comparacao.fonte.codigo()`.
  - Os auxiliares `normalizar_emissor`, `centavos`, `com_pares`, `padrao`, `ids`, `total_aplicado`, `faltando`, `quantidade_texto` e `Contexto::item` passam a `pub(super)`, para a Task 7.

- [ ] **Step 1: Ajustar os testes existentes da B3 ao novo contrato e escrever os novos**

Em `tests/importacao.rs`: troque `UsarB3`/`na_b3` pelos novos nomes; apague os testes de "fora da B3" e de `Remover`, substituindo-os por:

```rust
#[test]
fn investimento_que_nao_veio_na_planilha_nao_aparece() {
    let db = db_com(&[cdb_manual("Banco Sem Planilha")]);
    let c = comparar(&db, &ler_posicao(BDR_RF).unwrap(), hoje()).unwrap();
    assert!(c.itens.iter().all(|i| i.ativo != "Banco Sem Planilha"));
    assert_eq!(c.fonte, Fonte::B3);
}

#[test]
fn importacao_grava_a_fonte() {
    // aplicar uma comparação da B3 → SELECT fonte FROM importacao = 'b3'
}
```

Em `tests/db.rs`: `SCHEMA_VERSION == 8`.

- [ ] **Step 2: Rodar e ver falhar** — `cargo test -p guru-core --test importacao --test db`. Expected: FAIL (não compila).

- [ ] **Step 3: Implementar** os renomes, a migração, a remoção de `fora_da_b3`/`Remover`/`Operacao::Apagar` e a coluna `fonte`. Os textos de erro de `conferir_vinculos` passam a falar em "linha do arquivo" no lugar de "linha da planilha". Nenhum comportamento da B3 muda além de "fora da B3" sumir.

- [ ] **Step 4: Rodar** — `cargo test -p guru-core`. Expected: PASS.

- [ ] **Step 5: Commit** — `git commit -m "Importação: neutra quanto à fonte e sem sugerir apagar o que ficou de fora"`

---

### Task 5: Um investimento por corretora na B3

**Files:**
- Modify: `crates/guru-core/src/importacao/comparacao.rs`
- Create: `crates/guru-core/src/importacao/instituicao.rs`
- Test: `crates/guru-core/tests/importacao.rs`, `#[cfg(test)]` em `instituicao.rs`

**Interfaces:**
- Consumes: `normalizar_emissor` (Task 4), `INSTITUICOES` (catálogo).
- Produces: `pub(crate) fn chave_instituicao(texto: &str) -> String`.

- [ ] **Step 1: Escrever os testes**

```rust
#[test]
fn chave_instituicao_liga_ao_catalogo() {
    assert_eq!(chave_instituicao("BTG PACTUAL CTVM S/A"), chave_instituicao("BTG Pactual"));
    assert_eq!(chave_instituicao("XP INVESTIMENTOS CCTVM S/A"), chave_instituicao("XP Investimentos"));
    assert_eq!(chave_instituicao("NU INVEST CORRETORA DE VALORES S.A."), chave_instituicao("Nubank"));
    assert_eq!(chave_instituicao("BANCO DO BRASIL S/A"), chave_instituicao("Banco do Brasil"));
    assert_eq!(chave_instituicao("ITAU CORRETORA DE VALORES S/A"), chave_instituicao("Itaú"));
    assert_ne!(chave_instituicao("CORRETORA DESCONHECIDA LTDA"), chave_instituicao("BTG Pactual"));
    assert_ne!(chave_instituicao("XP Investimentos"), chave_instituicao("BTG Pactual"));
}
```

Marcas (palavra inteira do nome normalizado, ou a sequência de palavras): `BTG`→"BTG Pactual"; `XP`→"XP Investimentos"; `INTER`→"Banco Inter"; `RICO`→"Rico"; `NU`, `NUINVEST`, `NUBANK`→"Nubank"; `BANCO DO BRASIL`, `BB`→"Banco do Brasil"; `ITAU`→"Itaú"; `BRADESCO`→"Bradesco"; `SANTANDER`→"Santander"; `CAIXA`→"Caixa Econômica Federal"; `C6`→"C6 Bank". Sem marca → o próprio nome normalizado. A chave devolvida é o nome do catálogo normalizado.

Em `tests/importacao.rs` (a fixture `posicao-completa.xlsx` já tem o mesmo ticker em duas instituições? Se não, crie as linhas com `ler_posicao_abas` e `Aba`/`Celula` em memória, como os testes atuais fazem):

```rust
#[test]
fn mesmo_ticker_em_duas_corretoras_vira_dois_itens() {
    // B3: PETR4 10 na "XP INVESTIMENTOS CCTVM S/A" e 1 na "BTG PACTUAL CTVM S/A"; carteira vazia
    // → dois itens novos, sugestão com quantidade 10 e 1 e a instituição de cada linha
}

#[test]
fn cada_corretora_casa_com_o_seu_investimento() {
    // carteira: PETR4 10 em "XP Investimentos" e PETR4 1 em "BTG Pactual"; planilha igual → dois "igual"
}

#[test]
fn soma_antiga_vira_atualizar_da_corretora() {
    // carteira: PETR4 11 em "XP Investimentos", codigo_b3 = "PETR4" (gravado pela soma antiga)
    // planilha: 10 na XP e 1 na BTG → XP = Atualizar (11 → 10, ação padrão UsarArquivo), BTG = Novo
    // e aplicar sem decisões não muda a quantidade da XP
}

#[test]
fn tesouro_separado_por_corretora() {
    // mesmo título e vencimento em duas instituições → dois itens
}
```

- [ ] **Step 2: Rodar e ver falhar** — `cargo test -p guru-core --test importacao && cargo test -p guru-core --lib importacao::instituicao`. Expected: FAIL.

- [ ] **Step 3: Implementar**

- `GrupoRv` e `GrupoTesouro` passam a agrupar por `(ticker, aba, chave_instituicao)` e `(tipo, vencimento, chave_instituicao)`; `instituicoes: Vec<&str>` vira `instituicao: &str` (a primeira grafia vista); a função `juntar` sai.
- Os filtros de `pares` em `renda_variavel`, `renda_fixa` e `tesouro` exigem `chave_instituicao(&i.instituicao) == chave_instituicao(linha)`.
- O texto exibido em `Item.instituicao` é o da linha da planilha.

- [ ] **Step 4: Rodar** — `cargo test -p guru-core`. Expected: PASS.

- [ ] **Step 5: Commit** — `git commit -m "Importação da B3: um investimento por ativo e por corretora"`

---

### Task 6: Saldo do extrato no valor atual

**Files:**
- Modify: `crates/guru-core/src/investimento/repositorio.rs` (campos e SELECT), `crates/guru-core/src/mercado/valorizacao.rs`
- Test: `crates/guru-core/tests/valorizacao.rs`

**Interfaces:**
- Consumes: colunas `saldo_centavos`, `saldo_data` (Task 4).
- Produces:
  - `Investimento { …, pub saldo_centavos: Option<i64>, pub saldo_data: Option<String> }` (`AAAA-MM-DD`).
  - `Origem::Extrato`, código `"extrato"`, rótulo `"saldo do extrato da corretora"`.
  - Regra em `valorizar_com`: para `fundo`, `prev` e `cripto`, se `saldo_centavos` e `saldo_data` existem e `saldo_data <= data` → `com_atual(aplicado, saldo, Origem::Extrato, Some(saldo_data))`; senão o comportamento atual (`SemFonte`).

- [ ] **Step 1: Escrever os testes**

```rust
#[test]
fn fundo_com_saldo_do_extrato() {
    // fundo aplicado 1.000,00, saldo 1.050,00 em 2026-10-30
    // valorizar em 2026-11-01 → atual 105000, Origem::Extrato, data 2026-10-30, rentabilidade positiva
}

#[test]
fn saldo_futuro_nao_vale_para_datas_anteriores() {
    // mesmo fundo valorizado em 2026-10-01 → aplicado, Origem::SemFonte
}

#[test]
fn fundo_sem_saldo_continua_sem_fonte() { /* o teste atual de SemFonte continua passando */ }
```

Grave o saldo no teste com um `UPDATE investimentos SET saldo_centavos = …, saldo_data = …` direto (a gravação pela importação é da Task 8).

- [ ] **Step 2: Rodar e ver falhar** — `cargo test -p guru-core --test valorizacao`. Expected: FAIL.
- [ ] **Step 3: Implementar.** Procure os `match` exaustivos sobre `Origem` (`grep -rn "Origem::" crates`) e inclua `Extrato`. O renderer já mostra `origemValorRotulo` e `dataValor` para origens com valor (`app/renderer/js/telas/_tabela.js:45-50`); não precisa mudar.
- [ ] **Step 4: Rodar** — `cargo test -p guru-core`. Expected: PASS.
- [ ] **Step 5: Commit** — `git commit -m "Valorização: fundos e cripto pelo saldo do extrato"`

---

### Task 7: Comparação do extrato da BTG com a carteira

**Files:**
- Create: `crates/guru-core/src/importacao/comparacao_btg.rs`
- Modify: `crates/guru-core/src/importacao/mod.rs`, `crates/guru-core/src/lib.rs` (`comparar_btg`)
- Test: `crates/guru-core/tests/importacao_btg.rs`

**Interfaces:**
- Consumes: `ExtratoBtg`/`AtivoBtg` (Task 3); `Comparacao`, `Item`, `Estado`, `Acao`, `DadosArquivo`, `Completar`, `Fonte` e auxiliares (Task 4); `chave_instituicao` (Task 5).
- Produces: `pub fn comparar_btg(db: &Db, extrato: &ExtratoBtg, hoje: NaiveDate) -> Result<Comparacao, CoreError>` — `fonte: Fonte::Btg`, `data_arquivo: Some(extrato.data)`, `avisos` e `linhas_lidas` do extrato, chave dos itens `btg:<índice do ativo>`.

Regras por ativo (instituição de tudo: `"BTG Pactual"`; `pares` sempre filtrados por `chave_instituicao` igual e por `!usados`):

| Ativo | Pares por código | Pares sem código (só investimentos com `codigo_b3` vazio) | Comparado | `no_arquivo` | Sugestão de novo |
|---|---|---|---|---|---|
| Renda variável | `tipo` começa com `rv-` e `ticker` igual (sem diferenciar maiúsculas) | — | quantidade | `quantidade_texto` | tipo, instituição, ticker, quantidade, `preco_medio_centavos`, `data_aplicacao` = `extrato.data` |
| Renda fixa | `rf-*` e `codigo_b3 == codigo` | `rf-*`, mesmo `vencimento`, `normalizar_emissor` igual, mesmo tipo | valor aplicado (±1 centavo) | `formatar_reais(aplicado)` | tipo, instituição, emissor, indexador, taxa, benchmark, `data_aplicacao` = primeira aquisição, vencimento, valor aplicado, liquidez, IR padrão do tipo |
| Fundo | `tipo == "fundo"` e `codigo_b3 == cnpj` | `tipo == "fundo"`, `normalizar_emissor(emissor) == normalizar_emissor(nome)` | valor aplicado (±1) | idem | tipo `fundo`, instituição, emissor = nome, `data_aplicacao` = primeira compra, valor aplicado, liquidez e IR padrão do tipo |
| Cripto | `tipo == "cripto"` e `codigo_b3 == normalizar_emissor(nome)` | `tipo == "cripto"`, nome normalizado igual | valor aplicado, se o extrato traz; sem ele um par único é `Igual` | `formatar_reais(saldo)` + " (saldo)" quando não há aplicado | tipo `cripto`, instituição, emissor = nome, valor aplicado se houver, `data_aplicacao` = `extrato.data` |

- Estado e ações: nenhum par → `Novo` [`Adicionar`, `Ignorar`]; pares → `com_pares(&pares, igual)` (um par com código e valor diferente → `Atualizar` [`UsarArquivo`, `Manter`]; um par sem código e diferente → `Divergente` [`Manter`, `UsarArquivo`]; vários pares → `Divergente` [`Manter`, `Vincular(id)`…, `Adicionar`] e nenhum fica `usado`).
- `DadosArquivo`: `codigo` (identidade de Global Constraints); `quantidade` só na renda variável; `aplicado_centavos` em renda fixa, fundo e cripto com aplicado; `preco_medio_centavos` na renda variável; `saldo` = `(saldo_bruto, referencia)` em fundo (só se os dois existem) e `(saldo_bruto, extrato.data)` em cripto; `completar` em renda fixa (`emissor`, `indexador`, `taxa`, `benchmark`).
- Valor que não cabe em centavos → aviso `Valor fora do limite em <ativo>.` e o ativo não é comparado (como a B3).

- [ ] **Step 1: Escrever os testes** (carteira montada com o `inserir` dos testes atuais; extrato = `ler_extrato(COMPLETO)` ou `ExtratoBtg` montado à mão):

```rust
#[test]
fn carteira_vazia_da_so_novos_com_sugestao_completa() {
    // todos Novo; o CDB, a LCA, os fundos e as ações sem `faltando`; a cripto sem `faltando` (tem aplicado)
    // c.fonte == Fonte::Btg; c.data_arquivo == Some(2026-10-31)
}

#[test]
fn btg_casa_ativo_vindo_da_b3() {
    // carteira: ABCD3 10 em "BTG PACTUAL CTVM S/A", codigo_b3 "ABCD3" → Igual
    // carteira: CDB codigo_b3 "CDBA26FICT1" em "BTG PACTUAL CTVM S/A", aplicado 5.000,00 → Igual
}

#[test]
fn mesmo_ticker_em_outra_corretora_nao_casa() {
    // carteira: ABCD3 10 em "XP Investimentos" → item da BTG é Novo; o da XP nem aparece
}

#[test]
fn fundo_lancado_a_mao_casa_pelo_nome() {
    // carteira: fundo "Fundo Exemplo RF LP", BTG Pactual, aplicado 1.000,00, sem código → Igual
    // mesmo fundo com aplicado 900,00 → Divergente [Manter, UsarArquivo]
}

#[test]
fn dois_fundos_com_o_mesmo_nome_viram_divergente_com_vincular() {}

#[test]
fn aplicado_diferente_com_codigo_vira_atualizar() {
    // carteira: CDB com codigo_b3 "CDBA26FICT1", aplicado 4.000,00 → Atualizar, padrão UsarArquivo
}
```

- [ ] **Step 2: Rodar e ver falhar** — `cargo test -p guru-core --test importacao_btg`. Expected: FAIL.
- [ ] **Step 3: Implementar `comparar_btg`.** Ordene os itens por `Estado::ordem()` como `comparar`.
- [ ] **Step 4: Rodar** — `cargo test -p guru-core`. Expected: PASS.
- [ ] **Step 5: Commit** — `git commit -m "Importação BTG: comparação com a carteira"`

---

### Task 8: Aplicação — saldo, campos vazios e preço médio

**Files:**
- Modify: `crates/guru-core/src/importacao/aplicacao.rs`
- Test: `crates/guru-core/tests/importacao_btg.rs`, `crates/guru-core/tests/importacao.rs`

**Interfaces:**
- Consumes: `DadosArquivo` (Task 4), `comparar_btg` (Task 7).
- Produces: novas operações internas (`Operacao::Saldo { id, centavos, data }`, `Operacao::Completar { id, completar, preco_medio }`); `aplicar_importacao` com a mesma assinatura.

Regras:
- **Confirmado** = item `Igual` com um par, ou ação `UsarArquivo`, ou `Vincular(id)`, ou `Adicionar` (no investimento recém-inserido). Item `Divergente` mantido não é confirmado.
- Em todo item confirmado: grava o código (como hoje), o `saldo` se houver, e o `completar` — todos com `contar = false`.
- `Completar` só preenche o que está vazio, num `UPDATE` só, lendo os valores antigos: `emissor = COALESCE(emissor, :e)`; `indexador`, `taxa` e `benchmark` só quando `taxa IS NULL` (os três juntos, com `CASE WHEN taxa IS NULL`); `preco_medio_centavos = COALESCE(preco_medio_centavos, :p)`.
- `UsarArquivo` na renda variável: se o preço médio gravado é `NULL`, usa o `preco_medio_centavos` do arquivo para o recálculo e o grava; se não há nenhum dos dois, o erro de campo atual ("Informe o preço médio.").
- `Saldo`: `UPDATE investimentos SET saldo_centavos = :c, saldo_data = :d WHERE id = :id` (substitui sempre: o extrato mais recente vale).

- [ ] **Step 1: Escrever os testes**

```rust
#[test]
fn adicionar_fundo_grava_codigo_e_saldo() {
    // aplica o extrato completo numa carteira vazia com Adicionar em tudo
    // → fundo "FUNDO EXEMPLO RF LP": codigo_b3 "11111111000111", aplicado 100000, saldo 105000 em 2026-10-30
    // → valorizar em 2026-11-06 dá Origem::Extrato
}

#[test]
fn igual_atualiza_o_saldo_sem_contar_alteracao() {
    // fundo já igual, saldo antigo 100000 → depois de aplicar: saldo 105000, aplicadas == 0
}

#[test]
fn divergente_mantido_nao_ganha_saldo_nem_codigo() {}

#[test]
fn cdb_vindo_da_b3_ganha_taxa_sem_perder_o_que_tinha() {
    // CDB com codigo "CDBA26FICT1", emissor "Banco Fictício", taxa NULL, indexador NULL
    // → depois: taxa "12.50", indexador "pre", emissor continua "Banco Fictício"
    // CDB com taxa "13" → depois: taxa continua "13"
}

#[test]
fn usar_arquivo_preenche_preco_medio_vazio() {
    // ação com quantidade 5 e preço médio NULL; extrato 10 a 20,00 → quantidade 10, preço 2000, aplicado 20000
}

#[test]
fn importacao_btg_grava_a_fonte() { /* SELECT fonte FROM importacao → 'btg' */ }
```

- [ ] **Step 2: Rodar e ver falhar** — `cargo test -p guru-core --test importacao_btg`. Expected: FAIL.
- [ ] **Step 3: Implementar.** O código e o saldo do `Adicionar` usam o id devolvido por `inserir_validado` dentro de `executar`.
- [ ] **Step 4: Rodar** — `cargo test -p guru-core`. Expected: PASS (inclusive os testes da B3).
- [ ] **Step 5: Commit** — `git commit -m "Importação: grava saldo e completa campos vazios nos itens confirmados"`

---

### Task 9: Ponte Node

**Files:**
- Modify: `crates/guru-node/src/importacao.rs`, `crates/guru-node/src/lib.rs`, `crates/guru-node/index.d.ts` (regerado pelo build)
- Test: `crates/guru-node/test/importacao.test.mjs`

**Interfaces:**
- Consumes: `ler_extrato`, `comparar_btg` (Tasks 3 e 7); `Fonte`, `Acao::UsarArquivo` (Task 4).
- Produces (JS):
  - `core.compararImportacaoBtg(bytes: Buffer, arquivo: string): ComparacaoJs` — mesmo token e guarda da `compararImportacao`.
  - `ComparacaoJs` ganha `fonte: "b3" | "btg"` e `dataArquivo: string | null` (`AAAA-MM-DD`).
  - `ItemImportacaoJs.naB3` → `noArquivo`.
  - Decisões aceitam `"usar_arquivo"`; `"usar_b3"` e `"remover"` passam a ser "Ação desconhecida".

- [ ] **Step 1: Escrever os testes**

```js
const btg = (nome) => readFileSync(new URL(`../../guru-core/tests/fixtures/btg/${nome}`, import.meta.url));

test("comparar o extrato da BTG numa carteira vazia", () => {
  const c = abrir();
  const r = c.compararImportacaoBtg(btg("extrato-completo.pdf"), "extrato.pdf");
  assert.equal(r.fonte, "btg");
  assert.equal(r.dataArquivo, "2026-10-31");
  assert.equal(r.itens.length, 7);
  assert.ok(r.itens.every((i) => i.estado === "novo" && i.instituicao === "BTG Pactual"));
});

test("aplicar o extrato grava e a última importação é a de hoje", () => {});

test("PDF de outro banco dá ARQUIVO_INVALIDO", () => {
  assert.throws(() => abrir().compararImportacaoBtg(btg("outro-banco.pdf"), "x.pdf"), { code: "ARQUIVO_INVALIDO" });
});

test("comparar a B3 depois da BTG invalida o token da BTG", () => {});
```

Ajuste os testes atuais (`naB3` → `noArquivo`, `usar_b3` → `usar_arquivo`, sem `fora_da_b3`/`remover`) e inclua `assert.equal(r.fonte, "b3")`.

- [ ] **Step 2: Rodar e ver falhar** — `cd crates/guru-node && npm run build && npm test`. Expected: FAIL.
- [ ] **Step 3: Implementar.** `compararImportacaoBtg` dentro de `blindar`, como `compararImportacao`.
- [ ] **Step 4: Rodar** — mesmo comando. Expected: PASS.
- [ ] **Step 5: Commit** — `git commit -m "Ponte Node: importação do extrato da BTG"`

---

### Task 10: Dois botões, tela por fonte e e2e

**Files:**
- Modify: `app/main.js`, `app/preload.js`, `app/renderer/js/api.js`, `app/renderer/js/telas/investimentos.js`, `app/renderer/js/telas/importacao.js`
- Test: `app/test/e2e/importacao.spec.js`, `app/test/e2e/importacao-btg.spec.js` (novo), `app/test/unit/api.test.mjs` (se cobrir a forma das chamadas)

**Interfaces:**
- Consumes: `compararImportacaoBtg`, `fonte`, `dataArquivo`, `noArquivo`, `usar_arquivo` (Task 9).
- Produces:
  - IPC `importacao:escolher` recebe `fonte` (`"b3"` | `"btg"`); outra → erro com código `FONTE_DESCONHECIDA`. Preload: `escolher: (fonte) => ipcRenderer.invoke("importacao:escolher", fonte)`; `api.importacao.escolher(fonte)`.
  - Rotas: `#/investimentos/importar` (B3, como hoje) e `#/investimentos/importar/btg`.

Textos (exatos):

| | B3 | BTG |
|---|---|---|
| Botão na tela de Investimentos | "Importar da B3" | "Importar da BTG" |
| Título | "Importar da B3" | "Importar da BTG" |
| Botão de escolher / depois | "Escolher planilha" / "Escolher outra planilha" | "Escolher PDF" / "Escolher outro PDF" |
| Dica inicial | (a atual) | "Gere o “Extrato da Conta Investimento” em PDF no app ou no site da BTG e escolha o arquivo aqui." |
| Coluna | "Na planilha B3" | "No extrato BTG" |
| Ação `usar_arquivo` (estado atualizar / divergente) | "Atualizar" / "Usar valor da B3" | "Atualizar" / "Usar valor da BTG" |
| Cartão do arquivo | "· Área do Investidor B3 · N linhas lidas" | "· Extrato BTG de dd/mm/aaaa · N ativos lidos" |
| Nota do rodapé | "Fundos e previdência normalmente não aparecem na B3." | "Conta corrente e valores em trânsito não são importados." |
| Expirada | "A comparação expirou. Escolha o arquivo de novo." | idem |
| Diálogo | filtro "Planilha do Excel" `xlsx` | filtro "Extrato em PDF" `pdf` |
| Arquivo grande | "O arquivo é grande demais para uma planilha de posição." | "O arquivo é grande demais para um extrato da BTG." |
| Variável de teste | `GURU_TESTE_ARQUIVO_B3` | `GURU_TESTE_ARQUIVO_BTG` |

- [ ] **Step 1: Escrever os testes e2e**

`importacao-btg.spec.js` (mesmos auxiliares de `importacao.spec.js`, com `GURU_TESTE_ARQUIVO_BTG` apontando para `crates/guru-core/tests/fixtures/btg/…`):
- `os dois botões de importação aparecem em Investimentos`: links "Importar da B3" e "Importar da BTG" visíveis; o segundo leva ao título "Importar da BTG".
- `importar o extrato completo numa carteira vazia`: escolher → cartão com "Extrato BTG de 31/10/2026 · 7 ativos lidos" → aplicar → lista de investimentos com "FUNDO EXEMPLO RF LP" e o valor atual do fundo com título contendo "saldo do extrato da corretora".
- `seção desconhecida aparece como aviso`: o aviso com "Tesouro Direto" visível.
- `PDF de outro banco mostra o erro`: faixa de alerta com "Este PDF não é um Extrato da Conta Investimento da BTG."
- `cancelar o diálogo não muda nada` (`GURU_TESTE_ARQUIVO_BTG=cancelar`).

Em `importacao.spec.js`: retire os casos de "fora da B3"/"remover" e acrescente `investimento lançado à mão não aparece na revisão da B3`.

- [ ] **Step 2: Rodar e ver falhar** — `cd app && npx playwright test test/e2e/importacao-btg.spec.js test/e2e/importacao.spec.js`. Expected: FAIL.
- [ ] **Step 3: Implementar.** Em `main.js`, `escolherPlanilhaB3` vira `escolherArquivo(fonte)` com uma tabela `FONTES` (filtro, variável de teste, mensagem de tamanho, função do núcleo); `LIMITE_ARQUIVO` de 20 MB para as duas. Em `importacao.js`, `render` lê `parametros[1] === "btg"`; os textos da tabela acima saem de um objeto por fonte; `CLASSE_ESTADO`, `marcas` e `MUDAM` perdem `fora_da_b3`/`remover`; `MUDAM` troca `usar_b3` por `usar_arquivo`.
- [ ] **Step 4: Rodar** — `cd app && npm test && npx playwright test`. Expected: PASS.
- [ ] **Step 5: Commit** — `git commit -m "Importação BTG: botão, tela por fonte e testes de ponta a ponta"`

---

### Task 11: Conferência com o PDF real e documentação

**Files:**
- Modify: `crates/guru-core/tests/importacao_btg.rs` (teste `#[ignore]`)
- Modify: `docs/roadmap.md` (linha "8. Importação da BTG" e o link deste plano), `README.md` (se listar o que o app importa)

- [ ] **Step 1: Teste manual com o PDF real, fora do repositório**

```rust
#[test]
#[ignore = "precisa do extrato real: GURU_EXTRATO_BTG=<caminho>"]
fn extrato_real() {
    let caminho = std::env::var("GURU_EXTRATO_BTG").expect("GURU_EXTRATO_BTG");
    let e = ler_extrato(&std::fs::read(caminho).unwrap()).unwrap();
    println!("{e:#?}");
    assert!(e.avisos.is_empty(), "{:?}", e.avisos);
    assert!(!e.ativos.is_empty());
}
```

- [ ] **Step 2: Rodar com o PDF do usuário**

Run: `GURU_EXTRATO_BTG=<caminho do extrato real> cargo test -p guru-core --test importacao_btg extrato_real -- --ignored --nocapture`
Expected: PASS; a saída tem 2 fundos, 2 CDBs, 1 ação e 1 BDR, sem cripto (o extrato só tem movimentação de cripto), e data 2026-11-06. Confira cada valor contra as páginas 2 a 5 do PDF (`pdftotext -layout` serve para conferir). Se algo não bater (por exemplo a escala do tamanho da fonte da Task 1), corrija na task dona do código, com um teste que reproduza o caso usando dados fictícios, e rode de novo. **Não copie nenhum valor do PDF real para o repositório.**

- [ ] **Step 3: Rodar o app de verdade**

Run: `cd app && npm start`, Investimentos → "Importar da BTG" → escolher o PDF real → conferir a revisão → Cancelar (não aplicar no banco do usuário sem ele pedir).
Expected: a revisão mostra os 6 ativos com instituição "BTG Pactual" e nenhum "fora da …".

- [ ] **Step 4: Documentação.** Em `docs/roadmap.md`, a linha `| 8. Importação da BTG | Ler o extrato em PDF da BTG e comparar com o cadastrado; um investimento por corretora nas duas importações; valor atual de fundos e cripto pelo extrato. | 3 |` e o link deste plano na lista.

- [ ] **Step 5: Verificação completa**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && (cd crates/guru-node && npm test) && (cd app && npm test && npx playwright test)`
Expected: tudo passa.

- [ ] **Step 6: Commit** — `git commit -m "Importação BTG: conferência com extrato real e roteiro"`
