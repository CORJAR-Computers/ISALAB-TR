import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Toaster } from "sonner";
import App from "./App";
import { SplashScreen } from "./components/splash/SplashScreen";
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

/**
 * Detecta si esta webview es la ventana splash de Tauri
 * (etiquetada "splash" en tauri.conf.json). En navegador plano
 * (vite/Chrome) no existe el runtime de Tauri y se devuelve false.
 */
function isSplashWindow(): boolean {
  try {
    return getCurrentWindow().label === "splash";
  } catch {
    return false;
  }
}

const isSplash = isSplashWindow();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      {isSplash ? (
        <SplashScreen />
      ) : (
        <>
          <App />
          <Toaster
            position="top-right"
            richColors
            toastOptions={{
              className: "font-sans",
            }}
          />
        </>
      )}
    </QueryClientProvider>
  </StrictMode>,
);
