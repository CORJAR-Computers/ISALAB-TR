import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  AnalyzerImportMapping,
  CreateSampleInput,
  PanelInput,
  RegisterResultInput,
  RegisterResultsInput,
} from "@/bindings";

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

/** Carga por lotes (grilla de panel / importación): invalida la ficha y los
 *  contadores de la mesa de trabajo. */
export function useRegisterLabResults() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: RegisterResultsInput) => api.registerLabResults(input),
    onSuccess: (results) => {
      const sampleId = results[0]?.sampleId;
      qc.invalidateQueries({ queryKey: ["clinical-history"] });
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      if (sampleId != null) qc.invalidateQueries({ queryKey: ["sample", sampleId] });
    },
  });
}

/** Registra la calidad preanalítica (interferencia HIL) de una muestra. */
export function useSetSampleQuality() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: {
      id: number;
      qualityIndex: string | null;
      qualitySeverity: string | null;
      qualityNote: string | null;
    }) => api.setSampleQuality(input.id, input.qualityIndex, input.qualitySeverity, input.qualityNote),
    onSuccess: (sample) => {
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample", sample.id] });
    },
  });
}

/** Rechaza una muestra con motivo (RECIBIDA/EN_PROCESO → RECHAZADA). */
export function useRejectSample() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, reason }: { id: number; reason: string }) =>
      api.rejectSample(id, reason),
    onSuccess: (sample) => {
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["sample", sample.id] });
    },
  });
}

/** Reabre una muestra rechazada (RECHAZADA → RECIBIDA). */
export function useReopenSample() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.reopenSample(id),
    onSuccess: (sample) => {
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["sample", sample.id] });
    },
  });
}

// ---- Paneles de analitos ----

export function usePanels() {
  return useQuery({ queryKey: ["panels"], queryFn: api.listPanels });
}

export function usePanelAnalytes(panelId: number | null) {
  return useQuery({
    queryKey: ["panel-analytes", panelId],
    queryFn: () => api.listPanelAnalytes(panelId!),
    enabled: panelId != null,
  });
}

export function useSavePanel() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: PanelInput) => api.savePanel(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["panels"] }),
  });
}

export function useDeletePanel() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deletePanel(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["panels"] }),
  });
}

// ---- Importación desde analizador (CSV) ----

export function usePreviewAnalyzerImport() {
  return useMutation({ mutationFn: (path: string) => api.previewAnalyzerImport(path) });
}

export function useImportAnalyzerResults() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: { path: string; mapping: AnalyzerImportMapping }) =>
      api.importAnalyzerResults(input.path, input.mapping),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["clinical-history"] });
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
