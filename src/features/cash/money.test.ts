import { describe, expect, it } from "vitest";
import { CASH_DENOMINATIONS, parseAmount } from "./money";

/**
 * `parseAmount` es la única puerta entre lo que el cajero teclea ("119.000",
 * "40.000,50") y los importes que se envían al backend y se suman en el
 * arqueo. Un fallo aquí cobra de menos sin que nadie lo note, así que los
 * formatos aceptados quedan fijados por test.
 */
describe("parseAmount", () => {
  it.each([
    ["40000.50", 40000.5],
    ["40.000,50", 40000.5],
    ["119.000", 119000],
    ["1.234.567", 1234567],
    ["1.234.567,89", 1234567.89],
    ["0", 0],
    [" 12 000 ", 12000],
    ["-5000", -5000],
  ])("convierte %j en %d", (input, expected) => {
    expect(parseAmount(input)).toBe(expected);
  });

  it.each(["", "   ", "abc", "1,2,3", "--5"])(
    "devuelve 0 para un importe no numérico (%j)",
    (input) => {
      expect(parseAmount(input)).toBe(0);
    },
  );
});

describe("CASH_DENOMINATIONS", () => {
  it("va de mayor a menor, sin repetir y sin denominaciones no positivas", () => {
    expect(CASH_DENOMINATIONS.length).toBeGreaterThan(0);
    for (let i = 1; i < CASH_DENOMINATIONS.length; i += 1) {
      expect(CASH_DENOMINATIONS[i]).toBeLessThan(CASH_DENOMINATIONS[i - 1]);
    }
    expect(new Set(CASH_DENOMINATIONS).size).toBe(CASH_DENOMINATIONS.length);
    expect(CASH_DENOMINATIONS.every((value) => value > 0)).toBe(true);
  });
});
