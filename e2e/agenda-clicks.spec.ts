import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { installIpcMock } from "./mock";
import { gotoView } from "./navigation";

/**
 * Clic sobre una cita de la agenda.
 *
 * Los dos caminos del clic dependen de comandos que el mock no modelaba
 * (`check_in_appointment` y `get_clinical_history`), así que hasta ahora no se
 * podían comprobar: el clic en una PROGRAMADA solo mostraba un toast de error y
 * el de una ATENDIDA dejaba el historial en esqueletos para siempre. Aquí se
 * ejercitan los dos con las citas que siembra el propio mock (Rocky 09:00
 * PROGRAMADA, Luna 11:00 ATENDIDA) y se verifica lo que el usuario ve.
 *
 * Las tarjetas se localizan por su nombre accesible, que es el detalle completo
 * de la cita (`HH:MM · código · paciente · servicio · estado`): por eso los
 * patrones fijan código y estado y dejan el resto abierto, para no romperse por
 * un cambio de copy ni por el año del código.
 */
async function openAgenda(page: Page) {
  await installIpcMock(page);
  await page.goto("/");
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  await gotoView(page, "Agenda de citas");
  await expect(
    page.getByRole("heading", { name: "Agenda de citas" }).first(),
  ).toBeVisible();
  // La rejilla lleva `aria-busy` mientras la consulta está pendiente.
  await expect(page.locator("[aria-busy]")).toHaveAttribute(
    "aria-busy",
    "false",
  );
}

test("clic en una cita PROGRAMADA hace check-in y la deja EN ATENCIÓN", async ({
  page,
}) => {
  await openAgenda(page);

  await page.getByRole("button", { name: /C-\d{4}-0001 .* Programada/ }).click();

  // El mock responde `check_in_appointment` con la Visit y pasa la cita a
  // EN_ATENCION; la mutación invalida la agenda y la tarjeta se repinta.
  const enAtencion = page.getByRole("button", {
    name: /C-\d{4}-0001 .* En atención$/,
  });
  await expect(enAtencion).toBeVisible();
  // El toast de error era el síntoma de que faltaba el handler.
  await expect(page.getByText("No se pudo hacer el check-in")).toHaveCount(0);

  // El check-in creó la visita: aparece en el historial del paciente.
  await enAtencion.click();
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
