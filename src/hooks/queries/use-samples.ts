import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { CreateSampleInput, RegisterResultInput } from "@/bindings";

export function useSamples(status: string | null, search: string, enabled = true) {
  return useQuery({
    queryKey: ["samples", status, search],
    queryFn: () => api.listSamples(status, search.trim() || null),
    placeholderData: (prev) => prev,
    enabled,
  });
}

/** Contadores por estado (sin filtros) para las pestañas de la mesa de trabajo. */
export function useSampleCounts() {
  return useQuery({
    queryKey: ["sample-counts"],
    queryFn: api.countSamples,
  });
}

/** Bandeja de trabajo diaria: pendientes por tipo con tiempo transcurrido.
 *  Se auto-refresca cada minuto porque el tiempo transcurrido avanza en vivo. */
export function useWorklist() {
  return useQuery({
    queryKey: ["worklist"],
    queryFn: api.getWorklist,
    refetchInterval: 60_000,
  });
}

export function useSample(id: number | null) {
  return useQuery({
    queryKey: ["sample", id],
    queryFn: () => api.getSample(id!),
    enabled: id != null,
  });
}

export function useCreateSample() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateSampleInput) => api.createSample(input),
    onSuccess: (sample) => {
      qc.invalidateQueries({
        queryKey: ["clinical-history", sample.patientId],
      });
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export function useRegisterLabResult() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: RegisterResultInput) => api.registerLabResult(input),
    onSuccess: (result) => {
      qc.invalidateQueries({ queryKey: ["clinical-history"] });
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["sample", result.sampleId] });
    },
  });
}

export function useSetSampleStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: string }) =>
      api.setSampleStatus(id, status),
    onSuccess: (sample) => {
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["sample", sample.id] });
      qc.invalidateQueries({
        queryKey: ["clinical-history", sample.patientId],
      });
    },
  });
}

/** Adjunta fotos (placas, frotis, electroforesis) a un resultado y refresca
 *  la ficha de la muestra y el historial del paciente. */
export function useAttachResultFile(sampleId: number | null) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ resultId, sourcePath }: { resultId: number; sourcePath: string }) =>
      api.attachResultFile(resultId, sourcePath),
    onSuccess: () => {
      if (sampleId != null) {
        qc.invalidateQueries({ queryKey: ["sample", sampleId] });
      }
      qc.invalidateQueries({ queryKey: ["clinical-history"] });
    },
  });
}

/** Elimina un adjunto (borra archivo + registro) y refresca la ficha. */
export function useDeleteResultAttachment(sampleId: number | null) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteResultAttachment(id),
    onSuccess: () => {
      if (sampleId != null) {
        qc.invalidateQueries({ queryKey: ["sample", sampleId] });
      }
      qc.invalidateQueries({ queryKey: ["clinical-history"] });
    },
  });
}
