import { test, expect, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";
import { entityCode, entityCodePattern } from "./dates";
import { gotoView } from "./navigation";

/**
 * Fronteras de fecha con `page.clock`.
 *
 * La app fecha con la hora LOCAL (Firebird `CURRENT_TIMESTAMP`; el frontend usa
 * `localDateISO`): la cola de recepción filtra por el día local y el mock sella
 * `ARRIVED_AT`/los códigos con la fecha del reloj. Estos tests fijan el reloj
 * del navegador en el borde exacto del día y comprueban que:
 *   1. a las 23:59 el filtro sigue en el día en curso y el check-in se sella
 *      ahí (medianoche),
 *   2. tras la medianoche la cola apunta al día siguiente (cambio de mes),
 *   3. el año nuevo cambia el consecutivo del código (V-AAAA-NNNN).
 *
 * Sin `page.clock` con un año fijo, un literal "2026" en el mock o en un spec
 * quedaría desfasado al llegar enero: por eso los códigos se derivan.
 */

test.beforeEach(async ({ page }) => {
  await page.addInitScript({
    path: fileURLToPath(new URL("./ipc-mock.script.js", import.meta.url)),
  });
  await page.setViewportSize({ width: 1440, height: 900 });
});

/** Fija el reloj del navegador ANTES de cargar la app. */
async function loginAt(page: Page, time: Date) {
  await page.clock.install({ time });
  await page.goto("/");
  await expect(page.getByLabel("Usuario")).toBeVisible();
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  await expect(
    page.getByRole("heading", { name: "Panel de control" }).first(),
  ).toBeVisible();
}

const dateFilter = (page: Page) =>
  page.getByLabel("Filtrar por fecha de llegada");

/** Check-in con un servicio del catálogo (Consulta general, 60.000). */
async function checkIn(page: Page) {
  await page.getByRole("button", { name: "Registrar visita" }).click();
  const dialog = page.getByRole("dialog");
  await expect(
    dialog.getByRole("heading", { name: "Nueva visita" }),
  ).toBeVisible();
  await dialog.getByPlaceholder(/Buscar paciente/).fill("Rocky");
  await dialog.getByRole("combobox").first().click();
  await page.getByRole("option", { name: /Rocky/ }).click();
  await dialog.getByRole("combobox").nth(1).click();
  await page.getByRole("option", { name: /Consulta general/ }).click();
  await dialog.getByRole("button", { name: "Registrar visita" }).click();
  await expect(page.getByText("Visita registrada")).toBeVisible();
}

test("medianoche: el filtro y el check-in usan el día local en curso", async ({
  page,
}) => {
  // 23:59:30 del 14 de mayo (hora local del equipo).
  await loginAt(page, new Date(2026, 4, 14, 23, 59, 30));
  await gotoView(page, "Recepción de pacientes");

  await expect(dateFilter(page)).toHaveValue("2026-05-14");
  await expect(page.getByText(/No hay visitas con estos filtros/)).toBeVisible();

  await checkIn(page);
  // El sello del mock sale del reloj fijado: misma fecha que el filtro, así
  // que la visita entra en la cola del día (la lista filtra por arrivedAt).
  await expect(page.getByText(entityCodePattern("V", 1)).first()).toBeVisible();
  await expect(
    page.getByRole("button").filter({ hasText: entityCode("V", 1) }).first(),
  ).toContainText("En espera");
});

test("cambio de mes: pasada la medianoche la cola apunta al día siguiente", async ({
  page,
}) => {
  // 31 de marzo 23:59:30 → 1 de abril.
  await loginAt(page, new Date(2026, 2, 31, 23, 59, 30));
  await gotoView(page, "Recepción de pacientes");
  await expect(dateFilter(page)).toHaveValue("2026-03-31");

  // Avanza el reloj por la medianoche (la página sigue montada).
  await page.clock.fastForward("00:01:00");

  // Al remontar (navegar fuera y volver) el filtro inicial es el día nuevo.
  await gotoView(page, "Panel de control");
  await gotoView(page, "Recepción de pacientes");
  await expect(dateFilter(page)).toHaveValue("2026-04-01");
  await expect(page.getByText(/No hay visitas con estos filtros/)).toBeVisible();

  await checkIn(page);
  await expect(page.getByText(entityCodePattern("V", 1)).first()).toBeVisible();
  // Con el filtro en el 1 de abril, la visita sellada ese día aparece en la cola.
  await expect(
    page.getByRole("button").filter({ hasText: entityCode("V", 1) }).first(),
  ).toContainText("En espera");
});

test("cambio de año: la cola y el código usan el año nuevo", async ({
  page,
}) => {
  // 31 de diciembre 23:59:30 → 1 de enero del año siguiente.
  await loginAt(page, new Date(2026, 11, 31, 23, 59, 30));
  await gotoView(page, "Recepción de pacientes");
  await expect(dateFilter(page)).toHaveValue("2026-12-31");

  await page.clock.fastForward("00:01:00");
  await gotoView(page, "Panel de control");
  await gotoView(page, "Recepción de pacientes");
  await expect(dateFilter(page)).toHaveValue("2027-01-01");

  await checkIn(page);
  // El consecutivo sale de la fecha del reloj: V-2027-0001, no V-2026-0001.
  await expect(
    page.getByText(entityCodePattern("V", 1, 2027)).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("button").filter({ hasText: entityCode("V", 1, 2027) }).first(),
  ).toContainText("En espera");
});
