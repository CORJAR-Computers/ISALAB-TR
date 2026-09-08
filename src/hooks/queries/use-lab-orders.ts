import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { AccessionOrderInput, CreateLabOrderInput } from "@/bindings";

export function useLabOrders(status: string | null, search: string, enabled = true) {
  return useQuery({
    queryKey: ["lab-orders", status, search],
    queryFn: () => api.listLabOrders(status, search.trim() || null),
    placeholderData: (prev) => prev,
    enabled,
  });
}

/** Contadores por estado (sin filtros) para las pestañas de órdenes. */
export function useLabOrderCounts() {
  return useQuery({
    queryKey: ["lab-order-counts"],
    queryFn: api.countLabOrders,
  });
}

/** Detalle de una orden: pruebas solicitadas y muestras accesionadas. */
export function useLabOrder(id: number | null) {
  return useQuery({
    queryKey: ["lab-order", id],
    queryFn: () => api.getLabOrder(id!),
    enabled: id != null,
  });
}

/** Órdenes de un paciente (sección "Órdenes de laboratorio" del historial). */
export function usePatientLabOrders(patientId: number | null) {
  return useQuery({
    queryKey: ["lab-orders", "patient", patientId],
    queryFn: () => api.listPatientLabOrders(patientId!),
    enabled: patientId != null,
  });
}

/** Orden de la que proviene una muestra (null si no nació de una orden). */
export function useOrderForSample(sampleId: number | null) {
  return useQuery({
    queryKey: ["lab-orders", "for-sample", sampleId],
    queryFn: () => api.getOrderForSample(sampleId!),
    enabled: sampleId != null,
  });
}

export function useCreateLabOrder() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateLabOrderInput) => api.createLabOrder(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["lab-orders"] });
      qc.invalidateQueries({ queryKey: ["lab-order-counts"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export function useSetLabOrderStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: string }) =>
      api.setLabOrderStatus(id, status),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["lab-orders"] });
      qc.invalidateQueries({ queryKey: ["lab-order-counts"] });
      qc.invalidateQueries({ queryKey: ["lab-order"] });
      qc.invalidateQueries({ queryKey: ["lab-orders", "for-sample"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export function useAccessionLabOrder() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: AccessionOrderInput) => api.accessionLabOrder(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["lab-orders"] });
      qc.invalidateQueries({ queryKey: ["lab-order"] });
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}
