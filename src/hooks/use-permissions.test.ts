import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { capabilitiesForRole, type Capabilities } from "./use-permissions";

/**
 * Paridad entre la matriz de capacidades del frontend
 * (`capabilitiesForRole` en src/hooks/use-permissions.ts) y la del backend
 * (`capabilities_for_role` en src-tauri/src/auth.rs).
 *
 * La UI filtra menú y botones con la copia de TS; los comandos exigen la
 * capacidad equivalente en Rust. Si una cambia sin la otra, la UI ofrece
 * acciones que el backend rechaza (o esconde acciones permitidas), y eso no
 * lo detecta ningún test de comportamiento: cada lado pasa su propia suite.
 * Este test lee auth.rs como texto y compara los siete flags de cada rol.
 */

/** Nombre del flag en Rust para cada campo de Capabilities del frontend. */
const FLAGS: Array<[keyof Capabilities, string]> = [
  ["canReceive", "can_receive"],
  ["canClinical", "can_clinical"],
  ["canGroom", "can_groom"],
  ["canBill", "can_bill"],
  ["canStay", "can_stay"],
  ["canInventory", "can_inventory"],
  ["isAdmin", "is_admin"],
];

type Flags = Partial<Record<string, boolean>>;

/**
 * Extrae de auth.rs los brazos de `capabilities_for_role`:
 * `"ROL" => Capabilities { can_receive: true, … }`, `"ADMIN" =>
 * Capabilities::admin()` y el brazo por defecto `_ =>`.
 */
export function parseRustCapabilities(source: string): {
  byRole: Record<string, Flags>;
  fallback: Flags;
} {
  const fnStart = source.indexOf("pub fn capabilities_for_role");
  if (fnStart === -1) {
    throw new Error(
      "No se encontró `pub fn capabilities_for_role` en src-tauri/src/auth.rs",
    );
  }
  const fnEnd = source.indexOf("\n}", fnStart);
  const body = source.slice(fnStart, fnEnd === -1 ? undefined : fnEnd);

  const armStarts = [...body.matchAll(/"([A-Z_]+)"\s*=>/g)].map((m) => ({
    role: m[1],
    index: m.index ?? 0,
  }));
  const fallbackIndex = body.indexOf("_ =>");

  const flagsOf = (text: string, role: string): Flags => {
    // `Capabilities::admin()` es el atajo de "todo permitido".
    if (text.includes("Capabilities::admin()")) {
      return Object.fromEntries(FLAGS.map(([, rust]) => [rust, true]));
    }
    const flags: Flags = {};
    for (const [, rust] of FLAGS) {
      const match = text.match(new RegExp(`${rust}:\\s*(true|false)`));
      if (!match) {
        throw new Error(
          `El brazo "${role}" de capabilities_for_role no declara ${rust}`,
        );
      }
      flags[rust] = match[1] === "true";
    }
    return flags;
  };

  const byRole: Record<string, Flags> = {};
  armStarts.forEach((arm, i) => {
    const end = armStarts[i + 1]?.index ?? fallbackIndex ?? body.length;
    byRole[arm.role] = flagsOf(body.slice(arm.index, end), arm.role);
  });

  const fallback =
    fallbackIndex === -1
      ? Object.fromEntries(FLAGS.map(([, rust]) => [rust, false]))
      : flagsOf(body.slice(fallbackIndex), "_");

  return { byRole, fallback };
}

// Vitest corre con cwd = raíz del proyecto (también en CI).
const rustSource = readFileSync(
  join(process.cwd(), "src-tauri", "src", "auth.rs"),
  "utf-8",
);
const { byRole: rustByRole, fallback: rustFallback } =
  parseRustCapabilities(rustSource);

describe("paridad de capacidades (use-permissions.ts ↔ auth.rs)", () => {
  const roles = Object.keys(rustByRole);

  it("el backend declara los cinco roles esperados", () => {
    expect(roles.sort()).toEqual([
      "ADMIN",
      "AUXILIAR",
      "PELUQUERO",
      "RECEPCION",
      "VETERINARIO",
    ]);
  });

  it.each(roles)("%s tiene la misma matriz en la UI y en Rust", (role) => {
    const caps = capabilitiesForRole(role);
    const diffs = FLAGS.filter(([ts, rust]) => {
      const rustValue = rustByRole[role][rust];
      return caps[ts] !== rustValue;
    }).map(([ts, rust]) => `${ts} (UI=${String(caps[ts])}, Rust=${String(rustByRole[role][rust])})`);

    expect(diffs, `Divergencia en ${role}`).toEqual([]);
  });

  it("un rol desconocido (GUEST) no tiene capacidades en ninguno de los dos", () => {
    const guest = capabilitiesForRole("GUEST");
    for (const [ts, rust] of FLAGS) {
      expect(guest[ts]).toBe(rustFallback[rust]);
    }
    expect(rustFallback).toEqual(
      Object.fromEntries(FLAGS.map(([, rust]) => [rust, false])),
    );
  });

  it("isVet e isVetOrAdmin se derivan del rol, no de flags independientes", () => {
    for (const role of [...roles, "GUEST"]) {
      const caps = capabilitiesForRole(role);
      expect(caps.isVet).toBe(role === "VETERINARIO");
      expect(caps.isVetOrAdmin).toBe(role === "ADMIN" || role === "VETERINARIO");
    }
  });
});
