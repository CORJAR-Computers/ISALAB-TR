// ============================================================================
// Limpia los residuos temporales `.temp-archive` que deja `rustc`/`ar` en
// `src-tauri/target/` cuando una compilación falla en Windows.
//
// Contexto: al crear un `.rlib`, rustc escribe en un directorio temporal
// `.tmpXXXX.temp-archive` y lo elimina al terminar. Si Windows Defender (u
// otro antivirus) mantiene el archivo bloqueado en ese instante, la
// eliminación falla con `os error 32` y el directorio queda huérfano; en la
// siguiente compilación cargo vuelve a fallar al intentar reutilizarlo.
//
// Uso:      node scripts/clean-target-orphans.mjs
// Automático: se ejecuta al inicio de `npm run tauri:dev` y `npm run tauri:build`.
//
// El directorio a limpiar se resuelve con `cargo metadata` para respetar
// `CARGO_TARGET_DIR`, `[build] target-dir` (src-tauri/.cargo/config.toml) o el
// default src-tauri/target.
// ============================================================================
import { execFileSync } from "node:child_process";
import { readdirSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SRC_TAURI = join(ROOT, "src-tauri");
const DEFAULT_TARGET = join(SRC_TAURI, "target");

const ORPHAN_SUFFIX = ".temp-archive";

/** Resuelve el directorio de compilación efectivo que usará cargo. */
function resolveTargetDir() {
  try {
    const stdout = execFileSync(
      "cargo",
      ["metadata", "--no-deps", "--format-version", "1"],
      { cwd: SRC_TAURI, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"], windowsHide: true },
    );
    const meta = JSON.parse(stdout);
    if (meta?.target_directory) return meta.target_directory;
  } catch {
    // cargo no disponible o crate con errores: se cae al default.
  }
  return DEFAULT_TARGET;
}

const TARGET = resolveTargetDir();

// No se filtra por antigüedad a propósito: el script se ejecuta antes de
// compilar (no hay build en curso) y, si se usa a mano durante una
// compilación activa, Windows mantiene bloqueado el directorio en uso y
// `rmSync` falla (EBUSY/EPERM) quedando registrado como omitido.

/** Recorre el árbol y produce las rutas de las entradas `*.temp-archive`. */
function* findOrphans(dir) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return; // directorio inexistente o sin permisos: nada que limpiar
  }
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name.endsWith(ORPHAN_SUFFIX)) {
        yield full;
      } else {
        yield* findOrphans(full);
      }
    } else if (entry.name.endsWith(ORPHAN_SUFFIX)) {
      yield full;
    }
  }
}

/** Tamaño total de una ruta (archivo o árbol de directorios), en bytes. */
function sizeOf(path) {
  try {
    const st = statSync(path);
    if (st.isFile()) return st.size;
    let total = 0;
    for (const child of readdirSync(path)) total += sizeOf(join(path, child));
    return total;
  } catch {
    return 0;
  }
}

/** Formatea un número de bytes de forma legible. */
function fmt(bytes) {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(2)} GiB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MiB`;
  return `${Math.max(Math.round(bytes / 1024), 1)} KiB`;
}

let removed = 0;
let freed = 0;
const skipped = [];

for (const orphan of findOrphans(TARGET)) {
  const bytes = sizeOf(orphan);
  try {
    rmSync(orphan, { recursive: true, force: true });
    removed += 1;
    freed += bytes;
  } catch {
    // Sigue bloqueado por otro proceso: se omite, no es un error fatal.
    skipped.push(orphan);
  }
}

console.log(`🧹 Limpieza de huérfanos ${ORPHAN_SUFFIX} en ${TARGET}`);

if (removed === 0 && skipped.length === 0) {
  console.log("✓ Sin residuos que eliminar.");
} else {
  if (removed > 0) console.log(`✓ Eliminados: ${removed} (~${fmt(freed)} liberados)`);
  if (skipped.length > 0) {
    console.log(`⏭ Omitidos (bloqueados por otro proceso): ${skipped.length}`);
    for (const skip of skipped.slice(0, 5)) console.log(`   - ${skip}`);
    if (skipped.length > 5) console.log(`   … y ${skipped.length - 5} más`);
  }
}
