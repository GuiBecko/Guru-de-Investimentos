import { test } from "node:test";
import assert from "node:assert/strict";
import { licencaAceita, montarTexto } from "../../scripts/avisos-terceiros.mjs";

test("licencaAceita: OR basta uma, AND exige todas, WITH só a exceção listada", () => {
  for (const ok of [
    "MIT",
    "MIT OR Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT",
    "(MIT OR Apache-2.0) AND Unicode-3.0",
    "Apache-2.0 AND ISC",
    "MIT OR Apache-2.0 OR LGPL-2.1-or-later",
    "Unlicense OR MIT",
    "MIT/Apache-2.0",
    "OFL-1.1",
  ]) {
    assert.equal(licencaAceita(ok), true, ok);
  }
  for (const nao of [
    "GPL-3.0",
    "MIT AND GPL-3.0",
    "GPL-2.0 WITH Classpath-exception-2.0",
    "MIT WITH LLVM-exception",
    "",
    null,
    undefined,
    "(MIT OR",
  ]) {
    assert.equal(licencaAceita(nao), false, String(nao));
  }
});

test("montarTexto: cabeçalho, seções em ordem de nome e versão, SPDX para pacote sem arquivo e rodapé", () => {
  const texto = montarTexto({
    projeto: "MIT License\n\nCopyright (c) Projeto",
    fontes: "SIL OPEN FONT LICENSE Version 1.1",
    rust: [
      { nome: "serde", versao: "1.0.10", licenca: "MIT OR Apache-2.0", textos: ["texto serde 10"] },
      { nome: "anyhow", versao: "1.0.0", licenca: "MIT", textos: ["texto anyhow"] },
      { nome: "serde", versao: "1.0.9", licenca: "MIT OR Apache-2.0", textos: ["texto serde 9"] },
      { nome: "napi", versao: "3.0.0", licenca: "MIT", textos: [] },
    ],
    npm: [{ nome: "electron-updater", versao: "6.8.10", licenca: "MIT", textos: ["texto updater"] }],
  });
  assert.ok(texto.startsWith("MIT License\n\nCopyright (c) Projeto"));
  assert.ok(texto.indexOf("SIL OPEN FONT LICENSE") < texto.indexOf("== anyhow"));
  const ordem = ["== anyhow 1.0.0 (MIT) ==", "== napi 3.0.0 (MIT) ==", "== serde 1.0.9 (MIT OR Apache-2.0) ==", "== serde 1.0.10 (MIT OR Apache-2.0) =="];
  const posicoes = ordem.map((s) => texto.indexOf(s));
  assert.ok(posicoes.every((p) => p >= 0), posicoes.join(","));
  assert.deepEqual([...posicoes].sort((a, b) => a - b), posicoes);
  const napi = texto.slice(posicoes[1], posicoes[2]);
  assert.match(napi, /Texto da licença em https:\/\/spdx\.org\/licenses\/MIT\.html/);
  assert.ok(texto.indexOf("texto serde 9") < texto.indexOf("texto serde 10"));
  assert.match(texto, /== electron-updater 6\.8\.10 \(MIT\) ==\n\ntexto updater/);
  assert.ok(
    texto.trimEnd().endsWith(
      "O Electron e o Chromium trazem as próprias licenças nos arquivos LICENSE e LICENSES.chromium.html da pasta de instalação.",
    ),
  );
});
