import { Loader2 } from "lucide-react";
import logoSidebar from "@/assets/logo_sidebar.png";

/**
 * Ventana splash mostrada al iniciar la aplicación.
 * Se renderiza únicamente en la ventana Tauri etiquetada como "splash".
 */
export function SplashScreen() {
  return (
    <div className="from-primary via-primary/95 to-emerald-700 relative flex h-svh w-screen items-center justify-center overflow-hidden bg-gradient-to-br text-primary-foreground">
      {/* Fondo decorativo */}
      <div className="bg-white/10 absolute -top-24 -left-24 size-72 rounded-full blur-3xl animate-glow-pulse" />
      <div className="bg-white/10 absolute -right-24 -bottom-24 size-72 rounded-full blur-3xl animate-glow-pulse [animation-delay:1.5s]" />

      <div className="animate-fade-in-up relative flex flex-col items-center gap-6">
        <div className="bg-white/15 flex h-28 w-64 items-center justify-center rounded-3xl p-4 shadow-2xl ring-1 ring-white/20 backdrop-blur-sm">
          <img
            src={logoSidebar}
            alt="ISALAB"
            className="max-h-full w-full object-contain"
            draggable={false}
          />
        </div>

        <div className="text-center">
          <p className="text-xl font-bold tracking-wide">ISALAB</p>
          <p className="text-white/70 text-xs">
            Laboratorio Veterinario
          </p>
        </div>

        <div className="flex items-center gap-2 text-white/80">
          <Loader2 className="size-4 animate-spin" />
          <span className="text-xs">Iniciando…</span>
        </div>
      </div>
    </div>
  );
}
