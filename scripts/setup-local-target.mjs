// ============================================================================
// Configura automáticamente el directorio de compilación de cargo
// (CARGO_TARGET_DIR vía `[build] target-dir`) en una ubicación estable FUERA
// del proyecto, para sacar el target del escaneo del antivirus.
//
// Detecta el mejor disco disponible, crea la carpeta y edita
// `src-tauri/.cargo/config.toml` programáticamente (preservando la sección
// `[env]` y los comentarios existentes). Es idempotente: si ya hay un
// `target-dir` configurado y usable, lo conserva (no cambia de disco ni
// invalida la caché).
//
// Uso:
//   node scripts/setup-local-target.mjs                       # detecta, crea y configura
//   node scripts/setup-local-target.mjs --print               # solo muestra la ruta elegida
//   node scripts/setup-local-target.mjs --unset               # quita target-dir del config
//   node scripts/setup-local-target.mjs --move                # mueve src-tauri/target (caché)
//                                                             #   a la nueva ubicación
//   node scripts/setup-local-target.mjs --defender-exclusion  # además, excluye la carpeta
//                                                             #   en Windows Defender (UAC)
// ============================================================================
import { spawnSync } from "node:child_process";
import {
  accessSync,
  constants,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import { dirname, join, parse } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const CONFIG_PATH = join(ROOT, "src-tauri", ".cargo", "config.toml");
const DEFAULT_TARGET = join(ROOT, "src-tauri", "target");

const ARGS = new Set(process.argv.slice(2));
const WANT_PRINT = ARGS.has("--print");
const WANT_UNSET = ARGS.has("--unset");
const WANT_MOVE = ARGS.has("--move");
const WANT_DEFENDER = ARGS.has("--defender-exclusion");

// ── Detección de ubicación ─────────────────────────────────────────────────

/** Letras de los discos existentes en Windows (p. ej. ["C", "D"]). */
function listDrives() {
  const drives = [];
  for (let i = 0; i < 26; i++) {
    const letter = String.fromCharCode(65 + i);
    try {
      accessSync(`${letter}:/`);
      drives.push(letter);
    } catch {
      // disco inexistente
    }
  }
  return drives;
}

/**
 * Elige la mejor ruta para el target. Preferencia:
 *   1. El disco del proyecto (carpeta fuera del árbol del repo): el movimiento
 *      de caché es instantáneo en el mismo volumen y es la ubicación más
 *      predecible para el desarrollador.
 *   2. Un disco de datos distinto del sistema y del proyecto (si existe).
 *   3. El disco del sistema.
 * En Unix usa ~/.rust-targets/isalab.
 */
function detectLocation() {
  if (process.platform !== "win32") {
    return join(os.homedir(), ".rust-targets", "isalab");
  }
  const projectDrive = parse(ROOT).root.replace(/[\\/]/g, "").replace(":", "").toUpperCase();
  const systemDrive = (process.env.SystemDrive || "C:").replace(":", "").toUpperCase();

  const drives = listDrives();
  const secondary = drives.filter((d) => d !== systemDrive && d !== projectDrive).sort();
  const candidates = [...new Set([projectDrive, ...secondary, systemDrive])];

  return `${candidates[0]}:/rust-targets/isalab`;
}

/**
 * Crea la carpeta del target y verifica que sea escribible.
 * Devuelve la ruta si funciona, o null si no.
 */
function ensureTargetDir(location) {
  try {
    mkdirSync(location, { recursive: true });
    accessSync(location, constants.W_OK);
    return location;
  } catch {
    return null;
  }
}

/** Lee el `target-dir` ya configurado en `[build]` (o null si no existe). */
function readConfiguredTargetDir() {
  if (!existsSync(CONFIG_PATH)) return null;
  const lines = readFileSync(CONFIG_PATH, "utf8").replace(/\r\n/g, "\n").split("\n");
  const idx = findSection(lines, "build");
  if (idx === -1) return null;
  for (let i = idx + 1; i < sectionEnd(lines, idx); i++) {
    const m = lines[i].match(/^\s*target-dir\s*=\s*"([^"]*)"/);
    if (m) return m[1];
  }
  return null;
}

/**
 * Ubicación efectiva: si ya hay un `target-dir` configurado y usable, lo
 * conserva (idempotencia — no cambia de disco silenciosamente y no invalida
 * la caché). Solo detecta automáticamente cuando no hay configuración previa.
 */
function resolveLocation() {
  const configured = readConfiguredTargetDir();
  if (configured && ensureTargetDir(configured)) return configured;
  return detectLocation();
}

// ── Edición del config.toml ────────────────────────────────────────────────

const SECTION_RE = /^\s*\[([^\]]+)\]\s*$/;
const TARGET_DIR_RE = /^\s*target-dir\s*=/;

function findSection(lines, name) {
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(SECTION_RE);
    if (m && m[1].trim() === name) return i;
  }
  return -1;
}

/** Índice (exclusivo) donde termina la sección que empieza en `start`. */
function sectionEnd(lines, start) {
  for (let i = start + 1; i < lines.length; i++) {
    if (SECTION_RE.test(lines[i])) return i;
  }
  return lines.length;
}

/** Escribe (o actualiza) `target-dir` en la sección `[build]`. */
function setTargetDir(content, tomlPath) {
  const lines = content.length > 0 ? content.replace(/\r\n/g, "\n").split("\n") : [];
  const idx = findSection(lines, "build");

  if (idx === -1) {
    if (lines.length && lines[lines.length - 1].trim() !== "") lines.push("");
    lines.push("[build]", `target-dir = "${tomlPath}"`);
  } else {
    const end = sectionEnd(lines, idx);
    let replaced = false;
    for (let i = idx + 1; i < end; i++) {
      if (TARGET_DIR_RE.test(lines[i])) {
        lines[i] = `target-dir = "${tomlPath}"`;
        replaced = true;
        break;
      }
    }
    if (!replaced) {
      // Insertar tras los comentarios del encabezado de la sección.
      let insertAt = idx + 1;
      while (
        insertAt < end &&
        (lines[insertAt].trim() === "" || lines[insertAt].trim().startsWith("#"))
      ) {
        insertAt++;
      }
      lines.splice(insertAt, 0, `target-dir = "${tomlPath}"`);
    }
  }

  return `${lines.join("\n").trimEnd()}\n`;
}

/** Elimina `target-dir` y la sección `[build]` entera si queda sin claves. */
function unsetTargetDir(content) {
  const lines = content.replace(/\r\n/g, "\n").split("\n");
  const idx = findSection(lines, "build");
  if (idx === -1) return content;

  const end = sectionEnd(lines, idx);
  const kept = lines.slice(idx + 1, end).filter((l) => !TARGET_DIR_RE.test(l));
  const hasRealKeys = kept.some((l) => l.trim() !== "" && !l.trim().startsWith("#"));

  if (hasRealKeys) {
    // Se conserva la sección y sus comentarios, sin la línea target-dir.
    return `${[...lines.slice(0, idx), lines[idx], ...kept, ...lines.slice(end)]
      .join("\n")
      .trimEnd()}\n`;
  }
  // Sección vacía: se elimina por completo, incluidos sus comentarios.
  return `${lines.slice(0, idx).concat(lines.slice(end)).join("\n").trimEnd()}\n`;
}

// ── Movimiento de la caché existente ───────────────────────────────────────

function moveExistingTarget(dest) {
  if (!existsSync(DEFAULT_TARGET)) {
    console.log("ℹ  No existe src-tauri/target: no hay caché que mover.");
    return;
  }
  if (existsSync(dest) && readdirSync(dest).length > 0) {
    console.log("⚠  La nueva ubicación ya contiene datos: no se mueve la caché.");
    return;
  }
  try {
    mkdirSync(dirname(dest), { recursive: true });
    // Si la carpeta destino existe pero está vacía (p. ej. la pre-creó
    // ensureTargetDir), se elimina antes de renombrar: en Windows no se
    // puede renombrar un directorio sobre uno ya existente.
    if (existsSync(dest)) rmSync(dest, { recursive: true, force: true });
    renameSync(DEFAULT_TARGET, dest);
    console.log(`✓  Caché movida de ${DEFAULT_TARGET} a ${dest}`);
  } catch (err) {
    console.log(`⚠  No se pudo mover la caché: ${err.message}`);
    console.log("   Muévela manualmente mientras no haya una compilación en curso.");
  }
}

// ── Exclusión de Windows Defender ──────────────────────────────────────────

/**
 * Excluye la carpeta contenedora del target en Windows Defender (requiere
 * elevación: aparece un aviso UAC que el usuario debe aceptar). Verifica el
 * resultado leyendo la lista de exclusiones que escribe el proceso elevado.
 */
function applyDefenderExclusion(targetDir) {
  if (process.platform !== "win32") {
    console.log("ℹ  Windows Defender solo aplica en Windows: se omite la exclusión.");
    return;
  }

  const exclusionPath = dirname(targetDir).replace(/\//g, "\\");
  const tmpDir = os.tmpdir();
  const ps1 = join(tmpDir, "isalab-defender-exclusion.ps1");
  const logFile = join(tmpDir, "isalab-defender-exclusion.log");
  rmSync(logFile, { force: true });

  // Se escribe un .ps1 temporal y se ejecuta elevado: evita el anidamiento de
  // comillas de PowerShell (rutas con espacios incluidas).
  writeFileSync(
    ps1,
    [
      `Add-MpPreference -ExclusionPath '${exclusionPath}'`,
      `(Get-MpPreference).ExclusionPath | Out-File -FilePath '${logFile}' -Encoding utf8`,
    ].join("\n"),
  );

  try {
    spawnSync(
      "powershell",
      [
        "-NoProfile",
        "-Command",
        `Start-Process powershell -Verb RunAs -Wait -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','${ps1}'`,
      ],
      { stdio: "ignore", windowsHide: true, timeout: 120_000 },
    );
  } catch {
    // PowerShell no disponible o el proceso elevado no pudo lanzarse.
  }
  rmSync(ps1, { force: true });

  let ok = false;
  try {
    const log = readFileSync(logFile, "utf8").replace(/^\uFEFF/, "");
    ok = log
      .split(/\r?\n/)
      .some((l) => l.trim().toLowerCase() === exclusionPath.toLowerCase());
  } catch {
    // Sin log: el proceso elevado no llegó a escribir (UAC cancelado).
  }
  rmSync(logFile, { force: true });

  if (ok) {
    console.log(`✓  Exclusión de Windows Defender aplicada: ${exclusionPath}`);
  } else {
    console.log(`⚠  No se pudo verificar la exclusión de ${exclusionPath}`);
    console.log("   Puede que se haya cancelado el aviso UAC; aplícala a mano:");
    console.log(`   Add-MpPreference -ExclusionPath '${exclusionPath}'`);
  }
}

// ── Flujo principal ────────────────────────────────────────────────────────

const location = resolveLocation();
const tomlPath = location.replace(/\\/g, "/"); // "/" evita escapes en TOML

if (WANT_PRINT) {
  console.log(`Ubicación elegida: ${tomlPath}`);
  process.exit(0);
}

if (WANT_UNSET) {
  if (!existsSync(CONFIG_PATH)) {
    console.log("ℹ  No existe config.toml: nada que quitar.");
    process.exit(0);
  }
  const before = readFileSync(CONFIG_PATH, "utf8");
  const after = unsetTargetDir(before);
  writeFileSync(CONFIG_PATH, after);
  console.log("✓  target-dir eliminado de src-tauri/.cargo/config.toml");
  console.log("   cargo volverá a compilar en src-tauri/target (o donde apunte CARGO_TARGET_DIR).");
  process.exit(0);
}

const ready = ensureTargetDir(location);
if (!ready) {
  console.log(`⚠  No se pudo crear/escribir en ${location}; el config se escribirá de todos modos.`);
} else {
  console.log(`✓  Carpeta lista: ${ready}`);
}

if (WANT_MOVE) moveExistingTarget(location);

const existing = existsSync(CONFIG_PATH) ? readFileSync(CONFIG_PATH, "utf8") : "";
const next = setTargetDir(existing, tomlPath);
writeFileSync(CONFIG_PATH, next);

console.log(`✓  src-tauri/.cargo/config.toml → target-dir = "${tomlPath}"`);
console.log("   Prioridad: env CARGO_TARGET_DIR > este target-dir > src-tauri/target por defecto.");

if (WANT_DEFENDER) {
  applyDefenderExclusion(location);
} else if (process.platform === "win32") {
  console.log("💡  Para excluir la ruta en Windows Defender (recomendado):");
  console.log(`   npm run setup:target -- --defender-exclusion`);
  console.log(`   # o manualmente: Add-MpPreference -ExclusionPath '${location}'`);
}
