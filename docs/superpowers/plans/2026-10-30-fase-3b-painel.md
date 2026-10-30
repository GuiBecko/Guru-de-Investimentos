# Fase 3b: Painel da carteira — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A tela Carteira vira o painel principal do protótipo:
- saudação;
- números do topo;
- evolução do patrimônio contra o total aportado;
- rosca por classe contra o alvo;
- cartões por classe com minigráfico;
- rentabilidade acumulada contra CDI, IPCA e BOVA11;
- mês a mês;
- instituições, vencimentos, altas e quedas.

O detalhe da classe ganha o gráfico contra um benchmark à escolha.

**Architecture:**
- O núcleo calcula o histórico com a valorização da fase 3a: o valor de cada investimento em cada fim de mês dos últimos 6 ou 12 meses e hoje. Com ele calcula os retornos mensais, os benchmarks nas mesmas datas e as listas do painel. Tudo sai numa chamada `painel(meses, classe?)`.
- A interface desenha os gráficos em SVG com um módulo próprio, sem biblioteca, com `createElementNS` e classes de CSS (a CSP não deixa `style=`).

**Tech Stack:** o mesmo da fase 3a. Nenhuma dependência nova.

**Spec:**
- [`docs/mvp.md`](../../mvp.md): telas 5 e 6, decisões "Saudação", "Valor atual", "Ibovespa e IFIX" e "Classes".
- Protótipo: `docs/prototipo/index.html:645-800` (telas 5 e 6).

## Global Constraints

- Tudo das fases anteriores continua valendo:
  - segurança da janela e CSP (`script-src 'self'`, `style-src 'self'`);
  - nada de `innerHTML` com dados nem atributo `style=`;
  - `vigente()` depois de cada await;
  - pt-BR;
  - códigos de erro em SCREAMING_SNAKE_CASE;
  - preload sem lógica;
  - IPC com checagem de origem;
  - dinheiro em centavos `i64`, taxas em `Decimal`, somas com `checked_add`.
- **Fora desta fase:**
  - Proventos (número do topo e gráfico): não há fonte. O COTAHIST não traz proventos; fica para a importação da B3 (fase 4) ou V2.
  - Insights da IA e notícias: fase 6.
  - O painel não mostra espaço reservado para nenhum deles.
- **Datas do painel** (`datas_painel(hoje, meses)`): o último dia de cada um dos `meses` meses anteriores ao mês de `hoje`, em ordem crescente, mais `hoje`, que é a data de mercado.
  - Exemplo: `hoje = 2026-10-28`, `meses = 12` → `2025-10-31, 2025-11-30, …, 2026-09-30, 2026-10-28`, 13 pontos.
  - `meses` só pode ser 6 ou 12; outro valor → `CoreError` de validação com código `PERIODO_INVALIDO`.
- **Patrimônio em D** = soma de `valorizar(inv, D).atual` dos investimentos com `data_aplicacao ≤ D`. **Aportado em D** = soma do aplicado desses mesmos investimentos. Investimento aplicado depois de D não entra em nenhum dos dois.
- **Retorno do intervalo** `(D₀, D₁]` (método de Dietz simples):
  - `F` = aplicado dos investimentos com `D₀ < data_aplicacao ≤ D₁`.
  - `V₀` = patrimônio em D₀, `V₁` = patrimônio em D₁.
  - `r = (V₁ − V₀ − F) / (V₀ + F/2)`.
  - Denominador ≤ 0 → retorno ausente (`None`), nunca divisão por zero.
- **Acumulado da carteira** no ponto i = `Π(1 + r_k) − 1` para k ≤ i. Retorno ausente conta como fator 1. O ponto 0 vale 0.
- **Benchmarks acumulados** de D₀ (primeiro ponto) até cada Dᵢ:
  - CDI e Selic: `Π(1 + t_d/100)` para d em `(D₀, Dᵢ]`.
  - IPCA: `Π(1 + ipca_m/100)` dos meses cujo último dia está em `(D₀, Dᵢ]` e que estão guardados.
  - BOVA11 e XFIX11: `cotacao_em(t, Dᵢ) / cotacao_em(t, D₀) − 1`.
  - Sem dado em D₀, o benchmark inteiro fica ausente. Sem dado num ponto, só aquele ponto fica ausente.
- Percentuais trafegam em **centésimos de ponto percentual** `i64`, arredondados meio para cima: `+1,07%` → `107`. A interface formata com 2 casas quando |x| < 10% e 1 casa acima disso.
- **Benchmark padrão por classe** no detalhe:
  - `rf` → CDI;
  - `acoes` → BOVA11;
  - `fii` → XFIX11;
  - `ext` → CDI;
  - `outros` → CDI.
- **Opções do seletor:** CDI, Selic, IPCA, BOVA11, XFIX11.
- **Rótulos dos benchmarks:** "CDI", "Selic", "IPCA", "BOVA11 (Ibovespa)", "XFIX11 (IFIX)".
- Trabalho na branch `fase-3b-painel`. Commits em português terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Os e2e rodam com a janela escondida, no modo de fixtures e com o lançador comum `app/test/e2e/_app.js`, que espera a atualização da abertura.

## Review Focus

1. **Carteira nova** (tudo aplicado este mês, ou um investimento só): nenhum ponto anterior tem patrimônio.
   - Esperado: retornos ausentes, acumulado 0, gráficos sem `NaN` e sem quebrar.
   - O texto "Ainda não há histórico para comparar." aparece no lugar dos gráficos de rentabilidade.
   - Teste na Tarefa 1 e na Tarefa 6.
2. **Benchmark sem dado no início** (BOVA11 sem cotação no primeiro fim de mês, IPCA atrasado).
   - Esperado: a série some do gráfico e a legenda diz "sem dados" ao lado do rótulo, sem zerar a linha.
   - Teste na Tarefa 2 e na Tarefa 7.
3. **Aplicação no último dia do mês e aplicação depois de D:** a primeira entra no ponto daquele mês e a segunda não. Teste na Tarefa 1.
4. **Trocar 6/12 meses ou o benchmark rápido várias vezes:** a resposta mais antiga não pode sobrescrever a mais nova (`vigente()` ou um contador local). Teste na Tarefa 6.
5. **Tema escuro:** as cores das séries vêm dos tokens, que já têm versão escura, nunca de cores fixas no JS. Teste: nenhum `#` de cor nos módulos JS de gráfico (checado na revisão e por um teste unitário que lê o arquivo).

---

### Tarefa 1: Histórico e retornos da carteira

**Files:**
- Create: `crates/guru-core/src/painel/mod.rs`, `crates/guru-core/src/painel/historico.rs`
- Modify: `crates/guru-core/src/lib.rs`, `crates/guru-core/src/error.rs` (código `PERIODO_INVALIDO`, se for preciso um variant novo)
- Test: unidade em `historico.rs`; `crates/guru-core/tests/painel.rs`

**Interfaces:**
- Consumes: `valorizar_com`/`Series` (`mercado/valorizacao.rs`), `Db::listar_investimentos`, `Classe`.
- Produces:
  - `pub fn datas_painel(hoje: NaiveDate, meses: u32) -> Result<Vec<NaiveDate>, CoreError>`
  - `pub struct PontoHistorico { pub data: NaiveDate, pub patrimonio_centavos: i64, pub aportado_centavos: i64, pub retorno_centesimos: Option<i64> /* intervalo que termina aqui; None no ponto 0 */, pub acumulado_centesimos: i64 }`
  - `pub struct Historico { pub pontos: Vec<PontoHistorico>, pub por_classe: Vec<(Classe, Vec<i64> /* patrimônio por ponto */)> }`. `por_classe` traz só as classes com algum investimento, na ordem de `Classe`.
  - `pub fn historico(db: &Db, hoje: NaiveDate, meses: u32, classe: Option<Classe>) -> Result<Historico, CoreError>`. Com `classe`, os pontos consideram só os investimentos daquela classe.
  - As séries são carregadas uma vez (`Series::carregar(db, hoje)`). `valorizar_com` precisa avaliar corretamente numa data D anterior a `hoje`; confira e, se preciso, ajuste para usar só pontos ≤ D.

- [ ] **Step 1: Testes que falham**, com as fixtures da fase 3a gravadas no banco e `hoje = 2026-10-28`:
  - `datas_painel(2026-10-28, 12)` dá os 13 pontos do exemplo das Global Constraints; `datas_painel(2026-03-15, 6)` começa em `2025-09-30`; `meses = 7` dá `PERIODO_INVALIDO`.
  - Uma ação ITUB4 aplicada em 2026-10-01: ausente em todos os pontos de fim de mês (patrimônio 0, aportado 0) e presente em 28/10 com `150 × 3978`.
  - Um CDB aplicado em 2026-09-30 entra no ponto de 30/09 (Review Focus 3).
  - Retorno com Dietz: monte o caso sintético `V₀ = 100000`, `F = 20000`, `V₁ = 121000`. Esperado `r = 1000/110000` → 91 centésimos (0,91%).
  - Carteira com tudo aplicado depois do primeiro ponto: `retorno_centesimos` é `None` onde o denominador é ≤ 0, e `acumulado_centesimos` é 0 até o primeiro retorno válido (Review Focus 1).
  - Acumulado encadeado: dois retornos de 1,00% dão 2,01% (201).
  - `classe = Some(Classe::Acoes)` ignora o CDB.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** — `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`.
- [ ] **Step 5: Commit** — "Histórico do patrimônio e retornos da carteira".

---

### Tarefa 2: Benchmarks, números do topo e listas do painel

**Files:**
- Create: `crates/guru-core/src/painel/benchmark.rs`, `crates/guru-core/src/painel/listas.rs`
- Modify: `crates/guru-core/src/painel/mod.rs`
- Test: unidade nos dois arquivos; `crates/guru-core/tests/painel.rs`

**Interfaces:**
- Consumes: Tarefa 1; `Db::indice_entre`, `Db::ipca_meses`, `Db::cotacao_em`.
- Produces:
  - `pub enum Referencia { Cdi, Selic, Ipca, Bova11, Xfix11 }` (o nome `Benchmark` já existe em `investimento`), com códigos `cdi`, `selic`, `ipca`, `bova11`, `xfix11` e os rótulos das Global Constraints.
  - `pub fn benchmark_padrao(classe: Classe) -> Referencia`
  - `pub fn acumulado_benchmark(db: &Db, r: Referencia, datas: &[NaiveDate]) -> Result<Option<Vec<Option<i64>>>, CoreError>`. O resultado é `None` quando falta o dado em D₀ e, quando há, traz um valor por data (centésimos, ponto 0 = 0).
  - `pub struct Numeros { pub patrimonio_centavos: i64, pub aportado_centavos: i64, pub mes_centesimos: Option<i64> /* retorno do último intervalo */, pub mes_cdi_centesimos: Option<i64>, pub periodo_centesimos: i64 /* acumulado no último ponto */, pub periodo_cdi_centesimos: Option<i64>, pub rendeu_centavos: i64 /* V_último − V₀ − Σ F do período */ }`
  - `pub struct Instituicao { pub nome: String, pub atual_centavos: i64, pub percentual_decimos: i64 }`, ordenadas do maior para o menor; o empate vai por nome.
  - `pub struct Vencimento { pub id: i64, pub nome: String, pub vencimento: NaiveDate, pub atual_centavos: i64 }`, com `vencimento ≥ hoje`, em ordem crescente, no máximo 5.
  - `pub struct Variacao { pub ticker: String, pub variacao_decimos: i64 }`, para os tickers de renda variável da carteira com cotação em `hoje` e no último fim de mês, com datas diferentes, ordenados do maior para o menor.
  - `pub struct Painel { pub datas: Vec<NaiveDate>, pub historico: Historico, pub benchmarks: Vec<(Referencia, Option<Vec<Option<i64>>>)>, pub numeros: Numeros, pub instituicoes: Vec<Instituicao>, pub vencimentos: Vec<Vencimento>, pub variacoes: Vec<Variacao> }`
  - `pub fn painel(db: &Db, hoje: NaiveDate, meses: u32, classe: Option<Classe>) -> Result<Painel, CoreError>`. Sem classe, os benchmarks são `[Cdi, Ipca, Bova11]`, na ordem do gráfico do protótipo. Com classe, vêm os cinco, para o seletor do detalhe. Com classe, as listas consideram só aquela classe.

- [ ] **Step 1: Testes que falham** (fixtures da 3a):
  - CDI acumulado entre 2026-09-30 e 2026-10-28 = produto dos fatores da fixture nesse intervalo, calculado no teste a partir do arquivo.
  - IPCA de `datas_painel(2026-10-28, 12)`: entram os meses de nov/2025 a set/2026 que estão na fixture.
  - BOVA11 sem cotação em D₀ (a fixture só tem 28/10): `acumulado_benchmark` devolve `None` (Review Focus 2). Grave uma cotação sintética de BOVA11 em D₀ e confira a razão.
  - `Numeros.rendeu_centavos` num caso sintético com um aporte no meio do período.
  - Instituições: dois investimentos na "Rico" e um no "Inter" somam 100,0% e vêm ordenados.
  - Vencimentos: o vencido não entra; vêm no máximo 5, em ordem.
  - Variações: um ticker com cotação só em `hoje` não aparece.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** (workspace, clippy all-targets, fmt).
- [ ] **Step 5: Commit** — "Benchmarks, números e listas do painel".

---

### Tarefa 3: Tesouro com vencimento que não existe

Pendência da fase 3a: um título `td-*` com vencimento digitado errado nunca ganha preço, então `tesouro_incompleto` fica verdadeiro para sempre e o CSV de 15 MB é baixado a cada rodada.

**Files:**
- Modify: `crates/guru-core/src/mercado/atualizador.rs`, `crates/guru-core/src/mercado/repositorio.rs` (se precisar de leitura nova)
- Test: `crates/guru-core/tests/atualizador.rs`

**Interfaces:**
- Produces: depois de ler o CSV do Tesouro sem erro, o atualizador grava em `cobertura_serie` uma chave por título da carteira, `tesouro:<tipo>:<vencimento>`, com `desde = hoje`. Isso não precisa de migração.
  - `tesouro_incompleto` passa a ignorar os títulos sem preço que já têm a chave, porque foram procurados num CSV baixado.
  - A atualização diária continua: se o Tesouro não foi atualizado hoje, o CSV é baixado e as chaves são regravadas.

- [ ] **Step 1: Teste que falha** — `td-selic` com vencimento `2031-07-15`, que não existe no CSV da fixture:
  - a primeira rodada baixa o CSV;
  - a segunda rodada no mesmo `hoje` não baixa (o falso conta as chamadas), e o investimento continua `sem_dados`;
  - com `hoje + 1 dia`, baixa de novo.
  - O teste do título novo cadastrado no mesmo dia (fase 3a) continua passando.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar.**
- [ ] **Step 5: Commit** — "Tesouro com vencimento inexistente não baixa o CSV a cada rodada".

---

### Tarefa 4: Painel no addon e na ponte

**Files:**
- Create: `crates/guru-node/src/painel.rs`
- Modify: `crates/guru-node/src/lib.rs`, `app/main.js`, `app/preload.js`, `app/renderer/js/api.js`
- Test: `crates/guru-node/test/painel.test.mjs`, `app/test/unit/api.test.mjs`, `app/test/e2e/abrir.spec.js` (superfície)

**Interfaces:**
- Produces (addon): `core.painel(meses: number, classe?: string): PainelJs`, calculado com a data de mercado (`data_mercado()`, como as outras leituras valorizadas). Campos em camelCase:
  - `datas: string[]` (`YYYY-MM-DD`)
  - `pontos: { patrimonioCentavos, aportadoCentavos, retornoCentesimos: number | null, acumuladoCentesimos }[]`
  - `porClasse: { classe, rotulo, patrimonio: number[] }[]`
  - `benchmarks: { codigo, rotulo, valores: (number | null)[] | null }[]`
  - `numeros: { patrimonioCentavos, aportadoCentavos, mesCentesimos, mesCdiCentesimos, periodoCentesimos, periodoCdiCentesimos, rendeuCentavos }`, com `null` onde é `Option`
  - `instituicoes: { nome, atualCentavos, percentualDecimos }[]`
  - `vencimentos: { id, nome, vencimento, atualCentavos }[]`
  - `variacoes: { ticker, variacaoDecimos }[]`
  - `benchmarkPadrao?: string`, só com classe.
  - Classe desconhecida → código `NAO_ENCONTRADO`; `meses` fora de 6/12 → `PERIODO_INVALIDO`.
- Produces (IPC/api): canal `carteira:painel` e `api.carteira.painel(meses, classe)`. A superfície continua a mesma lista de chaves; `carteira` ganha só o método.

- [ ] **Step 1: Testes que falham**:
  - O addon com fixtures e `hoje: "2026-10-28"` devolve 13 `datas` para 12 meses e 7 para 6.
  - `benchmarks` tem `cdi`, `ipca` e `bova11` sem classe, e os cinco com `classe: "rf"`, com `benchmarkPadrao === "cdi"`.
  - `meses = 3` → `PERIODO_INVALIDO`; `classe: "xyz"` → `NAO_ENCONTRADO`.
  - Unitário do `api.js`; e2e: `api.carteira.painel(12)` responde.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** (Rust, addon build+test, unitários, `npx playwright test abrir`).
- [ ] **Step 5: Commit** — "Painel no addon e na ponte".

---

### Tarefa 5: Módulo de gráficos e saudação

**Files:**
- Create: `app/renderer/js/graficos.js` (geometria pura + construção do SVG), `app/renderer/js/saudacao.js`
- Modify: `app/renderer/css/telas.css` (classes das séries e do gráfico, com os tokens `--accent`, `--bench`, `--c1…--c5`, `--gridline`, `--up` e `--down`), `app/renderer/js/formato.js` (`formatarCentesimos`)
- Test: `app/test/unit/graficos.test.mjs`, `app/test/unit/saudacao.test.mjs`, `app/test/unit/formato.test.mjs`

**Interfaces:**
- Produces, geometria pura testável em Node:
  - `escala(dominio: [min, max], faixa: [a, b]) -> (v) => number`. Com domínio degenerado (min == max), devolve o meio da faixa.
  - `dominioComZero(valores: (number|null)[]) -> [min, max]`: ignora `null` e sempre inclui 0.
  - `caminhoLinha(pontos: ({x, y} | null)[]) -> string`, o `d` do path. `null` quebra a linha com um novo `M`.
  - `arcosRosca(fatias: number[], raio, raioInterno) -> string[]`, um `d` por fatia, começando no topo em sentido horário. Fatia 0 dá string vazia; uma fatia única dá um anel completo.
  - `indiceMaisProximo(xs: number[], x: number) -> number`
- Produces, construção (DOM, testada nos e2e):
  - `graficoLinhas(datas: string[], series: { rotulo, valores: (number|null)[], classe: string, tracejada?: boolean }[], formatarValor: (v) => string, rotuloAcessivel: string) -> Element`
  - `graficoBarras(datas, valores: (number|null)[], formatarValor, rotuloAcessivel) -> Element`, com barras positivas em `up` e negativas em `down`.
  - `graficoRosca(fatias: { rotulo, valor, classe }[], rotuloAcessivel) -> Element`
  - `minigrafico(valores: number[], classe) -> Element`
  - Todos criam `<svg role="img" aria-label=…>` com `createElementNS`, grade pontilhada e eixo com as datas em `mm/aa`. Ao passar o mouse aparece uma dica (`<div class="dica">`) com a data e os valores do ponto mais próximo. Cores só por classe de CSS.
- `formatarCentesimos(c: number | null) -> string`: `107` → `+1,07%`, `-2350` → `−23,5%`, `0` → `0,00%`, `null` → `—`. Usa o sinal de menos tipográfico, como `formatarRentabilidade`.
- `saudacao({ agora: Date, painel: PainelJs, sorteio: () => number }) -> { data: string, frase: string }`:
  - `data` sai por extenso em pt-BR com `Intl`, por exemplo "quarta, 28 de outubro de 2026".
  - O cumprimento depende da hora: das 5h às 11h59 "Bom dia"; das 12h às 17h59 "Boa tarde"; nas outras horas "Boa noite".
  - A `frase` é `"<cumprimento>. <texto>"`, com o texto sorteado (`sorteio()` em [0,1)) entre os modelos aplicáveis desta lista exata:
    - sempre: "Bom ver você por aqui."
    - sempre: "Sua carteira está atualizada com os dados de mercado mais recentes."
    - sempre: "Um passo de cada vez: o que importa é a constância dos aportes."
    - sempre: "Tudo o que está aqui fica só no seu computador."
    - se `mesCentesimos > mesCdiCentesimos`, ambos não nulos: "Seu mês está acima do CDI até agora."
    - se `mesCentesimos < mesCdiCentesimos`, ambos não nulos: "O mês está abaixo do CDI até agora. Meses assim fazem parte."
    - se há variação > 0: "<ticker> é o destaque do mês, com <+x,x%>." (a maior alta)
    - se há vencimento em até 30 dias: "<nome> vence em <dd/mm>."
    - se `rendeuCentavos > 0`: "Seus investimentos renderam <R$ x> no período."
  - A frase nunca recomenda compra ou venda.

- [ ] **Step 1: Testes que falham**:
  - `escala([0,10],[0,100])(5) === 50`; domínio degenerado dá o meio.
  - `caminhoLinha` com `null` no meio tem dois `M`.
  - `arcosRosca([1,1], …)` dá dois arcos que somam a volta; `[1]` dá um anel.
  - `formatarCentesimos` cobre os quatro exemplos.
  - `saudacao`: às 9h com `sorteio = () => 0` dá "Bom dia. Bom ver você por aqui."; às 13h dá "Boa tarde"; às 22h dá "Boa noite".
  - `saudacao`: com mês acima do CDI, o modelo do CDI aparece na lista de aplicáveis. Exporte `modelosAplicaveis(painel, hoje)` para testar.
  - `saudacao`: a data de 2026-10-28 sai "quarta, 28 de outubro de 2026".
  - Teste que lê o texto de `graficos.js` e falha se achar `#[0-9a-fA-F]{3,6}` (Review Focus 5).
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** — `cd app && npm test`.
- [ ] **Step 5: Commit** — "Gráficos em SVG e saudação".

---

### Tarefa 6: Tela Carteira como painel

**Files:**
- Modify: `app/renderer/js/telas/carteira.js` (a função do painel geral; se passar de ~250 linhas, mova o painel para `app/renderer/js/telas/carteira/painel.js`), `app/renderer/css/telas.css`
- Test: `app/test/e2e/carteira.spec.js` (ajustes), `app/test/e2e/painel.spec.js` (novo)

**Interfaces:**
- Consumes: Tarefas 4 e 5, `api.carteira.resumo()` (rosca e cartões: percentual e alvo), `dataDoMercado`/`textoNota`.

Layout (protótipo, tela 5, sem proventos, insights nem notícias):
- **Topo:**
  - a data por extenso e a frase da saudação, sorteadas uma vez por abertura do app, guardadas no módulo e não a cada redesenho;
  - à direita, o seletor "6 meses" / "12 meses" (`aria-pressed`, padrão 12) e o link "Tenho dinheiro para aportar" para `#/aporte`.
- **Números**, cinco cartões:
  - "Patrimônio total", com "aportado R$ x" embaixo;
  - "No mês", com "CDI ±x" embaixo;
  - "Em 12 meses" ou "Em 6 meses", com "CDI ±x" embaixo;
  - "Rendeu em 12 meses" ou "Rendeu em 6 meses", em reais;
  - "Total aportado".
- **Gráficos:**
  - "Evolução do patrimônio": patrimônio cheio e total aportado tracejado.
  - "Por classe": a rosca pelo atual e a legenda com `atual · alvo`.
  - Os cartões por classe como hoje, cada um com o minigráfico de `porClasse`.
  - "Rentabilidade acumulada": a carteira contra os três benchmarks, cada um com a sua cor e tracejado.
  - "Mês a mês": as barras de `retornoCentesimos`.
- **Listas:**
  - "Por instituição", com a % do patrimônio;
  - "Altas e quedas no mês", com até 3 altas e 3 quedas, ▲/▼ e as classes `up`/`down`;
  - "Próximos vencimentos", com o nome e a data, e o nome como link para `#/investimentos/<id>`.
  - Lista vazia mostra "Nada por aqui ainda.".
- **Nota** no fim: `textoNota(data)`, a mesma da fase 3a.
- **Sem histórico** (nenhum retorno válido no período): os dois gráficos de rentabilidade dão lugar a "Ainda não há histórico para comparar." (Review Focus 1).
- **Carteira vazia:** fica como está.
- **Trocar o período** refaz só a chamada `painel(meses)` e o desenho. Use um contador local para descartar a resposta antiga (Review Focus 4), além do `vigente()`.

- [ ] **Step 1: Testes e2e que falham** (fixtures, data de mercado 28/10/2026). Os investimentos são criados pela API e depois atualizados com o helper que clica em "Atualizar":
  - A saudação começa com "Bom dia", "Boa tarde" ou "Boa noite" e a data está por extenso.
  - "Patrimônio total" mostra a soma pelo valor atual.
  - Os cinco cartões têm os rótulos exatos, e trocar para "6 meses" muda os rótulos para "Em 6 meses" e "Rendeu em 6 meses".
  - Há SVGs com os `aria-label` "Evolução do patrimônio contra o total aportado", "Distribuição do patrimônio por classe", "Rentabilidade acumulada da carteira contra CDI, IPCA e BOVA11" e "Rentabilidade mês a mês".
  - Uma carteira só com ITUB4 de 2026-10-01 mostra "Ainda não há histórico para comparar.".
  - A instituição aparece com 100,0%.
  - Um CDB com vencimento em 2027 aparece em "Próximos vencimentos" com link para a edição.
  - Os testes atuais da carteira (cartões, alvo, detalhe) continuam passando, ajustados ao novo layout só onde o texto mudou.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** — `cd app && npm test && npx playwright test`.
- [ ] **Step 5: Commit** — "Painel da carteira".

---

### Tarefa 7: Detalhe da classe com benchmark

**Files:**
- Modify: `app/renderer/js/telas/carteira.js` (a parte do detalhe), `app/renderer/css/telas.css`
- Test: `app/test/e2e/carteira.spec.js`

Comportamento (protótipo, tela 6):
- **Cabeçalho:** à direita do título, "Valor" com o atual da classe e "Em 12 meses" com o acumulado da classe.
- **Card do gráfico:**
  - legenda "Sua <classe em minúsculas>" contra o benchmark escolhido;
  - seletor com os cinco benchmarks e `aria-pressed`, começando no padrão da classe;
  - gráfico de linhas acumuladas com o `aria-label` "Rentabilidade acumulada da classe contra o benchmark".
- **Benchmark sem dados:** o botão fica habilitado. Ao escolhê-lo, a linha do benchmark some e aparece "<rótulo>: sem dados no período." (Review Focus 2).
- **Tabela e nota:** ficam como na fase 3a.
- **Trocar de benchmark** não refaz a chamada: os cinco já vêm em `painel(12, classe)`.

- [ ] **Step 1: Testes e2e que falham**:
  - Ações começa com BOVA11 pressionado e renda fixa começa com CDI.
  - Clicar em IPCA marca IPCA, e a legenda mostra "IPCA".
  - Com a fixture, BOVA11 não tem cotação no início do período, então escolher BOVA11 mostra "BOVA11 (Ibovespa): sem dados no período.".
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** — suíte inteira (Rust, addon, unitários e e2e) uma vez.
- [ ] **Step 5: Commit** — "Detalhe da classe contra benchmark".

---

### Tarefa 8: Documentação

**Files:**
- Modify:
  - `docs/mvp.md`, decisões:
    - "Rentabilidade da carteira": "Retorno mensal pelo método de Dietz simples (desconta os aportes do mês), encadeado no período. Histórico de 6 ou 12 meses, pelos fechamentos de fim de mês.";
    - "Proventos": "Fora do painel até haver fonte (importação da B3 ou V2).";
    - na tela 5, tirar "proventos", "insights da IA e notícias" do que já existe e anotar "(fase 6)" para insights e notícias.
  - `docs/roadmap.md`: fase 3b com o link deste plano.
  - `README.md`: status da fase.

- [ ] **Step 1: Editar os arquivos.**
- [ ] **Step 2: Commit** — "Documentação do painel".
