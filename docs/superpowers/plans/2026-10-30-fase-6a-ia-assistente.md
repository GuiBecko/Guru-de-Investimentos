# Fase 6a: Configurar IA e Assistente — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O usuário configura o Claude com a própria chave, guardada no cofre do sistema. Ele escreve instruções próprias, escolhe o que o app manda como contexto (desligando ou editando cada parte) e conversa com o Assistente. As respostas chegam aos poucos, citam fontes da web com números e mostram num painel tudo o que a IA está vendo.

**Architecture:**
- **Núcleo:** um módulo `ia` com estas partes:
  - configuração e privacidade no SQLite;
  - a chave no cofre do sistema, atrás do trait `Cofre`;
  - o montador do contexto;
  - o cliente HTTP da Messages API com streaming SSE, atrás do trait `ClienteIa`, com uma implementação real (`ureq`) e uma de arquivos para os testes;
  - o acumulador de eventos, que monta os blocos completos e numera as fontes;
  - as conversas gravadas.
- **Addon:** roda cada pergunta num `AsyncTask`, que escreve o progresso num estado compartilhado. A tela consulta esse estado a cada 150 ms com `iaProgresso`, sem callback do Rust para o JS. A tela pode pedir para parar.
- **Interface:** as telas Configurar IA e Assistente trocam o "em construção".

**Tech Stack:** o mesmo das fases anteriores, mais:
- `keyring = "4.2.0"` no `guru-core`, com as features padrão: Keychain, Credential Manager e Secret Service via `zbus`, sem libdbus;
- a Messages API em HTTP puro com `ureq`, porque Rust não tem SDK oficial.

**Spec:**
- [`docs/mvp.md`](../../mvp.md): telas 8 e 9, mais as decisões "Regulatório", "Custo da IA", "Contexto da IA" e "Dados de mercado".
- [`docs/arquitetura.md`](../../arquitetura.md): "Chave da IA" e "Rede".
- Protótipo: `docs/prototipo/index.html:848-945` (telas 8 e 9).

## Fora desta fase (vão para a 6b)

Ficam para a 6b:
- o chat por classe no Próximo aporte, com o campo de direção e a meta;
- os insights no painel;
- as notícias com link.

Ficam para a V2:
- OpenAI, Gemini e Ollama (o card deles aparece com a etiqueta "V2");
- a IA sugerir metas.

## Global Constraints

- **Tudo das fases anteriores continua valendo:**
  - CSP;
  - nada de `innerHTML` com dados nem `style=`;
  - `vigente()` depois de cada await;
  - pt-BR com acentos;
  - erros em SCREAMING_SNAKE_CASE;
  - preload sem lógica;
  - IPC com checagem de origem;
  - dinheiro em centavos;
  - e2e com a janela escondida via `lancar()`, sem `show()` nem `showInactive()`.
- **A chave nunca vai para o SQLite, para o log nem para a interface:**
  - fica no cofre, com serviço `guru-de-investimentos` e usuário `anthropic`;
  - o SQLite guarda só os 4 últimos caracteres (`chave_final`), para mostrar `sk-ant-…4f2a`;
  - o main só repassa a chave digitada para o addon, e o addon não a devolve em nenhuma resposta.
- **Testes nunca chamam a API real nem gastam dinheiro do usuário.** A rede é trocada por arquivos nestes casos:
  - no núcleo, pelo `ClienteFixture`;
  - no addon e no e2e, por `OpcoesCore.pastaIa`, que o main só passa com `GURU_IA_FIXTURES` e fora do app empacotado.
  - O cofre é trocado por um em memória com `OpcoesCore.cofreMemoria = true` (`GURU_IA_COFRE=memoria`, mesmas condições).
- **Rede:** só dois destinos novos, ambos exatos:
  - `https://api.anthropic.com/v1/messages` (POST);
  - `https://api.anthropic.com/v1/models?limit=1` (GET, para validar a chave, sem custo).
  - Os dois recebem os cabeçalhos `x-api-key`, `anthropic-version: 2026-10-21`, `content-type: application/json` e o `User-Agent` do app.
  - Prazos: 60 s para conectar e 10 min para a resposta inteira. Uma resposta acima de 8 MB é cortada com erro.
  - `docs/arquitetura.md` passa a dizer que só a Anthropic recebe dado do usuário, com a chave dele e só as partes ligadas.
- **Modelos oferecidos**, nesta ordem, com o código e o rótulo exatos:
  - `claude-opus-5-5`, "Opus 5.5 · mais capaz" (padrão);
  - `claude-sonnet-5-5`, "Sonnet 5.5 · equilíbrio";
  - `claude-haiku-5-5`, "Haiku 5.5 · mais barato".
- **Corpo do pedido:**
  ```json
  { "model": <modelo>, "max_tokens": 32000, "stream": true,
    "system": [ {"type":"text","text": SISTEMA},
                {"type":"text","text": <contexto da conversa>, "cache_control":{"type":"ephemeral"}} ],
    "messages": <histórico>, "output_config": {"effort":"medium"},
    "tools": [<busca, se ligada>] }
  ```
  - Não vão `temperature`, `top_p`, `top_k` nem `thinking`. A omissão de `thinking` é o pensamento adaptativo desses modelos.
  - **Busca na web:**
    - Opus e Sonnet: `{"type":"web_search_20260209","name":"web_search","max_uses":5,"user_location":{"type":"approximate","country":"BR","timezone":"America/Sao_Paulo"}}`.
    - Haiku: o mesmo objeto, com `"type":"web_search_20250305"`.
  - **Fallback:** com Opus e Sonnet, o corpo leva também `"fallbacks":"default"` e o cabeçalho `anthropic-beta: server-side-fallback-2026-07-01`.
- **Texto `SISTEMA`** (fixo, sem data, para o cache):
  > Você é o assistente do Guru de Investimentos, um app brasileiro de carteira pessoal. Responda em português do Brasil, com clareza e sem jargão desnecessário. Apresente opções, com prós e contras, e deixe a decisão com o usuário: não faça recomendação individual de investimento nem diga o que ele deve comprar ou vender. Use o contexto enviado sobre a carteira, o perfil, as metas e o mercado. Para fatos de mercado recentes (decisões do Copom, projeções, notícias), busque na web quando a busca estiver disponível e cite a fonte. Quando algo for opinião sua, sem fonte, diga que é opinião. Se faltar um dado para responder bem, diga qual.
- **Histórico enviado:** cada conversa reenvia as mensagens anteriores exatamente como foram gravadas, só acrescentando no fim.
  - As do usuário vão como texto.
  - As da IA vão com os blocos completos recebidos (`thinking` com assinatura, `text` com `citations`, `server_tool_use`, `web_search_tool_result`), sem tirar nem editar nada.
  - O contexto é congelado na conversa: é montado quando a conversa nasce, gravado nela e reusado em toda pergunta. Uma conversa nova pega o contexto atualizado.
- **`pause_turn`:** o app reenvia o pedido com a resposta parcial acrescentada como mensagem da IA, até 3 vezes.
- **`refusal`:** grava a resposta como veio e mostra, abaixo dela, o aviso "A IA preferiu não responder a esta pergunta.".
- **Erros:** em qualquer erro, ou quando o usuário para, a pergunta pendente não fica gravada, e a tela devolve o texto dela para o campo. Código e mensagem exatos:

  | Situação | Código | Mensagem |
  |---|---|---|
  | sem chave | `IA_SEM_CHAVE` | "Configure a sua chave em Configurar IA." |
  | 401 ou 403 | `IA_CHAVE_RECUSADA` | "A Anthropic recusou a sua chave. Confira em Configurar IA." |
  | 429 | `IA_LIMITE` | "Limite de uso da sua conta Anthropic atingido. Tente de novo em instantes." |
  | 500 a 599, incluindo 529 | `IA_INDISPONIVEL` | "A Anthropic está indisponível agora. Tente de novo em instantes." |
  | outro 4xx | `IA_PEDIDO_INVALIDO` | "A Anthropic recusou o pedido: <message do corpo de erro>" |
  | evento `error` no meio da resposta | `IA_INDISPONIVEL` | (a mesma mensagem) |
  | rede ou prazo | `IA_SEM_CONEXAO` | "Sem conexão com a Anthropic." |
  | parado pelo usuário | `IA_CANCELADA` | "Resposta interrompida." |
- **Validar e salvar a chave:**
  - **Formato:** começa com `sk-ant-`, tem de 20 a 300 caracteres e nenhum espaço. Se não, "Isso não parece uma chave da Anthropic (começa com sk-ant-).".
  - **Conferência:** a chave é testada com `GET /v1/models?limit=1`.
    - 200: grava no cofre e em `chave_final`.
    - 401 ou 403: "A Anthropic recusou esta chave.".
    - Rede: "Sem conexão com a Anthropic. A chave não foi salva.".
  - **Remover:** "Remover chave" apaga do cofre e limpa `chave_final`.
- **Privacidade:** três chaves, que valem para o contexto montado a partir daí:
  - **"Apenas percentuais"** (ligada por padrão): a carteira vai sem nenhum valor em reais, só em percentuais do total, e as metas vão só com o percentual atingido;
  - **"Nome das instituições"** (desligada por padrão): sem ela, instituição e emissor não aparecem;
  - **"Buscar na web"** (ligada por padrão).
- **Partes do contexto,** nesta ordem, com código, rótulo e conteúdo automático:
  - `instrucoes`, "Suas instruções": o campo livre de instruções. Não é editável no bloco: edita-se o campo.
  - `perfil`, "Perfil e objetivo": o perfil com a data do teste, a alocação-alvo e o texto do objetivo.
  - `carteira`, "Carteira": o percentual de cada classe com o alvo, e uma linha por investimento com estes dados:
    - tipo e ticker ou indexador com a taxa;
    - vencimento e liquidez;
    - o percentual do total (e, sem "Apenas percentuais", o valor atual);
    - a data dos valores.
  - `mercado`, "Mercado": o acumulado de 12 meses de CDI, Selic e IPCA pelas funções do painel, a última cotação de BOVA11 e XFIX11, e a data mais recente de cada fonte.
  - `metas`, "Metas": nome, prazo, percentual atingido e aporte mensal necessário (este em reais só sem "Apenas percentuais").
  - O fim do contexto leva a linha "Hoje é dd/mm/aaaa." (a data real).
  - Cada parte pode ser desligada (`ligada = 0`) ou editada (`texto_editado`). A parte editada vai com o texto do usuário, palavra por palavra, e ganha a marca "editado por você" e o botão "voltar ao automático", que zera `texto_editado`.
  - **Estimativa de tokens:** `ceil(caracteres / 4)`, mostrada como "≈ N tokens", sem chamar a API.
- **Fontes numeradas:**
  - cada `citations_delta` do tipo `web_search_result_location` recebe um número pela ordem da primeira aparição da URL;
  - ao fechar o bloco de texto citado, o texto exibido ganha os marcadores `[n]` das citações daquele bloco, sem repetir;
  - a fonte guarda URL, título, domínio, a data (`page_age` do `web_search_result` de mesma URL, se houver) e o trecho citado;
  - o painel "O que a IA está vendo" lista as fontes da web numeradas e, em "Seus dados", as partes ligadas do contexto da conversa, com a data dos dados. O mercado com dado de mais de 7 dias ganha a marca "antigo".
- **Uso do mês:** guarda tokens de entrada, saída, leitura e escrita de cache por mês, e mostra "Uso este mês: ~N mil tokens (≈ US$ X,XX)". O custo é estimado com estes preços por milhão de tokens (entrada / saída / leitura de cache):
  - Opus 5.5: 4 / 20 / 0,20;
  - Sonnet 5.5: 2 / 10 / 0,20;
  - Haiku 5.5: 0,10 / 0,50 / 0,01;
  - a escrita de cache custa 1,25 vez a entrada;
  - as buscas na web ficam fora da conta, e o texto diz "sem contar buscas na web".
- **Aviso fixo no Assistente:** "Conteúdo educativo gerado por IA com os seus dados e fontes da web. Não é recomendação individual de investimento. Confira as fontes."
- Trabalho na branch `fase-6a-ia-assistente`. Commits em português terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **A chave vazando:** ela não pode aparecer em nenhum retorno do addon, log, erro, `conteudo_json`, `contexto` gravado nem arquivo. Teste: depois de salvar e conversar, nenhuma resposta da API da interface contém a chave, e o SQLite não a contém.
2. **Privacidade de verdade:** com "Apenas percentuais" ligada, nenhum `R$` nem número de centavos da carteira e das metas pode aparecer no contexto. Sem "Nome das instituições", nenhum nome de instituição ou emissor cadastrado pode aparecer. Uma parte desligada não pode ir no corpo do pedido.
3. **Histórico só acrescenta:** a segunda pergunta de uma conversa com busca reenvia a primeira resposta da IA com os blocos idênticos ao gravado, inclusive `encrypted_content`, `signature` e `citations`. O `system` da segunda pergunta é igual byte a byte ao da primeira, mesmo que a configuração tenha mudado no meio.
4. **Resposta que falha ou é interrompida no meio:** não deixa mensagem pela metade nem pergunta órfã gravada, e o texto volta para o campo. Uma pergunta nova na mesma conversa funciona.
5. **SSE quebrado em pedaços arbitrários:** uma linha cortada no meio, eventos `ping`, um evento desconhecido ou um `data:` com JSON inválido. O acumulador não entra em pânico, ignora o desconhecido e trata o JSON inválido como `IA_INDISPONIVEL`.

---

### Task 1: Núcleo — configuração, cofre e contexto

**Files:**
- Create: `crates/guru-core/src/migrations/0006_ia.sql`, `crates/guru-core/src/ia/mod.rs`, `crates/guru-core/src/ia/config.rs`, `crates/guru-core/src/ia/cofre.rs`, `crates/guru-core/src/ia/contexto.rs`
- Modify: `crates/guru-core/Cargo.toml` (`keyring = "4.2.0"`), `crates/guru-core/src/lib.rs`, `crates/guru-core/src/db.rs` (migração 6, `SCHEMA_VERSION = 6`), `crates/guru-core/tests/db.rs` (versão 6)
- Test: `crates/guru-core/tests/ia_contexto.rs`

**Interfaces:**
- **Schema (`0006_ia.sql`):**
  ```sql
  CREATE TABLE ia_config (id INTEGER PRIMARY KEY CHECK (id = 1), modelo TEXT NOT NULL,
    instrucoes TEXT NOT NULL, apenas_percentuais INTEGER NOT NULL, nomes_instituicoes INTEGER NOT NULL,
    busca_web INTEGER NOT NULL, chave_final TEXT);
  INSERT INTO ia_config VALUES (1, 'claude-opus-5-5', '', 1, 0, 1, NULL);
  CREATE TABLE ia_contexto_parte (parte TEXT PRIMARY KEY, ligada INTEGER NOT NULL, texto_editado TEXT);
  CREATE TABLE conversa (id INTEGER PRIMARY KEY, titulo TEXT NOT NULL, modelo TEXT NOT NULL,
    contexto TEXT NOT NULL, fontes_internas TEXT NOT NULL, busca_web INTEGER NOT NULL,
    criada_em TEXT NOT NULL, atualizada_em TEXT NOT NULL);
  CREATE TABLE mensagem (id INTEGER PRIMARY KEY, conversa_id INTEGER NOT NULL REFERENCES conversa (id) ON DELETE CASCADE,
    papel TEXT NOT NULL CHECK (papel IN ('user','assistant')), conteudo_json TEXT NOT NULL, texto TEXT NOT NULL,
    fontes_json TEXT NOT NULL, aviso TEXT, criada_em TEXT NOT NULL);
  CREATE TABLE ia_uso (mes TEXT NOT NULL, modelo TEXT NOT NULL, entrada INTEGER NOT NULL, saida INTEGER NOT NULL,
    cache_leitura INTEGER NOT NULL, cache_escrita INTEGER NOT NULL, PRIMARY KEY (mes, modelo));
  ```
- **Produces:**
  ```rust
  pub const MODELOS: &[(&str, &str)];               // (código, rótulo) na ordem das Global Constraints
  pub struct ConfigIa { pub modelo: String, pub instrucoes: String, pub apenas_percentuais: bool,
                        pub nomes_instituicoes: bool, pub busca_web: bool, pub chave_final: Option<String> }
  impl Db {
      pub fn config_ia(&self) -> Result<ConfigIa, CoreError>;
      pub fn salvar_config_ia(&self, modelo: &str, instrucoes: &str, apenas_percentuais: bool,
                              nomes_instituicoes: bool, busca_web: bool) -> Result<Vec<ErroCampo>, CoreError>;
      pub(crate) fn gravar_chave_final(&self, final4: Option<&str>) -> Result<(), CoreError>;
      pub fn partes_contexto(&self, hoje: NaiveDate) -> Result<Vec<ParteContexto>, CoreError>;
      pub fn salvar_parte_contexto(&self, parte: &str, ligada: bool, texto_editado: Option<&str>) -> Result<Vec<ErroCampo>, CoreError>;
      pub fn contexto_montado(&self, hoje: NaiveDate) -> Result<ContextoMontado, CoreError>;
  }
  pub struct ParteContexto { pub parte: &'static str, pub rotulo: &'static str, pub ligada: bool,
                             pub editado: bool, pub texto: String, pub resumo: String, pub tokens: u32,
                             pub data_dados: Option<NaiveDate> }
  pub struct ContextoMontado { pub texto: String, pub fontes_internas: Vec<FonteInterna>, pub tokens: u32 }
  pub struct FonteInterna { pub parte: String, pub rotulo: String, pub data_dados: Option<NaiveDate>, pub antigo: bool }
  pub trait Cofre: Send + Sync { fn ler(&self) -> Result<Option<String>, CoreError>;
      fn gravar(&self, chave: &str) -> Result<(), CoreError>; fn apagar(&self) -> Result<(), CoreError>; }
  pub struct CofreSistema;   // keyring, serviço "guru-de-investimentos", usuário "anthropic"
  pub struct CofreMemoria(Mutex<Option<String>>);
  pub fn validar_formato_chave(chave: &str) -> Option<ErroCampo>;   // campo "chave"
  pub fn estimar_tokens(texto: &str) -> u32;  // ceil(chars/4)
  ```
  - `resumo` é a linha cinza do bloco, por exemplo "automático · 27 investimentos" ou "editado por você".
  - Quando `ligada` é falso, `texto` continua sendo o texto que iria, para a tela mostrar.
  - `contexto_montado` junta só as partes ligadas, cada uma sob um título `## <rótulo>`, e termina com "Hoje é dd/mm/aaaa.".
  - Um erro do cofre vira `CoreError` com o código `IA_COFRE_INDISPONIVEL` e a mensagem "Não foi possível usar o cofre de senhas do sistema.".
  - `salvar_config_ia` recusa um modelo fora de `MODELOS` (campo `modelo`) e instruções acima de 4.000 caracteres (campo `instrucoes`, "Use no máximo 4.000 caracteres.").
  - `salvar_parte_contexto` recusa uma parte desconhecida e um texto editado acima de 20.000 caracteres. `instrucoes` nunca guarda texto editado.

- [ ] **Step 1: Testes que falham** (`tests/ia_contexto.rs`):
  - `config_padrao`: Opus, percentuais ligado, instituições desligado, busca ligada, sem chave.
  - `salvar_config_valida_modelo_e_instrucoes`.
  - `contexto_com_apenas_percentuais_nao_tem_reais`: carteira com um CDB de R$ 9.000,00 e uma meta. O texto não contém "R$" nem "9000" nem "9.000".
  - `contexto_sem_instituicoes_omite_nomes`: CDB do "Banco Fictício" emitido por "Emissor Exemplo"; nenhum dos dois nomes aparece. Com a chave ligada, aparecem.
  - `parte_desligada_fica_fora_do_montado`.
  - `parte_editada_vai_literal_e_volta_ao_automatico`.
  - `montado_termina_com_a_data`.
  - `mercado_antigo_depois_de_7_dias`: com o status de mercado de 10 dias atrás, `antigo` é verdadeiro.
  - `estimativa_de_tokens`: "abcd" → 1 e "abcde" → 2.
  - `formato_da_chave`: os três casos de recusa e um aceito.
  - `cofre_memoria_le_grava_apaga`.
- [ ] **Step 2:** `cargo test -p guru-core --test ia_contexto`. Esperado: FAIL.
- [ ] **Step 3:** implementar. Usar as funções que já existem no núcleo (resumo da carteira, perfil, objetivo, metas e progresso, `acumulado_benchmark`, status das fontes) para montar os textos. `CofreSistema` usa a API do `keyring` 4.2 (ler a documentação do crate em `~/.cargo/registry`).
- [ ] **Step 4:** `cargo test`, `cargo clippy --all-targets -- -D warnings` e `cargo fmt --check`. Esperado: PASS.
- [ ] **Step 5: Commit** com a mensagem "IA: configuração, cofre da chave e montagem do contexto".

### Task 2: Núcleo — cliente da Messages API, acumulador e conversas

**Files:**
- Create: `crates/guru-core/src/ia/cliente.rs`, `crates/guru-core/src/ia/sse.rs`, `crates/guru-core/src/ia/conversa.rs`
- Modify: `crates/guru-core/src/ia/mod.rs`, `crates/guru-core/src/lib.rs`
- Create fixtures: `crates/guru-core/tests/fixtures/ia/simples.sse`, `busca.sse` (uma busca, dois blocos de texto, três citações de duas URLs, `page_age` numa delas), `pause.sse` (termina em `pause_turn`), `recusa.sse` (`stop_reason: "refusal"`), `quebrado.sse` (`ping`, um evento desconhecido e um `data:` com JSON inválido no meio do texto)
- Test: `crates/guru-core/tests/ia_conversa.rs`

**Interfaces:**
- Consumes: Task 1 (`ConfigIa`, `Cofre`, `contexto_montado`, tabelas).
- Produces:
  ```rust
  pub enum ErroIa { SemChave, ChaveRecusada, Limite, Indisponivel, PedidoInvalido(String), SemConexao, Cancelada }
  // ErroIa -> CoreError com os códigos e mensagens exatos da tabela de erros
  pub trait ClienteIa: Send + Sync {
      fn validar_chave(&self, chave: &str) -> Result<(), ErroIa>;
      /// Envia o corpo e entrega cada linha `data:` já parseada ao `ao_evento`; para quando `cancelar` vira true.
      fn enviar(&self, chave: &str, corpo: &serde_json::Value, cancelar: &AtomicBool,
                ao_evento: &mut dyn FnMut(&serde_json::Value)) -> Result<(), ErroIa>;
  }
  pub struct ClienteHttp;            // ureq, destinos exatos, limites e prazos das Global Constraints
  pub struct ClienteFixture { pub pasta: PathBuf }
  pub struct Fonte { pub numero: u32, pub url: String, pub titulo: String, pub dominio: String,
                     pub data: Option<String>, pub trecho: String }
  pub struct Progresso { pub texto: String, pub fontes: Vec<Fonte>, pub buscando: bool, pub terminado: bool }
  pub struct Acumulador { /* blocos completos, texto exibido, fontes, stop_reason, uso */ }
  impl Acumulador { pub fn novo() -> Self; pub fn evento(&mut self, e: &serde_json::Value) -> Result<(), ErroIa>;
                    pub fn progresso(&self) -> Progresso; pub fn blocos(&self) -> &[serde_json::Value];
                    pub fn stop_reason(&self) -> Option<&str>; }
  pub fn corpo_pedido(modelo: &str, contexto: &str, busca_web: bool, mensagens: &[serde_json::Value]) -> (serde_json::Value, Option<&'static str>); // (corpo, anthropic-beta)
  pub struct ConversaResumo { pub id: i64, pub titulo: String, pub atualizada_em: String }
  pub struct MensagemGravada { pub id: i64, pub papel: String, pub texto: String, pub fontes: Vec<Fonte>, pub aviso: Option<String> }
  pub struct Conversa { pub id: i64, pub titulo: String, pub modelo: String, pub fontes_internas: Vec<FonteInterna>, pub mensagens: Vec<MensagemGravada> }
  impl Db {
      pub fn listar_conversas(&self) -> Result<Vec<ConversaResumo>, CoreError>;   // mais recente primeiro
      pub fn conversa(&self, id: i64) -> Result<Conversa, CoreError>;            // NAO_ENCONTRADO se não existe
      pub fn apagar_conversa(&self, id: i64) -> Result<(), CoreError>;
      pub fn uso_do_mes(&self, hoje: NaiveDate) -> Result<UsoMes, CoreError>;     // tokens somados e custo em centésimos de dólar
  }
  /// Uma pergunta inteira: cria a conversa se `conversa_id` é None (título = 60 primeiros caracteres
  /// da pergunta, contexto congelado), envia, trata pause_turn (até 3) e grava a pergunta e a resposta
  /// numa transação só no fim. Qualquer erro devolve Err sem gravar nada. `progresso` recebe cada atualização.
  pub fn perguntar(db: &Db, cofre: &dyn Cofre, cliente: &dyn ClienteIa, hoje: NaiveDate,
                   conversa_id: Option<i64>, pergunta: &str, cancelar: &AtomicBool,
                   progresso: &mut dyn FnMut(Progresso)) -> Result<i64, CoreError>;   // devolve o id da conversa
  pub fn salvar_chave(db: &Db, cofre: &dyn Cofre, cliente: &dyn ClienteIa, chave: &str) -> Result<Vec<ErroCampo>, CoreError>;
  pub fn remover_chave(db: &Db, cofre: &dyn Cofre) -> Result<(), CoreError>;
  ```
- **Fixtures:** `ClienteFixture.enviar` escolhe o arquivo pela última pergunta do usuário:
  - pergunta com `#busca` → `busca.sse`;
  - `#pause` → `pause.sse`, e na segunda chamada `simples.sse`;
  - `#recusa` → `recusa.sse`;
  - `#quebrado` → `quebrado.sse`;
  - `#erro401` → `ChaveRecusada`; `#erro529` → `Indisponivel`;
  - qualquer outra → `simples.sse`.
  - Os eventos são entregues com um intervalo de 20 ms entre eles, para a tela ver o texto chegando.
  - `validar_chave` devolve `ChaveRecusada` se a chave contém `recusada`, e Ok nos outros casos.
- **SSE:** `ClienteHttp` lê o corpo em blocos e junta as linhas. Só as linhas `data:` são parseadas; `event:`, linhas vazias e `ping` são ignorados.

- [ ] **Step 1: Testes que falham** (`tests/ia_conversa.rs`, com `ClienteFixture` e `CofreMemoria`):
  - `corpo_tem_os_campos_exatos`: sem `temperature`, `thinking` nem `top_p`.
    - O `system` tem dois blocos, e o segundo leva `cache_control`.
    - `output_config.effort` é `"medium"`.
    - A busca é `web_search_20260209` no Opus e `web_search_20250305` no Haiku, e não vai com a busca desligada.
    - `fallbacks` e o beta aparecem só para Opus e Sonnet.
  - `sem_chave_da_erro_e_nao_grava`.
  - `pergunta_simples_grava_pergunta_e_resposta`.
  - `busca_numera_fontes_por_url`: três citações de duas URLs viram as fontes 1 e 2. Os marcadores `[1]` e `[2]` aparecem no texto, e a data vem do `page_age`.
  - `segunda_pergunta_reenvia_blocos_identicos_e_mesmo_system`:
    - registrar os corpos enviados (um `ClienteIa` de teste que embrulha o `ClienteFixture`);
    - mudar a configuração entre as duas perguntas;
    - conferir que o `system` não mudou e que `messages[1]` é igual ao `conteudo_json` gravado.
  - `pause_turn_reenvia_e_junta`.
  - `recusa_grava_com_aviso`.
  - `erro_no_meio_nao_grava_nada` (`#erro529` e `#quebrado`).
  - `cancelar_nao_grava_nada`.
  - `chave_nunca_aparece_no_banco`: varre todas as tabelas e colunas de texto depois de salvar a chave e conversar.
  - `salvar_chave_valida_formato_e_api`.
  - `uso_do_mes_soma_e_estima_custo`.
- [ ] **Step 2:** `cargo test -p guru-core --test ia_conversa`. Esperado: FAIL.
- [ ] **Step 3:** implementar. `ClienteHttp` usa o `ureq` já presente e recusa qualquer URL que não seja uma das duas exatas. Os fixtures `.sse` seguem o formato real da API, com estes eventos:
  - `message_start`;
  - `content_block_start`;
  - `content_block_delta` com `text_delta`, `citations_delta`, `input_json_delta`, `thinking_delta` e `signature_delta`;
  - `content_block_stop`;
  - `message_delta` com `stop_reason` e `usage`;
  - `message_stop`.
- [ ] **Step 4:** `cargo test`, clippy e fmt. Esperado: PASS.
- [ ] **Step 5: Commit** com a mensagem "IA: cliente da Messages API com streaming, fontes numeradas e conversas".

### Task 3: Ponte — addon, main, preload e api

**Files:**
- Create: `crates/guru-node/src/ia.rs`, `crates/guru-node/test/ia.test.mjs`, `app/test/e2e/fixtures/ia/` (cópia dos `.sse` da Task 2)
- Modify: `crates/guru-node/src/lib.rs`, `crates/guru-node/src/mercado.rs` (`OpcoesCore` ganha `pasta_ia: Option<String>` e `cofre_memoria: Option<bool>`, ambos só para teste), `app/main.js`, `app/preload.js`, `app/renderer/js/api.js`, `app/test/unit/api.test.mjs`, `app/test/e2e/abrir.spec.js` (lista de canais), `app/test/e2e/_app.js` (passa `GURU_IA_FIXTURES` e `GURU_IA_COFRE=memoria` em todo lançamento)

**Interfaces:**
- **Addon (`Core`):**
  - `configIa()`: devolve `{ modelo, modelos: [{codigo, rotulo}], instrucoes, apenasPercentuais, nomesInstituicoes, buscaWeb, chave: string|null, uso: { tokens, custoCentesimosDolar } }`. O campo `chave` vem mascarado: `"sk-ant-…" + chave_final`.
  - `salvarConfigIa(dados)`: devolve `ErroCampoJs[]`.
  - `salvarChaveIa(chave)`: devolve `ErroCampoJs[]`. Assíncrona (`AsyncTask`), porque chama a rede.
  - `removerChaveIa()`.
  - `contextoIa()`: devolve `{ partes: ParteJs[], tokens }`.
  - `salvarParteContexto(parte, ligada, textoEditado|null)`: devolve `ErroCampoJs[]`.
  - `conversas()`, `conversa(id)` e `apagarConversa(id)`.
  - `perguntarIa(conversaId|null, pergunta)`: devolve `{ pedido: number }`. Dispara o `AsyncTask` e guarda o progresso em `Mutex<HashMap<u32, EstadoPedido>>`.
  - `iaProgresso(pedido)`: devolve `{ texto, fontes, buscando, terminado, conversaId|null, erro: {code, message}|null }`.
  - `cancelarIa(pedido)`.
  - Um pedido terminado é removido do mapa na primeira leitura que já o mostra terminado. Um pânico vira `ERRO_INTERNO`, como no `blindar`.
- **Canais e API:**
  - Os canais são `ia:config`, `ia:salvarConfig`, `ia:salvarChave`, `ia:removerChave`, `ia:contexto`, `ia:salvarParte`, `ia:conversas`, `ia:conversa`, `ia:apagarConversa`, `ia:perguntar`, `ia:progresso` e `ia:cancelar`, todos na tabela `canais`.
  - `api.ia = { config, salvarConfig, salvarChave, removerChave, contexto, salvarParte, conversas, conversa, apagarConversa, perguntar, progresso, cancelar }`.
- **Main:** com `GURU_IA_FIXTURES` e fora do app empacotado, passa `pastaIa`. Com `GURU_IA_COFRE=memoria`, passa `cofreMemoria: true`. No app empacotado, o cofre é sempre o do sistema e o cliente é sempre o `ClienteHttp`.

- [ ] **Step 1: Testes que falham** (`ia.test.mjs`, com `pastaIa` e `cofreMemoria`):
  - salvar uma chave válida faz `configIa().chave` terminar com os 4 últimos caracteres, e nenhum campo devolve a chave inteira;
  - `perguntarIa` + polling de `iaProgresso` termina com texto e `conversaId`;
  - `#erro401` termina com `erro.code === "IA_CHAVE_RECUSADA"`;
  - `cancelarIa` termina com `IA_CANCELADA`;
  - `api.test.mjs` cobre os 12 métodos.
- [ ] **Step 2:** rodar os testes. Esperado: FAIL.
- [ ] **Step 3:** implementar.
- [ ] **Step 4:** build do addon, `npm test` no addon e no app, e só `abrir.spec.js` no Playwright. Esperado: PASS.
- [ ] **Step 5: Commit** com a mensagem "IA: ponte do addon até a interface".

### Task 4: Tela Configurar IA

**Files:**
- Modify: `app/renderer/js/telas/ia.js`, a folha de estilos do renderer (só o que faltar: chave liga/desliga e bloco de contexto, com os tokens existentes)
- Create: `app/test/e2e/ia-config.spec.js`

**Tela** (layout do protótipo, tela 9):
- **Coluna da esquerda, provedores:**
  - o card "Claude (Anthropic)" com "ativo" quando há chave e, embaixo, "Modelo: <rótulo> · chave sk-ant-…XXXX";
  - os cards OpenAI, Google Gemini e "Modelo local (Ollama)" com a etiqueta "V2", não clicáveis;
  - o formulário da chave: campo `type=password` com o rótulo "Chave da API da Anthropic" e os botões "Salvar chave" e, quando houver chave, "Remover chave";
  - a frase "A chave fica guardada só neste computador, no cofre de senhas do sistema.";
  - o seletor de modelo;
  - a linha de uso do mês;
  - o card "Privacidade" com as três chaves, cada uma um `input type=checkbox role=switch`.
- **Coluna da direita:**
  - o card "Instruções para a sua IA" ("vai em toda conversa"): `textarea` com contador "N/4.000" e o botão "Salvar instruções";
  - o card "Contexto enviado à IA", com "≈ N tokens" no total e um bloco por parte. Cada bloco tem:
    - o título numerado e o resumo;
    - a chave liga/desliga, que salva na hora;
    - "editar", que abre um `textarea` com o texto, com os botões "Salvar" e "Cancelar";
    - quando editado, a etiqueta "editado por você" e o link "voltar ao automático".
- Cada mudança salva chama o núcleo e redesenha a partir do retorno.

- [ ] **Step 1: e2e que falham:**
  - salvar uma chave inválida (`abc`) mostra a mensagem de formato;
  - salvar `sk-ant-teste-recusada-0000` mostra "A Anthropic recusou esta chave.";
  - salvar uma chave boa mostra "ativo" e "sk-ant-…" com os 4 finais, e "Remover chave" volta ao estado sem chave;
  - trocar o modelo persiste depois de navegar e voltar;
  - desligar "Carteira" persiste e baixa o total de tokens;
  - editar "Perfil e objetivo" mostra "editado por você", e "voltar ao automático" tira a marca;
  - com um CDB cadastrado e "Apenas percentuais" ligada, o texto do bloco Carteira não contém "R$".
- [ ] **Step 2:** `npx playwright test ia-config.spec.js`. Esperado: FAIL.
- [ ] **Step 3:** implementar.
- [ ] **Step 4:** a suíte inteira (`npm test` e `npx playwright test`). Esperado: verde e sem janela na tela.
- [ ] **Step 5: Commit** com a mensagem "IA: tela Configurar IA".

### Task 5: Tela Assistente, documentação

**Files:**
- Modify: `app/renderer/js/telas/assistente.js` (pode criar `app/renderer/js/telas/assistente/*.js` se passar de ~250 linhas), a folha de estilos (balões, painel de fontes), `docs/mvp.md` (decisão "IA"), `docs/arquitetura.md` (Rede e Chave), `docs/roadmap.md` (fase 6 dividida em 6a e 6b, com o link do plano), `README.md` (status "fases 0 a 5 e 6a concluídas")
- Create: `app/renderer/js/markdown.js` e `app/test/unit/markdown.test.mjs`, `app/test/e2e/assistente.spec.js`

**Tela** (layout do protótipo, tela 8):
- **Sem chave:** o convite "Para conversar com a IA, configure a sua chave." e o link "Configurar IA" (`#/ia`).
- **Com chave, três colunas.**
  - **Esquerda:** o botão "Nova conversa" e a lista de conversas (título e data), com "Apagar" em cada uma (com `confirm`).
  - **Centro:**
    - os balões, com os do usuário à direita;
    - sem mensagens, três sugestões: "O que mudou no mercado esta semana?", "Minha carteira está alinhada ao meu perfil?" e "Quanto pago de IR nos meus investimentos?";
    - o campo `textarea`, onde Enter envia e Shift+Enter quebra a linha, com o botão "Enviar", que vira "Parar" durante a resposta;
    - enquanto busca, a linha "Buscando na web…";
    - o aviso fixo no rodapé.
  - **Direita**, "O que a IA está vendo", com o número de fontes:
    - "Fontes da web": número, título como link `https` (abre no navegador pelo handler que já existe), domínio e data;
    - "Seus dados": as fontes internas da conversa, com a data e "antigo" quando for o caso.
- **Polling:** `api.ia.progresso` a cada 150 ms enquanto o pedido não termina, parando quando a tela deixa de ser vigente (e então chama `cancelar`).
- **Erro:** a mensagem aparece abaixo do campo e o texto da pergunta volta para ele.
- **`markdown.js`** tem duas funções:
  - `analisarMarkdown(texto, numerosDeFontes) -> Bloco[]`, pura e sem DOM, testada em unidade. Um bloco é `{tipo: "p"|"titulo"|"ul"|"ol", itens|trechos}`, e um trecho é `{tipo: "texto"|"negrito"|"fonte", valor}`.
  - `renderizarMarkdown(blocos) -> DocumentFragment`, que monta a árvore só com `textContent` e elementos criados, coberta pelo e2e.
  - O analisador entende:
  - parágrafos;
  - `**negrito**`;
  - listas `- ` e `1. `;
  - títulos `#` a `###`, como `<strong>` num parágrafo;
  - `[n]` de uma fonte existente, que vira `<sup><a href="#fonte-n">n</a></sup>`.
  - Todo o resto vira texto. Nunca usa `innerHTML`.

- [ ] **Step 1: Testes que falham:**
  - **`markdown.test.mjs`**, sobre `analisarMarkdown`: negrito, listas, título, marcador de fonte existente (vira trecho `fonte`) e inexistente (fica como texto), e `<script>` ficando como texto literal.
  - **`assistente.spec.js`:**
    - sem chave mostra o convite;
    - com chave (salva pela tela Configurar IA), clicar numa sugestão mostra a resposta de `simples.sse` chegando, e a conversa aparece na lista;
    - `#busca` mostra os marcadores 1 e 2 e duas fontes no painel, com o link `https`;
    - uma segunda pergunta na mesma conversa funciona;
    - `#erro401` mostra a mensagem da chave e devolve o texto ao campo;
    - "Parar" no meio de `simples.sse` mostra "Resposta interrompida." e nada fica gravado;
    - apagar a conversa a tira da lista;
    - nenhum erro de console ou CSP durante tudo isso.
- [ ] **Step 2:** rodar os testes. Esperado: FAIL.
- [ ] **Step 3:** implementar a tela e os docs.
- [ ] **Step 4:** a suíte inteira:
  - `cargo test`, clippy e fmt;
  - os testes do addon;
  - `npm test`;
  - `npx playwright test`.
  - Esperado: tudo verde e sem janela na tela.
- [ ] **Step 5: Commit** com a mensagem "IA: tela Assistente com fontes numeradas e documentação".
