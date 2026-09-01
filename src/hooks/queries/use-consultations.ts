import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { CreateConsultationInput } from "@/bindings";

export function useConsultations(status: string | null, search: string, enabled = true) {
  return useQuery({
    queryKey: ["consultations", status, search],
    queryFn: () => api.listConsultations(status, search.trim() || null),
    placeholderData: (prev) => prev,
    enabled,
  });
}

/** Contadores por estado (sin filtros) para las pestañas de la agenda. */
export function useConsultationCounts() {
  return useQuery({
    queryKey: ["consultation-counts"],
    queryFn: api.countConsultations,
  });
}

export function useSetConsultationStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: string }) =>
      api.setConsultationStatus(id, status),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["consultations"] });
      qc.invalidateQueries({ queryKey: ["consultation-counts"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export function useCreateConsultation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateConsultationInput) =>
      api.createConsultation(input),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({
        queryKey: ["clinical-history", vars.patientId],
      });
      qc.invalidateQueries({ queryKey: ["patient", vars.patientId] });
      qc.invalidateQueries({ queryKey: ["consultations"] });
      qc.invalidateQueries({ queryKey: ["consultation-counts"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}
