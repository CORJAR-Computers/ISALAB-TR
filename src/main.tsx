import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import {
  MutationCache,
  QueryCache,
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import { Toaster } from "@/components/ui/sonner";
import App from "./App";
import "./index.css";

// Red de seguridad de diagnóstico: las páginas muestran su propio estado de
// error, pero cuando una query o mutación falla en segundo plano (sin UI que
// la lea) el error se perdería. Aquí queda en consola con su clave, que es lo
// mínimo para diagnosticar un fallo de base de datos o de IPC.
const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error, query) => {
      console.error("[query] fallo al consultar", query.queryKey, error);
    },
  }),
  mutationCache: new MutationCache({
    onError: (error, _vars, _ctx, mutation) => {
      console.error(
        "[mutation] fallo al ejecutar",
        mutation.options.mutationKey ?? mutation.options.scope,
        error,
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
