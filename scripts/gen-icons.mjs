// ============================================================================
// Genera los iconos de la app.
// Uso: node scripts/gen-icons.mjs
// Salida: src-tauri/icons/ (icon.png, 32x32.png, 128x128.png, 128x128@2x.png, icon.ico)
//
// Fuente preferida: `icono.ico` en la raíz del proyecto (icono real del
// cliente). Si no existe, se cae al diseño procedural por defecto.
// ============================================================================
import { execFileSync } from "node:child_process";
import { deflateSync } from "node:zlib";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = join(ROOT, "src-tauri", "icons");

// Si hay un icono real (icono.ico) en la raíz, lo usa como fuente oficial.
const SOURCE_ICO = join(ROOT, "icono.ico");
if (existsSync(SOURCE_ICO)) {
  console.log("Usando icono.ico como fuente oficial…");
  // Windows: npx.cmd es un script batch; shell: true es obligatorio.
  execFileSync("npx", ["tauri", "icon", SOURCE_ICO], {
    cwd: ROOT,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  process.exit(0);
}

// ---------- CRC32 (estándar PNG) ----------
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

// ---------- Encoder PNG (RGBA) ----------
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

function encodePng(width, height, rgba) {
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type RGBA
  // scanlines con filtro 0
  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (stride + 1)] = 0;
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }
  const idat = deflateSync(raw, { level: 9 });
  return Buffer.concat([
    sig,
    chunk("IHDR", ihdr),
    chunk("IDAT", idat),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// ---------- Diseño del icono ----------
// Fondo: cuadrado redondeado con gradiente teal → verde; huella canina blanca
// y una cruz médica pequeña como acento clínico.
function inRoundedRect(x, y, size, radius) {
  const half = size / 2;
  const cx = Math.abs(x - half);
  const cy = Math.abs(y - half);
  const dx = Math.max(cx - (half - radius), 0);
  const dy = Math.max(cy - (half - radius), 0);
  return dx * dx + dy * dy <= radius * radius;
}

function inEllipse(x, y, cx, cy, rx, ry) {
  const dx = (x - cx) / rx;
  const dy = (y - cy) / ry;
  return dx * dx + dy * dy <= 1;
}

function lerp(a, b, t) {
  return a + (b - a) * t;
}

function renderIcon(size) {
  const rgba = Buffer.alloc(size * size * 4);
  const s = (v) => (v / 100) * size; // coordenadas en porcentaje

  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const i = (y * size + x) * 4;
      let r = 0, g = 0, b = 0, a = 0;

      // Fondo redondeado
      if (inRoundedRect(x + 0.5, y + 0.5, size, size * 0.22)) {
        const t = y / size;
        r = lerp(13, 15, t);   // teal profundo
        g = lerp(148, 160, t); // → verde
        b = lerp(136, 105, t);
        a = 255;
      }

      // Huella: 4 dedos + almohadilla
      const white = [255, 255, 255];
      const toes = [
        [38, 30, 5.5, 7.5],
        [50, 25, 6, 8],
        [62, 30, 5.5, 7.5],
        [50, 36, 8, 6],
      ];
      const px = s(50), py = s(62);
      for (const [cx, cy, rx, ry] of toes) {
        if (inEllipse(x + 0.5, y + 0.5, s(cx), s(cy), s(rx), s(ry))) {
          r = white[0]; g = white[1]; b = white[2]; a = 255;
        }
      }
      if (inEllipse(x + 0.5, y + 0.5, px, py, s(16), s(11))) {
        r = white[0]; g = white[1]; b = white[2]; a = 255;
      }

      // Cruz médica pequeña (esquina inferior derecha del fondo)
      const cross = s(78);
      const cr = size * 0.035;
      if (
        inRoundedRect(x + 0.5, y + 0.5, size, size * 0.22) &&
        ((Math.abs(x - cross) <= cr && Math.abs(y - s(88)) <= s(4.2)) ||
          (Math.abs(y - s(88)) <= cr && Math.abs(x - cross) <= s(4.2)))
      ) {
        r = 13; g = 148; b = 136; a = 255; // reemplaza fondo con teal sólido
      }

      rgba[i] = r;
      rgba[i + 1] = g;
      rgba[i + 2] = b;
      rgba[i + 3] = a;
    }
  }
  return rgba;
}

// ---------- ICO multiescala (PNG embebido, compatible Win Vista+) ----------
function encodeIco(sizes) {
  const images = sizes.map((s) => ({
    size: s,
    png: encodePng(s, s, renderIcon(s)),
  }));
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2); // tipo icono
  header.writeUInt16LE(images.length, 4);

  const entries = [];
  let offset = 6 + 16 * images.length;
  for (const img of images) {
    const e = Buffer.alloc(16);
    e[0] = img.size >= 256 ? 0 : img.size;
    e[1] = img.size >= 256 ? 0 : img.size;
    e.writeUInt16LE(1, 4); // planos
    e.writeUInt16LE(32, 6); // bpp
    e.writeUInt32LE(img.png.length, 8);
    e.writeUInt32LE(offset, 12);
    entries.push(e);
    offset += img.png.length;
  }
  return Buffer.concat([header, ...entries, ...images.map((i) => i.png)]);
}

// ---------- Generación ----------
mkdirSync(OUT, { recursive: true });

const outputs = [
  ["icon.png", encodePng(512, 512, renderIcon(512))],
  ["32x32.png", encodePng(32, 32, renderIcon(32))],
  ["128x128.png", encodePng(128, 128, renderIcon(128))],
  ["128x128@2x.png", encodePng(256, 256, renderIcon(256))],
  ["icon.ico", encodeIco([16, 24, 32, 48, 64, 128, 256])],
];

for (const [name, buf] of outputs) {
  writeFileSync(join(OUT, name), buf);
  console.log(`✓ ${name} (${(buf.length / 1024).toFixed(1)} KiB)`);
}
console.log(`Iconos generados en ${OUT}`);
