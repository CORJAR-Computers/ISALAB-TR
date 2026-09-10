import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

/**
 * Sin `refetchInterval`: las métricas del dashboard derivan de
 * muestras/resultados, cuyos cambios llegan al frontend por los eventos
 * Firebird (use-firebird-events.ts), que invalidan `["dashboard"]` — también
 * para las escrituras en segundo plano (importación por carpeta vigilada del
 * analizador), que no tienen mutation del lado UI. El polling de 30 s era
 * redundante (H6 de la revisión de escalabilidad).
 */
export function useDashboardStats() {
  return useQuery({
    queryKey: ["dashboard"],
    queryFn: api.getDashboardStats,
  });
}
