import type { Page } from "@playwright/test";

/**
 * Control de la inyección de fallos del mock IPC (e2e/ipc-mock.script.js).
 *
 * Permite que los comandos seleccionados rechacen como lo haría el backend con
 * la base caída, para comprobar que la UI muestra el error (no un estado vacío
 * silencioso) y que el reintento recupera de verdad.
 */
export type FaultConfig = {
  /** Todos los comandos del mock. */
  all?: boolean;
  /** Comandos exactos, p. ej. ["get_session"]. */
  commands?: string[];
  /** Grupos con nombre del mock: session, dashboard, reception, patients… */
  groups?: string[];
  /** Prefijos de comando, p. ej. ["list_", "count_"]. */
  prefixes?: string[];
  /** Solo las próximas N llamadas fallan (fallo de una vez). */
  remaining?: number;
  /** Tipo del AppError simulado. Por defecto "Database". */
  errorType?: string;
  /** Mensaje del AppError simulado. Por defecto el de base caída. */
  message?: string;
};

export const FAULTS_KEY = "e2e:ipc-faults";

export const DB_DOWN_MESSAGE =
  "No se pudo conectar con la base de datos (Firebird no responde).";

declare global {
  interface Window {
    __E2E_IPC_FAULTS__: {
      key: string;
      groups(): string[];
      get(): FaultConfig;
      set(config: FaultConfig): FaultConfig;
      clear(): FaultConfig;
      stats(): { total: number; byCommand: Record<string, number> };
    };
  }
}

/**
 * Fija los fallos ANTES de que arranque la app (se leen de localStorage en cada
 * invoke, así sobreviven a un reload). Úsalo cuando el fallo debe afectar al
 * bootstrap, p. ej. get_session.
 */
export async function setFaultsBeforeLoad(page: Page, config: FaultConfig) {
  await page.addInitScript(
    ({ key, json }: { key: string; json: string }) => {
      window.localStorage.setItem(key, json);
    },
    { key: FAULTS_KEY, json: JSON.stringify(config) },
  );
}

/** Fija los fallos con la app ya cargada (para fallos puntuales de una vista). */
export async function setFaults(page: Page, config: FaultConfig) {
  await page.evaluate((cfg) => {
    window.__E2E_IPC_FAULTS__.set(cfg);
  }, config);
}

/** Quita todos los fallos: la siguiente consulta o mutación funciona. */
export async function clearFaults(page: Page) {
  await page.evaluate(() => {
    window.__E2E_IPC_FAULTS__.clear();
  });
}

/** Nº de invocaciones de un comando (incluye las fallidas). */
export async function callCount(page: Page, command: string) {
  return page.evaluate(
    (cmd) => window.__E2E_IPC_FAULTS__.stats().byCommand[cmd] ?? 0,
    command,
  );
}
