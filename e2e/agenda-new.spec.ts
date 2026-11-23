import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { openAgenda, todayISO } from "./agenda";

/**
 * Programar una cita desde el diálogo: el aviso de creada y la tarjeta en la
 * semana, y el rechazo cuando el horario se cruza con otra cita del paciente.
 *
 * La cita nueva recibe el id 3 del mock (las dos sembradas ocupan 1 y 2), de
 * ahí el código `C-<año>-0003` que fijan las búsquedas.
 */

/** Abre el diálogo, lo completa (paciente, servicio y hora) y programa. */
async function programAppointment(
  page: Page,
  { patient, time }: { patient: string; time: string },
) {
  await page.getByRole("button", { name: "Nueva cita" }).click();
  const dialog = page.getByRole("dialog");

  await dialog.getByPlaceholder(/Buscar paciente/).fill(patient);
  await dialog.getByRole("combobox").first().click();
  await page.getByRole("option", { name: new RegExp(patient) }).click();

  await dialog.getByRole("combobox").nth(1).click();
  await page.getByRole("option", { name: /Consulta general/ }).click();

  // El campo de fecha no va en un `FormControl`, así que no está atado a su
  // etiqueta y se localiza por tipo.
  await dialog
    .locator('input[type="datetime-local"]')
    .fill(`${todayISO()}T${time}`);
  await dialog.getByRole("button", { name: "Programar cita" }).click();
  return dialog;
}

test("programar una cita la deja en la semana como PROGRAMADA", async ({
  page,
}) => {
  await openAgenda(page);
  await programAppointment(page, { patient: "Rocky", time: "15:00" });

  await expect(page.getByText("Cita programada")).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: /^15:00 · C-\d{4}-0003 · Rocky · Consulta general · Programada$/,
    }),
  ).toBeVisible();
});

test("programar encima de otra cita muestra el error de solapamiento", async ({
  page,
}) => {
  await openAgenda(page);
  // La cita sembrada de Rocky es hoy 09:00–09:30: a las 09:15 se cruza.
  const dialog = await programAppointment(page, {
    patient: "Rocky",
    time: "09:15",
  });

  // El backend rechaza con Validation y el diálogo lo muestra sin cerrarse.
  await expect(page.getByText("No se pudo programar la cita")).toBeVisible();
  await expect(page.getByText(/Ya existe una cita solapada/)).toBeVisible();
  await expect(dialog).toBeVisible();

  // No se creó ninguna cita: el mock no avanza el id tras el rechazo.
  await expect(page.getByRole("button", { name: /C-\d{4}-0003/ })).toHaveCount(
    0,
  );
});
