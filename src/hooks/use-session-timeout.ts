import { useEffect, useRef } from "react";
import { toast } from "sonner";
import { useSessionStore } from "@/stores/session-store";

// 45 minutos de inactividad
const TIMEOUT_MS = 45 * 60 * 1000;

export function useSessionTimeout() {
  const session = useSessionStore((s) => s.session);
  const logout = useSessionStore((s) => s.logout);
  const timerRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    // Si no hay sesión, no necesitamos controlar inactividad
    if (!session) return;

    const resetTimer = () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      timerRef.current = setTimeout(() => {
        void handleTimeout();
      }, TIMEOUT_MS);
    };

    const handleTimeout = async () => {
      // Limpiar eventos preventivamente
      cleanup();
      
      // Cerrar sesión
      await logout();
      
      toast.error("Sesión expirada por inactividad", {
        description: "Por tu seguridad, la sesión se ha cerrado tras 45 minutos de inactividad.",
        duration: 8000,
      });
    };

    const cleanup = () => {
      window.removeEventListener("mousemove", resetTimer);
      window.removeEventListener("keydown", resetTimer);
      window.removeEventListener("click", resetTimer);
      window.removeEventListener("scroll", resetTimer);
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };

    // Registrar eventos para detectar actividad
    window.addEventListener("mousemove", resetTimer);
    window.addEventListener("keydown", resetTimer);
    window.addEventListener("click", resetTimer);
    window.addEventListener("scroll", resetTimer);

    // Iniciar el temporizador
    resetTimer();

    return cleanup;
  }, [session, logout]);
}
