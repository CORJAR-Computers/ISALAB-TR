import { defineConfig, devices } from "@playwright/test";

/**
 * Smoke test E2E del frontend (React) contra el dev server de Vite.
 *
 * La app es Tauri de escritorio: en el navegador no existe el runtime de
 * Tauri, así que los tests inyectan un mock de `window.__TAURI_INTERNALS__`
 * (ver e2e/ipc-mock.script.js) que implementa los comandos del flujo smoke
 * con estado en memoria. El backend real (Rust + Firebird) está cubierto por
 * `cargo test`; aquí se valida la UI de extremo a extremo.
 *
 * `npm run dev` expone el servidor en http://localhost:1420 (strictPort).
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  timeout: 60_000,
  expect: { timeout: 10_000 },
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI
    ? [["list"], ["html", { open: "never" }]]
    : "list",

  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    viewport: { width: 1280, height: 800 },
  },

  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],

  webServer: {
    command: "npm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
