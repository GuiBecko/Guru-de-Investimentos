// Os workflows nunca publicam sozinhos: a release é só rascunho, por tag, e o
// electron-builder roda sempre com --publish never. Lê os .yml como texto.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const raiz = path.resolve(__dirname, "../../..");
const ler = (rel) => fs.readFileSync(path.join(raiz, rel), "utf8");
const lerSe = (rel) => (fs.existsSync(path.join(raiz, rel)) ? ler(rel) : "");

const CI = ".github/workflows/ci.yml";
const RELEASE = ".github/workflows/release.yml";
const ci = () => ler(CI);
const release = () => lerSe(RELEASE);

// Linhas sem os comentários de linha inteira (o texto dos comentários pode
// citar as palavras proibidas para explicar por que não estão lá).
const linhas = (texto) =>
  texto.split(/\r?\n/).filter((l) => !/^\s*#/.test(l));
const contar = (texto, trecho) =>
  linhas(texto).reduce((n, l) => n + l.split(trecho).length - 1, 0);

test("release_so_por_tag", () => {
  const r = release();
  assert.ok(r, `${RELEASE} não existe`);
  const corpo = linhas(r).join("\n");
  assert.match(corpo, /^on:\s*\n\s+push:\s*\n\s+tags:\s*\[\s*"v\*"\s*\]/m);
  assert.doesNotMatch(corpo, /^\s+branches:/m);
  assert.doesNotMatch(corpo, /pull_request/);
  assert.doesNotMatch(corpo, /workflow_dispatch/);
});

test("release_em_rascunho", () => {
  const r = release();
  const criacoes = linhas(r).filter((l) => l.includes("gh release create"));
  assert.equal(criacoes.length, 1, "uma única linha gh release create");
  assert.match(criacoes[0], /--draft(\s|$)/);
  assert.match(criacoes[0], /--verify-tag(\s|$)/);
  for (const [nome, texto] of [[CI, ci()], [RELEASE, r]]) {
    assert.equal(contar(texto, "gh release edit"), 0, `${nome} tem gh release edit`);
    assert.equal(contar(texto, "--draft=false"), 0, `${nome} tem --draft=false`);
  }
});

test("nunca_publica_pelo_builder", () => {
  const pacote = JSON.parse(ler("app/package.json"));
  for (const s of ["dist", "dist:dir"]) {
    assert.match(pacote.scripts[s], /--publish never$/, `script ${s} sem --publish never`);
  }
  const r = release();
  assert.ok(r, `${RELEASE} não existe`);
  for (const [nome, texto] of [[CI, ci()], [RELEASE, r]]) {
    for (const l of linhas(texto)) {
      if (l.includes("electron-builder")) {
        assert.match(l.trim(), /--publish never$/, `${nome}: ${l.trim()}`);
      }
      // `npm run dist` e `npm run dist:dir` chamam os scripts acima, que já
      // terminam com --publish never; o que vier depois de `--` só pode ser a arquitetura.
      const m = l.match(/npm run dist(:dir)?\b(.*)$/);
      if (m) {
        assert.match(m[2].trim(), /^(--\s+--(x64|arm64|\$\{\{\s*matrix\.arch\s*\}\})(\s+--publish never)?)?$/, `${nome}: ${l.trim()}`);
      }
    }
    assert.equal(contar(texto, "--publish always"), 0, `${nome} tem --publish always`);
    assert.equal(contar(texto, "onTagOrDraft"), 0, `${nome} tem onTagOrDraft`);
  }
});

test("escrita_so_no_rascunho", () => {
  assert.equal(contar(release(), "contents: write"), 1);
  assert.equal(contar(ci(), "contents: write"), 0);
});

test("ci_sem_segredos", () => {
  assert.equal(contar(ci(), "secrets."), 0);
});

test("smoke_nos_dois", () => {
  const r = release();
  assert.ok(r, `${RELEASE} não existe`);
  for (const [nome, texto] of [[CI, ci()], [RELEASE, r]]) {
    const corpo = linhas(texto).join("\n");
    assert.match(corpo, /--guru-smoke/, `${nome} sem --guru-smoke`);
    assert.match(corpo, /GURU_SMOKE_OK/, `${nome} não confere GURU_SMOKE_OK`);
  }
  // No CI, os três sistemas na pasta desempacotada.
  const corpoCi = linhas(ci()).join("\n");
  for (const caminho of [
    "dist/linux-unpacked/guru-de-investimentos",
    "dist/win-unpacked/Guru de Investimentos.exe",
    "dist/mac-arm64/Guru de Investimentos.app/Contents/MacOS/Guru de Investimentos",
  ]) {
    assert.ok(corpoCi.includes(caminho), `ci.yml sem o smoke em ${caminho}`);
  }
});

test("quatro_runners_na_release", () => {
  const corpo = linhas(release()).join("\n");
  for (const os of ["ubuntu-latest", "windows-latest", "macos-latest", "macos-15-intel"]) {
    assert.ok(corpo.includes(os), `release.yml sem ${os}`);
  }
});

// Sem certificado cadastrado, CSC_LINK vazio faria o electron-builder ler a pasta como certificado.
test("segredo_vazio_nao_chega_ao_build", () => {
  const corpo = linhas(release()).join("\n");
  assert.match(corpo, /for v in CSC_LINK CSC_KEY_PASSWORD WIN_CSC_LINK/);
  assert.match(corpo, /if \[ -z "\$\{!v\}" \]; then unset "\$v"; fi/);
});
