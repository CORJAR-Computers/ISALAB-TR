/**
 * Acciones de estado de una cita y sus etiquetas.
 *
 * Vive fuera del componente para poder probar la matriz sin DOM y para que la
 * rejilla y el diálogo de detalle compartan las mismas etiquetas. La matriz de
 * abajo tiene que caber dentro de la del backend (`APPOINTMENT_FROM` de
 * appointments.rs y la regla del check-in de repositories/appointments.rs): un
 * botón de más termina siempre en un toast de error.
 */

/** Etiqueta legible de cada estado (`status` de `AppointmentListItem`). */
export const STATUS_LABEL: Record<string, string> = {
  PROGRAMADA: "Programada",
  CONFIRMADA: "Confirmada",
  EN_ATENCION: "En atención",
  ATENDIDA: "Atendida",
  NO_ASISTIO: "No asistió",
  CANCELADA: "Cancelada",
};

export function statusLabel(status: string): string {
  return STATUS_LABEL[status] ?? status;
}

export type AppointmentActionId = "confirm" | "checkin" | "no-show" | "cancel";

export type AppointmentAction = {
  id: AppointmentActionId;
  label: string;
};

const CONFIRM: AppointmentAction = { id: "confirm", label: "Confirmar" };
const CHECK_IN: AppointmentAction = {
  id: "checkin",
  label: "Hacer check-in",
};
const NO_SHOW: AppointmentAction = {
  id: "no-show",
  label: "Marcar no asistió",
};
const CANCEL: AppointmentAction = { id: "cancel", label: "Anular cita" };

/**
 * Acciones ofrecidas por estado, en el orden en que se pintan.
 *
 * La principal va primero (confirmar o hacer check-in) y las que cierran la
 * cita al final. `ATENDIDA` no está porque el check-in ya la deja así; desde
 * ahí el backend no admite ninguna otra transición.
 */
const ACTIONS: Record<string, AppointmentAction[]> = {
  PROGRAMADA: [CONFIRM, CHECK_IN, NO_SHOW, CANCEL],
  CONFIRMADA: [CHECK_IN, NO_SHOW, CANCEL],
  EN_ATENCION: [NO_SHOW, CANCEL],
  ATENDIDA: [],
  NO_ASISTIO: [],
  CANCELADA: [],
};

/** Acciones disponibles para el estado de la cita (vacío = solo consultar). */
export function appointmentActions(status: string): AppointmentAction[] {
  return ACTIONS[status] ?? [];
}
