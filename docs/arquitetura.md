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
- **`preload.js`** expõe `window.guru` com `contextBridge`. Cada função chama um canal fixo. A interface nunca recebe `ipcRenderer` nem nada do Node.
- **`renderer/`** desenha as telas e chama `window.guru.*`. Não faz requisição de rede.

### Por que addon nativo e não um processo separado

- Não abre porta local, então outro programa do computador não consegue falar com o núcleo.
- A N-API é estável entre versões do Node e do Electron, então o addon não precisa ser recompilado a cada atualização do Electron.
- O napi-rs gera os tipos TypeScript (`index.d.ts`) das funções expostas, o que documenta o contrato da ponte.
- Um `panic` no Rust vira erro JS em vez de derrubar o app.

A alternativa (binário Rust separado falando JSON por stdin/stdout) continua viável. Se o addon der problema no empacotamento, a troca fica restrita ao `guru-node` e ao `main.js`, porque o `guru-core` não muda.

## Dados

- Tudo fica na pasta de dados do usuário que o Electron informa (`app.getPath("userData")`): `guru.db` e, depois, cache de indicadores.
- **Dinheiro em centavos, como inteiro (`i64`).** Nunca `f64` para valor em reais.
- **Taxas e percentuais em decimal exato** (`rust_decimal`), por exemplo `112.5` (% do CDI) ou `6.25` (% a.a.).
- Datas como `YYYY-MM-DD` no banco e `chrono::NaiveDate` no Rust.
- O esquema evolui por migrações numeradas dentro do `guru-core`. Um banco criado por uma versão mais nova do app não é aberto por uma mais antiga: o app avisa em vez de arriscar estragar os dados.
- A chave da IA fica no cofre do sistema (Keychain, Credential Manager, Secret Service). Nunca vai para o SQLite nem para a interface.

## Segurança da janela

- `contextIsolation: true`, `sandbox: true`, `nodeIntegration: false`.
- Content-Security-Policy sem scripts externos: `default-src 'self'`. As fontes IBM Plex ficam empacotadas no app.
- Links externos (fontes citadas, notícias) abrem no navegador padrão pelo `shell.openExternal`, e só com `https:`.
- Uma instância por vez (`app.requestSingleInstanceLock`), para dois processos não gravarem no mesmo banco.

## Rede

Só o `guru-core` acessa a rede, e só estes destinos:

| Destino | Para quê | Fase |
|---------|----------|------|
| `api.bcb.gov.br` (SGS) | Selic, CDI, IPCA | 3 |
| Tesouro Transparente | Taxas e preços do Tesouro Direto | 3 |
| IBGE (SIDRA) | IPCA oficial | 3 |
| B3, série histórica (arquivo diário COTAHIST) | Preço de fechamento de ações, FIIs e ETFs | 3 |
| `api.anthropic.com` | Claude, com a chave do usuário | 6 |

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

A CI (GitHub Actions) roda tudo em Linux, Windows e macOS.

## Em aberto

- **Modelo padrão do Claude.** Definido na fase 6, com o usuário podendo trocar.
