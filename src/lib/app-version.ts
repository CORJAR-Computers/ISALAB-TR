import { getVersion } from "@tauri-apps/api/app";

// Inyectado por Vite desde package.json (define en vite/vitest config).
// Solo se usa como respaldo fuera del runtime Tauri (navegador dev / tests).
declare const __APP_VERSION__: string | undefined;

let cached: string | null = null;

/**
 * Versión instalada de ISALAB. Fuente principal: el runtime Tauri
 * (`getVersion`, lee tauri.conf.json de la app instalada). Si no hay runtime
 * Tauri (navegador en `vite dev`, tests, preview), cae al valor de
 * package.json inyectado por Vite.
 */
/** Versión de package.json inyectada por Vite (respaldo sin runtime Tauri). */
function fallbackVersion(): string {
  return typeof __APP_VERSION__ === "string" ? __APP_VERSION__ : "0.0.0";
}

export async function getAppVersion(): Promise<string> {
  if (cached) return cached;
  try {
    // El runtime puede responder vacío (p. ej. un mock de IPC o un plugin
    // ausente): en ese caso también se usa el respaldo de package.json.
    cached = (await getVersion()) || fallbackVersion();
  } catch {
    cached = fallbackVersion();
  }
  return cached;
}