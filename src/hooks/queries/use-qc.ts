import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { QcMaterialInput, QcRunInput } from "@/bindings";

export function useQcMaterials() {
  return useQuery({ queryKey: ["qc-materials"], queryFn: api.listQcMaterials });
}

export function useQcTargets(controlMaterialId: number | null) {
  return useQuery({
    queryKey: ["qc-targets", controlMaterialId],
    queryFn: () => api.listQcTargets(controlMaterialId!),
    enabled: controlMaterialId != null,
  });
}

export function useSaveQcMaterial() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: QcMaterialInput) => api.saveQcMaterial(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["qc-materials"] });
      qc.invalidateQueries({ queryKey: ["qc-analyzer-status"] });
    },
  });
}

export function useDeleteQcMaterial() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteQcMaterial(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["qc-materials"] });
      qc.invalidateQueries({ queryKey: ["qc-runs"] });
      qc.invalidateQueries({ queryKey: ["qc-analyzer-status"] });
    },
  });
}

export function useRecordQcRun() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: QcRunInput) => api.recordQcRun(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["qc-runs"] });
      qc.invalidateQueries({ queryKey: ["qc-chart"] });
      qc.invalidateQueries({ queryKey: ["qc-analyzer-status"] });
    },
  });
}

export function useQcRuns(controlMaterialId: number | null) {
  return useQuery({
    queryKey: ["qc-runs", controlMaterialId],
    queryFn: () => api.listQcRuns(controlMaterialId),
  });
}

export function useDeleteQcRun() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteQcRun(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["qc-runs"] });
      qc.invalidateQueries({ queryKey: ["qc-chart"] });
      qc.invalidateQueries({ queryKey: ["qc-analyzer-status"] });
    },
  });
}

export function useQcChart(controlMaterialId: number | null, analyteId: number | null) {
  return useQuery({
    queryKey: ["qc-chart", controlMaterialId, analyteId],
    queryFn: () => api.getQcChart(controlMaterialId!, analyteId!),
    enabled: controlMaterialId != null && analyteId != null,
  });
}

export function useQcAnalyzerStatus() {
  return useQuery({
    queryKey: ["qc-analyzer-status"],
    queryFn: api.listQcAnalyzerStatus,
  });
}