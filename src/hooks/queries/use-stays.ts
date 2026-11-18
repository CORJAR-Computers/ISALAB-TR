import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  CheckOutStayInput,
  CreateStayEventInput,
  CreateStayInput,
  SaveResourceInput,
} from "@/bindings";

/** Espacios físicos de la clínica (caniles, gateras, boxes…). */
export function useResources(includeInactive = false) {
  return useQuery({
    queryKey: ["resources", includeInactive],
    queryFn: () => api.listResources(includeInactive),
    placeholderData: (prev) => prev,
  });
}

/** Tablero de estancias: recursos con su ocupación actual. */
export function useResourceBoard() {
  return useQuery({
    queryKey: ["resource-board"],
    queryFn: api.listResourceBoard,
    placeholderData: (prev) => prev,
  });
}

/** Estancias con filtros opcionales (estado, tipo, paciente). */
export function useStays(
  status: string | null = null,
  kind: string | null = null,
  patientId: number | null = null,
) {
  return useQuery({
    queryKey: ["stays", status, kind, patientId],
    queryFn: () => api.listStays(status, kind, patientId),
    placeholderData: (prev) => prev,
  });
}

/** Estancia completa por id. */
export function useStay(id: number | null) {
  return useQuery({
    queryKey: ["stay", id],
    queryFn: () => api.getStay(id!),
    enabled: id != null,
  });
}

/** Bitácora de cuidados de una estancia. */
export function useStayEvents(stayId: number | null) {
  return useQuery({
    queryKey: ["stay-events", stayId],
    queryFn: () => api.listStayEvents(stayId!),
    enabled: stayId != null,
    placeholderData: (prev) => prev,
  });
}

export function useSaveResource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: SaveResourceInput) => api.saveResource(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["resources"] });
      qc.invalidateQueries({ queryKey: ["resource-board"] });
    },
  });
}

export function useCreateStay() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateStayInput) => api.createStay(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["stays"] });
      qc.invalidateQueries({ queryKey: ["stay"] });
      qc.invalidateQueries({ queryKey: ["resource-board"] });
    },
  });
}

export function useCheckOutStay() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: number; input: CheckOutStayInput }) =>
      api.checkOutStay(id, input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["stays"] });
      qc.invalidateQueries({ queryKey: ["stay"] });
      qc.invalidateQueries({ queryKey: ["resource-board"] });
    },
  });
}

export function useCancelStay() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.cancelStay(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["stays"] });
      qc.invalidateQueries({ queryKey: ["stay"] });
      qc.invalidateQueries({ queryKey: ["resource-board"] });
    },
  });
}

export function useAddStayEvent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      stayId,
      input,
    }: {
      stayId: number;
      input: CreateStayEventInput;
    }) => api.addStayEvent(stayId, input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["stay-events"] });
      qc.invalidateQueries({ queryKey: ["stay"] });
    },
  });
}
