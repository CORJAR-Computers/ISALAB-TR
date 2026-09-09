import { test, expect, type Page, type Locator } from "@playwright/test";
import { fileURLToPath } from "node:url";

/**
 * Valores de referencia veterinarios (Settings → Analizadores).
 *
 * Con el mock de IPC (ipc-mock.script.js) que replica el seed de la migración
 * 0021 (Valores_Referencia_Veterinarios.md):
 *
 * 1. Los rangos sembrados del perfil GENERAL son VISIBLES en la tabla
 *    (canino/felino/equino) y EDITABLES (pencil → cambiar valor → guardar).
 * 2. El flujo "Nuevo analito" crea un analito desde el diálogo de rango, lo
 *    selecciona en el selector de analitos y permite guardar el rango nuevo.
 */

const VIEW_W = 1440;
const VIEW_H = 900;

test.beforeEach(async ({ page }) => {
  await page.addInitScript({
    path: fileURLToPath(new URL("./ipc-mock.script.js", import.meta.url)),
  });
  await page.setViewportSize({ width: VIEW_W, height: VIEW_H });
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

/**
 * Navega a Configuración y devuelve la tarjeta "Equipos de laboratorio".
 * Nota: el título de la tarjeta es un <div data-slot="card-title"> (shadcn
 * CardTitle), no un elemento heading.
 */
async function gotoAnalyzersCard(page: Page): Promise<Locator> {
  await page
    .locator("aside")
    .getByRole("button", { name: "Configuración", exact: true })
    .click();
  const card = page
    .locator("[data-slot='card']")
    .filter({ hasText: "Equipos de laboratorio" })
    .first();
  await card.scrollIntoViewIfNeeded();
  await expect(card).toBeVisible();
  return card;
}

/** Selector de equipo de la zona "Rangos de referencia" (primer combobox de la tarjeta). */
function rangesSelector(card: Locator) {
  return card.getByRole("combobox").first();
}

/** Combobox del diálogo de rango (Analito/Especie/Sexo). */
function dialogCombobox(page: Page, name: string) {
  return page.getByRole("dialog").getByRole("combobox", { name });
}

test("rangos sembrados del .md visibles en GENERAL y editables", async ({
  page,
}) => {
  await login(page);
  const card = await gotoAnalyzersCard(page);

  // El selector de rangos arranca en el primer equipo activo no-GENERAL
  // (MINDRAY B2800). Cambiamos al perfil GENERAL, que tiene el seed del .md.
  await rangesSelector(card).click();
  await page
    .getByRole("option", { name: "Perfil GENERAL (lectura manual)" })
    .click();

  // Rango sembrado canino + felino + equino del catálogo del .md visible.
  const row = (text: string) =>
    page.getByRole("row").filter({ hasText: text });
  await expect(
    row("Glucosa").filter({ hasText: "Canino" }),
  ).toContainText("70 – 126 mg/dL");
  await expect(
    row("Glucosa").filter({ hasText: "Felino" }),
  ).toContainText("74 – 159 mg/dL");
  await expect(
    row("Fosfatasa alcalina").filter({ hasText: "Equino" }),
  ).toContainText("88 – 261 U/L");

  // Editable: cambiar el máximo de la Glucosa canina y guardar.
  await row("Glucosa")
    .filter({ hasText: "Canino" })
    .getByRole("button", { name: "Editar" })
    .click();
  const dialog = page.getByRole("dialog");
  await expect(
    dialog.getByRole("heading", { name: "Editar rango" }),
  ).toBeVisible();
  await dialog.getByLabel("Valor máximo").fill("140");
  await dialog.getByRole("button", { name: "Guardar cambios" }).click();

  // La tabla refleja el valor guardado (mock update_reference_range).
  await expect(
    row("Glucosa").filter({ hasText: "Canino" }),
  ).toContainText("70 – 140 mg/dL");
});

test("nuevo analito desde el diálogo de rango y rango creado", async ({
  page,
}) => {
  await login(page);
  const card = await gotoAnalyzersCard(page);

  // MINDRAY B2800 es el equipo por defecto y es donde se habilita "Nuevo rango".
  await expect(rangesSelector(card)).toContainText("MINDRAY B2800");
  await card.getByRole("button", { name: "Nuevo rango" }).click();

  const dialog = page.getByRole("dialog");
  await expect(
    dialog.getByRole("heading", { name: "Nuevo rango de referencia" }),
  ).toBeVisible();

  // Activar el formulario inline "Nuevo analito" y completarlo.
  await dialog.getByRole("button", { name: "Nuevo analito" }).click();
  await dialog.getByPlaceholder("Código (p. ej. LACT)").fill("LACT");
  await dialog.getByPlaceholder("Nombre (p. ej. Lactato)").fill("Lactato");
  await dialog.getByPlaceholder("Unidad (p. ej. mmol/L)").fill("mmol/L");
  await dialog.getByPlaceholder("Método (opcional)").fill("Enzimático");
  await dialog.getByRole("button", { name: "Crear y seleccionar" }).click();

  // El analito creado queda seleccionado en el selector de analitos.
  const analyteBox = dialogCombobox(page, "Analito");
  await expect(analyteBox).toContainText("Lactato (mmol/L)");

  // Aparece también en las opciones del selector (catálogo actualizado).
  await analyteBox.click();
  await expect(page.getByRole("option", { name: /Lactato/ })).toBeVisible();
  await page.keyboard.press("Escape");

  // Completar el resto del rango y guardarlo.
  await dialogCombobox(page, "Especie").click();
  await page.getByRole("option", { name: "Canino" }).click();
  await dialog.getByLabel("Valor mínimo").fill("0.5");
  await dialog.getByLabel("Valor máximo").fill("2.5");
  await dialog.getByRole("button", { name: "Crear rango" }).click();

  // La fila nueva aparece en la tabla del equipo MINDRAY B2800.
  const row = page.getByRole("row").filter({ hasText: "Lactato" });
  await expect(row).toContainText("Canino");
  await expect(row).toContainText("0.5 – 2.5 mmol/L");

  // Y el analito ya está disponible para un rango posterior.
  await card.getByRole("button", { name: "Nuevo rango" }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Nuevo analito" })
    .click();
  await dialogCombobox(page, "Analito").click();
  await expect(
    page.getByRole("option", { name: /Lactato \(mmol\/L\)/ }),
  ).toBeVisible();
});