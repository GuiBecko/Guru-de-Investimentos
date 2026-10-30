# Arquitetura

Como o Guru de Investimentos é montado por dentro. O que o app faz está em [`mvp.md`](mvp.md). A ordem de construção está em [`roadmap.md`](roadmap.md).

## Stack

| Camada | Tecnologia |
|--------|------------|
| Janela e empacotamento | Electron |
| Interface | HTML, CSS e JavaScript puros, sem framework e sem etapa de build |
| Regras, dados e rede | Rust |
| Ponte entre os dois | Addon nativo Node com [napi-rs](https://napi.rs) |
| Banco local | SQLite, via `rusqlite` com a biblioteca embutida (`bundled`) |
| Chave da IA | Cofre do sistema operacional, via crate `keyring` |
| HTTP | `reqwest` com `rustls` |

## Camadas

```
┌──────────────────────── Electron ────────────────────────┐
│  renderer/  (HTML, CSS, JS)                              │
│     window.guru.*  ──  só o que o preload expõe          │
│        │ ipcRenderer.invoke("canal", dados)              │
│  preload.js  (contextBridge, sem lógica)                 │
│        │                                                 │
│  main.js  (janela, ciclo de vida, roteia IPC → Rust)     │
│        │ chamada de função                               │
├────────┼─────────────────────────────────────────────────┤
│  guru-node  (Rust, napi-rs: converte tipos e erros)      │
│        │                                                 │
│  guru-core  (Rust puro: domínio, SQLite, importação,     │
│              indicadores, IA)                            │
└──────────────────────────────────────────────────────────┘
```

- **`guru-core`** guarda toda regra de negócio e todo acesso a disco e rede. Não sabe que existe Electron e é testado só com `cargo test`.
- **`guru-node`** é fino. Converte tipos JS ↔ Rust, transforma erros em `Error` do JS com um `code` e roda o trabalho pesado fora da thread principal (funções `async` do napi-rs).
- **`main.js`** cria a janela, registra um handler de IPC por função exposta e repassa a chamada ao addon. Não tem regra de negócio.
- **`preload.js`** expõe `window.guru` com `contextBridge`. Cada função chama um canal fixo. A interface nunca recebe `ipcRenderer` nem nada do Node. O `main.js` responde sempre com um envelope, `{ ok: true, value }` ou `{ ok: false, code, message }`, e o preload o devolve como está, porque o Electron só deixa passar a mensagem de um erro.
- **`renderer/`** desenha as telas. Chama sempre por `renderer/js/api.js`, que transforma a falha do envelope em `Error` com `code`. Não faz requisição de rede.

### Por que addon nativo e não um processo separado

- Não abre porta local, então outro programa do computador não consegue falar com o núcleo.
- A N-API é estável entre versões do Node e do Electron, então o addon não precisa ser recompilado a cada atualização do Electron.
- O napi-rs gera os tipos TypeScript (`index.d.ts`) das funções expostas, o que documenta o contrato da ponte.
- Um `panic` no Rust vira erro JS em vez de derrubar o app.

A alternativa (binário Rust separado falando JSON por stdin/stdout) continua viável. Se o addon der problema no empacotamento, a troca fica restrita ao `guru-node` e ao `main.js`, porque o `guru-core` não muda.

## Dados

- Tudo fica na pasta de dados do usuário que o Electron informa (`app.getPath("userData")`), no arquivo `guru.db`.
- **Dinheiro em centavos, como inteiro (`i64`).** Nunca `f64` para valor em reais.
- **Taxas e percentuais em decimal exato** (`rust_decimal`), por exemplo `112.5` (% do CDI) ou `6.25` (% a.a.).
- Datas como `YYYY-MM-DD` no banco e `chrono::NaiveDate` no Rust.
- O esquema evolui por migrações numeradas dentro do `guru-core`. Um banco criado por uma versão mais nova do app não é aberto por uma mais antiga: o app avisa em vez de arriscar estragar os dados.
- A chave da IA fica no cofre do sistema (Keychain, Credential Manager, Secret Service). Nunca vai para o SQLite nem para a interface.

Dados de mercado ficam em sete tabelas, criadas pela migração `0004`:

| Tabela | Guarda |
|--------|--------|
| `indice_diario` | CDI e Selic por dia |
| `ipca_mensal` | IPCA por mês |
| `tesouro_preco` | Taxa e preço unitário de compra e base de cada título, por dia |
| `cotacao` | Preço de fechamento de cada ticker por dia, em centavos |
| `fonte_status` | Última data, última atualização e erro de cada fonte |
| `cobertura_serie` | Desde quando cada série já foi pedida, para não repetir pedidos |
| `b3_alvo` | Para cada data pedida à B3, o pregão encontrado e os tickers pedidos |

A migração `0005` acrescenta `investimentos.codigo_b3` (código do ativo na planilha, usado para casar a renda fixa) e a tabela `importacao`, com a data, o nome do arquivo, o total de linhas e quantas foram aplicadas em cada importação.

## Segurança da janela

- `contextIsolation: true`, `sandbox: true`, `nodeIntegration: false`.
- Content-Security-Policy sem scripts externos: `default-src 'self'`. As fontes IBM Plex ficam empacotadas no app.
- Links externos (fontes citadas, notícias) abrem no navegador padrão pelo `shell.openExternal`, e só com `https:`.
- Uma instância por vez (`app.requestSingleInstanceLock`), para dois processos não gravarem no mesmo banco.

## Rede

Só o `guru-core` acessa a rede, e só estes destinos. Qualquer outra URL é recusada antes da requisição. Cada requisição tem timeout de 60 s e `User-Agent: GuruDeInvestimentos/<versão>`. Nenhuma requisição leva dado do usuário: só datas e códigos de série nas URLs.

| Destino | Para quê | Fase |
|---------|----------|------|
| `api.bcb.gov.br` (SGS) | Selic e CDI; IPCA pela série 433 do SGS (dado do IBGE publicado pelo Banco Central) | 3 |
| Tesouro Transparente | Taxas e preços do Tesouro Direto | 3 |
| B3, série histórica (arquivo diário COTAHIST) | Preço de fechamento de ações, FIIs e ETFs | 3 |
| `api.anthropic.com` | Claude, com a chave do usuário | 6 |

## Atualização de mercado

O atualizador roda no addon, em uma thread de fundo com a própria conexão ao banco. Os downloads não travam a tela.

- Busca só o que falta. Uma aplicação antiga cadastrada depois faz o app buscar para trás até a data dela.
- Da B3 guarda o último pregão e o último pregão de cada um dos 12 meses anteriores.
- Cada fonte falha sozinha. A que falhou mostra a mensagem e mantém os dados que já tinha; as outras seguem.
- Uma nova tentativa depois de 5 s. Na B3 só erro de rede é repetido, porque arquivo que não existe significa dia sem pregão.
- Resposta HTML ou XML de erro do SGS, com status 200, conta como falha da fonte.

O app atualiza ao abrir, se alguma fonte não foi atualizada hoje, e de novo depois de salvar um investimento. O botão "Atualizar" da barra lateral faz a mesma coisa sob demanda.

## Importação da B3

A planilha de posição é lida e comparada com a carteira no `guru-core`. Ela nunca sai do computador: a rede não é usada nessa etapa.

- O diálogo de arquivo fica no processo principal (`main.js`). Antes de ler, o tamanho do arquivo é conferido, e o caminho nunca passa para o addon.
- O addon recebe só os bytes e o nome do arquivo, lê o XLSX (`importacao/xlsx.rs`, sobre `zip` e `quick-xml`) e guarda a última comparação sob um token. O token é consumido ao aplicar, então a mesma comparação não é gravada duas vezes.
- Aplicar roda em uma única transação: tudo ou nada.
- Limites contra arquivo malicioso: 20 MB no arquivo; 50 MB descompactados por parte do ZIP; 50 abas; 100 mil linhas; 256 colunas; 2 milhões de células; 100 MB de texto. Acima disso, ou se não for XLSX, o erro é `ARQUIVO_INVALIDO`.
- Na barra lateral, o rodapé mostra "Última importação B3: dd/mm/aaaa".

### Modo de teste da importação

Nos e2e, `GURU_TESTE_ARQUIVO_B3=<caminho>` faz o diálogo devolver esse arquivo, e `GURU_TESTE_ARQUIVO_B3=cancelar` simula o cancelamento. As duas valem só fora do app empacotado.

## Repositório

```
Guru-de-Investimentos/
├── Cargo.toml              workspace Rust
├── crates/
│   ├── guru-core/          domínio, banco, rede
│   └── guru-node/          addon napi-rs
├── app/
│   ├── package.json        Electron e scripts
│   ├── main.js
│   ├── preload.js
│   ├── renderer/
│   │   ├── index.html
│   │   ├── css/            tokens, base, telas
│   │   ├── fonts/          IBM Plex empacotada
│   │   └── js/             roteador e uma pasta por tela
│   └── test/               testes node:test e e2e
└── docs/
```

## Testes

| O quê | Como |
|-------|------|
| `guru-core` | `cargo test` |
| Ponte `guru-node` | `node --test` carregando o addon compilado |
| JS puro da interface (roteador, formatação) | `node --test` |
| App inteiro | Playwright com `_electron.launch` |

A CI (GitHub Actions) roda fmt, clippy e os testes do núcleo Rust só em Linux; o addon (build e testes) e o app (testes unitários e E2E) rodam em Linux, Windows e macOS.

### Modo de teste de mercado

Os testes do núcleo nunca acessam a rede: usam a pasta `crates/guru-core/tests/fixtures/` (dados reais de 28/10/2026) e um `Baixador` falso.

Nos e2e, o app lê os arquivos de uma pasta em vez de baixar, com duas variáveis:

- `GURU_MERCADO_FIXTURES=<pasta>`: pasta com `sgs-<serie>.json`, `tesouro.csv` e `COTAHIST_D<ddMMaaaa>.ZIP`.
- `GURU_MERCADO_HOJE=AAAA-MM-DD`: data que substitui "hoje" só nos dados de mercado (atualização e valorização). Os e2e usam `2026-10-28`.

As duas valem só fora do app empacotado.

## Em aberto

- **Modelo padrão do Claude.** Definido na fase 6, com o usuário podendo trocar.
