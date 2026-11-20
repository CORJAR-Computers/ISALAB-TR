import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { installIpcMock } from "./mock";
import { gotoView } from "./navigation";
import { entityCodePattern } from "./dates";
import {
  callCount,
  clearFaults,
  DB_DOWN_MESSAGE,
  setFaults,
  setFaultsBeforeLoad,
} from "./faults";

/**
 * Fallos del backend simulados con la inyección del mock IPC.
 *
 * Lo que se comprueba no es solo que la app "no se rompa": es que un fallo de
 * una consulta no se disfrace de estado vacío, que el mensaje real llegue al
 * usuario y que el botón Reintentar recupere cuando el backend vuelve.
 */
test.beforeEach(async ({ page }) => {
  await installIpcMock(page);
});

async function login(page: Page) {
  await page.goto("/");
  await expect(page.getByLabel("Usuario")).toBeVisible();
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  await expect(
    page.getByRole("heading", { name: "Panel de control" }).first(),
  ).toBeVisible();
}

const bootError = (page: Page) =>
  page.getByText(/No se pudo restaurar la sesión anterior/);

test("arranque con la base caída: banner con el motivo real y login al recuperarse", async ({
  page,
}) => {
  // Un comando concreto: la restauración de la sesión falla, el resto no.
  await setFaultsBeforeLoad(page, { commands: ["get_session"] });
  await page.goto("/");

  // No se degrada a "no hay sesión": se explica por qué se pide login.
  await expect(bootError(page)).toBeVisible();
  await expect(bootError(page)).toContainText(DB_DOWN_MESSAGE);
  await expect(page.getByLabel("Usuario")).toBeVisible();

  // La base vuelve: el usuario entra sin recargar y el aviso desaparece.
  await clearFaults(page);
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  await expect(
    page.getByRole("heading", { name: "Panel de control" }).first(),
  ).toBeVisible();
  await expect(bootError(page)).toHaveCount(0);
});

test("cola de recepción: el grupo reception falla, muestra error y se recupera con Reintentar", async ({
  page,
}) => {
  await login(page);

  // Fallo por grupo: cubre list_visits (la cola) y las acciones de visita.
  await setFaults(page, { groups: ["reception"] });
  await gotoView(page, "Recepción de pacientes");

  const errorCard = page
    .getByRole("alert")
    .filter({ hasText: "No se pudo cargar la cola de visitas" });
  await expect(errorCard).toBeVisible();
  await expect(errorCard).toContainText(DB_DOWN_MESSAGE);
  // Y no se hace pasar por una cola vacía.
  await expect(page.getByText(/No hay visitas con estos filtros/)).toHaveCount(0);

  // react-query reintenta una vez (retry: 1): se espera ese reintento por
  // conteo de llamadas, no con una pausa, para que el clic sea el único
  // refetch pendiente y la recuperación sea determinista.
  await expect
    .poll(() => callCount(page, "list_visits"))
    .toBeGreaterThanOrEqual(2);

  await clearFaults(page);
  await errorCard.getByRole("button", { name: "Reintentar" }).click();

  await expect(errorCard).toHaveCount(0);
  await expect(page.getByText(/No hay visitas con estos filtros/)).toBeVisible();
});

test("una acción que devuelve error: aviso con el motivo, el diálogo sigue abierto y al reintentar se guarda", async ({
  page,
}) => {
  await login(page);
  await gotoView(page, "Recepción de pacientes");
  await expect(page.getByText(/No hay visitas con estos filtros/)).toBeVisible();

  // Solo falla la escritura: la vista sigue cargada y el usuario no pierde lo
  // que escribió en el formulario.
  await setFaults(page, { commands: ["create_visit"] });
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

  // El toast lleva el mensaje del backend, no un "algo salió mal" genérico.
  await expect(page.getByText("No se pudo registrar la visita")).toBeVisible();
  await expect(page.getByText(DB_DOWN_MESSAGE)).toBeVisible();
  // El diálogo no se cierra en falso: no se perdió la visita.
  await expect(
    dialog.getByRole("heading", { name: "Nueva visita" }),
  ).toBeVisible();
  await expect(page.getByText("Visita registrada")).toHaveCount(0);

  // La base vuelve y el mismo botón completa la operación.
  await clearFaults(page);
  await dialog.getByRole("button", { name: "Registrar visita" }).click();
  await expect(page.getByText("Visita registrada")).toBeVisible();
  await expect(page.getByText(entityCodePattern("V", 1)).first()).toBeVisible();
});

test("fallo total (all): la app sigue usable, avisa y se recupera al limpiar", async ({
  page,
}) => {
  await setFaultsBeforeLoad(page, { all: true });
  await page.goto("/");

  await expect(bootError(page)).toBeVisible();

  // Incluso con todo caído, el formulario responde y el error se explica.
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  // El toast del login (el banner de arranque sigue en pantalla con el mismo
  // mensaje, así que se acota al toast para no chocar en modo estricto).
  const loginToast = page
    .locator("[data-sonner-toast]")
    .filter({ hasText: "No se pudo iniciar sesión" });
  await expect(loginToast).toBeVisible();
  await expect(loginToast).toContainText(DB_DOWN_MESSAGE);

  await clearFaults(page);
  await page.getByRole("button", { name: "Entrar" }).click();
  await expect(
    page.getByRole("heading", { name: "Panel de control" }).first(),
  ).toBeVisible();
});
