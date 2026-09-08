import { useCallback, useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { toast } from "sonner";

export interface UpdateProgress {
  downloaded: number;
  contentLength: number;
}

/** Cada cuánto se vuelve a comprobar actualizaciones con la app abierta. */
const UPDATE_CHECK_INTERVAL_MS = 4 * 60 * 60 * 1000; // cada 4 horas

/**
 * Comprueba actualizaciones al arrancar la ventana principal y, mientras la
 * app permanece abierta, de forma periódica (cada 4 h). Ambas son silenciosas
 * y solo se ejecutan en builds de producción (en `vite dev` el plugin no
 * molesta). Expone además `checkNow()` para la comprobación manual con
 * feedback (p. ej. desde el diálogo "Acerca de").
 */
export function useAppUpdater() {
  const [available, setAvailable] = useState<Update | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [checking, setChecking] = useState(false);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const checkedRef = useRef(false);
  const checkingRef = useRef(false);
  const installingRef = useRef(false);

  const runCheck = useCallback(async (notify: boolean) => {
    if (checkingRef.current) return null;
    checkingRef.current = true;
    if (notify) setChecking(true);

    try {
      if (import.meta.env.DEV) {
        if (notify) {
          toast.info(
            "La comprobación de actualizaciones solo está disponible en la versión instalada de ISALAB.",
          );
        }
        return null;
      }

      const update = await check();
      if (update) {
        setAvailable(update);
      } else if (notify) {
        toast.success("Estás usando la versión más reciente de ISALAB.");
      }
      return update;
    } catch {
      if (notify) {
        toast.error(
          "No se pudo comprobar actualizaciones. Revisa tu conexión a internet e inténtalo de nuevo.",
        );
      }
      return null;
    } finally {
      checkingRef.current = false;
      if (notify) setChecking(false);
    }
  }, []);

  // Comprobación automática al arrancar (silenciosa: no molesta si no hay novedades).
  useEffect(() => {
    if (checkedRef.current) return;
    checkedRef.current = true;
    void runCheck(false);
  }, [runCheck]);

  // Comprobación periódica mientras la app está abierta (silenciosa). Si en
  // una pasada aparece una versión nueva, se abre el diálogo de actualización.
  useEffect(() => {
    if (import.meta.env.DEV) return;
    const id = window.setInterval(() => {
      void runCheck(false);
    }, UPDATE_CHECK_INTERVAL_MS);
    return () => window.clearInterval(id);
  }, [runCheck]);

  const checkNow = useCallback(() => {
    void runCheck(true);
  }, [runCheck]);

  const dismiss = useCallback(() => setAvailable(null), []);

  const install = useCallback(async () => {
    if (!available || installingRef.current) return;
    installingRef.current = true;
    setDownloading(true);
    setProgress({ downloaded: 0, contentLength: 0 });

    try {
      await available.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            setProgress({
              downloaded: 0,
              contentLength: event.data.contentLength ?? 0,
            });
            break;
          case "Progress":
            setProgress((prev) => ({
              downloaded: (prev?.downloaded ?? 0) + event.data.chunkLength,
              contentLength: prev?.contentLength ?? 0,
            }));
            break;
          case "Finished":
            break;
        }
      });
      setAvailable(null);
      toast.success("Actualización instalada. Reiniciando la aplicación…");
      setTimeout(() => {
        void relaunch();
      }, 800);
    } catch (error) {
      console.error("Error al instalar la actualización", error);
      toast.error("No se pudo instalar la actualización. Inténtalo de nuevo.");
      setDownloading(false);
      setProgress(null);
      installingRef.current = false;
    }
  }, [available]);

  return { available, downloading, checking, progress, checkNow, install, dismiss };
}