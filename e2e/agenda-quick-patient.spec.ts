import { expect, test } from "./fixtures";
import { openAgenda, todayISO } from "./agenda";

/**
 * Alta rápida de propietario+paciente desde el diálogo de «Nueva cita».
 *
 * El flujo real: la agenda abre su diálogo, el usuario pulsa el botón de alta
 * en línea (sin salir de la cita), registra propietario y mascota, y la cita se
 * programa con el paciente recién creado. El mock responde `create_patient` con
 * el `Patient` completo (id 3, código PAC-AAAA-0003) y reutiliza o crea el
 * propietario por documento.
 *
 * El alta tiene su propio diálogo, apilado sobre el de la cita; se cierra con
 * su botón «Cerrar» para volver a la cita (hoy el alta no se cierra sola).
 */
test("el alta rápida crea propietario y paciente y programa la cita", async ({
  page,
}) => {
  await openAgenda(page);

  await page.getByRole("button", { name: "Nueva cita" }).click();
  const appointmentDialog = page.getByRole("dialog");
  await appointmentDialog
    .getByRole("button", { name: "Nuevo paciente" })
    .click();

  // Diálogos apilados: el de alta es el último que se abrió.
  const patientDialog = page.getByRole("dialog").last();
  await expect(
    patientDialog.getByRole("heading", { name: "Nuevo paciente" }),
  ).toBeVisible();

  // Propietario nuevo (documento que no existe en el mock).
  await patientDialog.getByLabel("Número de documento").fill("1099887766");
  await patientDialog.getByLabel("Nombre completo").fill("Camila Restrepo");
  await patientDialog.getByLabel("Teléfono").fill("3105556677");

  // Paciente: nombre, especie (habilita las razas) y sexo son obligatorios.
  await patientDialog.getByLabel("Nombre", { exact: true }).fill("Mateo");
  await patientDialog.getByRole("combobox", { name: "Especie" }).click();
  await page.getByRole("option", { name: "Felino" }).click();
  await patientDialog.getByRole("combobox", { name: "Sexo" }).click();
  await page.getByRole("option", { name: "Hembra" }).click();
  await patientDialog.getByLabel("Fecha de nacimiento").fill("2023-05-10");

  await patientDialog.getByRole("button", { name: "Guardar paciente" }).click();

  // El alta devuelve el paciente con su id y especie, y la cita lo informa.
  await expect(page.getByText("Paciente Mateo registrado")).toBeVisible();
  await expect(page.getByText("#3 · Felino")).toBeVisible();
  await expect(page.getByText("Paciente listo para la cita")).toBeVisible();

  await patientDialog.getByRole("button", { name: "Cerrar" }).click();

  // De vuelta en la cita, el paciente nuevo queda seleccionado con el
  // propietario que se acaba de crear, y el alta ya no dispara el formulario
  // de la cita (el diálogo va en un portal, pero el submit sube por el árbol
  // de React; sin cortarlo aparecían «Selecciona el paciente» y el NaN del
  // servicio).
  await expect(
    appointmentDialog.getByRole("combobox", { name: /^Paciente/ }),
  ).toHaveText("Mateo · Felino · Camila Restrepo");
  await expect(
    appointmentDialog.getByText("Selecciona el paciente"),
  ).toHaveCount(0);

  await appointmentDialog.getByRole("combobox").nth(1).click();
  await page.getByRole("option", { name: /Consulta general/ }).click();
  await appointmentDialog
    .locator('input[type="datetime-local"]')
    .fill(`${todayISO()}T16:00`);
  await appointmentDialog.getByRole("button", { name: "Programar cita" }).click();

  // La cita es la tercera del mock (las dos sembradas ocupan 1 y 2).
  await expect(page.getByText("Cita programada")).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: /^16:00 · C-\d{4}-0003 · Mateo · Consulta general · Programada$/,
    }),
  ).toBeVisible();
});
