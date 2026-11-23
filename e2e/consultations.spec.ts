import type { Locator, Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { installIpcMock } from "./mock";
import { gotoView } from "./navigation";

/**
 * Agenda de consultas: contadores de las pestañas, filtros por estado y
 * búsqueda, alta desde el diálogo y cambios de estado desde la fila.
 *
 * El mock siembra cuatro consultas: dos pendientes de hoy (Rocky y Luna), una
 * completada y una cancelada de días atrás. Las filas se localizan por el
 * motivo, que es único entre los datos sembrados; los botones de pestaña se
 * localizan por su etiqueta porque además llevan el contador al lado.
 */

/** Consultas sembradas que quedan pendientes al abrir la vista. */
const PENDING = 2;
/** Total de consultas sembradas (pestaña «Todas»). */
const TOTAL = 4;

/** Login, navegación a la agenda de consultas y espera a que la tabla cargue. */
async function openConsultations(page: Page) {
  await installIpcMock(page);
  await page.goto("/");
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  await gotoView(page, "Agenda de consultas");
  await expect(
    page.getByRole("heading", { name: "Agenda de consultas" }).first(),
  ).toBeVisible();
  // La pestaña por defecto es «Pendientes»; mientras carga hay 5 esqueletos,
  // así que esperar el número de filas sembradas también espera a la consulta.
  await expect(page.getByRole("main").locator("tbody tr")).toHaveCount(PENDING);
}

/** Botón de una pestaña de estado (su nombre incluye el contador). */
function tab(main: Locator, label: string) {
  return main.getByRole("button", { name: new RegExp(`^${label}`) });
}

/** Fila de la tabla que contiene ese texto (motivo, paciente u hora). */
function row(main: Locator, text: string) {
  return main.locator("tbody tr").filter({ hasText: text });
}

test("las pestañas cuentan por estado y el listado filtra por estado y búsqueda", async ({
  page,
}) => {
  await openConsultations(page);
  const main = page.getByRole("main");

  // Contadores globales: no dependen del filtro ni de la búsqueda.
  await expect(tab(main, "Pendientes")).toHaveText(/^Pendientes\s*2$/);
  await expect(tab(main, "Completadas")).toHaveText(/^Completadas\s*1$/);
  await expect(tab(main, "Canceladas")).toHaveText(/^Canceladas\s*1$/);

  // «Todas» suma los tres estados.
  await main.getByRole("button", { name: "Todas" }).click();
  await expect(main.locator("tbody tr")).toHaveCount(TOTAL);

  // Completadas: solo la consulta de días atrás.
  await tab(main, "Completadas").click();
  await expect(main.locator("tbody tr")).toHaveCount(1);
  await expect(row(main, "Vacunación anual")).toBeVisible();

  // La búsqueda del backend cubre paciente, propietario y motivo.
  await main.getByRole("button", { name: "Todas" }).click();
  const search = main.getByPlaceholder(/Buscar por paciente/);
  await search.fill("vómito");
  await expect(main.locator("tbody tr")).toHaveCount(1);
  await expect(row(main, "Vómito recurrente")).toBeVisible();

  await search.fill("Luna");
  await expect(main.locator("tbody tr")).toHaveCount(2);

  await search.fill("juan");
  await expect(main.locator("tbody tr")).toHaveCount(TOTAL);
});

test("una consulta nueva aparece en la agenda con el estado elegido", async ({
  page,
}) => {
  await openConsultations(page);
  const main = page.getByRole("main");

  await page.getByRole("button", { name: "Nueva consulta" }).click();
  const dialog = page.getByRole("dialog");

  // El paciente se elige de la lista filtrada; al elegirlo aparece el formulario.
  await dialog.getByPlaceholder(/Buscar paciente/).fill("Rocky");
  await dialog.getByRole("button", { name: /Rocky/ }).click();
  await dialog.getByLabel(/Motivo de consulta/).fill("Control de peso");
  // El diálogo guarda por defecto como COMPLETADA; aquí se programa pendiente.
  await dialog.getByRole("combobox").click();
  await page.getByRole("option", { name: "Pendiente" }).click();
  await dialog.getByRole("button", { name: "Guardar consulta" }).click();

  await expect(page.getByText("Consulta registrada")).toBeVisible();
  // Cae en la pestaña activa (Pendientes) y el contador la suma.
  await expect(tab(main, "Pendientes")).toHaveText(/^Pendientes\s*3$/);
  await expect(main.locator("tbody tr")).toHaveCount(PENDING + 1);

  const created = row(main, "Control de peso");
  await expect(created).toHaveCount(1);
  await expect(created).toContainText("Rocky");
  await expect(created.getByText("Pendiente")).toBeVisible();
});

test("completar y cancelar desde la fila mueve la consulta de pestaña", async ({
  page,
}) => {
  await openConsultations(page);
  const main = page.getByRole("main");

  await row(main, "Control posquirúrgico")
    .getByRole("button", { name: "Completar" })
    .click();

  await expect(page.getByText("Consulta completada")).toBeVisible();
  // Sale de pendientes y los contadores se repintan con la respuesta nueva.
  await expect(row(main, "Control posquirúrgico")).toHaveCount(0);
  await expect(tab(main, "Pendientes")).toHaveText(/^Pendientes\s*1$/);
  await expect(tab(main, "Completadas")).toHaveText(/^Completadas\s*2$/);

  await row(main, "Vómito recurrente")
    .getByRole("button", { name: "Cancelar" })
    .click();

  await expect(page.getByText("Consulta cancelada")).toBeVisible();
  await expect(row(main, "Vómito recurrente")).toHaveCount(0);
  await expect(tab(main, "Pendientes")).toHaveText(/^Pendientes\s*0$/);
  await expect(tab(main, "Canceladas")).toHaveText(/^Canceladas\s*2$/);

  // En la pestaña de completadas la consulta ya no ofrece acciones de estado.
  await tab(main, "Completadas").click();
  const completed = row(main, "Control posquirúrgico");
  await expect(completed.getByText("Completada")).toBeVisible();
  await expect(completed.getByRole("button", { name: "Completar" })).toHaveCount(
    0,
  );
  await expect(completed.getByRole("button", { name: "Cancelar" })).toHaveCount(
    0,
  );
});
