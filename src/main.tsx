import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Toaster } from "@/components/ui/sonner";
import App from "./App";
import "./index.css";

const queryClient = new QueryClient({
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
