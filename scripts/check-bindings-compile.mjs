// ============================================================================
// Guarda de bindings #2: regenera src/bindings.ts y verifica que compile.
//
// Contexto: specta-typescript 0.0.12 imprimía *todo* `f64` como
// `number | null` (aunque el campo no fuera `Option`), así que regenerar con
// `cargo run --bin export_bindings` producía un archivo que rompía `tsc` en
// CashPage / InvoiceDetailDialog / PaymentDialog (specta-rs/specta#569). El
// repo se quedó con una copia arreglada a mano y la regeneración quedó
// prohibida de facto.
//
// `scripts/check-bindings.mjs` solo compara nombres de comandos: detecta
// comandos nuevos o eliminados, pero no un archivo que no compila. Este script
// cierra ese hueco:
//
//   1. Guarda el src/bindings.ts actual en memoria.
//   2. Regenera con el binario de export (debug).
//   3. Pasa `tsc -b --force` sobre el proyecto con el archivo recién generado.
//      Si el generador emite tipos que la UI no acepta, falla aquí y no en el
//      commit.
//   4. Compara las declaraciones `export type` del archivo regenerado con las
//      del archivo versionado: si Rust cambió un campo y nadie regeneró (o
//      alguien editó tipos a mano), también falla.
//   5. Restaura siempre el archivo original (el chequeo no ensucia el árbol).
//
// Uso:      node scripts/check-bindings-compile.mjs
// CI:       paso "Check generated bindings compile" en el job backend.
// Requiere: toolchain Rust (regenera) y node_modules instalado (tsc).
// ============================================================================

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const bindingsPath = join(root, "src/bindings.ts");
const original = readFileSync(bindingsPath, "utf-8");

/** Declaraciones `export type Nombre = { campo: tipo, ... };` -> Map anidado. */
const parseTypes = (src) => {
  const out = new Map();
  for (const m of src.matchAll(/export type (\w+) = \{([\s\S]*?)\n\};/g)) {
    const fields = new Map();
    for (const line of m[2].split("\n")) {
      const field = line.match(/^\s*(\w+)\??:\s*(.+?),\s*$/);
      if (field) fields.set(field[1], field[2].trim());
    }
    out.set(m[1], fields);
  }
  return out;
};

const diffTypes = (before, after) => {
  const diffs = [];
  for (const [type, fields] of after) {
    const prevType = before.get(type);
    if (!prevType) {
      diffs.push(`${type}: tipo exportado ahora y ausente en el archivo versionado`);
      continue;
    }
    for (const [field, ty] of fields) {
      const prev = prevType.get(field);
      if (prev === undefined) diffs.push(`${type}.${field}: campo nuevo (${ty})`);
      else if (prev !== ty) diffs.push(`${type}.${field}: ${prev} => ${ty}`);
    }
    for (const field of prevType.keys())
      if (!fields.has(field)) diffs.push(`${type}.${field}: campo eliminado`);
  }
  for (const type of before.keys())
    if (!after.has(type)) diffs.push(`${type}: tipo ya no se exporta`);
  return diffs;
};

const run = (cmd, args, cwd) =>
  execFileSync(cmd, args, { cwd, encoding: "utf-8", stdio: "pipe", shell: false });

let regenerated = "";
let failed = false;

try {
  // 1/2. Regenerar (el binario escribe en ../src/bindings.ts).
  try {
    run("cargo", ["run", "--quiet", "--bin", "export_bindings"], join(root, "src-tauri"));
  } catch (err) {
    console.error("No se pudo regenerar src/bindings.ts con el binario de export.");
    console.error(String(err.stderr ?? err.message).trim());
    process.exit(1);
  }
  regenerated = readFileSync(bindingsPath, "utf-8");

  // 3. El archivo generado tiene que compilar con el resto de la UI.
  //    Se invoca el binario de TypeScript con el propio node en vez de `npx`:
  //    en Windows `npx` es un .cmd y execFileSync no lo resuelve sin shell.
  const tsc = join(root, "node_modules/typescript/bin/tsc");
  try {
    run(process.execPath, [tsc, "-b", "--force"], root);
    console.log("tsc OK: el src/bindings.ts generado compila.");
  } catch (err) {
    failed = true;
    console.error(
      "El src/bindings.ts regenerado NO compila:\n\n" +
        String(err.stdout ?? "").trim() +
        "\n" +
        String(err.stderr ?? "").trim(),
    );
    console.error(
      "\nSuele ser specta exportando un tipo más laxo de lo que Rust produce " +
        "(p. ej. `f64` como `number | null`, specta-rs/specta#569). Revisa la " +
        "configuración del exportador en `specta_builder()` (src-tauri/src/lib.rs) " +
        "antes de tocar los tipos a mano.",
    );
  }

  // 4. Sin drift: los tipos del archivo versionado deben ser los que emite Rust.
  const committed = parseTypes(original);
  const generated = parseTypes(regenerated);
  const diffs = diffTypes(committed, generated);
  if (diffs.length > 0) {
    failed = true;
    console.error(
      `Los tipos de src/bindings.ts no coinciden con los que exporta Rust (${diffs.length} diferencia(s)):`,
    );
    for (const d of diffs.slice(0, 25)) console.error(`  - ${d}`);
    if (diffs.length > 25) console.error(`  ... y ${diffs.length - 25} más`);
    console.error(
      "\nRegenera y versiona el archivo: cd src-tauri && cargo run --bin export_bindings",
    );
  } else {
    console.log(`Tipos sincronizados: ${generated.size} declaraciones export type.`);
  }
} finally {
  // 5. Nunca dejar el árbol modificado por el chequeo.
  if (regenerated !== original) writeFileSync(bindingsPath, original);
}

process.exit(failed ? 1 : 0);
