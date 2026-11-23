import { describe, expect, it } from "vitest";
import {
  appointmentActions,
  STATUS_LABEL,
  statusLabel,
} from "./appointment-actions";

/**
 * La matriz de acciones de la UI tiene que caber dentro de la del backend: un
 * botón ofrecido en un estado que Rust rechaza solo produce un toast de error.
 * Estos tests la comparan con `APPOINTMENT_FROM` (appointments.rs) y con la
 * regla del check-in (repository), que no están accesibles desde el frontend.
 */

/** Estados de origen que acepta `set_appointment_status` para cada destino. */
const ALLOWED_FROM: Record<string, string[]> = {
  CONFIRMADA: ["PROGRAMADA"],
  EN_ATENCION: ["PROGRAMADA", "CONFIRMADA"],
  ATENDIDA: ["EN_ATENCION"],
  NO_ASISTIO: ["PROGRAMADA", "CONFIRMADA", "EN_ATENCION"],
  CANCELADA: ["PROGRAMADA", "CONFIRMADA", "EN_ATENCION"],
};

/** Estado al que lleva cada acción de `set_appointment_status`. */
const TARGET_STATUS = {
  confirm: "CONFIRMADA",
  "no-show": "NO_ASISTIO",
  cancel: "CANCELADA",
} as const;

/** El check-in solo se admite en citas que aún no tienen visita. */
const CHECK_IN_FROM = ["PROGRAMADA", "CONFIRMADA"];

const STATUSES = [
  "PROGRAMADA",
  "CONFIRMADA",
  "EN_ATENCION",
  "ATENDIDA",
  "NO_ASISTIO",
  "CANCELADA",
];

describe("appointmentActions", () => {
  it("cada acción ofrecida es una transición que el backend acepta", () => {
    for (const status of STATUSES) {
      for (const action of appointmentActions(status)) {
        if (action.id === "checkin") {
          expect(CHECK_IN_FROM, `${status} → check-in`).toContain(status);
        } else {
          expect(
            ALLOWED_FROM[TARGET_STATUS[action.id]],
            `${status} → ${action.id}`,
          ).toContain(status);
        }
      }
    }
  });

  it("los estados de cierre no ofrecen acciones", () => {
    for (const status of ["ATENDIDA", "NO_ASISTIO", "CANCELADA"]) {
      expect(appointmentActions(status)).toEqual([]);
    }
  });

  it("una cita PROGRAMADA se puede confirmar, atender, no asistir o anular", () => {
    expect(appointmentActions("PROGRAMADA").map((a) => a.id)).toEqual([
      "confirm",
      "checkin",
      "no-show",
      "cancel",
    ]);
  });

  it("una cita CONFIRMADA ya no se vuelve a confirmar", () => {
    expect(appointmentActions("CONFIRMADA").map((a) => a.id)).not.toContain(
      "confirm",
    );
  });

  it("una cita en atención no se atiende dos veces", () => {
    expect(appointmentActions("EN_ATENCION").map((a) => a.id)).not.toContain(
      "checkin",
    );
  });

  it("un estado desconocido no habilita nada", () => {
    expect(appointmentActions("BORRADOR")).toEqual([]);
  });
});

describe("statusLabel", () => {
  it.each(Object.entries(STATUS_LABEL))(
    "traduce %s como %s",
    (status, label) => {
      expect(statusLabel(status)).toBe(label);
    },
  );

  it("deja pasar un estado sin traducción", () => {
    expect(statusLabel("OTRO")).toBe("OTRO");
  });
});
