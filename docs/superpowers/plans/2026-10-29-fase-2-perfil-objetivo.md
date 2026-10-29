# Fase 2: Perfil e objetivo — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O usuário responde o questionário de perfil sem ver nenhuma prévia do resultado, recebe o perfil (Conservador, Moderado ou Arrojado) com avisos de inconsistência e uma alocação-alvo sugerida que pode ajustar, escreve o objetivo em texto livre, cria metas com valor e prazo, vincula investimentos às metas e vê o progresso de cada meta e a carteira contra a alocação-alvo.

**Architecture:** O questionário (perguntas, pontuação, travas, consistência e alocação sugerida) é dado e regra do `guru-core`. A interface pede o questionário ao núcleo, mostra uma pergunta por vez e só envia as respostas no fim; o núcleo calcula e grava o resultado. Metas, alocação-alvo e objetivo são tabelas novas (migração 0003). O progresso das metas e os percentuais da carteira contra o alvo são calculados no núcleo, em inteiros.

**Tech Stack:** o mesmo das fases 0 e 1.

**Spec:** [`docs/mvp.md`](../../mvp.md) (telas 1 e 2), protótipo [`docs/prototipo/index.html`](../../prototipo/index.html) (`#perfil` linhas 413-453, `#objetivo` linhas 456-508).

## Global Constraints

- Tudo das fases 0 e 1 continua valendo: segurança da janela, CSP, pt-BR, códigos de erro, preload sem lógica, interface sempre por `api.js`, `vigente()` depois de cada `await`, nada de `innerHTML` com dados nem `style=`, todo canal IPC por `tratar` com checagem de origem.
- Dinheiro em centavos `i64` com teto `MAX_CENTAVOS`; percentuais em inteiros (pontos percentuais inteiros na alocação-alvo, décimos de ponto nos percentuais calculados).
- **Nenhuma prévia do resultado durante o teste:** nada de pontuação, perfil parcial ou alocação antes da última resposta. A interface nem recebe a pontuação das opções.
- O texto da tela de resultado e da tela de perfil diz que o questionário segue as dimensões das regras de suitability da ANBIMA (objetivos, situação financeira, conhecimento) e que **não é recomendação de investimento**.
- Trabalho na branch `fase-2-perfil-objetivo`. Commits em português terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Os e2e abrem janelas na tela do usuário: enquanto itera, rode só os testes focados (`-g`); a suíte inteira uma vez antes do commit.

## Questionário (conteúdo exato)

Doze perguntas em três blocos. Cada opção vale de 0 a 3 pontos, na ordem listada (a primeira vale 0). As perguntas 4 e 12 são de consistência.

**Bloco "Objetivo e prazo"**

1. *Qual é o seu principal objetivo ao investir?* — Preservar o que tenho, sem perdas · Ganhar um pouco mais que a poupança, com pouca oscilação · Fazer o patrimônio crescer, aceitando oscilações · Buscar o maior retorno possível, mesmo com perdas temporárias grandes. — Por que perguntamos: "O objetivo define quanto risco faz sentido correr."
2. *Daqui a quanto tempo você deve precisar da maior parte deste dinheiro?* — Menos de 1 ano · De 1 a 3 anos · De 3 a 10 anos · Mais de 10 anos. — "Quanto mais tempo até o resgate, mais tempo a carteira tem para se recuperar de uma queda."
3. *Se seus investimentos caíssem 15% em um mês, o que você faria?* — Resgataria tudo para não perder mais · Resgataria uma parte · Manteria e esperaria recuperar · Aproveitaria para aplicar mais. — "Quedas fazem parte de qualquer investimento que rende mais que a poupança. Saber como você reage ajuda a não sugerir algo que vai tirar o seu sono."
4. *Para o próximo ano, qual destas carteiras você escolheria?* — Ganho certo de 6% · Resultado entre −3% e +12% · Resultado entre −10% e +20% · Resultado entre −25% e +40%. — "Ver os números lado a lado ajuda a confirmar quanto de oscilação você aceita."

**Bloco "Situação financeira"**

5. *Quanto da sua renda mensal vai para despesas fixas?* — Mais de 80% · De 60% a 80% · De 40% a 60% · Menos de 40%. — "Quem tem folga no orçamento aguenta melhor um período ruim sem precisar resgatar."
6. *Você tem uma reserva de emergência?* — Não tenho · Menos de 3 meses de custo de vida · De 3 a 6 meses · Mais de 6 meses. — "A reserva evita vender um investimento no pior momento por causa de um imprevisto."
7. *Quanto do seu patrimônio investido você pode precisar resgatar nos próximos 2 anos?* — Mais da metade · Entre um quarto e a metade · Menos de um quarto · Nada. — "Dinheiro que vai ser usado logo precisa ficar em investimentos com pouca oscilação."
8. *Quanto você tem investido hoje, sem contar o imóvel em que mora?* — Até R$ 10 mil · De R$ 10 mil a R$ 100 mil · De R$ 100 mil a R$ 1 milhão · Mais de R$ 1 milhão. — "O tamanho do patrimônio muda quanto uma oscilação pesa na sua vida."

**Bloco "Conhecimento e experiência"**

9. *Qual destas frases descreve melhor onde você já investiu?* — Só na poupança · Em renda fixa, como CDB e Tesouro Direto · Em fundos, FIIs ou ações · Em opções, derivativos ou criptoativos. — "Experiência com um tipo de investimento ajuda a entender os riscos dele."
10. *Há quanto tempo você investe fora da poupança?* — Nunca investi · Menos de 2 anos · De 2 a 5 anos · Mais de 5 anos. — "O tempo investindo costuma incluir já ter passado por alguma queda."
11. *Como você descreveria o seu conhecimento sobre investimentos?* — Nenhum · Básico · Bom · Avançado ou profissional. — "Responda pensando no que você sabe hoje, não no que gostaria de saber."
12. *Quando a taxa de juros da economia sobe, o que acontece com o preço de um título prefixado que você já tem?* — Não sei (0) · Sobe (0) · Não muda (0) · Cai (3). — "Esta pergunta ajuda a conferir a resposta anterior. Não existe problema em não saber."

A pergunta 12 foge da regra "0 a 3 na ordem": suas pontuações são exatamente 0, 0, 0 e 3.

Texto fixo ao lado de toda pergunta: "Responda pensando no que você faria de verdade, não no que acha que deveria fazer. Não existe resposta certa." e "Seu perfil e a alocação sugerida aparecem depois da última pergunta."

**Resultado**

- Pontos = soma das 12 respostas (0 a 36). Até 12: Conservador; de 13 a 24: Moderado; 25 ou mais: Arrojado.
- Travas, nesta ordem: pergunta 2 = "Menos de 1 ano" → Conservador, com o aviso "Como você deve precisar do dinheiro em menos de 1 ano, o perfil ficou Conservador."; pergunta 6 = "Não tenho" e perfil Arrojado → Moderado, com o aviso "Sem reserva de emergência, o perfil ficou limitado a Moderado. Montar a reserva vem antes."
- Consistência: pontos da 3 e da 4 diferem em 2 ou mais → "Suas respostas sobre quedas e oscilação não combinam. Vale refazer o teste com calma."; pergunta 11 com 2 ou mais pontos e pergunta 12 com 0 → "Você declarou mais conhecimento do que a resposta sobre títulos prefixados indica."
- Descrições: Conservador "Prioriza a segurança e a liquidez. Aceita pouca ou nenhuma oscilação."; Moderado "Aceita alguma oscilação em parte da carteira para buscar retorno acima da renda fixa no médio prazo."; Arrojado "Aceita oscilações fortes no curto prazo em busca de mais retorno no longo prazo."
- Alocação sugerida (pontos percentuais, classes na ordem rf, acoes, fii, ext, outros): Conservador 85, 5, 5, 5, 0; Moderado 50, 25, 10, 10, 5; Arrojado 30, 35, 10, 20, 5.
- Validade: perfil feito há mais de 24 meses mostra "Seu perfil tem mais de 2 anos. Refaça o teste."

## Metas (regras exatas)

- Campos: `nome` (1 a 60 caracteres; "Informe o nome da meta."), `valorAlvoCentavos` (> 0 e ≤ `MAX_CENTAVOS`; "Informe o valor da meta."), `prazo` no formato `YYYY-MM`, igual ou depois do mês atual ao criar ("O prazo precisa ser este mês ou depois."; vazio: "Informe o prazo."). Ao editar, um prazo já vencido pode ser mantido.
- Acumulado = soma do valor aplicado dos investimentos vinculados.
- Meses restantes = meses do mês atual até o mês do prazo, contando o mês do prazo (prazo no mês atual → 1). Prazo passado → 0.
- Aporte mensal necessário = `ceil((alvo − acumulado) / meses restantes)` quando há meses restantes e falta valor.
- Ritmo atual = `acumulado / meses desde a primeira aplicação vinculada` (contando o mês atual, mínimo 1), arredondado para baixo; só existe com ao menos um investimento vinculado.
- Situação, nesta ordem: `concluida` (acumulado ≥ alvo) → "Meta alcançada."; `vencida` (meses restantes = 0) → "O prazo passou. Faltam R$ X."; `sem_aportes` (nenhum investimento vinculado) → "Ainda sem investimentos vinculados."; `atrasada` (ritmo < aporte necessário) → "No ritmo atual (R$ X/mês), chega N meses depois." com `N = ceil((alvo − acumulado) / ritmo) − meses restantes` (ritmo 0 → "No ritmo atual, a meta não é alcançada."); senão `no_caminho` → "No caminho certo."
- Apagar uma meta desvincula os investimentos (`ON DELETE SET NULL`), não os apaga.
- Investimento ganha `metaId` opcional; um id que não existe → erro no campo `metaId`: "Escolha uma meta da lista."

## Review Focus

1. **Voltar e mudar uma resposta no meio do teste** recalcula tudo no fim, sem guardar resposta antiga; sair do teste no meio não grava nada. Teste na Tarefa 5.
2. **Alocação-alvo que não soma 100** (99, 101, campo vazio, número negativo, decimal) é recusada com mensagem e não grava. Teste na Tarefa 2 e na Tarefa 6.
3. **Meta com prazo no mês atual, no mês passado e daqui a anos** calcula meses e aporte necessário sem divisão por zero e sem número negativo. Teste na Tarefa 3.
4. **Apagar uma meta com investimentos vinculados** mantém os investimentos e o formulário deles abre sem erro. Teste na Tarefa 3 e na Tarefa 7.
5. **Banco da fase 1 com investimentos** migra para a versão 3 sem perder nada. Teste na Tarefa 2.

---

### Tarefa 1: Questionário e cálculo do perfil no `guru-core`

**Files:**
- Create: `crates/guru-core/src/perfil/mod.rs`, `crates/guru-core/src/perfil/questionario.rs` (conteúdo), `crates/guru-core/src/perfil/calculo.rs`
- Modify: `crates/guru-core/src/lib.rs`
- Test: testes de unidade nos dois arquivos

**Interfaces:**
- Produces:
  - `pub enum Perfil { Conservador, Moderado, Arrojado }` pelo mesmo `enum_catalogo!` (códigos `conservador`, `moderado`, `arrojado`; rótulos `Conservador`, `Moderado`, `Arrojado`; com `TODOS`), e `fn descricao(&self) -> &'static str`.
  - `pub struct Opcao { pub texto: &'static str, pub pontos: u8 }`, `pub struct Pergunta { pub numero: u8, pub bloco: &'static str, pub texto: &'static str, pub porque: &'static str, pub opcoes: [Opcao; 4] }`, `pub fn perguntas() -> &'static [Pergunta]` (12, na ordem), `pub const AVISO_FIXO: [&str; 2]`.
  - `pub struct ResultadoPerfil { pub perfil: Perfil, pub pontos: u8, pub avisos: Vec<String>, pub alocacao_sugerida: Vec<(Classe, u8)> }`
  - `pub fn calcular(respostas: &[u8]) -> Result<ResultadoPerfil, ErroCampo>` — exige exatamente 12 respostas de 0 a 3 (índice da opção); senão `ErroCampo { campo: "respostas", mensagem: "Responda todas as perguntas." }`.
  - `pub fn alocacao_sugerida(perfil: Perfil) -> Vec<(Classe, u8)>` (cinco classes na ordem de `Classe::TODOS`, soma 100).

- [ ] **Step 1: Testes que falham** — `doze_perguntas_em_tres_blocos` (4 por bloco, nomes exatos); `pergunta_12_pontua_so_a_certa` (0,0,0,3); `todas_zero_da_conservador`; `todas_tres_da_arrojado`; `limites_12_13_24_25` (montando respostas com essas somas e sem travas); `prazo_curto_trava_em_conservador` (com o aviso exato); `sem_reserva_limita_arrojado_a_moderado` (aviso exato) e `sem_reserva_nao_mexe_em_moderado`; `inconsistencia_quedas` e `inconsistencia_conhecimento` (avisos exatos), `respostas_coerentes_sem_aviso`; `alocacoes_somam_100`; `respostas_incompletas_ou_fora_da_faixa` (11 respostas, 13, um 4 → erro).
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.** Conteúdo como `static`, cálculo puro.
- [ ] **Step 4: Rodar e ver passar** — `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`.
- [ ] **Step 5: Commit** — "Questionário de perfil e cálculo do resultado no núcleo".

---

### Tarefa 2: Migração 0003 e repositório de perfil, alocação-alvo e objetivo

**Files:**
- Create: `crates/guru-core/src/migrations/0003_perfil_metas.sql`
- Create: `crates/guru-core/src/perfil/repositorio.rs`
- Modify: `crates/guru-core/src/db.rs` (`SCHEMA_VERSION` = 3), testes da versão do esquema no addon e no e2e (para 3, como na fase 1)
- Test: `crates/guru-core/tests/perfil.rs`

**Interfaces:**
- Consumes: `calcular`, `alocacao_sugerida`, `Perfil`, `Classe` (Tarefa 1).
- Produces:
  - Tabelas: `perfis (id, perfil TEXT, pontos INTEGER, respostas TEXT /* "0,2,1,…" */, avisos TEXT /* JSON array */, feito_em TEXT /* YYYY-MM-DD */)` (histórico; o atual é o de maior id); `alocacao_alvo (classe TEXT PRIMARY KEY, percentual INTEGER NOT NULL CHECK (percentual BETWEEN 0 AND 100))`; `objetivo (id INTEGER PRIMARY KEY CHECK (id = 1), texto TEXT NOT NULL, atualizado_em TEXT NOT NULL)`; `metas (id, nome, valor_alvo_centavos INTEGER NOT NULL CHECK (> 0), prazo TEXT NOT NULL /* YYYY-MM */, criado_em, atualizado_em)`; `ALTER TABLE investimentos ADD COLUMN meta_id INTEGER REFERENCES metas(id) ON DELETE SET NULL`.
  - `Db::registrar_perfil(&self, respostas: &[u8], hoje: NaiveDate) -> Result<Result<PerfilGravado, ErroCampo>, CoreError>` — calcula, grava o perfil e, **se ainda não existe alocação-alvo**, grava a sugerida.
  - `pub struct PerfilGravado { pub perfil: Perfil, pub avisos: Vec<String>, pub feito_em: String, pub vencido: bool, pub alocacao_sugerida: Vec<(Classe, u8)> }`; `Db::perfil_atual(&self, hoje) -> Result<Option<PerfilGravado>, CoreError>` (`vencido` = feito há mais de 24 meses).
  - `Db::alocacao_alvo(&self) -> Result<Option<Vec<(Classe, u8)>>, CoreError>` (cinco classes na ordem de `TODOS`, ou `None` se nunca definida).
  - `Db::salvar_alocacao_alvo(&self, valores: &[(String, i64)]) -> Result<Vec<ErroCampo>, CoreError>` — exige as cinco classes, cada uma inteiro de 0 a 100, soma 100; erros: por classe (`campo` = código da classe) "Use um número inteiro de 0 a 100."; geral (`campo` = "alocacao") "A soma precisa dar 100%. Hoje dá N%."; lista vazia quando gravou; nada é gravado se houver erro.
  - `Db::objetivo(&self) -> Result<Option<String>, CoreError>`, `Db::salvar_objetivo(&self, texto: &str) -> Result<Vec<ErroCampo>, CoreError>` (até 2000 caracteres após `trim`: "Use até 2.000 caracteres."; texto vazio apaga o objetivo).

- [ ] **Step 1: Testes que falham** — registrar perfil grava e `perfil_atual` devolve; o segundo registro vira o atual; registrar a primeira vez cria a alocação sugerida e a segunda vez **não** sobrescreve uma alocação editada; `vencido` com `feito_em` a 24 meses e 1 dia; alocação: aceita soma 100, recusa 99, 101, `-1`, 101 numa classe e classe faltando, com as mensagens exatas e sem gravar; objetivo: salva, lê, apaga com texto vazio, recusa 2001 caracteres; `migra_banco_da_versao_2` — banco com a 0002 e dois investimentos abre na versão 3 com os dois investimentos e `meta_id` nulo.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** (workspace, clippy, fmt; addon `npm run build && npm test`; e2e só o teste da versão do esquema com `-g`).
- [ ] **Step 5: Commit** — "Migração 0003 e repositório de perfil, alocação-alvo e objetivo".

---

### Tarefa 3: Metas, vínculo com investimentos e resumo contra o alvo

**Files:**
- Create: `crates/guru-core/src/metas/mod.rs`, `modelo.rs`, `validacao.rs`, `progresso.rs`, `repositorio.rs`
- Modify: `crates/guru-core/src/investimento/{modelo,validacao,repositorio}.rs` (campo `meta_id`), `crates/guru-core/src/lib.rs`
- Test: unidade em `progresso.rs` e `validacao.rs`; `crates/guru-core/tests/metas.rs`; ajustar `tests/investimentos.rs`

**Interfaces:**
- Consumes: Tarefa 2 (tabela `metas`, coluna `meta_id`), `MAX_CENTAVOS`.
- Produces:
  - `pub struct NovaMeta { pub nome: String, pub valor_alvo_centavos: Option<i64>, pub prazo: Option<String> }`
  - `pub enum Situacao { Concluida, Vencida, SemAportes, Atrasada, NoCaminho }` (códigos `concluida`, `vencida`, `sem_aportes`, `atrasada`, `no_caminho`)
  - `pub struct Meta { pub id: i64, pub nome: String, pub valor_alvo_centavos: i64, pub prazo: String, pub acumulado_centavos: i64, pub percentual_decimos: i64 /* 0..=1000, teto 1000 */, pub meses_restantes: i64, pub aporte_necessario_centavos: Option<i64>, pub ritmo_centavos: Option<i64>, pub situacao: Situacao, pub mensagem: String, pub investimentos: i64 }`
  - `pub fn progresso(alvo: i64, prazo: (i32, u32), vinculados: &[(i64, NaiveDate)], hoje: NaiveDate) -> (…campos calculados…)` — função pura com as regras da seção Metas; as mensagens usam o formato de reais `R$ 1.234,56` (adicione um `formatar_reais(centavos) -> String` no núcleo, com teste).
  - `Db::criar_meta(&NovaMeta, hoje) -> Result<Result<i64, Vec<ErroCampo>>, CoreError>`, `Db::atualizar_meta(id, &NovaMeta, hoje)` (mesmo formato; `NaoEncontrado`), `Db::apagar_meta(id)`, `Db::listar_metas(hoje) -> Vec<Meta>` (ordem: prazo, depois nome), `Db::meta(id, hoje) -> Meta`.
  - `NovoInvestimento.meta_id: Option<i64>` e `Investimento.meta_id: Option<i64>`, `Investimento.meta_nome: Option<String>`; id inexistente → `ErroCampo { campo: "metaId", mensagem: "Escolha uma meta da lista." }`.
  - `ResumoClasse` ganha `percentual_decimos: i64` (do total, `round(parte × 1000 / total)` em inteiros, meio para cima) e `alvo_percentual: Option<i64>`; `Resumo.classes` passa a incluir classes sem investimento que tenham alvo > 0 (com total 0), sempre na ordem de `Classe::TODOS`.

- [ ] **Step 1: Testes que falham**
  - `progresso`: prazo no mês atual (meses 1, aporte = falta inteira); prazo passado (vencida, sem aporte, mensagem com o valor que falta); prazo em 5 anos sem vínculos (sem_aportes); acumulado ≥ alvo (concluida, 100,0%); atrasada com o N exato (alvo 15.000.000, acumulado 4.120.000, primeira aplicação 24 meses antes do mês atual → 25 meses, meses restantes 50 → ritmo 164.800, necessário 217.600 → "No ritmo atual (R$ 1.648,00/mês), chega 17 meses depois."); no_caminho.
  - validação: nome vazio, 61 caracteres, valor 0, prazo `2026-09` com hoje `2026-10-29` → mensagens exatas; prazo passado aceito ao **editar**.
  - repositório: criar, listar com acumulado dos vinculados, editar, apagar desvincula (o investimento continua e `meta_id` fica `None`), `NAO_ENCONTRADO`.
  - investimento com `metaId` inexistente → erro de campo; com meta válida → `meta_nome` preenchido.
  - resumo: com alvo definido, classe sem investimento e alvo 10 aparece com total 0 e `alvo_percentual = Some(10)`; percentuais 88,97% → `890`.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** (workspace, clippy, fmt; addon `npm run build && npm test`).
- [ ] **Step 5: Commit** — "Metas, vínculo com investimentos e resumo contra o alvo".

---

### Tarefa 4: Addon, canais IPC e `api` para perfil, alocação, objetivo e metas

**Files:**
- Create: `crates/guru-node/src/perfil.rs`, `crates/guru-node/src/metas.rs`
- Modify: `crates/guru-node/src/lib.rs`, `crates/guru-node/src/investimentos.rs` (`metaId`, `metaNome`; `ResumoClasse` com `percentualDecimos`, `alvoPercentual`), `app/main.js`, `app/preload.js`, `app/renderer/js/api.js`
- Test: `crates/guru-node/test/perfil.test.mjs`, `crates/guru-node/test/metas.test.mjs`, `app/test/unit/api.test.mjs`, `app/test/e2e/abrir.spec.js` (superfície)

**Interfaces:**
- Consumes: Tarefas 1-3.
- Produces (addon, camelCase):
  - `core.questionario(): { perguntas: { numero, bloco, texto, porque, opcoes: string[] }[], avisosFixos: string[] }` — **sem os pontos**.
  - `core.registrarPerfil(respostas: number[]): { perfil?: PerfilJs, erros: ErroCampo[] }`; `core.perfilAtual(): PerfilJs | null`, com `PerfilJs = { perfil, perfilRotulo, descricao, avisos: string[], feitoEm, vencido, alocacaoSugerida: { classe, rotulo, percentual }[] }`.
  - `core.alocacaoAlvo(): { classe, rotulo, percentual }[] | null`; `core.salvarAlocacaoAlvo(valores: { classe: string, percentual: number }[]): ErroCampo[]`.
  - `core.objetivo(): string | null`; `core.salvarObjetivo(texto: string): ErroCampo[]`.
  - `core.listarMetas(): MetaJs[]`, `core.meta(id)`, `core.criarMeta(dados)`, `core.atualizarMeta(id, dados)` → `{ id?, erros }`, `core.apagarMeta(id)`; `MetaJs` espelha `Meta` com `situacao` em código.
- Produces (IPC e `api`), cada canal um `ipcRenderer.invoke`:
  - `api.perfil = { questionario, registrar, atual }` — canais `perfil:questionario`, `perfil:registrar`, `perfil:atual`.
  - `api.alocacao = { ler, salvar }` — `alocacao:ler`, `alocacao:salvar`.
  - `api.objetivo = { ler, salvar }` — `objetivo:ler`, `objetivo:salvar`.
  - `api.metas = { listar, ler, criar, atualizar, apagar }` — `metas:listar`, `metas:ler`, `metas:criar`, `metas:atualizar`, `metas:apagar`.
  - O teste de superfície passa a esperar `Object.keys(window.guru)` = `["app", "investimentos", "carteira", "perfil", "alocacao", "objetivo", "metas"]`.

- [ ] **Step 1: Testes que falham** — addon: `questionario` tem 12 perguntas, nenhuma opção traz pontos (`typeof opcoes[0] === "string"`); `registrarPerfil` com doze 0 → Conservador e alocação sugerida soma 100; com 11 respostas → `erros[0].campo === "respostas"`; `salvarAlocacaoAlvo` com soma 99 → erro `alocacao`; metas: criar, listar com `situacao: "sem_aportes"`, vincular um CDB e ver `acumuladoCentavos`; e2e: superfície e `api.metas.apagar(999)` → `NAO_ENCONTRADO`.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar** seguindo o padrão de `investimentos.rs` e da tabela `canais` em `main.js`.
- [ ] **Step 4: Rodar e ver passar** — Rust, addon, `cd app && npm test` e os e2e de `abrir.spec.js`.
- [ ] **Step 5: Commit** — "Perfil, alocação, objetivo e metas no addon e na ponte".

---

### Tarefa 5: Telas do questionário e do resultado

**Files:**
- Create: `app/renderer/js/telas/perfil/questionario.js`, `app/renderer/js/telas/perfil/resultado.js`
- Modify: `app/renderer/js/telas/perfil.js` (despacha por `parametros`), `app/renderer/css/telas.css`
- Test: `app/test/e2e/questionario.spec.js`

**Interfaces:**
- Consumes: `api.perfil.questionario`, `api.perfil.registrar`.
- Produces: `#/perfil/teste` e `#/perfil/resultado`.

Comportamento (protótipo `#perfil`, linhas 413-453):
- `#/perfil/teste`: linha "{bloco} · pergunta N de 12" e "Método ANBIMA"; barra de progresso inteira (`N-1` de 12 respondidas); card da pergunta com as 4 opções como botões de escolha única (`role="radio"` dentro de `role="radiogroup"`, setas do teclado funcionam); "Voltar" (desabilitado na 1ª) e "Próxima pergunta" (desabilitado sem escolha; na 12ª vira "Ver meu perfil"); card lateral "Por que perguntamos isso" com o `porque` e os dois avisos fixos.
- As respostas ficam só na memória da tela; voltar mostra a escolha anterior marcada e permite trocar. Sair da tela no meio descarta tudo (nada é gravado).
- "Ver meu perfil" chama `api.perfil.registrar(respostas)` e vai para `#/perfil/resultado`.
- `#/perfil/resultado` (lê `api.perfil.atual()`): título "Seu perfil: {Rótulo}", a descrição, os avisos (cada um num bloco de alerta), a alocação sugerida por classe, e os botões "Usar esta alocação" (salva a sugerida com `api.alocacao.salvar` e volta a `#/perfil`) e "Ajustar a alocação" (vai a `#/perfil` com o editor aberto: `#/perfil/alocacao`). Rodapé: "Questionário baseado nas dimensões das regras de suitability da ANBIMA: objetivos, situação financeira e conhecimento. Não é recomendação de investimento."
- Sem perfil gravado, `#/perfil/resultado` mostra "Você ainda não fez o teste." e o link para `#/perfil/teste`.

- [ ] **Step 1: Testes e2e que falham**
  - `não mostra pontuação nem perfil durante o teste` — percorre as 12 perguntas e, a cada uma, confirma que a página não contém "Conservador", "Moderado", "Arrojado", "pontos" nem "%" (fora do card lateral, que não tem números).
  - `voltar mantém e troca a resposta` — responde 1 e 2, volta, a 2ª opção escolhida continua marcada, troca, segue até o fim; o resultado reflete a troca (escolha respostas que mudam o perfil).
  - `todas as primeiras opções dão Conservador` com a alocação 85/5/5/5/0 na tela.
  - `prazo curto mostra o aviso da trava`.
  - `sair no meio não grava` — responde 5, vai para `#/carteira`, volta a `#/perfil/resultado` → "Você ainda não fez o teste."
  - `usar esta alocação salva e volta ao perfil`.
  - `teclado escolhe a opção` — foco no grupo, seta para baixo, Enter avança.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar.**
- [ ] **Step 5: Commit** — "Questionário de perfil sem prévia e tela de resultado".

---

### Tarefa 6: Tela Perfil e objetivo com alocação-alvo

**Files:**
- Create: `app/renderer/js/telas/perfil/painel.js`, `app/renderer/js/telas/perfil/alocacao.js`
- Modify: `app/renderer/js/telas/perfil.js`, `app/renderer/css/telas.css`
- Test: `app/test/e2e/perfil.spec.js`

**Interfaces:**
- Consumes: `api.perfil.atual`, `api.alocacao.ler/salvar`, `api.objetivo.ler/salvar`.
- Produces: `#/perfil` e `#/perfil/alocacao` (o painel com o editor aberto).

Comportamento (protótipo `#objetivo`, linhas 456-508):
- Cabeçalho "Perfil e objetivo" com a pílula "Perfil {rótulo} · feito em dd/mm/aaaa" e o link "Refazer o teste" (`#/perfil/teste`). Sem perfil: card "Descubra seu perfil de investidor" com o texto "São 12 perguntas, leva uns 3 minutos. O resultado só aparece no fim." e o botão "Fazer o teste". Perfil vencido: faixa "Seu perfil tem mais de 2 anos. Refaça o teste."
- Card "Seu objetivo": título "Por que você está investindo? O que seria sucesso para você?", dica "Escreva com suas palavras. A IA vai usar este texto nas análises.", `textarea` (contador "N/2000"), três exemplos clicáveis que inserem o texto ("Quero construir um patrimônio de R$ 50.000 investido", "Quero manter o patrimônio que já tenho", "Quero viver de renda a partir dos 55") e o botão "Salvar objetivo" com a confirmação "Objetivo salvo.".
- Card "Alocação-alvo": tabela com cada classe e o percentual alvo; botão "Ajustar". No modo de edição (`#/perfil/alocacao`): um campo inteiro por classe (`inputmode="numeric"`, texto, só dígitos), a soma ao vivo "Soma: N%" (vira alerta quando ≠ 100), "Usar a sugestão do perfil" (preenche com `alocacaoSugerida`, só aparece se houver perfil), "Salvar" e "Cancelar". Erros do núcleo marcam o campo da classe ou a soma. Sem alocação e sem perfil: "Faça o teste de perfil para receber uma sugestão, ou defina a sua." com o botão "Definir".
- O card de metas fica para a Tarefa 7; aqui, o painel reserva a coluna da direita com o título "Metas".
- Rodapé com o texto legal da Tarefa 5.

- [ ] **Step 1: Testes e2e que falham** — sem perfil mostra o convite e o botão leva ao teste; com perfil mostra a pílula com a data de hoje formatada; objetivo: digitar, salvar, reabrir o app e o texto está lá; exemplo clicado entra no texto; alocação: ajustar para 60/20/10/10/0 salva e mostra; soma 99 mostra "Soma: 99%" em alerta e o "Salvar" do núcleo devolve "A soma precisa dar 100%. Hoje dá 99%." sem gravar; "12,5" num campo é recusado no campo antes de enviar; "Usar a sugestão do perfil" preenche os valores do perfil.
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar.**
- [ ] **Step 5: Commit** — "Tela de perfil, objetivo e alocação-alvo".

---

### Tarefa 7: Metas na tela, vínculo no formulário e carteira contra o alvo

**Files:**
- Create: `app/renderer/js/telas/perfil/metas.js` (cards e formulário de meta)
- Modify: `app/renderer/js/telas/perfil/painel.js`, `app/renderer/js/telas/perfil.js`, `app/renderer/js/telas/formulario-investimento.js` e `formulario-investimento/dados.js` (campo "Vincular a uma meta"), `app/renderer/js/telas/carteira.js`, `app/renderer/css/telas.css`, `docs/mvp.md`
- Test: `app/test/e2e/metas.spec.js`, ajustes em `formulario.spec.js` e `carteira.spec.js`

**Interfaces:**
- Consumes: `api.metas.*`, `api.investimentos.*`, `api.carteira.resumo` (com `percentualDecimos` e `alvoPercentual`).
- Produces: cards de meta em `#/perfil`, `#/perfil/metas/nova`, `#/perfil/metas/<id>`.

Comportamento:
- Coluna "Metas" do painel: um card por meta com nome, prazo ("dez/2030"), "R$ acumulado / R$ alvo", barra de progresso inteira (largura pela classe CSS em passos de 5%, sem `style=`), "aporte necessário R$ X/mês" quando existe, e a mensagem da situação (verde para `no_caminho` e `concluida`, alerta para `atrasada` e `vencida`, neutra para `sem_aportes`); cada card é link para editar. Por último, o card tracejado "+ Nova meta". Sem metas: só o card "+ Nova meta" e o texto "Metas ajudam a acompanhar objetivos com valor e prazo, como a entrada de um imóvel."
- Formulário de meta: Nome, Valor da meta (reais, `paraCentavos`), Prazo (`input type="month"`); "Salvar", "Cancelar" e, ao editar, "Apagar meta" com diálogo "Apagar a meta {nome}? Os investimentos vinculados continuam na carteira."
- Formulário de investimento: novo campo "Vincular a uma meta" (`select` com "Nenhuma" e as metas por nome) na seção "Valores e condições", enviado como `metaId` (ausente quando "Nenhuma"); ao editar, mostra a meta vinculada.
- Carteira (`#/carteira`): com alocação-alvo definida, cada card mostra "alvo N%" e a diferença em pontos com sinal e uma casa ("+7,0 p.p." acima, "−8,0 p.p." abaixo, "no alvo" se |dif| < 0,5), usando `percentualDecimos` do núcleo (o cálculo de percentual no JS da fase 1 sai). Classe com alvo e sem investimento aparece com R$ 0,00.
- Docs: em `mvp.md`, nas Decisões, linha "Perfil" atualizada para "Questionário próprio de 12 perguntas nas dimensões da ANBIMA (objetivos, situação financeira, conhecimento), sem prévia do resultado, com travas por prazo e reserva e avisos de inconsistência. Três perfis: Conservador, Moderado e Arrojado."

- [ ] **Step 1: Testes e2e que falham** — criar meta e ver o card com "Ainda sem investimentos vinculados."; vincular um CDB de R$ 20.000,00 a uma meta de R$ 150.000,00 e ver "R$ 20.000,00 / R$ 150.000,00" e o aporte necessário; prazo no passado ao criar mostra "O prazo precisa ser este mês ou depois."; apagar meta com investimento vinculado: o investimento continua na lista e o formulário dele abre com "Nenhuma"; carteira com alvo 45/30/15/10/0 e investimentos mostra "alvo 45%" e a diferença com sinal; classe com alvo e sem investimento aparece com "R$ 0,00".
- [ ] **Step 2: Rodar e ver falhar.**
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Rodar e ver passar** — suíte inteira (Rust, addon, unitários e e2e) uma vez.
- [ ] **Step 5: Commit** — "Metas, vínculo nos investimentos e carteira contra o alvo".
