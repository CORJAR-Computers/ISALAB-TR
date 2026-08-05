#!/usr/bin/env node
// ─────────────────────────────────────────────────────────────────────────────
// scripts/prune-firebird.mjs
//
// Reduce el motor Firebird al subconjunto EMBEDDED que la app necesita en
// runtime: fbclient.dll + ICU + plugins + intl + tzdata + SECURITY5.FDB.
// Elimina las herramientas de servidor (firebird.exe, gbak, gfix, isql…),
// la documentación (doc/, examples/, include/, lib/, misc/) y el instalador
// de runtime (system32/), que la app empaquetada no utiliza.
//
// El resultado final de esta poda es el contenido mínimo que debe quedar en
// `src-tauri/binaries/firebird/` (recurso `binaries/firebird/**/*` del
// instalador). Antes de este script, el pipeline de CI copiaba el ZIP de
// Firebird COMPLETO (~68 MB) y el instalador crecía a ~20 MB innecesarios.
//
// Uso:  node scripts/prune-firebird.mjs [directorio]
//       (por defecto: src-tauri/binaries/firebird)
//
// Idempotente: puede ejecutarse en cada build sin miedo.
// ─────────────────────────────────────────────────────────────────────────────

import { existsSync, readdirSync, rmSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

const TARGET = resolve(
  process.cwd(),
  process.argv[2]?.trim() || "src-tauri/binaries/firebird"
);

// Directorios de primer nivel que la app no usa (herramientas, docs, SDK).
const DIRS_TO_REMOVE = ["doc", "examples", "include", "lib", "misc", "system32"];

// Archivos de primer nivel que no forman parte del runtime embedded.
// NOTA: NO incluir `README.md` — el repo trackea src-tauri/binaries/firebird/
// README.md (documentación propia del proyecto) y el readme del ZIP es
// `Readme.txt` (con distinta capitalización, ya cubierto aquí).
const FILES_TO_REMOVE = new Set([
  // Herramientas de servidor / utilidades de administración.
  "install_service.bat",
  "uninstall_service.bat",
  "databases.conf",
  // Residuos del ZIP de distribución.
  "firebird.log",
  "Readme.txt",
]);

// Piezas imprescindibles que deben sobrevivir a la poda (validación final).
// Se verifican archivos representativos dentro de los directorios clave, no
// solo el nombre del directorio: un plugins/ vacío rompería el motor en
// runtime y el script debe detectarlo.
const ESSENTIAL = [
  "fbclient.dll",
  "firebird.conf",
  "firebird.msg",
  "SECURITY5.FDB",
  "icudt63l.dat",
  "plugins/engine13.dll",
  "intl/fbintl.dll",
  "tzdata",
];

function formatMB(bytes) {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function dirSize(dir) {
  let total = 0;
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    total += statSync(full).isDirectory() ? dirSize(full) : statSync(full).size;
  }
  return total;
}

if (!existsSync(TARGET) || !statSync(TARGET).isDirectory()) {
  console.error(`prune-firebird: el directorio no existe: ${TARGET}`);
  process.exit(1);
}

const before = dirSize(TARGET);
let removedCount = 0;
let removedBytes = 0;

for (const entry of readdirSync(TARGET)) {
  const full = join(TARGET, entry);
  const isDir = statSync(full).isDirectory();
  const remove = isDir
    ? DIRS_TO_REMOVE.includes(entry)
    : entry.toLowerCase().endsWith(".exe") || FILES_TO_REMOVE.has(entry);

  if (remove) {
    const size = isDir ? dirSize(full) : statSync(full).size;
    rmSync(full, { recursive: true, force: true });
    removedCount++;
    removedBytes += size;
    console.log(`  - ${entry} (${formatMB(size)})`);
  }
}

const missing = ESSENTIAL.filter((f) => !existsSync(join(TARGET, f)));
if (missing.length > 0) {
  console.error(`prune-firebird: ERROR — faltan piezas esenciales tras la poda: ${missing.join(", ")}`);
  process.exit(1);
}

const after = dirSize(TARGET);
console.log(
  `prune-firebird: ${formatMB(before)} → ${formatMB(after)} ` +
    `(${removedCount} elementos eliminados, -${formatMB(removedBytes)})`
);
