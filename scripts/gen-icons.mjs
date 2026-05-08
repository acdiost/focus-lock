#!/usr/bin/env node
/**
 * Generates app icons for Tauri.
 * Outputs: src-tauri/icons/{16,32,64,128,256}x*.png, icon.ico, icon.icns
 * Run: node scripts/gen-icons.mjs
 */
import { deflateSync } from 'zlib';
import { writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const outDir = resolve(root, 'src-tauri/icons');

// ── CRC32 ─────────────────────────────────────────────
const crcTable = new Uint32Array(256);
for (let i = 0; i < 256; i++) {
  let c = i;
  for (let j = 0; j < 8; j++) c = c & 1 ? (0xedb88320 ^ (c >>> 1)) : c >>> 1;
  crcTable[i] = c;
}
function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = (crcTable[(c ^ b) & 0xff] ^ (c >>> 8)) >>> 0;
  return (c ^ 0xffffffff) >>> 0;
}

// ── PNG encoder ───────────────────────────────────────
function pngChunk(type, data) {
  const t = Buffer.from(type, 'ascii');
  const len = Buffer.allocUnsafe(4);
  len.writeUInt32BE(data.length);
  const crcBuf = Buffer.allocUnsafe(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([t, data])));
  return Buffer.concat([len, t, data, crcBuf]);
}

function encodePNG(pixels, w, h) {
  const raw = Buffer.allocUnsafe(h * (1 + w * 4));
  for (let y = 0; y < h; y++) {
    raw[y * (1 + w * 4)] = 0;
    pixels.copy(raw, y * (1 + w * 4) + 1, y * w * 4, (y + 1) * w * 4);
  }
  const ihdr = Buffer.allocUnsafe(13);
  ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    pngChunk('IHDR', ihdr),
    pngChunk('IDAT', deflateSync(raw, { level: 9 })),
    pngChunk('IEND', Buffer.alloc(0)),
  ]);
}

// ── Icon drawing ──────────────────────────────────────
// Orange circle (#f76707) with white clock hands
function drawIcon(size) {
  const px = Buffer.alloc(size * size * 4); // RGBA, zero = transparent
  const cx = size / 2, cy = size / 2;
  const outerR = size * 0.48;
  const innerR = outerR - Math.max(1.5, size * 0.055);
  const handW = Math.max(1, size * 0.07);

  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const dx = x - cx + 0.5, dy = y - cy + 0.5;
      const d = Math.sqrt(dx * dx + dy * dy);
      const i = (y * size + x) * 4;
      if (d <= innerR) {
        px[i] = 247; px[i + 1] = 103; px[i + 2] = 7; px[i + 3] = 255;
      } else if (d <= outerR) {
        px[i] = 210; px[i + 1] = 84; px[i + 2] = 0; px[i + 3] = 255;
      }
    }
  }

  // White clock hands: minute → 3 o'clock, hour → 12 o'clock
  const handLen = innerR * 0.58;
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const dx = x - cx + 0.5, dy = y - cy + 0.5;
      const i = (y * size + x) * 4;
      if (px[i + 3] === 0) continue; // outside circle

      const onMinute = Math.abs(dy) <= handW && dx >= 0 && dx <= handLen;
      const onHour   = Math.abs(dx) <= handW && dy <= 0 && -dy <= handLen * 0.7;
      if (onMinute || onHour) {
        px[i] = 255; px[i + 1] = 255; px[i + 2] = 255; px[i + 3] = 255;
      }
    }
  }

  return px;
}

// ── ICO encoder (PNG-in-ICO, Vista+) ─────────────────
function encodeICO(entries) {
  // entries: [{ size, png }]
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(entries.length, 4);

  const dirs = [];
  let offset = 6 + entries.length * 16;
  for (const { size, png } of entries) {
    const dir = Buffer.alloc(16);
    dir[0] = size >= 256 ? 0 : size;
    dir[1] = size >= 256 ? 0 : size;
    dir.writeUInt16LE(1, 4);
    dir.writeUInt16LE(32, 6);
    dir.writeUInt32LE(png.length, 8);
    dir.writeUInt32LE(offset, 12);
    dirs.push(dir);
    offset += png.length;
  }

  return Buffer.concat([header, ...dirs, ...entries.map(e => e.png)]);
}

// ── ICNS encoder (macOS) ──────────────────────────────
function encodeICNS(pngMap) {
  // pngMap: Map<size, Buffer>
  const typeMap = { 16: 'icp4', 32: 'icp5', 64: 'icp6', 128: 'ic07', 256: 'ic08', 512: 'ic09', 1024: 'ic10' };
  const chunks = [];
  for (const [size, png] of Object.entries(pngMap)) {
    const type = typeMap[size];
    if (!type) continue;
    const hdr = Buffer.alloc(8);
    hdr.write(type, 0, 'ascii');
    hdr.writeUInt32BE(8 + png.length, 4);
    chunks.push(hdr, png);
  }
  const body = Buffer.concat(chunks);
  const file = Buffer.alloc(8 + body.length);
  file.write('icns', 0, 'ascii');
  file.writeUInt32BE(file.length, 4);
  body.copy(file, 8);
  return file;
}

// ── Main ──────────────────────────────────────────────
const sizes = [16, 32, 64, 128, 256, 512, 1024];
const pngMap = {};

for (const s of sizes) {
  const px = drawIcon(s);
  pngMap[s] = encodePNG(px, s, s);
}

// Write individual PNGs
const named = { 32: '32x32.png', 128: '128x128.png', 256: '128x128@2x.png' };
for (const [size, name] of Object.entries(named)) {
  writeFileSync(`${outDir}/${name}`, pngMap[size]);
  console.log(`  ${name}`);
}

// Write icon.png (highest res, used by some bundlers)
writeFileSync(`${outDir}/icon.png`, pngMap[1024]);
console.log('  icon.png');

// Write icon.ico (Windows — includes 16,32,64,128,256)
const icoSizes = [16, 32, 64, 128, 256];
const ico = encodeICO(icoSizes.map(s => ({ size: s, png: pngMap[s] })));
writeFileSync(`${outDir}/icon.ico`, ico);
console.log('  icon.ico');

// Write icon.icns (macOS)
const icns = encodeICNS(pngMap);
writeFileSync(`${outDir}/icon.icns`, icns);
console.log('  icon.icns');

console.log('\nAll icons written to src-tauri/icons/');
