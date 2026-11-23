import { expect, test } from "./fixtures";
import { openAgenda, todayISO } from "./agenda";

/**
 * Programar una cita desde el diálogo: el aviso de creada y la tarjeta en la
 * semana, a su hora y en el estado inicial PROGRAMADA.
 *
 * La cita nueva recibe el id 3 del mock (las dos sembradas ocupan 1 y 2), de
 * ahí el código `C-<año>-0003` que fija la búsqueda.
 */
test("programar una cita la deja en la semana como PROGRAMADA", async ({
  page,
}) => {
  await openAgenda(page);

  await page.getByRole("button", { name: "Nueva cita" }).click();
  const dialog = page.getByRole("dialog");

  await dialog.getByPlaceholder(/Buscar paciente/).fill("Rocky");
  await dialog.getByRole("combobox").first().click();
  await page.getByRole("option", { name: /Rocky/ }).click();

  await dialog.getByRole("combobox").nth(1).click();
  await page.getByRole("option", { name: /Consulta general/ }).click();

  // El campo de fecha no va en un `FormControl`, así que no está atado a su
  // etiqueta y se localiza por tipo.
  await dialog
    .locator('input[type="datetime-local"]')
    .fill(`${todayISO()}T15:00`);
  await dialog.getByRole("button", { name: "Programar cita" }).click();

  await expect(page.getByText("Cita programada")).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: /^15:00 · C-\d{4}-0003 · Rocky · Consulta general · Programada$/,
    }),
  ).toBeVisible();
});
