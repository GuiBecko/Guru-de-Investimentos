// Gera app/avisos-terceiros.txt: a licença do projeto, a das fontes IBM Plex e uma seção por
// dependência que vai no pacote (Rust, a partir do guru-node, e npm de execução do app), com o texto
// dos arquivos LICENSE*/COPYING* de cada uma. Recusa licença fora da lista.
// Uso: node scripts/avisos-terceiros.mjs            (grava o arquivo)
//      node scripts/avisos-terceiros.mjs --conferir (sai com 1 se o gravado estiver desatualizado)
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

// CDLA-Permissive-2.0: só o webpki-roots (os certificados raiz da Mozilla, usados pelo ureq) usa;
// é uma licença permissiva de dados, sem copyleft.
// Python-2.0 (a licença da PSF): só o argparse, que o js-yaml do electron-updater usa.
// BlueOak-1.0.0: só o sax, que o builder-util-runtime do electron-updater usa. As duas são
// permissivas, sem copyleft, e o texto delas vai junto no arquivo gerado.
const ACEITAS = new Set([
  "MIT",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "0BSD",
  "CC0-1.0",
  "BSL-1.0",
  "Unicode-3.0",
  "Unicode-DFS-2016",
  "MPL-2.0",
  "OFL-1.1",
  "CDLA-Permissive-2.0",
  "Python-2.0",
  "BlueOak-1.0.0",
]);

const RODAPE =
  "O Electron e o Chromium trazem as próprias licenças nos arquivos LICENSE e LICENSES.chromium.html da pasta de instalação.";
const LINHA = "-".repeat(72);

// Expressão SPDX em árvore: { id, excecao? } | { op: "AND"|"OR", partes }. A barra antiga do
// Cargo ("MIT/Apache-2.0") vale como OR. Lança Error numa expressão malformada.
function analisar(expressao) {
  const tokens = String(expressao).replace(/\//g, " OR ").match(/\(|\)|[^\s()]+/g) ?? [];
  let i = 0;
  const olhar = () => tokens[i]?.toUpperCase();
  const consumir = () => {
    if (i >= tokens.length) throw new Error("expressão incompleta");
    return tokens[i++];
  };
  function atomo() {
    const t = consumir();
    if (t === "(") {
      const dentro = ou();
      if (consumir() !== ")") throw new Error("parêntese aberto");
      return dentro;
    }
    if (t === ")" || ["AND", "OR", "WITH"].includes(t.toUpperCase())) throw new Error(`inesperado: ${t}`);
    if (olhar() === "WITH") {
      i++;
      return { id: t, excecao: consumir() };
    }
    return { id: t };
  }
  function lista(op, proximo) {
    const partes = [proximo()];
    while (olhar() === op) {
      i++;
      partes.push(proximo());
    }
    return partes.length === 1 ? partes[0] : { op, partes };
  }
  const e = () => lista("AND", atomo);
  const ou = () => lista("OR", e);
  if (!tokens.length) throw new Error("expressão vazia");
  const arvore = ou();
  if (i !== tokens.length) throw new Error("sobra no fim da expressão");
  return arvore;
}

function aceita(no) {
  if (no.op === "OR") return no.partes.some(aceita);
  if (no.op === "AND") return no.partes.every(aceita);
  return ACEITAS.has(no.excecao ? `${no.id} WITH ${no.excecao}` : no.id);
}

function identificadores(no) {
  return no.op ? no.partes.flatMap(identificadores) : [no.id];
}

export function licencaAceita(expressaoSpdx) {
  if (typeof expressaoSpdx !== "string") return false;
  try {
    return aceita(analisar(expressaoSpdx));
  } catch {
    return false;
  }
}

function compararVersoes(a, b) {
  const pa = a.split(/[.+-]/);
  const pb = b.split(/[.+-]/);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const x = pa[i] ?? "";
    const y = pb[i] ?? "";
    if (x === y) continue;
    if (/^\d+$/.test(x) && /^\d+$/.test(y)) return Number(x) - Number(y);
    return x < y ? -1 : 1;
  }
  return 0;
}

const ordenar = (pacotes) =>
  [...pacotes].sort((a, b) => (a.nome === b.nome ? compararVersoes(a.versao, b.versao) : a.nome < b.nome ? -1 : 1));

const limpar = (texto) => texto.replace(/^﻿/, "").replace(/\r\n?/g, "\n").replace(/[ \t]+$/gm, "").trim();

function secao(p) {
  const corpo = p.textos.length
    ? p.textos.map(limpar).join(`\n\n`)
    : [...new Set(identificadores(analisar(p.licenca)))]
        .map((id) => `Texto da licença em https://spdx.org/licenses/${id}.html`)
        .join("\n");
  return `== ${p.nome} ${p.versao} (${p.licenca}) ==\n\n${corpo}`;
}

function grupo(titulo, pacotes) {
  const corpo = pacotes.length ? ordenar(pacotes).map(secao).join("\n\n\n") : "Nenhum.";
  return `${LINHA}\n${titulo}\n${LINHA}\n\n${corpo}`;
}

export function montarTexto({ projeto, fontes, rust, npm }) {
  return (
    [
      limpar(projeto),
      `${LINHA}\nFontes IBM Plex (renderer/fonts)\n${LINHA}\n\n${limpar(fontes)}`,
      grupo("Componentes Rust (núcleo do app)", rust),
      grupo("Componentes npm", npm),
      `${LINHA}\n${RODAPE}`,
    ].join("\n\n\n") + "\n"
  );
}

// Textos dos arquivos LICENSE*/COPYING* da pasta, em ordem de nome.
function textosDaPasta(pasta) {
  return fs
    .readdirSync(pasta, { withFileTypes: true })
    .filter((e) => e.isFile() && /^(licen[cs]e|copying)/i.test(e.name))
    .map((e) => e.name)
    .sort()
    .map((nome) => fs.readFileSync(path.join(pasta, nome), "utf8"));
}

// Dependências normais (sem build e dev) a partir do guru-node, de todas as plataformas.
function pacotesRust(raiz) {
  const saida = execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--locked", "--manifest-path", path.join(raiz, "Cargo.toml")],
    { cwd: raiz, encoding: "utf8", maxBuffer: 512 * 1024 * 1024, stdio: ["ignore", "pipe", "inherit"] },
  );
  const meta = JSON.parse(saida);
  const porId = new Map(meta.packages.map((p) => [p.id, p]));
  const nos = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const membros = new Set(meta.workspace_members);
  const inicio = meta.workspace_members.find((id) => porId.get(id).name === "guru-node");
  if (!inicio) throw new Error("guru-node não está no workspace");
  const vistos = new Set();
  const pilha = [inicio];
  while (pilha.length) {
    const id = pilha.pop();
    if (vistos.has(id)) continue;
    vistos.add(id);
    for (const d of nos.get(id).deps) {
      if (d.dep_kinds.some((k) => k.kind === null)) pilha.push(d.pkg);
    }
  }
  return [...vistos]
    .filter((id) => !membros.has(id))
    .map((id) => {
      const p = porId.get(id);
      return { nome: p.name, versao: p.version, licenca: p.license ?? "", textos: textosDaPasta(path.dirname(p.manifest_path)) };
    });
}

function licencaNpm(manifesto) {
  const l = manifesto.license ?? manifesto.licenses;
  if (typeof l === "string") return l;
  if (Array.isArray(l)) return l.map((x) => (typeof x === "string" ? x : x?.type)).join(" OR ");
  return l?.type ?? "";
}

// Dependências de execução do app, sem o @guru/core-node (o núcleo é o próprio projeto).
function pacotesNpm(appDir) {
  const npmCli = process.env.npm_execpath;
  const [cmd, args] = npmCli ? [process.execPath, [npmCli]] : ["npm", []];
  let saida;
  try {
    saida = execFileSync(cmd, [...args, "ls", "--omit=dev", "--all", "--json"], {
      cwd: appDir,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
      stdio: ["ignore", "pipe", "ignore"],
    });
  } catch (e) {
    // `npm ls` sai com 1 quando acha "problemas" (pacotes estranhos), mas a árvore vem no stdout.
    if (!e.stdout) throw e;
    saida = e.stdout;
  }
  const arvore = JSON.parse(saida);
  const achados = new Map();
  function visitar(dependencias, pastaPai) {
    for (const [nome, info] of Object.entries(dependencias ?? {})) {
      if (nome === "@guru/core-node") continue;
      // Resolve como o Node: a pasta node_modules mais próxima, subindo até o app.
      let pasta = null;
      for (let p = pastaPai; ; p = path.dirname(p)) {
        const candidato = path.join(p, "node_modules", nome);
        if (fs.existsSync(path.join(candidato, "package.json"))) {
          pasta = candidato;
          break;
        }
        if (p === appDir || !p.startsWith(appDir)) break;
      }
      if (!pasta) throw new Error(`pacote npm não encontrado: ${nome}`);
      const manifesto = JSON.parse(fs.readFileSync(path.join(pasta, "package.json"), "utf8"));
      const chave = `${nome}@${manifesto.version}`;
      if (!achados.has(chave)) {
        achados.set(chave, { nome, versao: manifesto.version, licenca: licencaNpm(manifesto), textos: textosDaPasta(pasta) });
      }
      visitar(info.dependencies, pasta);
    }
  }
  visitar(arvore.dependencies, appDir);
  return [...achados.values()];
}

function principal() {
  const appDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const raiz = path.resolve(appDir, "..");
  const destino = path.join(appDir, "avisos-terceiros.txt");
  const rust = pacotesRust(raiz);
  const npm = pacotesNpm(appDir);
  const recusados = [...rust, ...npm].filter((p) => !licencaAceita(p.licenca));
  if (recusados.length) {
    for (const p of ordenar(recusados)) {
      console.error(`Licença fora da lista: ${p.nome} ${p.versao} (${p.licenca || "sem licença"})`);
    }
    process.exit(1);
  }
  const texto = montarTexto({
    projeto: fs.readFileSync(path.join(raiz, "LICENSE"), "utf8"),
    fontes: fs.readFileSync(path.join(appDir, "renderer", "fonts", "OFL.txt"), "utf8"),
    rust,
    npm,
  });
  if (process.argv.includes("--conferir")) {
    const gravado = fs.existsSync(destino) ? fs.readFileSync(destino, "utf8").replace(/\r\n/g, "\n") : null;
    if (gravado !== texto) {
      console.error("avisos-terceiros.txt desatualizado: rode npm run avisos.");
      process.exit(1);
    }
    console.log(`avisos-terceiros.txt em dia (${rust.length} Rust, ${npm.length} npm).`);
    return;
  }
  fs.writeFileSync(destino, texto);
  console.log(`avisos-terceiros.txt gravado (${rust.length} Rust, ${npm.length} npm).`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) principal();
