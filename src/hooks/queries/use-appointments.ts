import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  CreateAppointmentInput,
  UpdateAppointmentStatusInput,
} from "@/bindings";

/**
 * Citas de una ventana de fechas (YYYY-MM-DD, ambas inclusive) con filtros
 * opcionales de estado y profesional. El rango es obligatorio: el backend no
 * devuelve «todas las citas», y la semana visible es la que se pide.
 */
export function useAppointments(
  from: string,
  to: string,
  status: string | null,
  professionalId: number | null,
  enabled = true,
) {
  return useQuery({
    queryKey: ["appointments", from, to, status, professionalId],
    queryFn: () => api.listAppointments(from, to, status, professionalId),
    placeholderData: (prev) => prev,
    enabled,
  });
}

export function useCreateAppointment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateAppointmentInput) => api.createAppointment(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["appointments"] }),
  });
}

export function useSetAppointmentStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      input,
    }: {
      id: number;
      input: UpdateAppointmentStatusInput;
    }) => api.setAppointmentStatus(id, input),
    onSuccess: (appt) => {
      qc.invalidateQueries({ queryKey: ["appointments"] });
      qc.invalidateQueries({ queryKey: ["appointment", appt.id] });
    },
  });
}

export function useCancelAppointment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.cancelAppointment(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["appointments"] }),
  });
}

export function useCheckInAppointment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.checkInAppointment(id),
    onSuccess: (visit) => {
      qc.invalidateQueries({ queryKey: ["appointments"] });
      qc.invalidateQueries({ queryKey: ["visits"] });
      qc.invalidateQueries({ queryKey: ["visit", visit.id] });
    },
  });
}
