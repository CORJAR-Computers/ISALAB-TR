import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { SaveAnalyzerSourceInput } from "@/bindings";

/** Fuentes automáticas configuradas (carpeta vigilada por analizador). */
export function useAnalyzerSources() {
  return useQuery({
    queryKey: ["analyzer-sources"],
    queryFn: api.listAnalyzerSources,
  });
}

export function useSaveAnalyzerSource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: SaveAnalyzerSourceInput) => api.saveAnalyzerSource(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzer-sources"] }),
  });
}

export function useDeleteAnalyzerSource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteAnalyzerSource(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzer-sources"] }),
  });
}

/** Sondea una fuente ahora (probar tras guardar la carpeta). */
export function usePollAnalyzerSource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (sourceId: number) => api.pollAnalyzerSource(sourceId),
    onSuccess: (_jobs, sourceId) => {
      qc.invalidateQueries({ queryKey: ["analyzer-import-jobs", sourceId] });
      qc.invalidateQueries({ queryKey: ["analyzer-import-jobs", "failed"] });
      qc.invalidateQueries({ queryKey: ["analyzer-sources"] });
    },
  });
}

/** Cola de importación de una fuente (más recientes primero). */
export function useAnalyzerImportJobs(sourceId: number | null, limit = 30) {
  return useQuery({
    queryKey: ["analyzer-import-jobs", sourceId],
    queryFn: () => api.listAnalyzerImportJobs(sourceId as number, limit),
    enabled: sourceId != null,
  });
}

/** Trabajos fallidos de todas las fuentes (para el aviso global). */
export function useFailedAnalyzerImports(limit = 20) {
  return useQuery({
    queryKey: ["analyzer-import-jobs", "failed"],
    queryFn: () => api.listFailedAnalyzerImports(limit),
  });
}

export function useDeleteAnalyzerImportJob() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (jobId: number) => api.deleteAnalyzerImportJob(jobId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzer-import-jobs"] }),
  });
}
