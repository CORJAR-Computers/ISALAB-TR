import { expect, test } from "./fixtures";
import { openAgenda } from "./agenda";

/**
 * Detalle de una cita y su historial: el check-in de una PROGRAMADA (que la
 * deja ATENDIDA y crea la visita) y el salto al historial clínico desde una ya
 * ATENDIDA, con las citas que siembra el mock.
 *
 * Las tarjetas se localizan por su nombre accesible (`HH:MM · código · paciente
 * · servicio · estado`); los patrones fijan código y estado y dejan el resto
 * abierto para no romperse por el copy ni por el año del código.
 */

test("el check-in desde el detalle deja la cita ATENDIDA y crea la visita", async ({
  page,
}) => {
  await openAgenda(page);

  await page
    .getByRole("button", { name: /C-\d{4}-0001 .* Programada$/ })
    .click();
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("button", { name: "Hacer check-in" }).click();

  // El check-in la deja ATENDIDA y la mutación repinta el detalle.
  await expect(page.getByText("Check-in realizado")).toBeVisible();
  await expect(dialog.locator("dd").first()).toHaveText("Atendida");

  // La visita quedó creada: desde el detalle se abre el historial con ella.
  await dialog.getByRole("button", { name: "Ver historial clínico" }).click();
  await expect(page.getByText(/^Visita V-\d{4}-\d{4}$/)).toBeVisible();
});

test("una cita ATENDIDA abre el historial con sus datos", async ({ page }) => {
  await openAgenda(page);

  await page.getByRole("button", { name: /C-\d{4}-0002 .* Atendida$/ }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Ver historial clínico" })
    .click();

  // Ficha del paciente y propietario (`patient` y `owner` de la respuesta).
  const main = page.getByRole("main");
  await expect(main.getByText("Luna").first()).toBeVisible();
  await expect(main.getByText("Juan Pérez").first()).toBeVisible();
  await expect(main.getByText("Canino · Criollo")).toBeVisible();

  // Actividad clínica: la visita y la sesión de estética del mock.
  await expect(main.getByText("Línea de tiempo clínica")).toBeVisible();
  await expect(main.getByText(/^Visita V-\d{4}-\d{4}$/)).toBeVisible();
  await expect(main.getByText("Estética · Baño completo")).toBeVisible();

  // La ficha no debe quedar con avisos de comandos sin modelar.
  await expect(
    main.getByText("No se pudo cargar las órdenes de laboratorio"),
  ).toHaveCount(0);
});
