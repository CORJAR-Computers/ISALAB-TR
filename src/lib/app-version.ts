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
export async function getAppVersion(): Promise<string> {
  if (cached) return cached;
  try {
    cached = await getVersion();
  } catch {
    cached = __APP_VERSION__ ?? "0.0.0";
  }
  return cached;
}