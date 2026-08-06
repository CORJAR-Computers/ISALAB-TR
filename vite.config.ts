import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  // Tauri expects a fixed port; fail if the port is in use.
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Tell Vite to ignore watching `src-tauri` — it will be recompiled by Rust.
      ignored: ["**/src-tauri/**"],
    },
  },
  // Env vars starting with the item of `envPrefix` will be exposed in tauri's
  // source code in `import.meta.env`. Default: VITE_*
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    // Tauri uses Chromium on Windows; improve browser compatibility.
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    // Vite 8 usa Oxc (rolldown) en vez de esbuild como minifier nativo.
    minify: !process.env.TAURI_ENV_DEBUG ? "oxc" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    rollupOptions: {
      output: {
        // Code-splitting: separa las dependencias pesadas (gráficas, markdown,
        // React, iconos) en chunks con caché propia para arranques más rápidos
        // en producción y mejor aprovechamiento del cache del webview.
        manualChunks(id: string) {
          if (!id.includes("node_modules")) return;
          // El orden de las comprobaciones importa: los grupos específicos
          // ("react-markdown", "recharts") deben evaluarse ANTES que el grupo
          // react genérico, que capturaría cualquier paquete con "react" o
          // "react-dom" en la ruta de su carpeta.
          if (
            id.includes("react-markdown") ||
            id.includes("remark-") ||
            id.includes("micromark") ||
            id.includes("mdast-") ||
            id.includes("unist-") ||
            id.includes("hast-") ||
            id.includes("unified") ||
            id.includes("vfile")
          ) {
            return "markdown";
          }
          if (
            id.includes("recharts") ||
            id.includes("d3-") ||
            id.includes("victory-vendor")
          ) {
            return "charts";
          }
          if (
            id.includes("/react/") ||
            id.includes("react-dom") ||
            id.includes("scheduler") ||
            // @floating-ui/react entra con React; sus dependencias (dom/utils)
            // también, para evitar el ciclo vendor -> react-vendor -> vendor.
            id.includes("@floating-ui")
          ) {
            return "react-vendor";
          }
          if (id.includes("lucide-react")) return "icons";
          return "vendor";
        },
      },
    },
  },
});
