import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { CreatePatientInput } from "@/bindings";

export function usePatients(search: string, enabled = true) {
  return useQuery({
    queryKey: ["patients", search],
    queryFn: () => api.listPatients(search),
    placeholderData: (prev) => prev,
    enabled,
  });
}

export function usePatient(id: number | null) {
  return useQuery({
    queryKey: ["patient", id],
    queryFn: () => api.getPatient(id!),
    enabled: id != null,
  });
}

/** Busca un paciente por su código (p. ej. escáner de código de barras). */
export function usePatientByCode(code: string | null) {
  const normalized = code?.trim() ?? null;
  return useQuery({
    queryKey: ["patient-by-code", normalized],
    queryFn: () => api.getPatientByCode(normalized!),
    enabled: normalized != null && normalized.length > 0,
  });
}

export function useClinicalHistory(patientId: number | null) {
  return useQuery({
    queryKey: ["clinical-history", patientId],
    queryFn: () => api.getClinicalHistory(patientId!),
    enabled: patientId != null,
  });
}

export function useCreatePatient() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreatePatientInput) => api.createPatient(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["patients"] });
    },
  });
}
