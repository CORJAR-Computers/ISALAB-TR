import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { installIpcMock } from "./mock";
import { gotoView } from "./navigation";
import { todayISO } from "./agenda";

/**
 * Agenda a escala, presupuesto de rendimiento y contrato de la ventana.
 *
 * El mock devuelve dos citas; una clínica con dos años de historia tiene miles.
 * Aquí se multiplican las filas respetando la forma de `AppointmentListItem`
 * (que el mock valida contra `bindings.ts`) y se comprueba:
 *
 *   1. Que `list_appointments` se pide SIEMPRE con `from`/`to` y que la ventana
 *      es la semana visible (domingo a sábado, hoy dentro). Sin el rango el
 *      comando no tiene sentido: el backend lo exige y el mock lo valida.
 *   2. Que la rejilla pinta 2.000 citas dentro del presupuesto (detecta un N+1
 *      o un re-render en bucle, no micro-latencia).
 *   3. Cuántos comandos IPC se invocan: si aparece uno por cita, el test lo
 *      dice con su desglose.
 */

const SCALE = 2_000;

/** Presupuesto de pintado en CI: 10× lo observado en local. */
const RENDER_BUDGET_MS = 10_000;

/** Mayor que el presupuesto: quien falla es la aserción, no el locator. */
const RENDER_WAIT_TIMEOUT_MS = 60_000;

/**
 * Sustituye las citas que devuelve el mock por `count` filas repartidas por los
 * siete días de la ventana pedida, sin tocar el estado del mock.
 *
 * Se envuelve `__TAURI_INTERNALS__.invoke` con un setter, igual que
 * e2e/scale.spec.ts: el script del mock se ejecuta después y asigna el objeto
 * real, que queda envuelto al vuelo.
 */
async function installScaleAgenda(page: Page, count: number) {
  await page.addInitScript((total: number) => {
    let wrapped: unknown = null;

    const pad = (n: number) => String(n).padStart(2, "0");

    /** Reparte `total` citas entre los siete días de la ventana. */
    const expand = (rows: Array<Record<string, unknown>>, from: string) => {
      if (!rows.length) return rows;
      const weekStart = new Date(`${from}T00:00:00Z`);
      const statuses = ["PROGRAMADA", "CONFIRMADA", "ATENDIDA"];
      const out: Array<Record<string, unknown>> = [];
      for (let i = 0; i < total; i += 1) {
        const day = new Date(weekStart);
        day.setUTCDate(weekStart.getUTCDate() + (i % 7));
        // Franjas cada 5 minutos con 30 de duración: hasta seis carriles.
        const minutes = (Math.floor(i / 7) * 5) % (24 * 60);
        // Clave del día con getters UTC: `toISOString()` está vetado aquí
        // porque de noche apunta al día siguiente.
        const dayKey = `${day.getUTCFullYear()}-${pad(
          day.getUTCMonth() + 1,
        )}-${pad(day.getUTCDate())}`;
        out.push({
          ...rows[i % rows.length],
          id: 10_000 + i,
          code: `C-${day.getUTCFullYear()}-${String(i + 1).padStart(5, "0")}`,
          patientId: 100 + i,
          patientName: `Paciente ${i + 1}`,
          scheduledAt: `${dayKey} ${pad(Math.floor(minutes / 60))}:${pad(
            minutes % 60,
          )}:00`,
          durationMin: 30,
          status: statuses[i % statuses.length],
        });
      }
      return out;
    };

    type Internals = {
      invoke: (cmd: string, args?: unknown) => Promise<unknown>;
    };

    function wrap(internals: Internals): Internals {
      return {
        ...internals,
        invoke: async (cmd: string, args?: unknown) => {
          const data = await internals.invoke(cmd, args);
          if (cmd !== "list_appointments") return data;

          const range = args as { from?: string; to?: string } | undefined;
          if (!range?.from || !range?.to) {
            throw new Error(
              "[e2e-scale] list_appointments sin ventana from/to",
            );
          }
          (
            window as unknown as {
              __E2E_AGENDA_RANGE__?: { from: string; to: string };
            }
          ).__E2E_AGENDA_RANGE__ = { from: range.from, to: range.to };

          return Array.isArray(data)
            ? expand(data as Array<Record<string, unknown>>, range.from)
            : data;
        },
      };
    }

    const existing = (
      window as unknown as { __TAURI_INTERNALS__?: Internals }
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
        __E2E_IPC_FAULTS__?: {
          stats: () => { total: number; byCommand: Record<string, number> };
        };
      }
    ).__E2E_IPC_FAULTS__?.stats();
    return stats ?? { total: 0, byCommand: {} };
  });
}

test("Agenda pinta 2.000 citas con ventana semanal y sin N+1", async ({
  page,
}) => {
  // El test mide tiempos: `slow` le da margen para llegar a la aserción del
  // presupuesto en vez de morir por el timeout del propio test.
  test.slow();

  await installIpcMock(page);
  await installScaleAgenda(page, SCALE);
  await login(page);

  const before = await commandStats(page);

  const startedAt = Date.now();
  await gotoView(page, "Agenda de citas");
  // Contar tarjetas es una consulta al DOM; buscar el texto de la última
  // obligaría a barrer 2.000 tarjetas en cada sondeo y se comería el margen.
  const cards = page.locator('[data-testid="agenda-day-column"] button');
  await expect(cards).toHaveCount(SCALE, { timeout: RENDER_WAIT_TIMEOUT_MS });
  const elapsed = Date.now() - startedAt;

  expect(
    elapsed,
    `${SCALE} citas tardaron ${elapsed} ms (presupuesto ${RENDER_BUDGET_MS} ms)`,
  ).toBeLessThan(RENDER_BUDGET_MS);

  // La consulta lleva la semana visible: siete días y hoy dentro.
  const range = await page.evaluate(
    () =>
      (
        window as unknown as {
          __E2E_AGENDA_RANGE__?: { from: string; to: string };
        }
      ).__E2E_AGENDA_RANGE__,
  );
  expect(range, "la agenda no pidió list_appointments con from/to").toBeTruthy();
  const from = range!.from;
  const to = range!.to;
  expect(
    Date.parse(`${to}T00:00:00Z`) - Date.parse(`${from}T00:00:00Z`),
    `ventana ${from}..${to} no cubre siete días`,
  ).toBe(6 * 24 * 60 * 60 * 1000);
  const today = todayISO();
  expect(from <= today, `la ventana empieza en ${from} y hoy es ${today}`).toBe(
    true,
  );
  expect(today <= to, `la ventana acaba en ${to} y hoy es ${today}`).toBe(true);

  const after = await commandStats(page);
  const delta = (cmd: string) =>
    (after.byCommand[cmd] ?? 0) - (before.byCommand[cmd] ?? 0);

  // Un listado completo no debe resolverse con un comando por cita.
  expect(delta("list_appointments")).toBeLessThanOrEqual(2);
  // Comandos de la pantalla (catálogos del diálogo de alta, permiso): un número
  // fijo que no crece con las filas. El tope cubre el montaje con margen; si un
  // día aparece un comando por cita, esto falla con su desglose.
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
    `una agenda de ${SCALE} citas invocó ${commands} comandos IPC: ${breakdown}`,
  ).toBeLessThanOrEqual(8);
});
