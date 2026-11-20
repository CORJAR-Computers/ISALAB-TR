/**
 * Códigos que el mock de IPC deriva de la fecha local del reloj del navegador
 * (M-AAAA-0001 muestras, V-AAAA-0001 visitas). Los specs no deben escribir el
 * año a mano: en enero (o con page.clock fijando otro año) el literal queda
 * desfasado y el test falla por la fecha, no por el comportamiento.
 *
 * `year` permite afirmar otro año cuando el test fija el reloj con clock.
 */
export function entityCode(
  kind: "M" | "V",
  seq: number,
  year: number = new Date().getFullYear(),
): string {
  return `${kind}-${year}-${String(seq).padStart(4, "0")}`;
}

/** RegExp del código (para locators por `name`). Los códigos no llevan metacaracteres. */
export function entityCodePattern(
  kind: "M" | "V",
  seq: number,
  year: number = new Date().getFullYear(),
): RegExp {
  return new RegExp(entityCode(kind, seq, year));
}
