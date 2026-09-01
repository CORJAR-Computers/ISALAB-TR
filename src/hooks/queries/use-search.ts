import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

/** Búsqueda global (paleta Ctrl+K) con debounce aplicado en el componente. */
export function useGlobalSearch(query: string, enabled = true) {
  const trimmed = query.trim();
  return useQuery({
    queryKey: ["global-search", trimmed],
    queryFn: () => api.globalSearch(trimmed),
    enabled: enabled && trimmed.length > 0,
    placeholderData: (prev) => prev,
    staleTime: 30_000,
  });
}
