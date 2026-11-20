import { useEffect, useState } from "react";
import { getAppVersion } from "@/lib/app-version";

/**
 * Versión instalada de ISALAB, compartida por el pie del sidebar ("Acerca de"
 * del menú) y el diálogo Acerca de: así ambos muestran siempre el mismo valor
 * (Tauri `getVersion` y, fuera del runtime, package.json inyectado por Vite).
 *
 * `null` mientras se resuelve.
 */
export function useAppVersion(): string | null {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void getAppVersion().then((v) => {
      if (!cancelled) setVersion(v);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  return version;
}
