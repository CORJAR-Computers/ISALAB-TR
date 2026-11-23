import { describe, expect, it } from "vitest";
import type { AppointmentListItem } from "@/bindings";
import {
  assignLanes,
  BASE_END_HOUR,
  BASE_START_HOUR,
  buildWeek,
  formatHour,
  formatTime,
  parseScheduledAt,
} from "./agenda-week";

/**
 * La agenda semanal mezcla tres cosas que fallan en silencio: el parseo de la
 * hora (si se interpreta como UTC, la cita cae en otro día oa otra franja), el
 * rango horario (recortar una cita nocturna la pinta a una hora falsa) y el
 * reparto en carriles (si se solapan, una tapa a la otra y desaparece de la
 * vista). Aquí se fija la aritmética; el layout real lo cubre
 * e2e/agenda-layout.spec.ts, porque jsdom no calcula posiciones.
 */

let nextId = 1;

/** Cita mínima con la forma de `AppointmentListItem`. */
function appt(scheduledAt: string, durationMin = 30): AppointmentListItem {
  const id = nextId++;
  return {
    id,
    code: `C-2026-${String(id).padStart(4, "0")}`,
    patientId: id,
    patientName: `Paciente ${id}`,
    ownerId: 1,
    ownerName: "Dueño",
    serviceId: 1,
    serviceName: "Consulta",
    categoryId: 1,
    categoryName: "GENERAL",
    scheduledAt,
    durationMin,
    status: "PROGRAMADA",
    professionalName: null,
  };
}

describe("parseScheduledAt", () => {
  it.each([
    ["2026-11-23 08:30:00", "2026-11-23", 8 * 60 + 30],
    ["2026-11-23 08:30", "2026-11-23", 8 * 60 + 30],
    ["2026-11-23T00:15:00", "2026-11-23", 15],
    ["2026-11-23 00:00:00", "2026-11-23", 0],
  ])("parsea %j como hora local", (value, day, minutes) => {
    expect(parseScheduledAt(value)).toEqual({ day, minutes });
  });
});

describe("formatHour / formatTime", () => {
  it("rellena a dos dígitos", () => {
    expect(formatHour(7)).toBe("07:00");
    expect(formatHour(22)).toBe("22:00");
    expect(formatTime(0)).toBe("00:00");
    expect(formatTime(9 * 60 + 5)).toBe("09:05");
    expect(formatTime(23 * 60 + 45)).toBe("23:45");
  });
});

describe("assignLanes", () => {
  it("sin citas no hay carriles", () => {
    expect(assignLanes([])).toEqual([]);
  });

  it("citas sin solape conservan el ancho completo", () => {
    const lanes = assignLanes([
      appt("2026-11-23 09:00:00", 60),
      appt("2026-11-23 11:00:00", 60),
    ]);
    expect(lanes.map((l) => [l.lane, l.span, l.lanes])).toEqual([
      [0, 1, 1],
      [0, 1, 1],
    ]);
  });

  it("dos citas solapadas se reparten en dos carriles", () => {
    const a = appt("2026-11-23 09:00:00", 60);
    const b = appt("2026-11-23 09:30:00", 60);
    const lanes = assignLanes([a, b]);
    expect(lanes.map((l) => [l.appt.id, l.lane, l.span, l.lanes])).toEqual([
      [a.id, 0, 1, 2],
      [b.id, 1, 1, 2],
    ]);
  });

  it("reutiliza un carril cuando la anterior ya terminó", () => {
    // 09:00–10:00, 09:30–11:00 y 10:30–12:00: la tercera cabe en el carril 0,
    // así que el grupo necesita 2 carriles y no 3.
    const a = appt("2026-11-23 09:00:00", 60);
    const b = appt("2026-11-23 09:30:00", 90);
    const c = appt("2026-11-23 10:30:00", 90);
    const lanes = assignLanes([c, a, b]); // desordenadas a propósito
    expect(lanes.map((l) => [l.appt.id, l.lane, l.lanes])).toEqual([
      [a.id, 0, 2],
      [b.id, 1, 2],
      [c.id, 0, 2],
    ]);
  });

  it("una cita sin duración no se apila sobre otra a la misma hora", () => {
    const a = appt("2026-11-23 09:00:00", 0);
    const b = appt("2026-11-23 09:00:00", 0);
    const lanes = assignLanes([a, b]);
    expect(lanes.map((l) => [l.appt.id, l.lane, l.lanes])).toEqual([
      [a.id, 0, 2],
      [b.id, 1, 2],
    ]);
  });

  it("rellena el hueco a la izquierda cuando varias citas acaban a la vez", () => {
    // 09:00–10:30 y tres citas que terminan a las 10:30. La de 10:30 cae en el
    // cuarto carril, pero el tercero ya está libre: ocupa dos cuartos, no uno.
    const a = appt("2026-11-23 09:00:00", 90);
    const b = appt("2026-11-23 10:15:00", 20);
    const c = appt("2026-11-23 10:15:00", 15);
    const d = appt("2026-11-23 10:15:00", 15);
    const e = appt("2026-11-23 10:30:00", 60);
    const lanes = assignLanes([a, b, c, d, e]);
    expect(lanes.map((l) => [l.appt.id, l.lane, l.span, l.lanes])).toEqual([
      [a.id, 0, 1, 4],
      [b.id, 1, 1, 4],
      [c.id, 2, 1, 4],
      [d.id, 3, 1, 4],
      [e.id, 2, 2, 4],
    ]);
  });

  it("si el reparto espejo deja una tarjeta más estrecha, se queda el de siempre", () => {
    // Empaquetando a la derecha, la cita de 09:00 perdería el segundo carril
    // que el reparto clásico sí le deja libre a su derecha: gana el ancho.
    const a = appt("2026-11-23 09:00:00", 60);
    const b = appt("2026-11-23 09:00:00", 20);
    const c = appt("2026-11-23 09:45:00", 15);
    const d = appt("2026-11-23 09:45:00", 20);
    const lanes = assignLanes([a, b, c, d]);
    expect(lanes.map((l) => [l.appt.id, l.lane, l.span, l.lanes])).toEqual([
      [a.id, 0, 1, 3],
      [b.id, 1, 2, 3],
      [c.id, 1, 1, 3],
      [d.id, 2, 1, 3],
    ]);
  });

  it("ensancha la cita posterior cuando el carril de su derecha se libera", () => {
    // Clúster de 3 carriles: 09:00–11:00, 09:00–09:30, 09:05–09:30 y
    // 09:30–10:00. A las 09:30 el carril 2 ya está libre, así que la cita de las
    // 09:30 ocupa dos carriles en vez de quedarse con un tercio del ancho.
    const a = appt("2026-11-23 09:00:00", 120);
    const b = appt("2026-11-23 09:00:00", 30);
    const c = appt("2026-11-23 09:05:00", 25);
    const d = appt("2026-11-23 09:30:00", 30);
    const lanes = assignLanes([a, b, c, d]);
    expect(lanes.map((l) => [l.appt.id, l.lane, l.span, l.lanes])).toEqual([
      [a.id, 0, 1, 3],
      [b.id, 1, 1, 3],
      [c.id, 2, 1, 3],
      [d.id, 1, 2, 3],
    ]);
  });

  it("no se ensancha sobre un carril ocupado solo en parte de su franja", () => {
    // 09:15–09:45 tapa el carril 1 justo cuando empieza la cita de 09:45 (que
    // cae ahí al liberarse el carril), así que ninguna de las dos se estira.
    const a = appt("2026-11-23 09:00:00", 60);
    const b = appt("2026-11-23 09:15:00", 30);
    const c = appt("2026-11-23 09:45:00", 30);
    const lanes = assignLanes([a, b, c]);
    expect(lanes.map((l) => [l.appt.id, l.lane, l.span, l.lanes])).toEqual([
      [a.id, 0, 1, 2],
      [b.id, 1, 1, 2],
      [c.id, 1, 1, 2],
    ]);
  });
});

describe("buildWeek", () => {
  // Miércoles 25 de noviembre de 2026: la semana empieza el domingo 22.
  const wednesday = new Date(2026, 10, 25, 15, 30);

  it("la semana va de domingo a sábado y las claves son locales", () => {
    const { days } = buildWeek(wednesday, []);
    expect(days).toHaveLength(7);
    expect(days.map((d) => d.key)).toEqual([
      "2026-11-22",
      "2026-11-23",
      "2026-11-24",
      "2026-11-25",
      "2026-11-26",
      "2026-11-27",
      "2026-11-28",
    ]);
    expect(days[0].date.getHours()).toBe(0);
  });

  it("sin citas pinta la franja base completa", () => {
    const { byDay, startHour, endHour } = buildWeek(wednesday, []);
    expect(startHour).toBe(BASE_START_HOUR);
    expect(endHour).toBe(BASE_END_HOUR + 1);
    expect(byDay.size).toBe(7);
    for (const list of byDay.values()) expect(list).toEqual([]);
  });

  it("agrupa cada cita en su día", () => {
    const within = appt("2026-11-23 09:00:00", 60);
    const another = appt("2026-11-28 18:00:00", 30);
    const { byDay } = buildWeek(wednesday, [within, another]);
    expect(byDay.get("2026-11-23")?.map((l) => l.appt.id)).toEqual([within.id]);
    expect(byDay.get("2026-11-28")?.map((l) => l.appt.id)).toEqual([
      another.id,
    ]);
    expect(byDay.get("2026-11-25")).toEqual([]);
  });

  it("ignora las citas de otra semana", () => {
    const { byDay, startHour, endHour } = buildWeek(wednesday, [
      appt("2026-12-01 09:00:00", 60),
    ]);
    expect(startHour).toBe(BASE_START_HOUR);
    expect(endHour).toBe(BASE_END_HOUR + 1);
    for (const list of byDay.values()) expect(list).toEqual([]);
  });

  it("amplía el rango para una cita antes de la franja base", () => {
    const { startHour, endHour } = buildWeek(wednesday, [
      appt("2026-11-23 06:30:00", 30),
    ]);
    expect(startHour).toBe(6);
    expect(endHour).toBe(BASE_END_HOUR + 1);
  });

  it("amplía el rango para una cita que cruza medianoche", () => {
    const { startHour, endHour } = buildWeek(wednesday, [
      appt("2026-11-23 23:45:00", 60),
    ]);
    expect(startHour).toBe(BASE_START_HOUR);
    // 23:45 + 60 min termina a las 00:45 del día siguiente: la rejilla llega
    // hasta las 24:00 (el borde) y la tarjeta se recorta al pintarse.
    expect(endHour).toBe(24);
  });

  it("no deja que una duración negativa encoja el rango", () => {
    const { endHour } = buildWeek(wednesday, [
      appt("2026-11-23 21:00:00", -120),
    ]);
    expect(endHour).toBe(BASE_END_HOUR + 1);
  });

  it("una cita a las 00:05 baja el inicio de la rejilla a las 00:00", () => {
    const { startHour, endHour } = buildWeek(wednesday, [
      appt("2026-11-23 00:05:00", 15),
    ]);
    expect(startHour).toBe(0);
    expect(endHour).toBe(BASE_END_HOUR + 1);
  });

  it("aplica el reparto en carriles dentro de cada día", () => {
    const early = appt("2026-11-23 09:00:00", 60);
    const late = appt("2026-11-23 09:30:00", 60);
    const { byDay } = buildWeek(wednesday, [early, late]);
    expect(byDay.get("2026-11-23")?.map((l) => [l.lane, l.lanes])).toEqual([
      [0, 2],
      [1, 2],
    ]);
  });
});
