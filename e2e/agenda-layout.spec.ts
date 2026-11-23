import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { installIpcMock } from "./mock";
import { gotoView } from "./navigation";

/**
 * Regresión de LAYOUT de la agenda semanal.
 *
 * Por qué es E2E y no un test de componente: jsdom no calcula layout, así que
 * una tarjeta fuera de su columna, un `top` mal calculado o una rejilla
 * colapsada (alto 0) son invisibles en Vitest. Los dos ya ocurrieron en este
 * componente y solo se veían abriendo la pantalla.
 *
 * Casos cubiertos:
 *   1. Citas nocturnas (00:15, 03:00 y 23:45 que cruza medianoche): la rejilla
 *      se amplía a 00:00–23:00 y cada cita cae en su hora real sin salirse.
 *   2. Semana sin citas: la franja base 07:00–22:00 se pinta completa y sin
 *      tarjetas.
 *
 * Las expectativas en píxeles dependen de HOUR_HEIGHT de
 * src/features/agenda/AppointmentCalendar.tsx. Si allí cambia, se actualizan
 * aquí a propósito: el test fija la relación hora → píxeles.
 */

const HOUR_HEIGHT = 48;

const pad = (n: number) => String(n).padStart(2, "0");

/** Fecha local de hoy: la agenda abre en la semana que la contiene. */
function todayISO(): string {
  const d = new Date();
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

type Seed = {
  id: number;
  hh: number;
  mm: number;
  durationMin: number;
  status: string;
  patient: string;
};

/** Filas con la forma de `AppointmentListItem` que devuelve el backend. */
function toRows(seeds: Seed[]) {
  return seeds.map((s) => ({
    id: s.id,
    code: `C-2026-000${s.id}`,
    patientId: s.id,
    patientName: s.patient,
    ownerId: 10,
    ownerName: "Juan Pérez",
    serviceId: 1,
    serviceName: "Consulta general",
    categoryId: 1,
    categoryName: "GENERAL",
    scheduledAt: `${todayISO()} ${pad(s.hh)}:${pad(s.mm)}:00`,
    durationMin: s.durationMin,
    status: s.status,
    professionalName: null,
  }));
}

/**
 * Instala el mock IPC y sustituye `list_appointments` por las citas del caso.
 * El resto de comandos siguen siendo los del mock, que valida cada respuesta
 * contra e2e/ipc-shapes.json.
 */
async function seedAgenda(page: Page, rows: unknown[]) {
  await installIpcMock(page);
  await page.addInitScript((data) => {
    const internals = (
      window as unknown as {
        __TAURI_INTERNALS__: {
          invoke: (cmd: string, args?: unknown, opts?: unknown) => unknown;
        };
      }
    ).__TAURI_INTERNALS__;
    const original = internals.invoke;
    internals.invoke = (cmd, args, opts) =>
      cmd === "list_appointments"
        ? Promise.resolve(data)
        : original(cmd, args, opts);
  }, rows);
}

async function openAgenda(page: Page) {
  await page.goto("/");
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  await gotoView(page, "Agenda de citas");
  await expect(
    page.getByRole("heading", { name: "Agenda de citas" }).first(),
  ).toBeVisible();
  // La rejilla lleva `aria-busy` mientras la consulta está pendiente; esperar a
  // que se resuelva evita medir la geometría antes de que exista.
  await expect(page.locator("[aria-busy]")).toHaveAttribute(
    "aria-busy",
    "false",
  );
}

/** Geometría real de la rejilla y de cada tarjeta, relativa a su columna. */
function readGrid(page: Page) {
  return page.evaluate((hourHeight) => {
    const columns = Array.from(
      document.querySelectorAll<HTMLElement>('[data-testid="agenda-day-column"]'),
    );
    const firstColumn = columns[0];

    // Las citas se reparten en la columna de su día (hoy puede no ser domingo),
    // así que se miden todas y cada una contra su propia columna.
    const cards = Array.from(
      document.querySelectorAll<HTMLElement>(
        '[data-testid="agenda-day-column"] button[title]',
      ),
    ).map((el) => {
      const card = el.getBoundingClientRect();
      const col = (el.parentElement as HTMLElement).getBoundingClientRect();
      return {
        time: (el.getAttribute("title") ?? "").split(" · ")[0],
        top: Math.round(card.top - col.top),
        height: Math.round(card.height),
        left: Math.round(card.left - col.left),
        width: Math.round(card.width),
        inside: card.top >= col.top - 1 && card.bottom <= col.bottom + 1,
      };
    });

    // Etiquetas horarias: filas de una hora con texto HH:00.
    const labels = Array.from(document.querySelectorAll<HTMLElement>("div"))
      .filter((el) => el.style.height === `${hourHeight}px`)
      .map((el) => (el.textContent ?? "").trim())
      .filter((text) => /^\d{2}:00$/.test(text));

    return {
      columns: columns.length,
      labels,
      columnHeight: Math.round(
        firstColumn?.getBoundingClientRect().height ?? 0,
      ),
      cards,
    };
  }, HOUR_HEIGHT);
}

test("citas nocturnas: rejilla ampliada y cada cita en su hora", async ({
  page,
}) => {
  await seedAgenda(
    page,
    toRows([
      {
        id: 1,
        hh: 0,
        mm: 15,
        durationMin: 30,
        status: "PROGRAMADA",
        patient: "Toby",
      },
      {
        id: 2,
        hh: 3,
        mm: 0,
        durationMin: 120,
        status: "CONFIRMADA",
        patient: "Nala",
      },
      {
        id: 3,
        hh: 23,
        mm: 45,
        durationMin: 60,
        status: "CONFIRMADA",
        patient: "Michi",
      },
    ]),
  );
  await openAgenda(page);
  await expect(page.getByTitle(/C-2026-0001/)).toBeVisible();

  const grid = await readGrid(page);

  // El rango pasa de 07:00–22:00 a 00:00–23:00 por las citas nocturnas.
  expect(grid.columns).toBe(7);
  expect(grid.labels).toHaveLength(24);
  expect(grid.labels[0]).toBe("00:00");
  expect(grid.labels[grid.labels.length - 1]).toBe("23:00");
  expect(grid.columnHeight).toBe(24 * HOUR_HEIGHT);

  // Sin solapes, cada cita ocupa su propia franja completa.
  expect(
    grid.cards.map(({ time, top, height, inside }) => ({
      time,
      top,
      height,
      inside,
    })),
  ).toEqual([
    // 00:15 → 0,25 h × 48 px; 30 min → 24 px.
    { time: "00:15", top: 12, height: 24, inside: true },
    // 03:00 → 3 h × 48 px; 120 min → 96 px.
    { time: "03:00", top: 144, height: 96, inside: true },
    // 23:45 + 60 min cruza medianoche: se recorta al hueco que queda hasta las
    // 24:00 (12 px) en vez de salirse de la rejilla.
    { time: "23:45", top: 1140, height: 12, inside: true },
  ]);
});

test("citas a la misma hora: se reparten en carriles sin taparse", async ({
  page,
}) => {
  await seedAgenda(
    page,
    toRows([
      {
        id: 1,
        hh: 9,
        mm: 0,
        durationMin: 60,
        status: "PROGRAMADA",
        patient: "Toby",
      },
      {
        id: 2,
        hh: 9,
        mm: 15,
        durationMin: 30,
        status: "CONFIRMADA",
        patient: "Nala",
      },
      {
        id: 3,
        hh: 9,
        mm: 30,
        durationMin: 60,
        status: "PROGRAMADA",
        patient: "Michi",
      },
      // Sin solape con las anteriores: debe conservar todo el ancho.
      {
        id: 4,
        hh: 14,
        mm: 0,
        durationMin: 30,
        status: "PROGRAMADA",
        patient: "Luna",
      },
    ]),
  );
  await openAgenda(page);
  await expect(page.getByTitle(/C-2026-0001/)).toBeVisible();

  const grid = await readGrid(page);
  const card = (time: string) => {
    const found = grid.cards.find((c) => c.time === time);
    expect(found, `no se encontró la cita de ${time}`).toBeTruthy();
    return found!;
  };

  const overlapping = [card("09:00"), card("09:15"), card("09:30")];

  // Cada cita mantiene su hora…
  expect(overlapping.map((c) => c.top)).toEqual([96, 108, 120]);
  // …y las tres ocupan carriles distintos, sin solaparse horizontalmente.
  expect(new Set(overlapping.map((c) => c.left)).size).toBe(3);
  const byLeft = [...overlapping].sort((a, b) => a.left - b.left);
  for (let i = 1; i < byLeft.length; i += 1) {
    expect(byLeft[i].left).toBeGreaterThanOrEqual(
      byLeft[i - 1].left + byLeft[i - 1].width - 1,
    );
  }
  for (const c of overlapping) expect(c.width).toBeGreaterThan(20);

  // La cita que no se solapa con nadie conserva el ancho completo de la columna.
  const lonely = card("14:00");
  expect(lonely.width).toBeGreaterThan(overlapping[0].width * 2);
});

test("semana sin citas: franja base completa y sin tarjetas", async ({
  page,
}) => {
  await seedAgenda(page, []);
  await openAgenda(page);

  const grid = await readGrid(page);

  expect(grid.cards).toEqual([]);
  expect(grid.columns).toBe(7);
  expect(grid.labels).toHaveLength(16);
  expect(grid.labels[0]).toBe("07:00");
  expect(grid.labels[grid.labels.length - 1]).toBe("22:00");
  expect(grid.columnHeight).toBe(16 * HOUR_HEIGHT);
});
