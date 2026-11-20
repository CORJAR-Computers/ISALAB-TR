// ============================================================================
// Guarda del mock IPC de los E2E: todo comando que la UI puede invocar debe
// tener handler en e2e/ipc-mock.script.js o estar en la allowlist explícita
// (e2e/ipc-mock.allowlist.mjs).
//
// Contexto: el mock sustituye a window.__TAURI_INTERNALS__.invoke en los tests
// de Playwright. Antes, un comando sin handler devolvía null: la página pintaba
// un estado vacío, el spec seguía verde y la llamada quedaba sin cubrir (y sin
// documentar). Ahora el mock lanza un error legible, y este script cierra el
// otro lado: que la lista de handlers no se quede atrás cuando se añaden
// comandos nuevos a Rust/bindings.
//
// Qué comprueba:
//   1. Cada comando de src/bindings.ts y de src/lib/api.ts tiene handler.
//   2. Si no lo tiene, está en la allowlist (con su motivo por dominio).
//   3. No hay entradas obsoletas en la allowlist: ni comandos que ya no
//      existen en la UI, ni comandos que ya tienen handler.
//
// Uso:      node scripts/check-e2e-mock.mjs  (o npm run check:e2e-mock)
// Enganche: `npm run pretest` (también en CI vía `npm test`) y paso propio en
//           el job e2e de .github/workflows/ci.yml.
// ============================================================================

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { allowlistedCommands } from "../e2e/ipc-mock.allowlist.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf-8");

const bindings = read("src/bindings.ts");
const api = read("src/lib/api.ts");
const mock = read("e2e/ipc-mock.script.js");

// Comandos que la UI puede llegar a invocar. Se miran las dos capas: los
// bindings generados por specta y la capa real de la app (src/lib/api.ts, que
// llama a invoke() con el nombre del comando y es la que usan las páginas).
const uiCommands = new Set([
  ...[...bindings.matchAll(/__TAURI_INVOKE\(\s*"([a-z0-9_]+)"/g)].map((m) => m[1]),
  ...[...api.matchAll(/invoke(?:<[\s\S]*?>)?\(\s*"([a-z0-9_]+)"/g)].map((m) => m[1]),
]);

// Handlers del mock: las claves del objeto `handlers`, que van a 4 espacios
// (las claves anidadas de los valores van a más). Los comandos con nombre no
// identificador (plugin:event|listen) van entrecomillados.
const handlersStart = mock.indexOf("const handlers = {");
if (handlersStart === -1) {
  console.error("No se encontró `const handlers = {` en e2e/ipc-mock.script.js");
  process.exit(1);
}
const handlerCommands = new Set(
  [
    ...mock
      .slice(handlersStart)
      .matchAll(/^ {4}(?:"([^"]+)"|([A-Za-z0-9_]+)):/gm),
  ].map((m) => m[1] ?? m[2]),
);

const allowlist = new Set(allowlistedCommands);

const withoutHandler = [...uiCommands]
  .filter((c) => !handlerCommands.has(c) && !allowlist.has(c))
  .sort();
const staleAllowlist = [...allowlist]
  .filter((c) => !uiCommands.has(c))
  .sort();
const redundantAllowlist = [...allowlist]
  .filter((c) => handlerCommands.has(c))
  .sort();

const problems = [];
if (withoutHandler.length > 0) {
  problems.push(
    `${withoutHandler.length} comando(s) sin handler en el mock IPC ni en la allowlist:\n` +
      withoutHandler.map((c) => `  - ${c}`).join("\n") +
      "\n\nAñade el handler en e2e/ipc-mock.script.js (con estado en memoria) o " +
      "declara el comando en e2e/ipc-mock.allowlist.mjs si los E2E no deben " +
      "ejercitarlo.",
  );
}
if (staleAllowlist.length > 0) {
  problems.push(
    `${staleAllowlist.length} entrada(s) de la allowlist que ya no son comandos de la UI:\n` +
      staleAllowlist.map((c) => `  - ${c}`).join("\n") +
      "\n\nQuítalas de e2e/ipc-mock.allowlist.mjs (comando renombrado o eliminado).",
  );
}
if (redundantAllowlist.length > 0) {
  problems.push(
    `${redundantAllowlist.length} comando(s) en la allowlist que ya tienen handler:\n` +
      redundantAllowlist.map((c) => `  - ${c}`).join("\n") +
      "\n\nQuítalos de e2e/ipc-mock.allowlist.mjs: la allowlist solo documenta " +
      "lo que el mock no modela.",
  );
}

if (problems.length > 0) {
  console.error("Mock IPC de los E2E desincronizado.\n");
  console.error(problems.join("\n\n"));
  process.exit(1);
}

console.log(
  `Mock IPC OK: ${uiCommands.size} comandos de la UI ` +
    `(${handlerCommands.size} handlers, ${allowlist.size} en allowlist).`,
);
