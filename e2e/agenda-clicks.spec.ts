import { expect, test } from "./fixtures";
import { openAgenda } from "./agenda";

/**
 * Clic sobre una cita: check-in de una PROGRAMADA (que la deja ATENDIDA) e
 * historial de una ya ATENDIDA, con las citas que siembra el mock.
 *
 * Las tarjetas se localizan por su nombre accesible (`HH:MM · código · paciente
 * · servicio · estado`); los patrones fijan código y estado y dejan el resto
 * abierto para no romperse por el copy ni por el año del código.
 */

test("clic en una cita PROGRAMADA hace check-in y la deja ATENDIDA", async ({
  page,
}) => {
  await openAgenda(page);

  await page.getByRole("button", { name: /C-\d{4}-0001 .* Programada/ }).click();

  // El check-in la deja ATENDIDA y la mutación repinta la tarjeta.
  const atendida = page.getByRole("button", {
    name: /C-\d{4}-0001 .* Atendida$/,
  });
  await expect(atendida).toBeVisible();

  // El check-in creó la visita: el segundo clic abre el historial con ella.
  await atendida.click();
  await expect(page.getByText(/^Visita V-\d{4}-\d{4}$/)).toBeVisible();
});

test("clic en una cita ATENDIDA abre el historial con sus datos", async ({
  page,
}) => {
  await openAgenda(page);

  await page.getByRole("button", { name: /C-\d{4}-0002 .* Atendida/ }).click();

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
