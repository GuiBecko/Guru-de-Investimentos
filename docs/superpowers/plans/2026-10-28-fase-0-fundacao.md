# Fase 0: Fundação — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O app abre uma janela Electron com a moldura verde granulada e o menu lateral do protótipo, carrega o núcleo Rust por um addon napi-rs, cria o banco SQLite na pasta de dados do usuário e mostra a versão do núcleo e do esquema. A CI valida tudo em Linux, Windows e macOS.

**Architecture:** Workspace Cargo com `guru-core` (Rust puro, banco e migrações) e `guru-node` (addon napi-rs). O `main.js` do Electron carrega o addon e repassa cada canal IPC a ele. O `preload.js` expõe `window.guru` por `contextBridge`. A interface é HTML, CSS e JS puros com um roteador por hash.

**Tech Stack:** Rust estável (edição 2024), `rusqlite` (feature `bundled`), `rusqlite_migration`, `thiserror`, `tempfile` (dev), napi-rs 3 (`napi`, `napi-derive`, `@napi-rs/cli`), Node 24 LTS, Electron (última estável, fixada no `package-lock.json`), Playwright (`@playwright/test`), GitHub Actions.

**Spec:** [`docs/arquitetura.md`](../../arquitetura.md) e [`docs/mvp.md`](../../mvp.md). Visual: [`docs/prototipo/index.html`](../../prototipo/index.html).

## Global Constraints

- Todo texto visível ao usuário em português do Brasil, com acentuação correta.
- Nome do produto: "Guru de Investimentos". No menu lateral, só "Guru".
- `BrowserWindow` com `contextIsolation: true`, `sandbox: true`, `nodeIntegration: false`.
- CSP da interface: `default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'`.
- Nenhuma requisição de rede nesta fase, nem da interface nem do núcleo.
- Nenhuma fonte ou script de CDN. IBM Plex Sans e IBM Plex Mono (licença OFL) ficam em `app/renderer/fonts/`.
- O banco se chama `guru.db` e fica em `app.getPath("userData")`.
- Erros que atravessam a ponte chegam ao JS como `Error` com `code` em `SCREAMING_SNAKE_CASE`.
- Todo o trabalho na branch `fase-0-fundacao`, criada a partir da `main` antes da Tarefa 1. Nada direto na `main`.
- Commits pequenos, mensagem em português, terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Pasta de dados com acento e espaço** (`C:\Users\João da Silva\AppData\...`): o banco abre normalmente. Teste na Tarefa 1 e na Tarefa 2.
2. **Banco criado por uma versão mais nova do app**: o núcleo recusa abrir com `SCHEMA_TOO_NEW` e não altera o arquivo. Teste na Tarefa 1; a tela de erro na Tarefa 3.
3. **Arquivo `guru.db` que não é SQLite** (corrompido ou outro arquivo com o mesmo nome): erro `DB_INVALID` com mensagem clara, sem apagar o arquivo. Teste na Tarefa 1.
4. **Addon ausente ou que falha ao carregar**: a janela mostra uma tela de erro em português em vez de ficar em branco. Teste na Tarefa 3.
5. **Segunda instância aberta**: não abre outra janela nem outro banco; foca a janela existente. Teste na Tarefa 3.

---

### Tarefa 1: Workspace Rust e banco do `guru-core`

**Files:**
- Create: `Cargo.toml` (workspace com `crates/*`, `resolver = "3"`)
- Create: `rust-toolchain.toml` (`channel = "stable"`, componentes `rustfmt` e `clippy`)
- Create: `crates/guru-core/Cargo.toml`
- Create: `crates/guru-core/src/lib.rs` (reexporta `Db`, `CoreError`, `CORE_VERSION`)
- Create: `crates/guru-core/src/error.rs`
- Create: `crates/guru-core/src/db.rs`
- Create: `crates/guru-core/src/migrations/0001_inicial.sql` (só a tabela `app_meta(chave TEXT PRIMARY KEY, valor TEXT NOT NULL)`)
- Create: `.gitignore` (`/target`, `node_modules/`, `*.node`, `app/test-results/`)
- Test: `crates/guru-core/tests/db.rs`

**Interfaces:**
- Produces:
  - `pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");`
  - `pub enum CoreError { SchemaTooNew { banco: u32, app: u32 }, DbInvalid(String), Io(std::io::Error), Sqlite(rusqlite::Error) }` com `pub fn code(&self) -> &'static str` devolvendo `"SCHEMA_TOO_NEW"`, `"DB_INVALID"`, `"IO"`, `"SQLITE"`.
  - `pub struct Db` com `pub fn open(data_dir: &Path) -> Result<Db, CoreError>`, `pub fn schema_version(&self) -> Result<u32, CoreError>` e `pub fn path(&self) -> &Path`.
  - `pub const SCHEMA_VERSION: u32` igual ao número de migrações (1 nesta fase).

- [ ] **Step 1: Escrever os testes que falham**

```rust
#[test]
fn abre_e_cria_o_banco_na_pasta() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(dir.path()).unwrap();
    assert_eq!(db.path(), dir.path().join("guru.db"));
    assert!(db.path().exists());
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
}

#[test]
fn reabrir_nao_reaplica_migracoes() {
    let dir = tempfile::tempdir().unwrap();
    drop(Db::open(dir.path()).unwrap());
    assert_eq!(Db::open(dir.path()).unwrap().schema_version().unwrap(), SCHEMA_VERSION);
}

#[test]
fn cria_a_pasta_se_nao_existir_e_aceita_acentos() {
    let dir = tempfile::tempdir().unwrap();
    let pasta = dir.path().join("João da Silva").join("Guru de Investimentos");
    let db = Db::open(&pasta).unwrap();
    assert!(db.path().exists());
}

#[test]
fn recusa_banco_de_versao_mais_nova_sem_alterar() {
    // cria o banco, define PRAGMA user_version = SCHEMA_VERSION + 1 por rusqlite,
    // guarda os bytes do arquivo, chama Db::open
    // espera Err(CoreError::SchemaTooNew { banco: SCHEMA_VERSION + 1, app: SCHEMA_VERSION })
    // e code() == "SCHEMA_TOO_NEW"; os bytes do arquivo continuam iguais
}

#[test]
fn recusa_arquivo_que_nao_e_sqlite_sem_apagar() {
    // escreve "isto não é um banco" em guru.db
    // espera code() == "DB_INVALID"; o conteúdo do arquivo continua igual
}
```

E um teste de unidade em `db.rs` (`#[cfg(test)] mod tests`), que enxerga a conexão privada:

```rust
#[test]
fn usa_wal_e_chaves_estrangeiras() {
    // após Db::open: PRAGMA journal_mode == "wal" e PRAGMA foreign_keys == 1
}
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p guru-core`
Expected: FAIL, não compila (`Db` não existe).

- [ ] **Step 3: Implementar `Db::open`**

Ordem dentro de `open`: `create_dir_all`; abrir a conexão; ler `PRAGMA user_version` (um erro `NotADatabase` aqui vira `DbInvalid`); se a versão do arquivo for maior que `SCHEMA_VERSION`, devolver `SchemaTooNew` antes de qualquer escrita; só então ligar `journal_mode=WAL`, `foreign_keys=ON`, `busy_timeout=5000` e aplicar as migrações com `rusqlite_migration` (que usa o próprio `user_version`).

- [ ] **Step 4: Rodar e ver passar**

Run: `cargo test -p guru-core && cargo clippy -p guru-core -- -D warnings && cargo fmt --check`
Expected: 6 testes PASS (5 de integração e 1 de unidade), sem avisos.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .gitignore crates/guru-core
git commit -m "Núcleo Rust: banco SQLite com migrações"
```

---

### Tarefa 2: Addon `guru-node`

**Files:**
- Create: `crates/guru-node/Cargo.toml` (`crate-type = ["cdylib"]`, depende de `guru-core`, `napi`, `napi-derive`; `build-dependencies` `napi-build`)
- Create: `crates/guru-node/build.rs`
- Create: `crates/guru-node/src/lib.rs`
- Create: `crates/guru-node/package.json` (nome `@guru/core-node`, privado, scripts `build` = `napi build --platform --release --js index.js --dts index.d.ts`)
- Generated (no commit): `crates/guru-node/index.js`, `index.d.ts`, `*.node`
- Test: `crates/guru-node/test/core.test.mjs`

**Interfaces:**
- Consumes: `Db::open`, `Db::schema_version`, `Db::path`, `CoreError::code`, `CORE_VERSION` (Tarefa 1).
- Produces (JS):
  - `class Core { static open(dataDir: string): Core; info(): AppInfo }`
  - `interface AppInfo { coreVersion: string; schemaVersion: number; dbPath: string }`
  - Erros: `Error` com `code` igual a `CoreError::code()` e `message` em português.

`Core` guarda o `Db` atrás de um `Mutex`. `info` é síncrono nesta fase (uma leitura de pragma). Funções lentas das fases seguintes serão `async`.

Gere `index.js` e `index.d.ts` no build e deixe-os fora do Git (adicione ao `.gitignore`), para o contrato vir sempre do Rust.

- [ ] **Step 1: Escrever o teste que falha**

```js
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
const { Core } = createRequire(import.meta.url)("../index.js");

test("abre o banco e informa as versões", () => {
  const dir = join(mkdtempSync(join(tmpdir(), "guru-")), "João da Silva");
  const info = Core.open(dir).info();
  assert.equal(info.schemaVersion, 1);
  assert.match(info.coreVersion, /^\d+\.\d+\.\d+/);
  assert.equal(info.dbPath, join(dir, "guru.db"));
});

test("erro do núcleo chega com code", () => {
  const dir = mkdtempSync(join(tmpdir(), "guru-"));
  writeFileSync(join(dir, "guru.db"), "isto não é um banco");
  assert.throws(() => Core.open(dir), { code: "DB_INVALID" });
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cd crates/guru-node && npm install && node --test test/`
Expected: FAIL, `Cannot find module '../index.js'`.

- [ ] **Step 3: Implementar `Core` em `src/lib.rs`**

As funções devolvem `napi::Result<T, String>`: o napi-rs usa o status genérico como `code` do erro JS, então `napi::Error::new(e.code().to_string(), e.to_string())` entrega `code: "DB_INVALID"`. As mensagens de `CoreError` (`#[error("...")]`) são em português.

- [ ] **Step 4: Rodar e ver passar**

Run: `cd crates/guru-node && npm run build && node --test test/`
Expected: 2 testes PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/guru-node .gitignore Cargo.lock
git commit -m "Ponte napi-rs entre o núcleo e o Node"
```

---

### Tarefa 3: Casca do Electron e IPC

**Files:**
- Create: `app/package.json` (`"main": "main.js"`, `"type": "commonjs"`, dependência local `"@guru/core-node": "file:../crates/guru-node"`, scripts `start`, `test` = `node --test test/unit/`, `e2e` = `playwright test`)
- Create: `app/main.js`
- Create: `app/preload.js`
- Create: `app/renderer/index.html` (provisório: mostra as versões; a Tarefa 4 troca pela moldura)
- Create: `app/renderer/erro.html` (tela de erro estática, sem script)
- Create: `app/renderer/js/app.js`
- Create: `app/playwright.config.js`
- Test: `app/test/e2e/abrir.spec.js`

**Interfaces:**
- Consumes: `Core.open`, `Core#info`, `AppInfo` (Tarefa 2).
- Produces:
  - `window.guru.app.info(): Promise<AppInfo>` (canal IPC `app:info`).
  - Variável de ambiente `GURU_DATA_DIR`: se definida, substitui `app.getPath("userData")`. Usada pelos testes para isolar o banco.
  - Variável de ambiente `GURU_ADDON_PATH`: se definida, carrega o addon desse caminho. Usada pelo teste de addon ausente.

Comportamento de `main.js`:
- `requestSingleInstanceLock()`; sem o lock, `app.quit()`. No evento `second-instance`, restaurar e focar a janela.
- Janela com título "Guru de Investimentos", 1280×800, mínimo 1100×700, as `webPreferences` das Global Constraints.
- Carregar o addon e abrir o banco dentro de `try`. Se falhar, carregar `erro.html` com a query `?code=<code>` e não registrar handlers. Sem `code` (addon ausente), usar `ADDON_INDISPONIVEL`.
- `erro.html` mostra "Não foi possível abrir o Guru de Investimentos." e uma frase por código: `SCHEMA_TOO_NEW` → "Seus dados foram salvos por uma versão mais nova do app. Atualize o Guru de Investimentos para continuar."; `DB_INVALID` → "O arquivo de dados está danificado. Nada foi apagado."; `ADDON_INDISPONIVEL` e demais → "O núcleo do app não carregou. Reinstale o Guru de Investimentos." Como a CSP não deixa script inline, a leitura do código vai em `renderer/js/erro.js`.
- `will-navigate` e `setWindowOpenHandler` bloqueiam navegação; links `https:` vão para `shell.openExternal`.

- [ ] **Step 1: Escrever os testes e2e que falham**

```js
// abrir.spec.js — cada teste usa um GURU_DATA_DIR temporário próprio
test("abre com título e mostra a versão do esquema", async () => {
  // _electron.launch({ args: ["."], cwd: app, env: { ...process.env, GURU_DATA_DIR } })
  // expect(await page.title()).toBe("Guru de Investimentos")
  // expect(page.getByTestId("schema-version")).toHaveText("1")
  // guru.db existe em GURU_DATA_DIR
});

test("a interface não tem acesso ao Node", async () => {
  // page.evaluate(() => typeof require) === "undefined"
  // page.evaluate(() => typeof process) === "undefined"
});

test("banco de versão mais nova mostra a tela de erro", async () => {
  // antes de lançar: new DatabaseSync(join(GURU_DATA_DIR, "guru.db")).exec("PRAGMA user_version = 99")
  // (node:sqlite, já incluso no Node 24)
  // expect(page.getByText("Atualize o Guru de Investimentos")).toBeVisible()
});

test("addon ausente mostra a tela de erro", async () => {
  // GURU_ADDON_PATH aponta para um caminho que não existe
  // expect(page.getByText("O núcleo do app não carregou")).toBeVisible()
});

test("segunda instância não abre outra janela", async () => {
  // lança a primeira; lança a segunda com o mesmo GURU_DATA_DIR;
  // a segunda encerra sozinha (processo sai) e a primeira continua com 1 janela
});
```

Para a segunda instância, o lock do Electron é por `userData`; defina `app.setPath("userData", GURU_DATA_DIR)` antes de pedir o lock, para os testes isolados não brigarem entre si.

- [ ] **Step 2: Rodar e ver falhar**

Run: `cd app && npm install && npx playwright test` (em Linux sem tela: `xvfb-run -a npx playwright test`)
Expected: FAIL, `main.js` não existe.

- [ ] **Step 3: Implementar `main.js`, `preload.js`, `index.html`, `app.js`, `erro.html`, `erro.js`**

`preload.js` expõe só `{ app: { info: () => ipcRenderer.invoke("app:info") } }`. `app.js` chama `guru.app.info()` e preenche elementos com `data-testid="core-version"` e `data-testid="schema-version"`.

- [ ] **Step 4: Rodar e ver passar**

Run: `cd app && npx playwright test`
Expected: 5 testes PASS.

- [ ] **Step 5: Commit**

```bash
git add app
git commit -m "Casca do Electron com IPC para o núcleo e tela de erro"
```

---

### Tarefa 4: Moldura visual e roteador

**Files:**
- Create: `app/renderer/fonts/` (IBM Plex Sans 300/400/500 e IBM Plex Mono 400/500 em `woff2`, com `OFL.txt`)
- Create: `app/renderer/css/tokens.css` (bloco `:root` e os dois blocos de tema escuro do protótipo, `docs/prototipo/index.html:13-60`, incluindo `--grain`, `--gridline` e `--pix`)
- Create: `app/renderer/css/base.css` (`@font-face`, corpo, `.win`, `.side`, painel de conteúdo, `.card`, `.btn`; copie as regras do protótipo, `docs/prototipo/index.html:110-320`, e remova o que é só da página de apresentação)
- Create: `app/renderer/js/router.js`
- Create: `app/renderer/js/telas/` (um módulo por item do menu, cada um com `export function render(el)` que escreve o título da tela e "Em construção")
- Modify: `app/renderer/index.html` (moldura + menu + área de conteúdo; as versões vão para um rodapé discreto do menu, mantendo os `data-testid`)
- Modify: `app/renderer/js/app.js`
- Test: `app/test/unit/router.test.mjs`, `app/test/e2e/navegar.spec.js`

**Interfaces:**
- Consumes: `window.guru.app.info()` (Tarefa 3).
- Produces:
  - `export const TELAS` em `router.js`, nesta ordem: `[{ id: "carteira", titulo: "Carteira" }, { id: "aporte", titulo: "Próximo aporte" }, { id: "assistente", titulo: "Assistente" }, { id: "investimentos", titulo: "Investimentos" }, { id: "perfil", titulo: "Perfil e objetivo" }, { id: "ia", titulo: "Configurar IA" }]`
  - `export function resolver(hash: string): Tela` — `"#/aporte"` → a tela `aporte`; vazio, `"#/"` ou id desconhecido → `carteira`.
  - `export function iniciar(raiz: HTMLElement): void` — escuta `hashchange`, marca o item `.on` no menu e chama o `render` da tela.

A barra de título do protótipo (`.win-bar`, com as três bolinhas) não entra: o Electron usa a barra nativa do sistema. A moldura verde granulada passa a ser o fundo da janela inteira.

- [ ] **Step 1: Escrever os testes que falham**

```js
// router.test.mjs
test("resolve cada tela pelo hash", () => {
  assert.equal(resolver("#/aporte").id, "aporte");
  assert.equal(resolver("#/ia").titulo, "Configurar IA");
});
test("hash vazio ou desconhecido cai na carteira", () => {
  for (const h of ["", "#", "#/", "#/nao-existe"]) assert.equal(resolver(h).id, "carteira");
});
test("menu na ordem do protótipo", () => {
  assert.deepEqual(TELAS.map(t => t.titulo),
    ["Carteira", "Próximo aporte", "Assistente", "Investimentos", "Perfil e objetivo", "Configurar IA"]);
});
```

```js
// navegar.spec.js
test("clicar no menu troca a tela e marca o item", async () => {
  // clica em "Assistente"; a área de conteúdo mostra o título "Assistente";
  // o item do menu tem a classe "on" e só ele
});
test("nenhum erro de CSP no console", async () => {
  // coleta page.on("console") de tipo "error" ao abrir e navegar por todas as telas; lista vazia
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cd app && npm test && npx playwright test navegar`
Expected: FAIL, `router.js` não existe.

- [ ] **Step 3: Implementar tokens, base, roteador e telas provisórias**

- [ ] **Step 4: Rodar e ver passar, e olhar**

Run: `cd app && npm test && npx playwright test`
Expected: todos PASS. Depois tire uma captura com `page.screenshot` nos temas claro e escuro e compare com a moldura do protótipo: verde `#04321F` granulado, menu à esquerda, painel claro quadriculado com cantos em pixel.

- [ ] **Step 5: Commit**

```bash
git add app
git commit -m "Moldura verde, menu lateral e roteador das telas"
```

---

### Tarefa 5: CI

**Files:**
- Create: `.github/workflows/ci.yml`
- Modify: `README.md` (seção "Rodando localmente": pré-requisitos Rust estável e Node 24; comandos `cd crates/guru-node && npm install && npm run build`, `cd app && npm install && npm start`)

**Interfaces:**
- Consumes: os comandos de teste das Tarefas 1 a 4.

Job `rust` (só Linux): `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`.
Job `app` em matriz `ubuntu-latest`, `windows-latest`, `macos-latest`: Rust estável, Node 24, cache de Cargo e npm, build do addon, `node --test` do addon, `npm test` e Playwright do app (no Linux com `xvfb-run -a`). Roda em `push` para `main` e em `pull_request`.

- [ ] **Step 1: Escrever o workflow**

- [ ] **Step 2: Validar localmente a sintaxe**

Run: `npx --yes @action-validator/cli .github/workflows/ci.yml` (ou `actionlint` se instalado)
Expected: sem erros.

- [ ] **Step 3: Commit, push numa branch e abrir PR**

```bash
git add .github README.md
git commit -m "CI em Linux, Windows e macOS"
git push -u origin fase-0-fundacao
```

Expected: os jobs `rust` e `app` (3 sistemas) ficam verdes no PR.
