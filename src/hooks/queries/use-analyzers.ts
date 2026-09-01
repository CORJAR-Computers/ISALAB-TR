import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { CreateAnalyzerInput, ReferenceRangeInput, UpdateAnalyzerInput } from "@/bindings";

/** Equipos analizadores (para el selector de muestras y la gestión). */
export function useAnalyzers() {
  return useQuery({ queryKey: ["analyzers"], queryFn: api.listAnalyzers });
}

/** Rangos de referencia de un equipo (o todos si `analyzerId` es null). */
export function useReferenceRanges(analyzerId: number | null) {
  return useQuery({
    queryKey: ["reference-ranges", analyzerId],
    queryFn: () => api.listReferenceRanges(analyzerId),
    enabled: analyzerId != null,
  });
}

export function useCreateAnalyzer() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateAnalyzerInput) => api.createAnalyzer(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzers"] }),
  });
}

export function useUpdateAnalyzer() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdateAnalyzerInput) => api.updateAnalyzer(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzers"] }),
  });
}

export function useSetAnalyzerActive() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, active }: { id: number; active: boolean }) =>
      api.setAnalyzerActive(id, active),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzers"] }),
  });
}

export function useDeleteAnalyzer() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteAnalyzer(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzers"] }),
  });
}

export function useCreateReferenceRange() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: ReferenceRangeInput) =>
      api.createReferenceRange(input),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: ["reference-ranges"] }),
  });
}

export function useUpdateReferenceRange() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: number; input: ReferenceRangeInput }) =>
      api.updateReferenceRange(id, input),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: ["reference-ranges"] }),
  });
}

export function useDeleteReferenceRange() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteReferenceRange(id),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: ["reference-ranges"] }),
  });
}
