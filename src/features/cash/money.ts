/** Convierte "40.000,50" / "40000.50" en 40000.5. */
export function parseAmount(value: string): number {
  const cleaned = value.replace(/\s/g, "").replace(/\.(?=\d{3}\b)/g, "");
  const normalized = cleaned.replace(",", ".");
  const parsed = Number(normalized);
  return Number.isFinite(parsed) ? parsed : 0;
}

/** Denominaciones de COP para el arqueo, de mayor a menor. */
export const CASH_DENOMINATIONS = [
  100000, 50000, 20000, 10000, 5000, 2000, 1000, 500, 200, 100, 50,
] as const;
