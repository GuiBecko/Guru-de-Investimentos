// Gera build/icon.png (1024 × 1024): o verde da marca com um "G" em pixel creme, na identidade
// do protótipo. Sem dependência: o PNG é montado à mão e comprimido com node:zlib.
// Uso: node scripts/icone.mjs
import { writeFileSync, mkdirSync } from "node:fs";
import { deflateSync } from "node:zlib";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const LADO = 1024;
const FUNDO = [0x04, 0x32, 0x1f]; // #04321F, --brand
const CREME = [0xf3, 0xec, 0xd6]; // #F3ECD6

// "G" numa grade de 8 × 8 pixels; cada pixel vira um bloco de 80 px, centrado no ícone.
const LETRA = [
  "..####..",
  ".##..##.",
  "##......",
  "##......",
  "##..####",
  "##....##",
  ".##..##.",
  "..####..",
];
const BLOCO = 80;
const MARGEM = (LADO - LETRA.length * BLOCO) / 2;

function corEm(x, y) {
  const c = Math.floor((x - MARGEM) / BLOCO);
  const l = Math.floor((y - MARGEM) / BLOCO);
  const dentro = x >= MARGEM && y >= MARGEM && l < LETRA.length && c < LETRA[0].length;
  return dentro && LETRA[l][c] === "#" ? CREME : FUNDO;
}

const TABELA_CRC = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(bytes) {
  let c = 0xffffffff;
  for (const b of bytes) c = TABELA_CRC[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function bloco(tipo, dados) {
  const tamanho = Buffer.alloc(4);
  tamanho.writeUInt32BE(dados.length);
  const corpo = Buffer.concat([Buffer.from(tipo, "ascii"), dados]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(corpo));
  return Buffer.concat([tamanho, corpo, crc]);
}

function png() {
  const cabecalho = Buffer.alloc(13);
  cabecalho.writeUInt32BE(LADO, 0);
  cabecalho.writeUInt32BE(LADO, 4);
  cabecalho[8] = 8; // bits por canal
  cabecalho[9] = 2; // RGB
  const linha = 1 + LADO * 3;
  const cru = Buffer.alloc(linha * LADO);
  for (let y = 0; y < LADO; y++) {
    cru[y * linha] = 0; // filtro "nenhum"
    for (let x = 0; x < LADO; x++) {
      const [r, g, b] = corEm(x, y);
      const i = y * linha + 1 + x * 3;
      cru[i] = r;
      cru[i + 1] = g;
      cru[i + 2] = b;
    }
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    bloco("IHDR", cabecalho),
    bloco("IDAT", deflateSync(cru, { level: 9 })),
    bloco("IEND", Buffer.alloc(0)),
  ]);
}

const destino = join(dirname(fileURLToPath(import.meta.url)), "..", "build", "icon.png");
mkdirSync(dirname(destino), { recursive: true });
writeFileSync(destino, png());
console.log(`Ícone gravado em ${destino}`);
