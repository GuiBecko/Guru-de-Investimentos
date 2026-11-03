# Fase 6b: IA no resto do app — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** No Próximo aporte, o usuário diz quanto vai aplicar, para onde quer levar o dinheiro (renda fixa, ações, experimentar algo novo, em texto livre também) e, se quiser, para qual meta. Cada classe da divisão abre um chat com a IA que já sabe tudo isso e o ajuda a escolher o investimento dentro dela. No painel da Carteira, um clique gera os insights da IA sobre a carteira e outro busca na web as notícias ligadas aos ativos, cada uma com link. Nada disso chama a IA sozinho: cada chamada gasta o dinheiro do usuário.

**Architecture:**
- **Núcleo:** o módulo `ia` da 6a ganha três peças:
  - o foco do Próximo aporte: um bloco a mais no contexto congelado da conversa e a coluna `conversa.foco`;
  - os insights, pedidos com saída estruturada (`output_config.format` com JSON Schema) e validados antes de gravar;
  - as notícias, pedidas com a busca na web e montadas só a partir das fontes que a resposta citou.
  - O envio com `pause_turn` sai de `perguntar` para uma função comum, que também passa a somar no `ia_uso` os tokens das respostas que falham ou são paradas depois do `message_start`.
- **Addon:** a thread com pedido e progresso da 6a vira genérica e serve à pergunta, à pergunta do aporte, aos insights e às notícias. A tela acompanha tudo pelo mesmo `iaProgresso`.
- **Interface:**
  - o chat da tela Assistente sai para um componente que a gaveta do Próximo aporte reusa;
  - a Carteira ganha a coluna de insights e notícias do protótipo.

**Tech Stack:** o mesmo da 6a, sem dependência nova.

**Spec:**
- [`docs/mvp.md`](../../mvp.md):
  - telas 5 ("Insights da IA e notícias com link") e 7 ("Cada classe abre um chat com a IA focado em escolher o investimento dentro dela");
  - as decisões "IA", "Dados de mercado" ("Busca na web da IA só para notícias") e "Próximo aporte".
- [`docs/roadmap.md`](../../roadmap.md): fase 6b.
- Protótipo:
  - `docs/prototipo/index.html:727-750`: coluna de insights e notícias da tela 5;
  - `docs/prototipo/index.html:800-845`: tela 7 com a gaveta de chat;
  - `docs/prototipo/index.html:939`: "Mercado … 3 notícias" no editor de contexto.
- O pedido do usuário para o Próximo aporte: um seletor do valor e da direção geral do dinheiro (renda fixa, ações, experimentar um investimento novo etc.). Todas essas escolhas vão para o contexto do chat daquela página, e a IA guia e tira dúvidas sobre o investimento.
- Plano da 6a, seção "Fora desta fase", e a pendência da revisão final: contar os tokens das respostas que falham ou são paradas.

## Decisões desta fase

- **Nada chama a IA sozinho:**
  - abrir a Carteira ou o Próximo aporte, ou dividir um valor, não chama a API;
  - os insights saem do botão "Gerar insights" e as notícias do botão "Buscar notícias";
  - os dois resultados ficam no SQLite com a data e aparecem até a próxima geração;
  - os botões são separados porque só as notícias usam a busca na web, que custa à parte.
- **Insights com saída estruturada e sem busca:** a API recusa (400) saída estruturada junto com citações, e a busca na web sempre cita. Por isso os insights usam só os dados do contexto, e o tipo "Mercado" fala dos dados de mercado e das notícias já guardadas.
- **Notícias sem saída estruturada:**
  - a resposta vem em linhas `- <ativo> | <título>`;
  - cada notícia só existe se a linha citou uma fonte da busca;
  - link, domínio e data vêm da citação, nunca do texto do modelo, para que o link seja sempre uma página que a busca devolveu de verdade.
- **Contexto das notícias reduzido:** vai só a lista de ativos (tickers e tipos de renda fixa com o indexador), sem nenhum valor e sem nome, mais as instruções do usuário.
  - Sem "Nome das instituições", nenhuma instituição ou emissor aparece.
  - Com a parte "Carteira" desligada, vão só as classes presentes, sem tickers.
- **Notícias no contexto:** a parte "Mercado" passa a levar as 3 notícias guardadas mais recentes (título, domínio e data), como no protótipo.
- **Chat por classe:**
  - nenhuma mensagem automática: a gaveta abre com três sugestões, e só o clique numa delas ou uma pergunta digitada chama a API;
  - toda classe da divisão tem o botão, inclusive a que recebe R$ 0, porque a direção do usuário pode apontar para ela;
  - a conversa nasce na primeira pergunta, com o contexto congelado como na 6a, e grava `foco` = código da classe;
  - ela aparece também na lista do Assistente, com a etiqueta "Próximo aporte · <classe>", e pode continuar lá.
- **Direção e meta só informam:** não mudam a divisão, que continua sendo o cálculo da fase 5. Não são gravadas fora do contexto da conversa, porque o histórico de decisões de aporte é da V2.
- **Valor do aporte em reais:** o valor do aporte e o da classe em foco vão em reais mesmo com "Apenas percentuais", porque são o assunto da conversa (aplicação mínima, número de cotas). O resto da carteira segue em percentuais, e a linha "Contexto: aporte de R$ …" no topo da gaveta deixa visível ao usuário que esse valor vai para a IA.
  - O bloco não leva o total da carteira, o total de nenhuma classe nem os percentuais "depois do aporte".
  - A divisão completa vai em percentual do aporte.
- **Tokens perdidos contam:** depois do `message_start`, os tokens da resposta entram no `ia_uso` mesmo quando ela falha, é parada pelo usuário ou abandonada porque a tela saiu de cena.
  - Nenhuma mensagem fica gravada nesses casos.
  - Um app fechado no meio da resposta perde a conta daquela resposta.
- **Sair da Carteira não para a geração** de insights e notícias, para não jogar fora o que já foi pago. Voltar à tela retoma o acompanhamento, e o botão "Parar" para de verdade.
- **"Registrar aporte" da gaveta** é só o link para o formulário de lançamento (`#/investimentos/novo`), sem preencher nada.

## Global Constraints

- **Tudo da 6a continua valendo:**
  - CSP;
  - nada de `innerHTML` com dados nem `style=`;
  - `vigente()` depois de cada await;
  - pt-BR com acentos e erros em SCREAMING_SNAKE_CASE;
  - preload sem lógica e IPC com checagem de origem;
  - dinheiro em centavos;
  - a chave nunca sai do cofre e os testes nunca usam a rede;
  - só os dois destinos exatos da Anthropic, com os cabeçalhos, prazos e limites;
  - os três modelos, o texto `SISTEMA` e as regras do corpo (sem `temperature`, `top_p`, `top_k` nem `thinking`; `fallbacks` e o beta só em Opus e Sonnet);
  - e2e com a janela escondida via `lancar()`.
- **Commits** em português, sem `Co-Authored-By` e sem nenhuma menção ao Claude.
- **Trabalho** na branch `fase-6b-ia-no-app`, criada a partir da `main` depois do merge da 6a.
- **Corpo dos insights:** é o corpo da 6a, com estas diferenças:
  - `system` igual: `SISTEMA` e o `contexto_montado` com `cache_control`;
  - `messages`: uma mensagem só do usuário, com o texto `PEDIDO_INSIGHTS`;
  - sem `tools`, mesmo com "Buscar na web" ligada;
  - `output_config`: `{"effort":"medium","format":{"type":"json_schema","schema": SCHEMA_INSIGHTS}}`.
- **`SCHEMA_INSIGHTS`:**
  ```json
  {"type":"object","additionalProperties":false,"required":["insights"],
   "properties":{"insights":{"type":"array","items":{"type":"object","additionalProperties":false,
     "required":["tipo","texto","acao","classe"],
     "properties":{
       "tipo":{"type":"string","enum":["rebalanceamento","meta","vencimento","concentracao","mercado"]},
       "texto":{"type":"string"},
       "acao":{"type":"string","enum":["aporte","meta","classe","conversar","nenhuma"]},
       "classe":{"anyOf":[{"type":"string","enum":["rf","acoes","fii","ext","outros"]},{"type":"null"}]}}}}}}
  ```
- **`PEDIDO_INSIGHTS`** (fixo):
  > Analise o contexto e escreva de 3 a 5 insights curtos sobre a carteira, em português do Brasil, cada um com no máximo 300 caracteres. Use os tipos rebalanceamento (desvio da alocação-alvo), meta (ritmo ou atraso das metas), vencimento (o que vence nos próximos 12 meses), concentracao (peso alto num ativo, setor ou instituição) e mercado (o que os dados de mercado e as notícias do contexto significam para a carteira). Para cada um, escolha a ação que leva ao lugar certo do app: aporte, meta, classe (com o código da classe), conversar ou nenhuma. Apresente fatos e opções, sem recomendar a compra ou a venda de um ativo específico. Use só os dados do contexto.
- **Validação dos insights:** o JSON é a concatenação dos blocos `text` da resposta.
  - Vira `IA_RESPOSTA_INVALIDA` quando:
    - não é JSON;
    - falta um campo;
    - um `enum` é desconhecido;
    - a lista vem vazia;
    - um `texto` vem vazio depois do `trim` ou passa de 500 caracteres;
    - `acao` é `classe` com `classe` nula;
    - o `stop_reason` é `max_tokens`.
  - Mais de 5 insights: ficam os 5 primeiros.
  - Em qualquer erro, os insights anteriores continuam gravados e na tela.
- **Corpo das notícias:** `corpo_pedido(modelo, contexto_noticias, true, [PEDIDO_NOTICIAS])`.
  - A busca na web vai sempre, com o objeto da 6a.
  - Não leva `format`.
  - Com "Buscar na web" desligada, nada é enviado e o pedido falha com `IA_BUSCA_DESLIGADA`.
- **`PEDIDO_NOTICIAS`** (fixo):
  > Busque na web notícias dos últimos 7 dias sobre os ativos listados no contexto e sobre os indicadores que os afetam (Selic, IPCA, câmbio). Escolha até 8 notícias relevantes, de veículos jornalísticos ou fontes oficiais. Responda só com uma linha por notícia, no formato `- <ativo ou classe> | <título em português, com até 100 caracteres>`, citando a matéria em cada linha. Não escreva mais nada.
- **Leitura das notícias** (`ler_noticias`):
  - os blocos `text` são percorridos em ordem e o texto é cortado nas quebras de linha;
  - cada linha guarda a URL da primeira citação `web_search_result_location` dos blocos que terminam nela;
  - uma linha vira notícia quando casa `^-\s*(.+?)\s*\|\s*(.+)$` e tem uma URL citada que começa com `https://`;
  - o título é o segundo grupo, sem marcadores `[n]`, cortado em 100 caracteres;
  - a etiqueta é o primeiro grupo, se for um ticker da carteira ou o rótulo de uma classe (sem diferença de maiúsculas); se não, "Mercado";
  - URL, domínio e data (o `page_age`) vêm da `Fonte` com a mesma URL;
  - uma URL repetida conta só na primeira vez, e ficam no máximo 8 notícias;
  - nenhuma linha válida não é erro: grava a lista vazia com a data.
- **Novos erros:**

  | Situação | Código | Mensagem |
  |---|---|---|
  | insights fora do formato | `IA_RESPOSTA_INVALIDA` | "A IA devolveu os insights num formato inesperado. Tente de novo." |
  | notícias com a busca desligada | `IA_BUSCA_DESLIGADA` | "Ligue “Buscar na web” em Configurar IA para buscar notícias." |
  | foco do aporte inválido | `IA_FOCO_INVALIDO` | a mensagem do campo (ver Task 1) |

- **Direções do aporte**, nesta ordem, com código e rótulo:
  - `renda_fixa`, "Renda fixa";
  - `acoes`, "Ações";
  - `fiis`, "FIIs";
  - `exterior`, "Exterior";
  - `novo`, "Experimentar algo novo";
  - `seguranca`, "Mais segurança".
  - Mais o texto livre "Outra direção ou detalhe (opcional)", com até 300 caracteres.
- **Bloco "Próximo aporte" do contexto**, inserido antes da linha "Hoje é", com o exemplo de Ações, direções e meta:
  ```
  ## Próximo aporte
  Valor deste aporte: R$ 2.000,00.
  Classe em foco: Ações, com R$ 1.100,00 deste aporte pela divisão de rebalanceamento (alvo 30%, hoje 8,0 p.p. abaixo do alvo).
  Divisão do aporte: Renda fixa 0,0%; Ações 55,0%; FIIs 15,0%; Exterior 30,0%.
  Direção que o usuário quer dar a este dinheiro: Ações; Experimentar algo novo. Nas palavras dele: "quero algo com liquidez".
  Meta ligada a este aporte: Entrada do apartamento · prazo 12/2030 · 27,5% atingido.
  O usuário quer ajuda para escolher em que investir dentro de Ações com este valor: apresente opções, com prós e contras, e tire as dúvidas dele.
  ```
  - **Situação da classe:**
    - "hoje X p.p. abaixo do alvo";
    - "hoje perto do alvo";
    - "hoje X p.p. acima do alvo";
    - "fora da alocação-alvo".
  - **Divisão:** inclui só as classes da divisão da fase 5, em percentual do aporte, em décimos.
  - **Sem direção:** "Direção que o usuário quer dar a este dinheiro: não indicada.".
  - **Sem texto livre:** a frase "Nas palavras dele" não aparece.
  - **Sem meta:** "Meta ligada a este aporte: nenhuma.".
  - **Linha da meta:** sai da mesma função da parte "Metas" e respeita "Apenas percentuais".
- **Notícias na parte "Mercado":** com notícias guardadas, a parte termina com estas linhas:
  ```
  Notícias da carteira (buscadas em dd/mm/aaaa):
  - <etiqueta>: <título> (<domínio>, <data>)
  ```
  - São as 3 primeiras notícias guardadas.
  - O resumo vira "automático · N notícias", com N de 1 a 3.
- **Textos fixos da interface:**
  - **Carteira sem chave:** "Para ver insights e notícias da sua carteira, configure a sua chave.", com o link "Configurar IA".
  - **Próximo aporte sem chave:** "Para conversar com a IA sobre cada classe, configure a sua chave.", com o link "Configurar IA".
  - **Nota de custo**, sob os botões da Carteira: "Cada geração usa a sua chave da Anthropic.".
  - **Aviso da coluna de insights:** "Gerado por IA com os seus dados. Não é recomendação individual de investimento.".
  - **Sugestões da gaveta:**
    - "Quais investimentos de <classe> combinam com a direção que escolhi?";
    - "Compare duas opções para esse valor, com prós e contras.";
    - "Quais custos, impostos e riscos devo olhar nesta classe?".

## Review Focus

1. **Custo só com clique:** abrir a Carteira, abrir o Próximo aporte, dividir e abrir a gaveta não chamam a API. Teste (Task 4 e Task 5): depois de tudo isso, `api.ia.config().uso.tokens` continua 0.
2. **Link de notícia só citado e `https`:** nenhuma notícia sai de uma linha sem citação, de uma URL `http:` ou de uma URL escrita pelo modelo. Teste (Task 2, `noticias_so_com_link_https_citado`): o fixture tem uma linha sem citação e uma citando uma URL `http://`, e as duas ficam de fora.
3. **Privacidade nos pedidos novos:** com as chaves padrão, o corpo das notícias não leva `R$`, instituição, emissor nem o `nome` do investimento. Com "Carteira" desligada, não leva tickers. O bloco do aporte não leva o total da carteira nem o de nenhuma classe. Testes (Task 1, `aporte_nao_leva_valores_da_carteira`; Task 2, `contexto_noticias_respeita_privacidade`).
4. **Saída inválida não quebra nem apaga:** o JSON fora do esquema vira `IA_RESPOSTA_INVALIDA`, e os insights anteriores continuam. Teste (Task 2, `insights_invalidos_mantem_os_anteriores`; Task 5, o e2e com `#insights-invalido`).
5. **Tokens de resposta perdida contam, mensagem não:** falha ou parada depois do `message_start` soma no `ia_uso` sem gravar mensagem. Um erro HTTP antes dele não soma nada. Teste (Task 1: `uso_conta_resposta_que_falha_depois_do_message_start`, `uso_conta_resposta_cancelada` e `erro_antes_do_message_start_nao_conta`).

---

### Task 1: Núcleo — tokens perdidos, foco do aporte e conversa por classe

**Files:**
- Create: `crates/guru-core/src/migrations/0007_ia_app.sql`, `crates/guru-core/src/ia/aporte.rs`
- Modify:
  - `crates/guru-core/src/ia/conversa.rs` (função comum de envio, `somar_uso`, `foco`);
  - `crates/guru-core/src/ia/contexto.rs` (a linha da meta vira `pub(crate)`);
  - `crates/guru-core/src/ia/mod.rs`, `crates/guru-core/src/lib.rs`;
  - `crates/guru-core/src/error.rs` (`FocoInvalido`);
  - `crates/guru-core/src/db.rs` (migração 7) e `crates/guru-core/tests/db.rs` (versão 7).
- Test: `crates/guru-core/tests/ia_aporte.rs`, `crates/guru-core/tests/ia_conversa.rs` (os três testes de uso)

**Interfaces:**
- Consumes: 6a (`perguntar`, `contexto_montado`, `Acumulador`, `Uso`, `ClienteFixture`) e fase 5 (`Db::proximo_aporte`, `Situacao`).
- **Schema (`0007_ia_app.sql`):**
  ```sql
  ALTER TABLE conversa ADD COLUMN foco TEXT;  -- NULL: Assistente; código da classe: chat do Próximo aporte
  CREATE TABLE ia_gerado (tipo TEXT PRIMARY KEY CHECK (tipo IN ('insights', 'noticias')),
    gerado_em TEXT NOT NULL, modelo TEXT NOT NULL, itens_json TEXT NOT NULL);
  ```
  A tabela `ia_gerado` é usada na Task 2; vem aqui para a migração ser uma só.
- **Produces:**
  ```rust
  pub const DIRECOES: [(&str, &str); 6];   // (código, rótulo) na ordem das Global Constraints
  pub struct FocoAporte { pub classe: String, pub valor_centavos: i64, pub direcoes: Vec<String>,
                          pub direcao_livre: String, pub meta_id: Option<i64> }
  impl Db {
      /// contexto_montado + o bloco "## Próximo aporte" antes de "Hoje é", e a fonte interna
      /// { parte: "aporte", rotulo: "Próximo aporte", data_dados: None, antigo: false } no fim.
      pub fn contexto_aporte(&self, hoje: NaiveDate, foco: &FocoAporte) -> Result<ContextoMontado, CoreError>;
  }
  /// Sempre cria a conversa (foco = classe); as perguntas seguintes usam `perguntar` com o id.
  #[allow(clippy::too_many_arguments)]
  pub fn perguntar_aporte(db: &Db, cofre: &dyn Cofre, cliente: &dyn ClienteIa, hoje: NaiveDate,
                          foco: &FocoAporte, pergunta: &str, cancelar: &AtomicBool,
                          progresso: &mut dyn FnMut(Progresso)) -> Result<i64, CoreError>;
  // ConversaResumo e Conversa ganham `pub foco: Option<Classe>`.
  // CoreError::FocoInvalido(String) -> código "IA_FOCO_INVALIDO", mensagem = o texto.
  pub(crate) fn somar_uso(conn: &rusqlite::Connection, mes: &str, modelo: &str, uso: Uso) -> Result<(), CoreError>;
  /// Envia, trata pause_turn (até 3) e devolve o acumulador com blocos. Recusa sem blocos vira
  /// IA_RECUSA e resposta sem blocos vira IA_INDISPONIVEL, como na 6a. Em qualquer Err depois de
  /// um message_start, soma o uso acumulado (melhor esforço: uma falha ao somar não troca o erro).
  pub(crate) fn enviar_com_pausas(db: &Db, cliente: &dyn ClienteIa, chave: &str, hoje: NaiveDate,
      modelo: &str, corpo: &dyn Fn(&[Value]) -> Value, mensagens: &[Value], cancelar: &AtomicBool,
      progresso: &mut dyn FnMut(Progresso)) -> Result<Acumulador, CoreError>;
  ```
- **Validação do foco:** cada caso devolve `FocoInvalido`, sem chamar a API.
  - Valor: a mesma validação da fase 5, com a mensagem dela.
  - Sem alocação-alvo: "Defina a sua alocação-alvo antes de conversar sobre o aporte.".
  - Classe fora da divisão: "Esta classe não está na divisão do aporte.".
  - Direção desconhecida: "Direção desconhecida.". Uma direção repetida conta uma vez só, e elas vão sempre na ordem de `DIRECOES`.
  - Texto livre acima de 300 caracteres: "Use no máximo 300 caracteres.".
  - Meta inexistente: "Esta meta não existe mais. Atualize a lista.".
- **Conversa do aporte:**
  - O título segue a regra da 6a (60 primeiros caracteres da pergunta).
  - `listar_conversas` e `conversa` devolvem o `foco`.
  - `gravar_troca` grava o `foco` só quando a conversa é nova.
  - `perguntar` e `perguntar_aporte` passam a usar `enviar_com_pausas`, e o uso das respostas boas continua na transação de `gravar_troca`.

- [ ] **Step 1: Testes que falham** (`tests/ia_aporte.rs`, com `ClienteFixture` e `CofreMemoria`, alvo 45/30/15/10/0 e a carteira de exemplo da fase 5):
  - `contexto_do_aporte_tem_valor_classe_direcao_e_meta`: o bloco é igual, linha a linha, ao das Global Constraints, para uma carteira montada com esses números. Ele vem antes de "Hoje é", e a última fonte interna é a do aporte.
  - `direcao_vazia_e_sem_meta`: as frases "não indicada." e "nenhuma." aparecem, e "Nas palavras dele" não.
  - `aporte_nao_leva_valores_da_carteira`: com um CDB de R$ 9.000,00 e "Apenas percentuais" ligada, o bloco não contém "9.000" nem o total da carteira formatado, mas contém "R$ 2.000,00".
  - `foco_invalido`: os seis casos da validação dão `IA_FOCO_INVALIDO` com a mensagem exata, e um `ClienteIa` que conta chamadas fica em 0.
  - `conversa_do_aporte_tem_foco_e_aparece_na_lista`: `listar_conversas()[0].foco == Some(Classe::Acoes)`. Uma conversa do Assistente tem `foco == None`.
  - `segunda_pergunta_reusa_o_contexto_do_aporte`: o `system` da segunda pergunta (por `perguntar` com o id) é igual byte a byte ao da primeira, mesmo depois de mudar a meta.
  - Em `tests/ia_conversa.rs`:
    - `uso_conta_resposta_que_falha_depois_do_message_start`: `#quebrado` dá `IA_INDISPONIVEL`, nenhuma mensagem gravada, e o uso do mês tem os tokens do `message_start` do fixture;
    - `uso_conta_resposta_cancelada`: cancelar no terceiro evento de `simples.sse` dá `IA_CANCELADA` e soma a entrada (1200) e a escrita de cache (1000);
    - `erro_antes_do_message_start_nao_conta`: `#erro529` deixa o uso em 0.
- [ ] **Step 2:** `cargo test -p guru-core --test ia_aporte --test ia_conversa`. Esperado: FAIL.
- [ ] **Step 3:** implementar. A divisão sai de `Db::proximo_aporte(hoje, valor)`, para a tela não poder mandar números que não batem.
- [ ] **Step 4:** `cargo test`, `cargo clippy --all-targets -- -D warnings` e `cargo fmt --check`. Esperado: PASS, inclusive os testes da 6a sem mudança.
- [ ] **Step 5: Commit** com a mensagem "IA: foco do Próximo aporte no contexto e tokens de respostas perdidas no uso".

### Task 2: Núcleo — insights e notícias

**Files:**
- Create:
  - `crates/guru-core/src/ia/gerados.rs`;
  - os fixtures `crates/guru-core/tests/fixtures/ia/insights.sse`, `insights_invalido.sse`, `noticias.sse` e `noticias_vazias.sse` (descritos abaixo).
- Modify:
  - `crates/guru-core/src/ia/cliente.rs` (`corpo_insights`, os dois `ErroIa` novos, o roteamento do `ClienteFixture`);
  - `crates/guru-core/src/ia/contexto.rs` (as notícias na parte "Mercado" e `contexto_noticias`);
  - `crates/guru-core/src/ia/mod.rs`, `crates/guru-core/src/lib.rs`.
- Test: `crates/guru-core/tests/ia_gerados.rs`

**Interfaces:**
- Consumes: Task 1 (`enviar_com_pausas`, `somar_uso`, `ia_gerado`) e 6a (`corpo_pedido`, `Fonte`, `contexto_montado`).
- **Produces:**
  ```rust
  pub fn corpo_insights(modelo: &str, contexto: &str) -> (Value, Option<&'static str>);
  // ErroIa::RespostaInvalida -> "IA_RESPOSTA_INVALIDA"; ErroIa::BuscaDesligada -> "IA_BUSCA_DESLIGADA"
  #[derive(Serialize, Deserialize)] pub struct Insight { pub tipo: String, pub texto: String,
                                                          pub acao: String, pub classe: Option<String> }
  #[derive(Serialize, Deserialize)] pub struct Noticia { pub titulo: String, pub url: String,
      pub dominio: String, pub data: Option<String>, pub etiqueta: String }
  pub struct Gerados<T> { pub gerado_em: NaiveDate, pub modelo: String, pub itens: Vec<T> }
  pub struct PainelIa { pub insights: Option<Gerados<Insight>>, pub noticias: Option<Gerados<Noticia>> }
  pub fn rotulo_insight(tipo: &str) -> &'static str;  // Rebalanceamento, Meta, Vencimento, Concentração, Mercado
  pub fn ler_insights(json: &str, stop_reason: Option<&str>) -> Result<Vec<Insight>, ErroIa>;
  pub fn ler_noticias(blocos: &[Value], fontes: &[Fonte], etiquetas: &[String]) -> Vec<Noticia>;
  impl Db {
      pub fn painel_ia(&self) -> Result<PainelIa, CoreError>;
      /// "## Ativos da carteira" (uma linha por classe presente: "Ações: ITUB4, WEGE3.";
      /// renda fixa e outros como "<tipo_rotulo> <rendimento>"), "## Suas instruções" se ligada
      /// e não vazia, e "Hoje é dd/mm/aaaa.". Nunca valor, `nome`, instituição sem a chave, nem
      /// tickers com a parte Carteira desligada (aí, só "Classes na carteira: Renda fixa, Ações.").
      pub fn contexto_noticias(&self, hoje: NaiveDate) -> Result<String, CoreError>;
  }
  /// Envia, valida e grava (ia_gerado + uso numa transação). Erro: nada muda no ia_gerado.
  pub fn gerar_insights(db: &Db, cofre: &dyn Cofre, cliente: &dyn ClienteIa, hoje: NaiveDate,
                        cancelar: &AtomicBool, progresso: &mut dyn FnMut(Progresso)) -> Result<(), CoreError>;
  pub fn buscar_noticias(db: &Db, cofre: &dyn Cofre, cliente: &dyn ClienteIa, hoje: NaiveDate,
                         cancelar: &AtomicBool, progresso: &mut dyn FnMut(Progresso)) -> Result<(), CoreError>;
  ```
  As etiquetas válidas de `buscar_noticias` são os tickers da carteira mais os rótulos das cinco classes.
- **`ClienteFixture`:** escolhe o arquivo antes das marcas da 6a.
  - **Insights** (o corpo tem `output_config.format`):
    - `#erro401` ou `#erro529` no `system[1]` dão os erros da 6a;
    - `#insights-invalido` → `insights_invalido.sse`;
    - qualquer outro → `insights.sse`.
  - **Notícias** (a última mensagem do usuário é `PEDIDO_NOTICIAS`):
    - as mesmas marcas de erro no `system[1]`;
    - `#noticias-vazias` → `noticias_vazias.sse`;
    - qualquer outro → `noticias.sse`.
  - As marcas chegam ao `system[1]` pelas instruções do usuário, que vão nos dois contextos.
- **Fixtures:** seguem o formato real da API, com pensamento no começo.
  - `insights.sse`: um bloco de texto com o JSON partido em vários `text_delta`. São 5 insights, um de cada tipo, com as ações `aporte`, `meta`, `conversar`, `classe` (`acoes`) e `nenhuma`.
  - `insights_invalido.sse`: o mesmo, com um `tipo` igual a `"outro"`.
  - `noticias.sse`: um `server_tool_use` e um `web_search_tool_result` com 8 resultados, um deles `http://`, e depois 8 linhas no formato:
    - 6 válidas, citando URLs `https` distintas (`ITUB4`, `Renda fixa`, `HGLG11`, `WEGE3`, `XPTO3`, que não está na carteira, e `Ações`);
    - 1 citando a URL `http://`;
    - 1 sem citação.
    - Pelo menos uma linha atravessa três blocos: texto sem citação, texto citado e a quebra de linha.
  - `noticias_vazias.sse`: o texto "Não encontrei notícias recentes." sem citação.

- [ ] **Step 1: Testes que falham** (`tests/ia_gerados.rs`):
  - `corpo_insights_tem_json_schema_e_nenhuma_ferramenta`:
    - `output_config` igual a `{"effort":"medium","format":{"type":"json_schema","schema":SCHEMA}}`;
    - sem `tools`, mesmo com a busca ligada;
    - uma mensagem só, igual a `PEDIDO_INSIGHTS`;
    - o `system` igual ao da 6a;
    - `fallbacks` só em Opus e Sonnet.
  - `corpo_noticias_tem_busca_e_nao_tem_format`.
  - `gerar_insights_grava_cinco_e_o_uso`: os rótulos saem de `rotulo_insight`, `gerado_em` é hoje, e o uso do mês sobe.
  - `insights_invalidos_mantem_os_anteriores`: gera uma vez, põe `#insights-invalido` nas instruções e gera de novo. O resultado é `IA_RESPOSTA_INVALIDA`, e `painel_ia()` devolve os 5 da primeira vez.
  - `ler_insights_recusa_fora_do_esquema`, um caso por regra da validação, mais "6 viram 5".
  - `noticias_so_com_link_https_citado`: 6 notícias, nenhuma `http:`, todas com URL igual à de uma `Fonte` e com o domínio e a data dela.
  - `etiqueta_desconhecida_vira_mercado`: `XPTO3` vira "Mercado" e `itub4` casa com `ITUB4`.
  - `noticias_vazias_gravam_lista_vazia`.
  - `contexto_noticias_respeita_privacidade`:
    - um CDB de R$ 9.000,00 com o `nome` "CDB Banco Fictício", do "Banco Fictício" e emitido por "Emissor Exemplo", e uma ação ITUB4;
    - com as chaves padrão, o corpo enviado não contém "R$", "9.000", "Banco Fictício" nem "Emissor Exemplo";
    - com "Carteira" desligada, não contém "ITUB4";
    - com "Nome das instituições" ligada, o nome da instituição aparece.
  - `busca_desligada_recusa_noticias`: `IA_BUSCA_DESLIGADA` e zero chamadas ao cliente.
  - `sem_chave_nao_chama_a_api`, para os dois pedidos.
  - `erro_mantem_o_cache`: com `#erro529` nas instruções, as notícias anteriores continuam.
  - `mercado_leva_as_3_noticias_mais_recentes`: depois de `buscar_noticias`, a parte "Mercado" termina com as 3 linhas do formato das Global Constraints e tem o resumo "automático · 3 notícias".
- [ ] **Step 2:** `cargo test -p guru-core --test ia_gerados`. Esperado: FAIL.
- [ ] **Step 3:** implementar. `gerar_insights` e `buscar_noticias` usam `enviar_com_pausas`, com o modelo da configuração.
- [ ] **Step 4:** `cargo test`, clippy e fmt. Esperado: PASS.
- [ ] **Step 5: Commit** com a mensagem "IA: insights com saída estruturada e notícias citadas da busca na web".

### Task 3: Ponte — addon, main, preload e api

**Files:**
- Modify:
  - `crates/guru-node/src/ia.rs` (`Ia::lancar` genérico; tipos `FocoAporteJs`, `PainelIaJs`, `InsightJs` e `NoticiaJs`; `foco` e `focoRotulo` nas conversas);
  - `crates/guru-node/src/lib.rs`, `crates/guru-node/test/ia.test.mjs`;
  - `app/main.js`, `app/preload.js`, `app/renderer/js/api.js`;
  - `app/test/unit/api.test.mjs`, `app/test/e2e/abrir.spec.js` (lista de canais).
- Create: cópias em `app/test/e2e/fixtures/ia/` dos quatro `.sse` da Task 2.

**Interfaces:**
- Consumes: Tasks 1 e 2.
- **Addon:**
  ```rust
  impl Ia {
      /// Thread própria, conexão própria, pedido no mapa; a tarefa devolve o id da conversa (ou None).
      fn lancar(&self, pasta_dados: PathBuf, tarefa: Box<dyn FnOnce(&Db, &dyn Cofre, &dyn ClienteIa,
          &AtomicBool, &mut dyn FnMut(Progresso)) -> Result<Option<i64>, CoreError> + Send>) -> napi::Result<u32, String>;
  }
  ```
  - `perguntarIa(conversaId|null, pergunta, foco?: FocoAporteJs|null)`: devolve `{ pedido }`.
    - Com `conversaId` nulo e `foco`, chama `perguntar_aporte`.
    - Com `conversaId`, o `foco` é ignorado.
    - `FocoAporteJs = { classe, valorCentavos, direcoes: string[], direcaoLivre, metaId: number|null }`.
  - `gerarInsightsIa()` e `buscarNoticiasIa()`: devolvem `{ pedido }`, que o `iaProgresso` da 6a acompanha, com `conversaId` nulo.
  - `painelIa()`: devolve `{ buscaWeb, insights, noticias }`.
    - `insights` é `{ geradoEm: "aaaa-mm-dd", modelo, itens: InsightJs[] } | null`, com `InsightJs = { tipo, rotulo, texto, acao, classe|null, classeRotulo|null }`.
    - `noticias` é `{ geradoEm, modelo, itens: NoticiaJs[] } | null`, com `NoticiaJs = { titulo, url, dominio, data|null, etiqueta }`.
  - `ConversaResumoJs` e `ConversaJs` ganham `foco: string|null` e `focoRotulo: string|null` (o rótulo da classe).
  - A pergunta da 6a passa a usar `lancar`, sem mudar o que devolve.
- **Canais e API:**
  - novos canais: `ia:painel`, `ia:gerarInsights` e `ia:buscarNoticias`;
  - `ia:perguntar` repassa o terceiro argumento (`foco ?? null`);
  - `api.ia` ganha `painel`, `gerarInsights` e `buscarNoticias`, e `perguntar(conversaId, pergunta, foco)`.

- [ ] **Step 1: Testes que falham** (`ia.test.mjs`, com `pastaIa` e `cofreMemoria`, alvo salvo e um investimento):
  - `perguntarIa(null, "Oi", foco de Ações)` termina com `conversaId`;
    - `conversas()[0].foco === "acoes"` e `focoRotulo === "Ações"`;
    - `conversa(id).fontesInternas` tem a parte `aporte`;
  - uma classe inexistente no foco termina com `erro.code === "IA_FOCO_INVALIDO"`;
  - `gerarInsightsIa` com polling termina sem erro, e `painelIa().insights.itens.length === 5`;
  - `buscarNoticiasIa` termina, e todas as `url` de `painelIa().noticias.itens` começam com `https://`;
  - com a busca desligada, `buscarNoticiasIa` termina com `IA_BUSCA_DESLIGADA`;
  - `api.test.mjs` cobre os 15 métodos de `api.ia` e o terceiro argumento de `perguntar`.
- [ ] **Step 2:** rodar os testes. Esperado: FAIL.
- [ ] **Step 3:** implementar.
- [ ] **Step 4:** build do addon, `npm test` no addon e no app, e só `abrir.spec.js` e `assistente.spec.js` no Playwright. Esperado: PASS.
- [ ] **Step 5: Commit** com a mensagem "IA: ponte do chat por classe, dos insights e das notícias".

### Task 4: Próximo aporte com direção, meta e chat por classe

**Files:**
- Create:
  - `app/renderer/js/telas/assistente/chat.js` (o chat que hoje está dentro de `assistente.js`);
  - `app/renderer/js/telas/aporte/gaveta.js`;
  - `app/test/unit/aporte-foco.test.mjs`;
  - `app/test/e2e/aporte-ia.spec.js`.
- Modify:
  - `app/renderer/js/telas/aporte.js`;
  - `app/renderer/js/telas/assistente.js` (usa `chat.js`; `#/assistente/<id>` abre a conversa);
  - `app/renderer/js/telas/assistente/conversas.js` (a etiqueta do foco);
  - `app/renderer/css/telas.css` (só o que faltar: chips de direção, grade com gaveta e etiqueta).

**Interfaces:**
- Consumes: Task 3 (`api.ia.perguntar` com `foco`, `foco` e `focoRotulo` nas conversas).
- **Produces:**
  ```js
  // chat.js: balões, sugestões, campo, Enviar/Parar, "Buscando na web…", aviso fixo, polling de 150 ms
  // e abandono quando a tela sai de cena, tudo como na 6a. `foco` vai só na primeira pergunta.
  export function criarChat({ vigente, sugestoes, placeholder, foco = null, aoMudar = () => {} })
    -> { elemento, mostrar(conversa /* ConversaJs|null */), conversa() /* ConversaJs|null */,
         emAndamento() /* boolean */, preencher(texto), fontes() /* { web, internas } */ };
  // aoMudar({ conversa, emAndamento }) depois de cada redesenho (o Assistente redesenha lista e painel).
  // aporte/gaveta.js
  export function resumoContexto({ valorCentavos, direcoes, direcaoLivre, metaNome }) -> string;
  export function gaveta({ vigente, parte, foco, metaNome, aoFechar }) -> HTMLElement;
  ```
- **Formato de `resumoContexto`:** "Contexto: aporte de R$ 2.000,00 · direção: Ações, Experimentar algo novo, "quero algo com liquidez" · meta: Entrada do apartamento · mais as partes ligadas em Configurar IA".
  - Sem direção nem texto livre: "direção: livre".
  - Sem meta: "meta: nenhuma".
- **Tela:**
  - **Formulário:**
    - o campo de valor da fase 5;
    - o `select` "Para qual meta?" (`#aporte-meta`), com "Nenhuma meta específica" e as metas cadastradas;
    - o grupo "Para onde quer levar esse dinheiro?" (`fieldset`), com um `checkbox` por direção, desenhados como chips;
    - o campo "Outra direção ou detalhe (opcional)" (`#aporte-direcao`, `maxlength=300`);
    - o botão "Dividir".
  - **Com chave:**
    - cada cartão da divisão ganha o botão "Conversar com a IA" (`aria-label` "Conversar com a IA sobre <classe>");
    - o botão abre a gaveta à direita, e abrir outra classe troca a gaveta.
  - **Sem chave:** nenhum botão, e acima dos cartões o card com o convite e o link.
  - **Gaveta:**
    - o título "<classe> · <R$ sugerido>", a etiqueta "Claude", a linha `resumoContexto` e o botão "Fechar";
    - o chat com o `placeholder` "Pergunte sobre esta classe…" e as três sugestões;
    - os links "Registrar aporte" (`#/investimentos/novo`) e, quando a conversa existe, "Ver no Assistente" (`#/assistente/<id>`);
    - embaixo, o `painel(web, internas)` da 6a.
  - **Durante uma resposta:** o formulário inteiro fica desabilitado.
  - **Dividir de novo:** fecha a gaveta.
  - **Mudar direção ou meta com a gaveta aberta:** vale só para uma conversa nova. A gaveta ganha o botão "Nova conversa", que recomeça com o foco atual.
- **Assistente:**
  - `#/assistente/<id>` abre a conversa `id`; uma conversa inexistente cai na tela normal, com a mensagem "Não foi possível abrir a conversa.";
  - na lista, a conversa com foco tem a etiqueta "Próximo aporte · <focoRotulo>".

- [ ] **Step 1: Testes que falham:**
  - **`aporte-foco.test.mjs`:** `resumoContexto` com tudo preenchido, sem direção e sem meta.
  - **`aporte-ia.spec.js`** (alvo salvo e um investimento criados pela `api`):
    - **sem chave:** dividir R$ 2.000,00 mostra o convite e nenhum "Conversar com a IA".
    - **com chave:**
      - marcar "Ações" e "Experimentar algo novo", escrever o texto livre, escolher uma meta e dividir não muda os valores da divisão (iguais aos de sem direção);
      - abrir a gaveta de Ações mostra o título e a linha de contexto exata;
      - até aqui, `uso.tokens === 0`;
      - clicar na primeira sugestão mostra a resposta de `simples.sse`, e "Seus dados" lista "Próximo aporte";
      - uma segunda pergunta na gaveta funciona;
      - "Ver no Assistente" abre a conversa com as 4 mensagens, e a lista mostra "Próximo aporte · Ações";
      - `#erro401` na gaveta mostra a mensagem da chave e devolve o texto ao campo;
      - dividir de novo fecha a gaveta;
      - nenhum erro de console ou CSP.
  - O `assistente.spec.js` da 6a continua verde sem mudança.
- [ ] **Step 2:** `npm test` e `npx playwright test aporte-ia.spec.js assistente.spec.js aporte.spec.js`. Esperado: FAIL no que é novo.
- [ ] **Step 3:** implementar. Primeiro extrair `chat.js` com o `assistente.spec.js` verde, depois a gaveta.
- [ ] **Step 4:** `npm test` e `npx playwright test`. Esperado: verde e sem janela na tela.
- [ ] **Step 5: Commit** com a mensagem "Próximo aporte: direção, meta e chat com a IA por classe".

### Task 5: Insights e notícias no painel, documentação

**Files:**
- Create: `app/renderer/js/telas/carteira/insights.js`, `app/test/unit/insights.test.mjs`, `app/test/e2e/insights.spec.js`
- Modify:
  - `app/renderer/js/telas/carteira/painel.js` (a coluna à direita, só com a carteira não vazia);
  - `app/renderer/js/telas/assistente.js` (`#/assistente/insight/<n>`);
  - `app/renderer/css/telas.css` (coluna, cartões de insight e lista de notícias, com os tokens existentes);
  - `docs/mvp.md`, `docs/roadmap.md`, `docs/arquitetura.md` e `README.md`.

**Interfaces:**
- Consumes: Task 3 (`api.ia.painel`, `gerarInsights`, `buscarNoticias`, `progresso`, `cancelar`, `config`).
- **Produces:**
  ```js
  export function acaoDoInsight(insight, indice) -> { texto, href } | null;
  //  aporte    -> { "Simular próximo aporte →", "#/aporte" }
  //  meta      -> { "Ver metas →", "#/perfil" }
  //  classe    -> { "Ver <classeRotulo> →", "#/carteira/<classe>" }
  //  conversar -> { "Conversar sobre isso →", "#/assistente/insight/<indice>" }
  //  nenhuma   -> null
  export function colunaIa(vigente) -> HTMLElement;   // carrega sozinha (config + painel)
  ```
- **Coluna** (layout do protótipo, 270 px à direita do corpo do painel):
  - **Sem chave:** o convite e o link "Configurar IA".
  - **Card "Insights":**
    - **Cabeçalho:** "Insights" e a etiqueta "Claude · hoje", ou "Claude · dd/mm" quando gerados em outro dia.
    - **Cartões:** cada um com a etiqueta do `rotulo` (com a classe `wa` em Rebalanceamento), o texto e o link de `acaoDoInsight`.
    - **Nunca gerados:** "Nenhum insight gerado ainda.".
    - **Botões:**
      - "Gerar insights", que vira "Gerar de novo" quando há insights;
      - durante a geração, "Gerando insights…" e "Parar";
      - a nota de custo embaixo.
  - **Seção "Notícias da sua carteira":**
    - **Cabeçalho:** a data da busca.
    - **Links:** um por notícia, com o título, e a linha "domínio · data · <etiqueta>". A data sai de `dataDaFonte` da 6a. O link abre no navegador pelo handler que já existe.
    - **Lista:** mostra 5. Com mais de 5, o botão "Ver todas as notícias →" mostra o resto.
    - **Lista vazia:** "Nenhuma notícia recente encontrada sobre os seus ativos.".
    - **Botões:** "Buscar notícias", que vira "Buscar de novo"; durante a busca, "Buscando notícias na web…" e "Parar".
    - **Busca desligada:** em vez do botão, a mensagem de `IA_BUSCA_DESLIGADA` com o link "Configurar IA".
  - **Erro:** a mensagem aparece abaixo do botão (`role=alert`), e os itens anteriores continuam.
  - **Aviso:** o aviso da coluna fica no rodapé.
- **Acompanhamento:**
  - os números dos pedidos em andamento ficam numa variável do módulo;
  - sair da Carteira só para o polling, e voltar retoma (o pedido terminado sai do mapa na primeira leitura, como na 6a);
  - "Parar" chama `api.ia.cancelar`;
  - ao terminar sem erro, a coluna relê `api.ia.painel()`.
- **Assistente com `#/assistente/insight/<n>`:**
  - abre uma conversa nova com o campo preenchido com `Sobre este insight do painel: "<texto>" `;
  - nada é enviado sozinho;
  - um índice inválido abre o Assistente normal.
- **Docs:**
  - **`docs/mvp.md`:**
    - a tela 5 perde "(fase 6)";
    - a decisão "IA" troca "Chat por classe, insights e notícias ficam para a fase 6b." por uma frase com a geração só por clique e guardada com a data, os insights com saída estruturada e sem busca, e as notícias da busca na web, só com link citado;
    - a decisão "Próximo aporte" ganha a direção e a meta, que vão só para o contexto do chat por classe e não mudam a divisão.
  - **`docs/roadmap.md`:** o link do plano da 6b.
  - **`docs/arquitetura.md`, seção "Rede":** os pedidos de insights e notícias, e o que cada um leva do usuário. Em "Modo de teste da IA", as marcas novas.
  - **`README.md`:** o status "fases 0 a 6 concluídas".

- [ ] **Step 1: Testes que falham:**
  - **`insights.test.mjs`:** `acaoDoInsight` nas cinco ações, e `href` sempre começando com `#/`.
  - **`insights.spec.js`** (um investimento ITUB4 e um CDB criados pela `api`):
    - **sem chave:** a Carteira mostra o convite da coluna.
    - **com chave:**
      - abrir a Carteira mostra "Nenhum insight gerado ainda." e `uso.tokens === 0`;
      - "Gerar insights" mostra 5 cartões, com as etiquetas Rebalanceamento, Meta, Vencimento, Concentração e Mercado, "Claude · hoje" e o link "Simular próximo aporte →" para `#/aporte`;
      - navegar para fora e voltar mostra os mesmos 5 sem gastar tokens (uso igual);
      - com `#insights-invalido` nas instruções, "Gerar de novo" mostra a mensagem de `IA_RESPOSTA_INVALIDA`, e os 5 continuam;
      - "Conversar sobre isso →" abre o Assistente com o campo preenchido e sem resposta pendente, e o uso não muda;
      - "Buscar notícias" mostra 5 links `https` com domínio e etiqueta. Nenhum é o `http://` do fixture, e `XPTO3` aparece como "Mercado". "Ver todas as notícias →" mostra a sexta;
      - depois disso, a parte Mercado em Configurar IA mostra "automático · 3 notícias";
      - com "Buscar na web" desligada, a seção mostra a mensagem com o link;
      - nenhum erro de console ou CSP.
- [ ] **Step 2:** rodar os testes. Esperado: FAIL.
- [ ] **Step 3:** implementar a coluna e os docs.
- [ ] **Step 4:** a suíte inteira:
  - `cargo test`, clippy e fmt;
  - os testes do addon;
  - `npm test`;
  - `npx playwright test`.
  - Esperado: tudo verde e sem janela na tela.
- [ ] **Step 5: Commit** com a mensagem "IA: insights e notícias no painel e documentação da fase 6".
