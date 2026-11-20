import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import {
  MutationCache,
  QueryCache,
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import { Toaster } from "@/components/ui/sonner";
import { api } from "@/lib/api";
import App from "./App";
import "./index.css";

/** Texto compacto de cualquier valor lanzado (Error, string, objeto serializable). */
function describeThrown(value: unknown): string {
  if (value instanceof Error) return `${value.name}: ${value.message}`;
  try {
    return JSON.stringify(value) ?? String(value);
  } catch {
    return String(value);
  }
}

/**
 * Reenvía el error al log en disco (`<app_data>/logs/isalab.log`).
 *
 * En la app empaquetada la consola del webview no deja rastro, así que un
 * fallo sin UI que lo muestre solo se puede diagnosticar si llega al archivo.
 * Es best-effort: nunca rompe la acción del usuario ni espera respuesta.
 */
function reportClientError(
  level: "error" | "warn",
  message: string,
  context?: string,
) {
  void api.logFrontend(level, message, context ?? null).catch(() => {
    /* sin log disponible: no es motivo para fallar la operación */
  });
}

// Red de seguridad de diagnóstico: las páginas muestran su propio estado de
// error, pero cuando una query o mutación falla en segundo plano (sin UI que
// la lea) el error se perdería. Aquí queda en consola con su clave, que es lo
// mínimo para diagnosticar un fallo de base de datos o de IPC.
// Errores que ninguna UI muestra (query de fondo, mutación sin diálogo) se
// registran en consola y en el log en disco: son justo los que nadie reporta.
window.addEventListener("error", (event) => {
  reportClientError(
    "error",
    event.message || "Error no controlado en la UI",
    `${event.filename || "?"}:${event.lineno}:${event.colno}`,
  );
});
window.addEventListener("unhandledrejection", (event) => {
  reportClientError(
    "error",
    "Promesa rechazada sin manejar",
    describeThrown(event.reason),
  );
});

const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error, query) => {
      console.error("[query] fallo al consultar", query.queryKey, error);
      reportClientError(
        "error",
        `Fallo al consultar ${describeThrown(error)}`,
        JSON.stringify(query.queryKey),
      );
    },
  }),
  mutationCache: new MutationCache({
    onError: (error, _vars, _ctx, mutation) => {
      console.error(
        "[mutation] fallo al ejecutar",
        mutation.options.mutationKey ?? mutation.options.scope,
        error,
      );
      reportClientError(
        "error",
        `Fallo al ejecutar ${describeThrown(error)}`,
        JSON.stringify(mutation.options.mutationKey ?? mutation.options.scope ?? null),
      );
    },
  }),
  defaultOptions: {
    queries: {
      staleTime: 10_000,
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

// La ventana splash de Tauri ya NO carga este bundle: apunta a un HTML
// estático (src/splash.html → splash.html en el build) que se pinta al
// instante sin esperar a que Vite/React bootstrapeen. Este bundle solo
// renderiza la app principal.
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
      <Toaster
        position="top-right"
        richColors
        toastOptions={{ className: "font-sans" }}
      />
    </QueryClientProvider>
  </StrictMode>,
);
