import type { Locator, Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { openAgenda } from "./agenda";

/**
 * Acciones de estado desde el detalle de una cita.
 *
 * El mock siembra dos citas de hoy: la 0001 (Rocky, PROGRAMADA) y la 0002
 * (Luna, ATENDIDA). Las tarjetas se localizan por el código; el estado solo se
 * fija donde importa, para no romperse por el copy.
 */

/** Abre el detalle de la cita con ese sufijo de código y lo devuelve. */
async function openDetail(page: Page, suffix: string) {
  await page
    .getByRole("button", { name: new RegExp(`C-\\d{4}-${suffix} `) })
    .click();
  const dialog = page.getByRole("dialog");
  await expect(
    dialog.getByRole("heading", {
      name: new RegExp(`^Cita C-\\d{4}-${suffix}$`),
    }),
  ).toBeVisible();
  return dialog;
}

/** Primer `dd` del detalle: el estado de la cita. */
function statusOf(dialog: Locator) {
  return dialog.locator("dd").first();
}

test("confirmar una cita PROGRAMADA la deja CONFIRMADA", async ({ page }) => {
  await openAgenda(page);
  const dialog = await openDetail(page, "0001");
  await expect(statusOf(dialog)).toHaveText("Programada");

  await dialog.getByRole("button", { name: "Confirmar" }).click();

  await expect(page.getByText("Cita confirmada")).toBeVisible();
  await expect(statusOf(dialog)).toHaveText("Confirmada");
  // Ya confirmada, el botón de confirmar no se ofrece otra vez.
  await expect(
    dialog.getByRole("button", { name: "Confirmar" }),
  ).toHaveCount(0);

  await dialog.getByRole("button", { name: "Cerrar" }).click();
  await expect(
    page.getByRole("button", { name: /C-\d{4}-0001 .* Confirmada$/ }),
  ).toBeVisible();
});

test("marcar no asistió cierra la cita y deja solo la consulta", async ({
  page,
}) => {
  await openAgenda(page);
  const dialog = await openDetail(page, "0001");

  await dialog.getByRole("button", { name: "Marcar no asistió" }).click();

  await expect(page.getByText("Cita cerrada como no asistió")).toBeVisible();
  await expect(statusOf(dialog)).toHaveText("No asistió");
  await expect(
    dialog.getByText(/no admite más cambios de estado/),
  ).toBeVisible();
  await expect(
    dialog.getByRole("button", { name: "Marcar no asistió" }),
  ).toHaveCount(0);

  await dialog.getByRole("button", { name: "Cerrar" }).click();
  await expect(
    page.getByRole("button", { name: /C-\d{4}-0001 .* No asistió$/ }),
  ).toBeVisible();
});

test("anular una cita pide confirmación y la deja CANCELADA", async ({
  page,
}) => {
  await openAgenda(page);
  const dialog = await openDetail(page, "0001");

  // El primer clic solo pregunta: hasta confirmar, la cita no cambia.
  await dialog.getByRole("button", { name: "Anular cita" }).click();
  await expect(dialog.getByText(/¿Anular la cita\?/)).toBeVisible();
  await expect(statusOf(dialog)).toHaveText("Programada");

  // Volver deshace la pregunta sin tocar la cita.
  await dialog.getByRole("button", { name: "Volver" }).click();
  await expect(
    dialog.getByRole("button", { name: "Anular cita" }),
  ).toBeVisible();

  await dialog.getByRole("button", { name: "Anular cita" }).click();
  await dialog.getByRole("button", { name: "Sí, anular" }).click();

  await expect(page.getByText("Cita anulada")).toBeVisible();
  await expect(statusOf(dialog)).toHaveText("Cancelada");

  await dialog.getByRole("button", { name: "Cerrar" }).click();
  await expect(
    page.getByRole("button", { name: /C-\d{4}-0001 .* Cancelada$/ }),
  ).toBeVisible();
});
