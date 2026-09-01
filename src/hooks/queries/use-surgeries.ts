import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { CreateSurgeryInput } from "@/bindings";

export function useSurgeries(status: string | null, search: string, enabled = true) {
  return useQuery({
    queryKey: ["surgeries", status, search],
    queryFn: () => api.listSurgeries(status, search.trim() || null),
    placeholderData: (prev) => prev,
    enabled,
  });
}

/** Contadores por estado (sin filtros) para las pestañas de la agenda quirúrgica. */
export function useSurgeryCounts() {
  return useQuery({
    queryKey: ["surgery-counts"],
    queryFn: api.countSurgeries,
  });
}

export function useCreateSurgery() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateSurgeryInput) => api.createSurgery(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["surgeries"] });
      qc.invalidateQueries({ queryKey: ["surgery-counts"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export function useSetSurgeryStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: string }) =>
      api.setSurgeryStatus(id, status),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["surgeries"] });
      qc.invalidateQueries({ queryKey: ["surgery-counts"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}
