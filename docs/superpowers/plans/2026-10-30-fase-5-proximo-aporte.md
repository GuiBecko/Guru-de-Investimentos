# Fase 5: Próximo aporte — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Na tela Próximo aporte, o usuário digita quanto vai investir e o app mostra quanto colocar em cada classe para a carteira chegar mais perto da alocação-alvo, sem vender nada. A conta é feita no núcleo, sem IA, e explica cada número.

**Architecture:**
- No núcleo, um módulo `aporte` com duas partes:
  - uma função pura que divide o valor;
  - um método do `Db` que lê o resumo da carteira e a alocação-alvo e chama essa função.
- O addon expõe `proximoAporte(valorCentavos)`, síncrono como `resumoCarteira`.
- O main ganha o canal `aporte:dividir` e a interface ganha `api.aporte.dividir`.
- A tela troca o "em construção" por um formulário e um card por classe.

**Tech Stack:** o mesmo das fases anteriores, sem dependência nova.

**Spec:**
- [`docs/mvp.md`](../../mvp.md): tela 7. O chat por classe fica para a fase 6.
- Protótipo: `docs/prototipo/index.html:800-845` (tela 7). O seletor "Para qual meta?" e o chat lateral ficam fora desta fase.

## Global Constraints

- **Tudo das fases anteriores continua valendo:**
  - segurança da janela e CSP;
  - nada de `innerHTML` com dados nem `style=`;
  - `vigente()` depois de cada await;
  - pt-BR;
  - códigos de erro em SCREAMING_SNAKE_CASE;
  - preload sem lógica;
  - IPC com checagem de origem;
  - dinheiro em centavos `i64`;
  - e2e com a janela escondida (`GURU_JANELA_OCULTA=1`), sem `show()` nem `showInactive()`.
- **Só aporte, nunca venda.** A sugestão de cada classe vale 0 ou mais, e a soma das sugestões é exatamente o valor digitado, ao centavo.
- **O valor de cada classe** é o `total_atual_centavos` de `Db::resumo_carteira(data_mercado)`. Investimento sem preço conta pelo valor aplicado, a mesma regra do painel.
- **Valor aceito:** de 100 até `MAX_CENTAVOS` (`crates/guru-core/src/investimento/validacao.rs:34`). Fora disso, o núcleo devolve `ErroCampo { campo: "valorCentavos" }` com uma destas mensagens:
  - abaixo de 100: "Informe um valor de pelo menos R$ 1,00.";
  - acima do limite: "Valor alto demais.".
- **Texto que `paraCentavos` não entende** (vazio, letras, sinal, `1.000,5`) é barrado na tela, que mostra "Informe o valor em reais, como 2.000,00." e não chama o núcleo.
- **Sem alocação-alvo:** o núcleo devolve o estado `sem_alvo`. A tela mostra "Para dividir o aporte, defina a sua alocação-alvo." e um botão "Definir alocação-alvo", que leva para `#/perfil/alocacao`.
- **Aviso fixo no rodapé da tela:** "Cálculo de rebalanceamento pela sua alocação-alvo, sem vender nada. Investimentos sem cotação contam pelo valor aplicado. Não é recomendação individual de investimento."
- **Nada é gravado.** O cálculo não tem histórico; o histórico de decisões de aporte é da V2.
- Trabalho na branch `fase-5-proximo-aporte`. Commits em português terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Regra da divisão (exata)

Entradas:
- `A`, o valor do aporte;
- para cada classe `c`: `V_c`, o valor atual (negativo conta como 0), e `t_c`, o alvo em % (soma 100).

Só as classes com `t_c > 0` recebem dinheiro.

1. **Ordenar** essas classes por `V_c / t_c` crescente. A comparação é `V_a·t_b < V_b·t_a` em `i128`; no empate, vale a ordem de `Classe::TODOS`.
2. **Achar o maior `k`** tal que `(A + ΣV_{1..k})·t_k ≥ V_k·Σt_{1..k}`. As `k` primeiras classes são as que recebem.
   - Ideia: todas sobem até a mesma fração `L` do seu alvo, com `L = (A + ΣV)/Σt`.
   - As que já estão acima dessa fração ficam com 0.
3. **Calcular a parte exata** de cada classe `c` que recebe: `x_c = ((A + ΣV)·t_c − V_c·Σt) / Σt`, em `i128`.
   - Cada `x_c` é arredondado para baixo.
   - Os centavos que sobram (`A − Σ⌊x_c⌋`, sempre menos que `k`) vão, um a um, para os maiores restos. No empate, vale a ordem de `Classe::TODOS`.
4. **Percentuais:**
   - atual = `decimos(V_c, ΣV_todas)`;
   - depois = `decimos(V_c + x_c, ΣV_todas + A)`.
   - `decimos` é o de `investimento::repositorio`; ele passa a ser `pub(crate)` visível ao módulo `aporte`.
5. **Situação de cada classe**, calculada antes do aporte com `d = atual_decimos − 10·t_c`:
   - `t_c = 0` → `ForaDoAlvo`;
   - `d ≤ −10` → `Abaixo`;
   - `d ≥ 10` → `Acima`;
   - senão → `Perto`.
   - Com a carteira vazia, toda classe com alvo fica `Abaixo`.
6. **Classes listadas:** as que têm `t_c > 0` ou `V_c > 0`. Ordem: sugestão decrescente; no empate, a ordem de `Classe::TODOS`.

Exemplo, que vira teste:
- Entrada: `A = 200000`, RF `V = 600000, t = 40`, Ações `V = 200000, t = 40`, FIIs `V = 200000, t = 20`.
- Ordem: Ações (5000), RF (15000) e FIIs (10000) → Ações, FIIs, RF.
- Com k=1: L = (200000+200000)/40 = 10000. Isso é ≥ 5000, então Ações entra.
- Com k=2: ΣV = 400000, Σt = 60, L = 600000/60 = 10000. Isso é ≥ 10000, então FIIs entra.
- Com k=3: L = 1200000/100 = 12000. Isso é < 15000, então RF não entra.
- Resultado: Ações 200000, FIIs 0, RF 0.
  - FIIs entra no conjunto, mas a parte dela é 0: ela está exatamente na fração `L`.
  - Depois do aporte: Ações 33,3%, RF 50,0%, FIIs 16,7%.

## Review Focus

1. **Carteira vazia com alvo definido:** divide exatamente pelo alvo. Com R$ 1.000,00 e alvo 40/40/20, sai 400,00/400,00/200,00.
2. **Valor mínimo** (R$ 1,00) com alvo em cinco classes que não divide exato: a soma dá 100 centavos, nenhuma parte é negativa, e as sobras seguem a regra dos maiores restos.
3. **Classe com posição e alvo 0** (por exemplo, cripto em "Outros" com alvo 0): recebe 0, aparece como "Fora da sua alocação-alvo", e o percentual depois cai.
4. **Valor no limite** (`MAX_CENTAVOS`) com patrimônio grande: nenhum estouro e nenhum pânico, porque toda conta é feita em `i128`, com resultado que cabe em `i64`.
5. **Trocar de tela durante a conta, ou mudar a alocação no Perfil e voltar:**
   - a tela velha não escreve nada (`vigente()`);
   - a nova conta usa o alvo atual, porque cada clique em "Dividir" chama o núcleo de novo.

---

### Task 1: Núcleo — divisão do aporte

**Files:**
- Create: `crates/guru-core/src/aporte/mod.rs`
- Modify: `crates/guru-core/src/lib.rs` (registra o módulo e reexporta os tipos)
- Modify: `crates/guru-core/src/investimento/repositorio.rs` (deixa `decimos` acessível ao crate, se ainda não estiver)
- Test: `crates/guru-core/tests/aporte.rs`

**Interfaces:**
- Consumes: `Db::resumo_carteira(hoje) -> Result<Resumo, CoreError>`, `Db::alocacao_alvo() -> Result<Option<Vec<(Classe, u8)>>, CoreError>`, `ErroCampo`, `MAX_CENTAVOS`, `Classe::{TODOS, codigo, rotulo}`.
- Produces:
  ```rust
  pub enum Situacao { Abaixo, Perto, Acima, ForaDoAlvo }   // codigo(): "abaixo" | "perto" | "acima" | "fora_do_alvo"
  pub struct ParteAporte {
      pub classe: Classe,
      pub atual_centavos: i64,
      pub alvo_percentual: u8,
      pub sugerido_centavos: i64,
      pub percentual_atual_decimos: i64,
      pub percentual_depois_decimos: i64,
      pub situacao: Situacao,
  }
  pub struct Aporte { pub valor_centavos: i64, pub patrimonio_centavos: i64, pub partes: Vec<ParteAporte> }
  pub enum ResultadoAporte { SemAlvo, Invalido(Vec<ErroCampo>), Dividido(Aporte) }
  pub fn dividir_aporte(valor_centavos: i64, classes: &[(Classe, i64, u8)]) -> Aporte  // (classe, atual, alvo)
  impl Db { pub fn proximo_aporte(&self, hoje: NaiveDate, valor_centavos: i64) -> Result<ResultadoAporte, CoreError> }
  ```
  `dividir_aporte` recebe só as classes que devem ser listadas e um valor já validado. `proximo_aporte`:
  - valida o valor antes de ler qualquer coisa;
  - devolve `SemAlvo` quando `alocacao_alvo()` for `None`;
  - junta as cinco classes de `Classe::TODOS` com o resumo (classe ausente no resumo = 0).

- [ ] **Step 1: Escrever os testes que falham** em `tests/aporte.rs`:
  - `exemplo_do_plano`: o exemplo da seção "Regra da divisão", com as sugestões e os `percentual_depois_decimos` 333, 500 e 167.
  - `carteira_vazia_divide_pelo_alvo`: 100000 com 40/40/20 → 40000/40000/20000.
  - `soma_exata_no_valor_minimo`: 100 com alvo 20/20/20/20/20 e carteira vazia. Soma = 100 e cada parte = 20.
  - `soma_exata_com_restos`: 100 com alvo 34/33/33 e carteira vazia. Soma = 100, partes 34/33/33.
  - `restos_desempatam_pela_ordem_das_classes`: 1 centavo com alvo 50/50 e carteira vazia → o centavo vai para a classe que vem primeiro em `Classe::TODOS`.
  - `classe_acima_do_alvo_nao_recebe`: com RF 90% e alvo 50, a sugestão de RF é 0 e a situação é `Acima`.
  - `aporte_grande_preenche_todas_as_classes`: com um valor que cobre todos os déficits, os percentuais depois ficam a ±1 décimo do alvo.
  - `classe_com_alvo_zero_fica_fora`: "Outros" com posição e alvo 0. Sugestão 0, `ForaDoAlvo`, e a classe aparece na lista.
  - `situacao_perto_dentro_de_um_ponto`: atual 20,5% com alvo 20 → `Perto`; atual 21,0% → `Acima`.
  - `ordem_por_sugestao_decrescente`.
  - `valor_no_limite_nao_estoura`: `MAX_CENTAVOS` com patrimônio de `MAX_CENTAVOS / 2`. Soma exata e nenhum pânico.
  - `db_sem_alvo_devolve_sem_alvo`, `db_valor_invalido_devolve_erros` (0, 99 e `MAX_CENTAVOS + 1`, com as mensagens exatas das Global Constraints) e `db_usa_valor_atual_do_resumo`, este último com dois investimentos lançados e o alvo salvo pelas APIs públicas.

- [ ] **Step 2:** `cargo test -p guru-core --test aporte`. Esperado: FAIL, porque o módulo `aporte` ainda não existe.

- [ ] **Step 3: Implementar** `dividir_aporte` com a regra exata acima, e `Db::proximo_aporte`. Toda a aritmética intermediária é em `i128`. O resultado de cada parte é ≤ `A`, então cabe em `i64`.

- [ ] **Step 4:** rodar `cargo test -p guru-core`, `cargo clippy --all-targets -- -D warnings` e `cargo fmt --check`. Esperado: tudo passa.

- [ ] **Step 5: Commit** com a mensagem "Próximo aporte: divisão por rebalanceamento no núcleo".

### Task 2: Ponte — addon, main, preload e api

**Files:**
- Create: `crates/guru-node/src/aporte.rs`
- Modify: `crates/guru-node/src/lib.rs`, `app/main.js`, `app/preload.js`, `app/renderer/js/api.js`
- Test: o arquivo de testes do addon que já cobre `resumoCarteira` (seguir o padrão existente) e `app/test/unit/api.test.mjs`

**Interfaces:**
- Consumes: `Db::proximo_aporte`, `ResultadoAporte`.
- Produces:
  - `Core#proximoAporte(valorCentavos: i64) -> ResultadoAporteJs`, com `#[napi(object, use_nullable = true)]`:
    ```
    { estado: "sem_alvo" | "invalido" | "dividido", erros: ErroCampoJs[], aporte: AporteJs | null }
    AporteJs  { valorCentavos, patrimonioCentavos, partes: ParteAporteJs[] }
    ParteAporteJs { classe, rotulo, atualCentavos, alvoPercentual, sugeridoCentavos,
                    percentualAtualDecimos, percentualDepoisDecimos, situacao }
    ```
  - O valor da conta é `data_mercado()`, como em `resumo_carteira`.
  - Canal `"aporte:dividir": (valorCentavos) => nucleo().proximoAporte(valorCentavos)` na tabela `canais`.
  - `window.guru.aporte.dividir` no preload.
  - `api.aporte = { dividir: (valorCentavos) => desembrulhar(window.guru.aporte.dividir(valorCentavos)) }`.

- [ ] **Step 1: Escrever os testes que falham.**
  - **Addon:**
    - sem alvo → `estado: "sem_alvo"`, `aporte: null`;
    - valor 0 → `"invalido"` com `erros[0].campo === "valorCentavos"`;
    - alvo salvo, carteira vazia e 100000 → `"dividido"` com a soma das `sugeridoCentavos` igual a 100000.
  - **`api.test.mjs`:** `api.aporte.dividir` repassa o valor e desembrulha o resultado, seguindo o padrão dos outros grupos.
- [ ] **Step 2:** rodar os testes. Esperado: FAIL.
- [ ] **Step 3:** implementar as quatro camadas acima.
- [ ] **Step 4:** fazer o build do addon e rodar os testes do addon e `npm test` em `app/`. Esperado: PASS.
- [ ] **Step 5: Commit** com a mensagem "Próximo aporte: ponte do addon até a interface".

### Task 3: Tela Próximo aporte

**Files:**
- Modify: `app/renderer/js/telas/aporte.js`, que troca o `emConstrucao` pela tela
- Modify: `app/renderer/js/telas/carteira/painel.js`, que exporta `COR_CLASSE` para a tela reusar as cores (se ainda não exporta)
- Modify: a folha de estilos do renderer, só se faltar alguma classe. Usar os tokens existentes (`card`, `row`, `mono`, `mid`, `marca f-cN`, `legal`, `field`, `affix`).
- Create: `app/test/e2e/aporte.spec.js`
- Modify: `docs/mvp.md` (decisão "Próximo aporte"), `docs/roadmap.md` (link do plano), `README.md` (status "fases 0 a 5 concluídas")

**Interfaces:**
- Consumes: `api.aporte.dividir`, `api.alocacao.ler` (para mostrar o convite antes de qualquer conta), `paraCentavos`, `formatarReais`, `formatarDecimos`.

**Tela:**
- Título "Próximo aporte".
- **Sem alocação-alvo** (`api.alocacao.ler()` é `null`): só o convite das Global Constraints, sem formulário.
- **Formulário:**
  - rótulo "Quanto vai aportar?", campo com o prefixo `R$` e `inputmode="decimal"`, e o botão "Dividir";
  - Enter envia;
  - o erro aparece abaixo do campo, com `aria-invalid` e `aria-describedby`, como nos outros formulários.
- **Resultado:**
  - Linha de resumo: "Patrimônio de R$ X passa a R$ Y com o aporte de R$ Z.".
  - Um card por parte, com estes elementos:
    - a marca da cor da classe, o rótulo e a sugestão em `mono mid`;
    - a linha "atual A% → depois B% · alvo C%";
    - o texto da situação.
  - O texto da situação depende do caso:
    - `abaixo`: "N p.p. abaixo do alvo." (N = `formatarDecimos(|d|)` sem o `%`);
    - `acima`: "N p.p. acima do alvo.";
    - `perto`: "Perto do alvo.";
    - `fora_do_alvo`: "Fora da sua alocação-alvo.";
    - quando a sugestão é 0, acrescenta-se " Nada neste aporte.".
  - Com patrimônio 0, aparece a linha extra "Carteira vazia: o aporte foi dividido pela alocação-alvo.".
  - Os cards não são clicáveis nesta fase; o chat por classe é da fase 6.
- **O aviso fixo** fica no rodapé, com a classe `legal`.
- **Cada envio** chama o núcleo de novo. O resultado anterior fica na tela até chegar o novo, e é descartado se a tela deixou de ser vigente.

- [ ] **Step 1: Escrever os e2e que falham** em `aporte.spec.js`, usando `lancar()` de `_app.js` com a janela escondida:
  - `sem alvo mostra o convite e leva ao perfil`: clica em "Definir alocação-alvo" e confere o hash `#/perfil/alocacao`.
  - `divide pela alocação com a carteira vazia`: salva o alvo 40/40/20 pela tela de Perfil e digita `1.000,00`. Confere os cards com R$ 400,00, R$ 400,00 e R$ 200,00, e o texto "Carteira vazia".
  - `classe acima do alvo recebe zero`: lança um CDB de R$ 9.000,00 e uma ação, digita `1.000,00` e confere que Renda fixa mostra R$ 0,00 e "Nada neste aporte.".
  - `valor inválido mostra erro sem calcular`: testa `abc` e depois `0,50`. O primeiro dá a mensagem da tela; o segundo dá "Informe um valor de pelo menos R$ 1,00.". Nenhum card aparece.
  - `trocar de tela no meio não quebra`: envia e troca para Carteira logo em seguida. A página não registra erro (`pageerror`).
  - Também acrescentar `aporte` à verificação de CSP de `navegar.spec.js`, se a lista de telas dele ainda não cobrir o novo conteúdo.
- [ ] **Step 2:** `npx playwright test aporte.spec.js` em `app/`. Esperado: FAIL.
- [ ] **Step 3:** implementar a tela e atualizar os docs:
  - `mvp.md` recebe a linha "Próximo aporte" na tabela de decisões, resumindo a regra da divisão, o "só aporte, nunca venda" e o "sem histórico".
- [ ] **Step 4:** rodar a suíte inteira:
  - `cargo test`, clippy e fmt;
  - os testes do addon;
  - `npm test`;
  - `npx playwright test`.
  - Esperado: tudo verde, sem nenhuma janela aparecendo na tela.
- [ ] **Step 5: Commit** com a mensagem "Próximo aporte: tela, testes de ponta a ponta e documentação".
