import { useCallback, useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { toast } from "sonner";

export interface UpdateProgress {
  downloaded: number;
  contentLength: number;
}

/**
 * Comprueba actualizaciones una sola vez al arrancar la ventana principal
 * (solo en builds de producción; en `vite dev` el plugin no molesta).
 * Si hay una versión nueva disponible, expone el estado y las acciones para
 * descargarla, instalarla y reiniciar la aplicación.
 */
export function useAppUpdater() {
  const [available, setAvailable] = useState<Update | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const checkedRef = useRef(false);

  useEffect(() => {
    if (checkedRef.current) return;
    checkedRef.current = true;

    if (import.meta.env.DEV) return;

    let cancelled = false;
    (async () => {
      try {
        const update = await check();
        if (!cancelled && update) setAvailable(update);
      } catch {
        // Sin conexión o navegador sin runtime Tauri: se ignora en silencio.
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  const dismiss = useCallback(() => setAvailable(null), []);

  const install = useCallback(async () => {
    if (!available) return;
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
    }
  }, [available]);

  return { available, downloading, progress, install, dismiss };
}
