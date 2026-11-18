import type { DbStatus } from "@/bindings";

/** Etiqueta corta del estado de conexión, para banners y diálogos. */
export const DB_STATUS_LABEL: Record<DbStatus, string> = {
  notConfigured: "Servidor sin configurar",
  clientMissing: "Cliente Firebird no encontrado",
  serverUnreachable: "Servidor inaccesible",
  authFailed: "Credenciales rechazadas",
  schemaTooOld: "Base de datos desactualizada",
  schemaTooNew: "Base de datos más nueva que la app",
  connected: "Conectado",
};

/** Variante visual del badge para cada estado. */
export function dbStatusVariant(
  status: DbStatus,
): "success" | "warning" | "destructive" {
  switch (status) {
    case "connected":
      return "success";
    case "schemaTooOld":
    case "schemaTooNew":
      return "warning";
    default:
      return "destructive";
  }
}
