import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Formatea un valor numérico en pesos colombianos (COP). */
export function formatCOP(value: number): string {
  return new Intl.NumberFormat("es-CO", {
    style: "currency",
    currency: "COP",
    maximumFractionDigits: 0,
  }).format(value);
}

/**
 * Fecha local (YYYY-MM-DD) de un instante.
 *
 * Nunca usa la fecha UTC (`toISOString`): el backend fecha con la hora local
 * del equipo (Firebird `CURRENT_TIMESTAMP`/`CURRENT_DATE`), así que la UI debe
 * filtrar con el mismo criterio. En Colombia (UTC-5) después de las 19:00 la
 * fecha UTC ya es la del día siguiente.
 *
 * `timeZone` permite fijar la zona (p. ej. en pruebas); por defecto usa la del
 * sistema.
 */
export function localDateISO(
  date: Date = new Date(),
  timeZone?: string,
): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(date);
  const part = (type: string) =>
    parts.find((p) => p.type === type)?.value ?? "";
  return `${part("year")}-${part("month")}-${part("day")}`;
}

/** Formatea una fecha/hora "YYYY-MM-DD HH:MM:SS" o "YYYY-MM-DD" a formato local. */
export function formatDateTime(value?: string | null): string {
  if (!value) return "—";
  const parsed = value.includes(" ")
    ? value.replace(" ", "T")
    : `${value}T00:00:00`;
  const d = new Date(parsed);
  if (Number.isNaN(d.getTime())) return value;
  return d.toLocaleString("es-CO", {
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function formatDate(value?: string | null): string {
  if (!value) return "—";
  const d = new Date(`${value}T00:00:00`);
  if (Number.isNaN(d.getTime())) return value;
  return d.toLocaleDateString("es-CO", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });
}

/** Edad legible a partir de una fecha de nacimiento "YYYY-MM-DD". */
export function formatAge(birthDate?: string | null): string {
  if (!birthDate) return "—";
  const birth = new Date(`${birthDate}T00:00:00`);
  if (Number.isNaN(birth.getTime())) return "—";
  const now = new Date();
  let months =
    (now.getFullYear() - birth.getFullYear()) * 12 +
    (now.getMonth() - birth.getMonth());
  if (now.getDate() < birth.getDate()) months -= 1;
  if (months < 0) months = 0;
  if (months < 12) {
    const m = Math.max(1, Math.round(months));
    return `${m} ${m === 1 ? "mes" : "meses"}`;
  }
  const years = Math.floor(months / 12);
  const rem = months % 12;
  return rem > 0 ? `${years} a ${rem} m` : `${years} años`;
}
