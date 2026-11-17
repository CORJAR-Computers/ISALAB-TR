import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  FinishGroomingSessionInput,
  UpsertGroomingSessionInput,
} from "@/bindings";

/** Ficha de sesión de peluquería de una línea de servicio, o null. */
export function useGroomingSession(visitServiceId: number | null) {
  return useQuery({
    queryKey: ["grooming-session", visitServiceId],
    queryFn: () => api.getGroomingSession(visitServiceId!),
    enabled: visitServiceId != null,
  });
}

/** Líneas de peluquería del tablero del día (visitas de esa fecha). */
export function useGroomingDay(date: string) {
  return useQuery({
    queryKey: ["grooming-day", date],
    queryFn: () => api.listGroomingDay(date),
    placeholderData: (prev) => prev,
  });
}

/** Próximos turnos sugeridos por sesiones cerradas. */
export function useGroomingReminders() {
  return useQuery({
    queryKey: ["grooming-reminders"],
    queryFn: () => api.listGroomingReminders(),
    placeholderData: (prev) => prev,
  });
}

/** Fotos antes/después (o documentos) de una visita. */
export function useVisitAttachments(visitId: number | null) {
  return useQuery({
    queryKey: ["visit-attachments", visitId],
    queryFn: () => api.listVisitAttachments(visitId!),
    enabled: visitId != null,
  });
}

export function useUpsertGroomingSession() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: UpsertGroomingSessionInput) =>
      api.upsertGroomingSession(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["grooming-session"] });
      qc.invalidateQueries({ queryKey: ["grooming-day"] });
      qc.invalidateQueries({ queryKey: ["visits"] });
      qc.invalidateQueries({ queryKey: ["visit"] });
    },
  });
}

export function useFinishGroomingSession() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      visitServiceId,
      input,
    }: {
      visitServiceId: number;
      input: FinishGroomingSessionInput;
    }) => api.finishGroomingSession(visitServiceId, input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["grooming-session"] });
      qc.invalidateQueries({ queryKey: ["grooming-day"] });
      qc.invalidateQueries({ queryKey: ["grooming-reminders"] });
      qc.invalidateQueries({ queryKey: ["visits"] });
      qc.invalidateQueries({ queryKey: ["visit"] });
    },
  });
}

export function useAddVisitAttachment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      sourcePath,
      visitId,
      kind,
    }: {
      sourcePath: string;
      visitId: number;
      kind: string | null;
    }) => api.addVisitAttachment(sourcePath, visitId, kind),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["visit-attachments"] }),
  });
}

export function useDeleteVisitAttachment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteVisitAttachment(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["visit-attachments"] }),
  });
}
