// A versão do app/package.json é a fonte: os crates, o pacote do addon e o Cargo.lock acompanham.
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const raiz = path.resolve(__dirname, "../../..");
const ler = (rel) => fs.readFileSync(path.join(raiz, rel), "utf8");
const pacoteApp = JSON.parse(ler("app/package.json"));
const versao = pacoteApp.version;

function versaoDoToml(rel) {
  const m = ler(rel).match(/^\[package\][^[]*?^version\s*=\s*"([^"]+)"/m);
  assert.ok(m, `sem version em ${rel}`);
  return m[1];
}

function versaoNoLock(nome) {
  const blocos = ler("Cargo.lock").split("[[package]]");
  const bloco = blocos.find((b) => new RegExp(`^name = "${nome}"$`, "m").test(b));
  assert.ok(bloco, `${nome} fora do Cargo.lock`);
  return bloco.match(/^version = "([^"]+)"$/m)[1];
}

test("a versão do app tem o formato X.Y.Z", () => {
  assert.match(versao, /^\d+\.\d+\.\d+$/);
});

test("crates, pacote do addon e Cargo.lock têm a versão do app", () => {
  assert.equal(versaoDoToml("crates/guru-core/Cargo.toml"), versao);
  assert.equal(versaoDoToml("crates/guru-node/Cargo.toml"), versao);
  assert.equal(JSON.parse(ler("crates/guru-node/package.json")).version, versao);
  assert.equal(versaoNoLock("guru-core"), versao);
  assert.equal(versaoNoLock("guru-node"), versao);
});

test("electron, electron-builder e electron-updater ficam em versão exata", () => {
  const todas = { ...pacoteApp.dependencies, ...pacoteApp.devDependencies };
  for (const nome of ["electron", "electron-builder", "electron-updater"]) {
    assert.ok(todas[nome], `${nome} fora do package.json`);
  }
  // O electron-updater roda no app instalado: vai em dependencies, não em devDependencies.
  assert.equal(pacoteApp.dependencies["electron-updater"], "6.8.10");
  for (const nome of ["electron", "electron-builder", "electron-updater"]) {
    assert.doesNotMatch(todas[nome], /^[\^~]/, `${nome} com faixa: ${todas[nome]}`);
  }
});
