import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  AddVisitServiceInput,
  CreateVisitInput,
  UpdateVisitServiceStatusInput,
} from "@/bindings";

/** Cola de visitas con filtros: fecha (YYYY-MM-DD), estado y búsqueda libre. */
export function useVisits(
  date: string | null,
  status: string | null,
  search: string,
  enabled = true,
) {
  return useQuery({
    queryKey: ["visits", date, status, search.trim()],
    queryFn: () => api.listVisits(date, status, search.trim() || null),
    placeholderData: (prev) => prev,
    enabled,
  });
}

/** Ficha completa de una visita (con líneas de servicio). */
export function useVisit(id: number | null) {
  return useQuery({
    queryKey: ["visit", id],
    queryFn: () => api.getVisit(id!),
    enabled: id != null,
  });
}

export function useCreateVisit() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateVisitInput) => api.createVisit(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["visits"] }),
  });
}

export function useSetVisitStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: string }) =>
      api.setVisitStatus(id, status),
    onSuccess: (visit) => {
      qc.invalidateQueries({ queryKey: ["visits"] });
      qc.invalidateQueries({ queryKey: ["visit", visit.id] });
    },
  });
}

export function useAddVisitService() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ visitId, input }: { visitId: number; input: AddVisitServiceInput }) =>
      api.addVisitService(visitId, input),
    onSuccess: (_line, vars) => {
      qc.invalidateQueries({ queryKey: ["visits"] });
      qc.invalidateQueries({ queryKey: ["visit", vars.visitId] });
    },
  });
}

export function useRemoveVisitService() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ visitId, lineId }: { visitId: number; lineId: number }) =>
      api.removeVisitService(visitId, lineId),
    onSuccess: (_void, vars) => {
      qc.invalidateQueries({ queryKey: ["visits"] });
      qc.invalidateQueries({ queryKey: ["visit", vars.visitId] });
    },
  });
}

export function useUpdateVisitServiceStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      visitId,
      lineId,
      input,
    }: {
      visitId: number;
      lineId: number;
      input: UpdateVisitServiceStatusInput;
    }) => api.updateVisitServiceStatus(visitId, lineId, input),
    onSuccess: (_line, vars) => {
      qc.invalidateQueries({ queryKey: ["visits"] });
      qc.invalidateQueries({ queryKey: ["visit", vars.visitId] });
    },
  });
}

export function useCheckoutVisit() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      visitId,
      paymentMethod,
      notes,
    }: {
      visitId: number;
      paymentMethod: string | null;
      notes: string | null;
    }) => api.checkoutVisit(visitId, paymentMethod, notes),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["visits"] });
      qc.invalidateQueries({ queryKey: ["visit"] });
      qc.invalidateQueries({ queryKey: ["invoices"] });
      qc.invalidateQueries({ queryKey: ["invoice-counts"] });
    },
  });
}
