import type { AppointmentListItem } from "@/bindings";
import { localDateISO } from "@/lib/utils";

/**
 * Lógica pura de la agenda semanal: parseo de `scheduledAt`, rango horario a
 * pintar, agrupación por día, reparto en carriles de las citas que se solapan y
 * formateos de hora.
 *
 * Vive fuera del componente para poder probarla con Vitest sin DOM: los bugs de
 * esta parte (cita que cae en la columna equivocada, rango que no incluye una
 * cita nocturna, solapes que se tapan) son de aritmética, no de CSS.
 */

/** Primera hora con etiqueta en la rejilla base. */
export const BASE_START_HOUR = 7;
/** Última hora con etiqueta en la rejilla base (la rejilla llega hasta las 23:00). */
export const BASE_END_HOUR = 22;
/** Duración mínima con la que una cita ocupa sitio al repartir carriles. */
const MIN_SLOT_MINUTES = 15;

export type DaySlot = { date: Date; key: string };

export type LanedAppointment = {
  appt: AppointmentListItem;
  /** Carril (columna) donde arranca la tarjeta dentro de su clúster. */
  lane: number;
  /**
   * Carriles que ocupa a lo ancho. Es 1 salvo que los carriles de su derecha
   * queden libres durante toda su franja (sub-clúster), en cuyo caso la tarjeta
   * se ensancha hasta el primero que sí está ocupado.
   */
  span: number;
  /** Carriles totales del clúster: denominador del ancho de cada tarjeta. */
  lanes: number;
};

export type WeekLayout = {
  /** Días de la semana visible, de domingo a sábado. */
  days: DaySlot[];
  /** Citas de la semana agrupadas por `DaySlot.key`. */
  byDay: Map<string, LanedAppointment[]>;
  /** Primera hora de la rejilla (0–23). */
  startHour: number;
  /** Hora final exclusiva de la rejilla (1–24). */
  endHour: number;
};

export function addDays(date: Date, days: number): Date {
  const d = new Date(date);
  d.setDate(d.getDate() + days);
  return d;
}

function startOfDay(date: Date): Date {
  const d = new Date(date);
  d.setHours(0, 0, 0, 0);
  return d;
}

/**
 * `scheduledAt` llega como `YYYY-MM-DD HH:MM:SS` (hora local, sin zona).
 * Se parsea a mano: `new Date("2026-11-23 08:30:00")` es dependiente del motor
 * (Safari lo rechaza) y el cálculo de la hora debe ser local, no UTC. Se acepta
 * también el separador `T` por si el backend cambia a ISO 8601.
 */
export function parseScheduledAt(value: string): {
  day: string;
  minutes: number;
} {
  const [day, time = "00:00:00"] = value.split(/[T ]/);
  const [hh = 0, mm = 0] = time.split(":").map(Number);
  return { day, minutes: hh * 60 + mm };
}

/** Minutos desde medianoche → `HH:MM`. */
export function formatTime(minutes: number): string {
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(
    minutes % 60,
  ).padStart(2, "0")}`;
}

/** Hora del día (0–24) → `HH:00`, la etiqueta de la columna de horas. */
export function formatHour(hour: number): string {
  return formatTime(hour * 60);
}

/**
 * Reparte en carriles las citas de un día para que las que se solapan se lean
 * todas en vez de taparse.
 *
 * Se agrupa por clústeres de solapamiento (una cita sin solape conserva todo el
 * ancho) y dentro de cada clúster se asigna a cada cita el primer carril que ya
 * haya quedado libre (`fin <= inicio`), que es el reparto mínimo con orden por
 * hora de inicio. Una cita sin duración ocupa `MIN_SLOT_MINUTES` para no quedar
 * exactamente encima de otra a la misma hora.
 *
 * Después, cada cita se expande hacia la derecha mientras los carriles
 * siguientes estén libres durante TODA su franja: así una cita de 30 min a las
 * 09:30 no se queda con un tercio del ancho cuando el carril de su derecha se
 * liberó a las 09:30. Esos carriles contiguos que una cita puede usar son su
 * sub-clúster; el ancho se calcula como `span / lanes` y nunca invade otra
 * tarjeta, porque un carril que aparece ocupado en cualquier instante de la
 * franja corta la expansión.
 */
export function assignLanes(appts: AppointmentListItem[]): LanedAppointment[] {
  const items = appts
    .map((appt) => {
      const start = parseScheduledAt(appt.scheduledAt).minutes;
      return {
        appt,
        start,
        end: start + Math.max(appt.durationMin, MIN_SLOT_MINUTES),
      };
    })
    .sort((a, b) => a.start - b.start || a.appt.id - b.appt.id);

  const out: LanedAppointment[] = [];
  let cluster: typeof items = [];
  let clusterEnd = -1;

  const flush = () => {
    if (cluster.length === 0) return;
    const laneEnds: number[] = [];
    const placed: {
      appt: AppointmentListItem;
      start: number;
      end: number;
      lane: number;
    }[] = [];
    for (const item of cluster) {
      let lane = laneEnds.findIndex((end) => end <= item.start);
      if (lane === -1) {
        lane = laneEnds.length;
        laneEnds.push(item.end);
      } else {
        laneEnds[lane] = item.end;
      }
      placed.push({ ...item, lane });
    }
    const lanes = laneEnds.length;
    for (const item of placed) {
      let span = 1;
      for (let lane = item.lane + 1; lane < lanes; lane += 1) {
        const busy = placed.some(
          (other) =>
            other !== item &&
            other.lane === lane &&
            other.start < item.end &&
            other.end > item.start,
        );
        if (busy) break;
        span += 1;
      }
      out.push({ appt: item.appt, lane: item.lane, lanes, span });
    }
    cluster = [];
    clusterEnd = -1;
  };

  for (const item of items) {
    if (cluster.length > 0 && item.start >= clusterEnd) flush();
    cluster.push(item);
    clusterEnd = Math.max(clusterEnd, item.end);
  }
  flush();

  return out;
}

/**
 * Construye la semana visible a partir de la fecha de referencia y las citas.
 *
 * El rango horario se amplía para incluir cualquier cita fuera de 07:00–22:00:
 * una cita de 06:30 o de 22:30 se dibuja en su hora real en vez de recortarse a
 * la primera/última franja. Con la agenda dentro de horario el rango es el base
 * y una semana sin citas pinta 07:00–23:00.
 */
export function buildWeek(
  viewDate: Date,
  appointments: AppointmentListItem[],
): WeekLayout {
  const weekStart = startOfDay(addDays(viewDate, -viewDate.getDay()));
  const days = Array.from({ length: 7 }, (_, i) => {
    const date = addDays(weekStart, i);
    return { date, key: localDateISO(date) };
  });

  const perDay = new Map<string, AppointmentListItem[]>(
    days.map((d) => [d.key, []]),
  );

  let firstHour = BASE_START_HOUR;
  let lastHour = BASE_END_HOUR + 1;

  for (const appt of appointments) {
    const { day, minutes } = parseScheduledAt(appt.scheduledAt);
    const bucket = perDay.get(day);
    if (!bucket) continue; // cita de otra semana
    bucket.push(appt);
    firstHour = Math.min(firstHour, Math.floor(minutes / 60));
    lastHour = Math.max(
      lastHour,
      Math.ceil((minutes + Math.max(appt.durationMin, 0)) / 60),
    );
  }

  const byDay = new Map<string, LanedAppointment[]>();
  for (const [key, list] of perDay) byDay.set(key, assignLanes(list));

  return {
    days,
    byDay,
    startHour: Math.max(0, firstHour),
    endHour: Math.min(24, lastHour),
  };
}
