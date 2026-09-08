// ============================================================================
// Guarda de bindings: verifica que src/bindings.ts esté sincronizado con la
// lista de comandos registrada en src-tauri/src/lib.rs (collect_commands!).
//
// Contexto: los bindings los regenera specta en cada build debug (npm run
// tauri dev), pero el repo conserva una copia comprometida. Históricamente el
// drift pasó desapercibido en CI porque la UI aún no invocaba los comandos
// nuevos (tsc no fallaba), así que 8 comandos de lab orders convivieron
// semanas sin bindings. Este script falla en cuanto Rust registra un comando
// que la copia comprometida no conoce.
//
// Qué comprueba (intencionalmente solo lo estable):
//   1. Cada comando de collect_commands! tiene su __TAURI_INVOKE("<snake>")
//      en bindings.ts.
//   2. Cada __TAURI_INVOKE de bindings.ts sigue registrado en lib.rs (si no,
//      sería un comando eliminado con bindings huérfanos).
//
// No compara textos completos: los doc-comments y el orden cambian con
// cualquier toque de specta; eso se regenera solo, sin romper CI.
//
// Uso:      node scripts/check-bindings.mjs
// CI:       paso "Check bindings.ts is up to date" en el job frontend.
// ============================================================================

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const libRs = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf-8");
const bindings = readFileSync(join(root, "src/bindings.ts"), "utf-8");

// Comandos registrados: los identificadores dentro de collect_commands![...]
// (elementos con commas; el delimitador de cierre es `])` seguido de punto).
const collectMatch = libRs.match(/collect_commands!\[([\s\S]*?)\]\)/);
if (!collectMatch) {
  console.error("No se encontró collect_commands![] en src-tauri/src/lib.rs");
  process.exit(1);
}
const rustCommands = new Set(
  collectMatch[1]
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0 && /^[a-z_][a-z0-9_]*$/.test(s)),
);

// Comandos consumidos por la copia comprometida de los bindings. Se admiten
// las dos formas que genera specta: `__TAURI_INVOKE<...>("cmd", ...)` (sin
// AppError, p. ej. dbHealth) y `__TAURI_INVOKE("cmd", ...)`, envuelta en
// typedError.
const invoked = new Set(
  [...bindings.matchAll(/__TAURI_INVOKE[^("]*\("([a-z0-9_]+)"/g)].map(
    (m) => m[1],
  ),
);

const missingInBindings = [...rustCommands].filter((c) => !invoked.has(c));
const orphanInBindings = [...invoked].filter((c) => !rustCommands.has(c));

if (missingInBindings.length === 0 && orphanInBindings.length === 0) {
  console.log(
    `bindings.ts OK: ${rustCommands.size} comandos registrados y sincronizados.`,
  );
  process.exit(0);
}

if (missingInBindings.length > 0) {
  console.error(
    `Faltan ${missingInBindings.length} comando(s) en src/bindings.ts:\n` +
      missingInBindings.map((c) => `  - ${c}`).join("\n") +
      "\n\nRegenera los bindings: npm run tauri dev (cualquier build debug)\ny haz commit del src/bindings.ts resultante.",
  );
}
if (orphanInBindings.length > 0) {
  console.error(
    `${orphanInBindings.length} comando(s) en src/bindings.ts ya no están registrados en lib.rs:\n` +
      orphanInBindings.map((c) => `  - ${c}`).join("\n"),
  );
}
process.exit(1);
