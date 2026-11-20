import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import type { Page } from "@playwright/test";

/**
 * Instalación del mock IPC para los specs E2E.
 *
 * Se inyectan dos scripts antes de que arranque la app:
 *   1. `ipc-mock.script.js`: el runtime de Tauri con estado en memoria.
 *   2. `window.__E2E_IPC_SHAPES__`: las formas de `e2e/ipc-shapes.json`
 *      (generadas desde `src/bindings.ts`), que el mock usa para validar cada
 *      respuesta y cada payload. Un campo mal nombrado rompe el test con el
 *      nombre del comando y la ruta del campo, en vez de aparecer en la clínica
 *      como una pantalla vacía.
 */
const mockPath = fileURLToPath(new URL("./ipc-mock.script.js", import.meta.url));
const shapesPath = fileURLToPath(new URL("./ipc-shapes.json", import.meta.url));

// Se lee una vez por proceso de test y se inyecta como texto: el navegador no
// puede importar el JSON directamente en un init script.
const shapesScript = `window.__E2E_IPC_SHAPES__ = ${readFileSync(shapesPath, "utf8")};`;

export async function installIpcMock(page: Page) {
  // Las formas primero: el mock las consulta al validar cada respuesta.
  await page.addInitScript({ content: shapesScript });
  await page.addInitScript({ path: mockPath });
}
