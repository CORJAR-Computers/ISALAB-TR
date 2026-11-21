import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { installIpcMock } from "./mock";
import { gotoView } from "./navigation";

/**
 * Datos a escala y presupuesto de rendimiento.
 *
 * El mock devuelve tres facturas; una clínica con dos años de historia tiene
 * miles. Aquí se multiplican las filas del listado (respetando la forma de
 * `InvoiceListItem`, que el mock valida contra `bindings.ts`) y se mide:
 *
 *   1. Cuánto tarda Facturación en pintar 2.000 filas (presupuesto amplio,
 *      pensado para detectar un N+1 o un re-render en bucle, no micro-latencia).
 *   2. Cuántos comandos IPC se invocan para un listado completo: si aparece un
 *      comando por fila, el test lo dice con el conteo exacto.
 */

const SCALE = 2_000;

/** Presupuesto de pintado en CI. Generoso a propósito: 10× el tiempo real
 * observado en local deja margen para runners lentos y solo falla ante una
 * regresión de orden de magnitud. */
const RENDER_BUDGET_MS = 10_000;

/**
 * Multiplica las facturas que devuelve el mock sin tocar su estado interno.
 *
 * Se envuelve `__TAURI_INTERNALS__.invoke` mediante un setter: el script del
 * mock se ejecuta después y asigna el objeto real, que queda envuelto al vuelo.
 */
async function installScaleData(page: Page, count: number) {
  await page.addInitScript((total: number) => {
    // El mock ya asignó `__TAURI_INTERNALS__` cuando corre este script: se
    // parte de ese objeto. Si se empieza en null, la app lee `null.invoke` al
    // restaurar la sesión y el login no avanza.
    let wrapped: unknown = null;

    const expand = (rows: Array<Record<string, unknown>>) => {
      if (!rows.length || rows.length >= total) return rows;
      const out: Array<Record<string, unknown>> = [];
      for (let i = 0; i < total; i += 1) {
        const base = rows[i % rows.length];
        const statuses = ["PENDIENTE", "ABONADA", "PAGADA", "ANULADA"];
        const paymentStatus = statuses[i % statuses.length];
        const totalAmount = 50_000 + (i % 40) * 12_500;
        const paid =
          paymentStatus === "PAGADA"
            ? totalAmount
            : paymentStatus === "ABONADA"
              ? Math.round(totalAmount / 2)
              : 0;
        out.push({
          ...base,
          id: 10_000 + i,
          invoiceNumber: `FAC-${String(i + 1).padStart(6, "0")}`,
          total: totalAmount,
          paidAmount: paid,
          balance: Math.max(0, totalAmount - paid),
          paymentStatus,
          status:
            paymentStatus === "ANULADA"
              ? "ANULADA"
              : paymentStatus === "PAGADA"
                ? "PAGADA"
                : "EMITIDA",
        });
      }
      return out;
    };

    function wrap(internals: { invoke: (cmd: string, args?: unknown) => Promise<unknown> }) {
      return {
        ...internals,
        invoke: async (cmd: string, args?: unknown) => {
          const data = await internals.invoke(cmd, args);
          if (cmd === "list_invoices" && Array.isArray(data)) {
            return expand(data as Array<Record<string, unknown>>);
          }
          return data;
        },
      };
    }

    const existing = (
      window as unknown as {
        __TAURI_INTERNALS__?: {
          invoke: (cmd: string, args?: unknown) => Promise<unknown>;
        };
      }
    ).__TAURI_INTERNALS__;
    wrapped = existing ? wrap(existing) : null;

    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      get: () => wrapped,
      set: (value) => {
        wrapped = value ? wrap(value) : value;
      },
    });
  }, count);
}

async function login(page: Page) {
  await page.goto("/");
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  await expect(
    page.getByRole("heading", { name: "Panel de control" }).first(),
  ).toBeVisible();
}

/** Conteo de invocaciones por comando desde el arranque del mock. */
async function commandStats(page: Page) {
  return page.evaluate(() => {
    const stats = (
      window as unknown as {
        __E2E_IPC_FAULTS__?: { stats: () => { total: number; byCommand: Record<string, number> } };
      }
    ).__E2E_IPC_FAULTS__?.stats();
    return stats ?? { total: 0, byCommand: {} };
  });
}

test("Facturación pinta 2.000 facturas dentro del presupuesto y sin N+1", async ({
  page,
}) => {
  await installIpcMock(page);
  await installScaleData(page, SCALE);
  await login(page);

  const before = await commandStats(page);

  const startedAt = Date.now();
  await gotoView(page, "Facturación");
  // La última fila del listado existe → el render terminó.
  await expect(
    page.getByText(`FAC-${String(SCALE).padStart(6, "0")}`, { exact: true }),
  ).toBeVisible();
  const elapsed = Date.now() - startedAt;

  expect(
    elapsed,
    `${SCALE} facturas tardaron ${elapsed} ms (presupuesto ${RENDER_BUDGET_MS} ms)`,
  ).toBeLessThan(RENDER_BUDGET_MS);

  const after = await commandStats(page);
  const delta = (cmd: string) =>
    (after.byCommand[cmd] ?? 0) - (before.byCommand[cmd] ?? 0);

  // Un listado completo no debe resolverse con un comando por fila.
  expect(delta("list_invoices")).toBeLessThanOrEqual(2);
  expect(delta("count_invoices")).toBeLessThanOrEqual(2);
  // Comandos de la pantalla (catálogos, contadores, permiso): un número fijo
  // que no crece con las filas. El tope cubre el montaje actual con margen;
  // si un día aparece un comando por factura, esto falla con su desglose.
  const breakdown = Object.keys(after.byCommand)
    .map((cmd) => [cmd, delta(cmd)] as const)
    .filter(([, count]) => count > 0)
    .map(([cmd, count]) => `${cmd}×${count}`)
    .join(", ");
  const commands = Object.keys(after.byCommand).reduce(
    (sum, cmd) => sum + delta(cmd),
    0,
  );
  expect(
    commands,
    `un listado de ${SCALE} filas invocó ${commands} comandos IPC: ${breakdown}`,
  ).toBeLessThanOrEqual(14);
});
