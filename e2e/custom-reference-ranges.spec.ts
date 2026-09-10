import { test, expect, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";

/**
 * Flujo completo del rango de referencia definido por el veterinario
 * (migración 0022): cuando un analito no tiene rango en el catálogo
 * (Urea en el mock — ni GENERAL ni MINDRAY lo cubren), el veterinario
 * captura el suyo en la grilla, la validación Normal/Alto/Bajo lo usa
 * con precedencia sobre el catálogo y queda persistido en el resultado.
 *
 * El mock de IPC reproduce la lógica del SP (customRefMin/Max con
 * precedencia, límites abiertos). La nota al pie del PDF se verifica a
 * nivel Rust (pdf_templates::layout::tests — los ops de printpdf no son
 * inspeccionables desde el navegador).
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

/** Crea la muestra (Rocky / Suero / GENERAL) y abre el detalle. */
async function createSample(page: Page) {
  await page.getByRole("button", { name: "Muestras & Laboratorio" }).click();
  await page.getByRole("button", { name: "Nueva toma de muestra" }).click();
  const newSample = page.getByRole("dialog");
  await expect(
    newSample.getByRole("heading", { name: "Nueva toma de muestra" }),
  ).toBeVisible();

  await newSample.getByPlaceholder(/Buscar paciente/).fill("Rocky");
  await newSample.locator("button").filter({ hasText: "Rocky" }).first().click();

  await newSample.getByRole("combobox", { name: "Tipo de muestra" }).click();
  await page.getByRole("option", { name: "Suero" }).click();
  await newSample.getByRole("button", { name: "Registrar muestra" }).click();

  const success = page.getByRole("dialog");
  await expect(
    success.getByRole("heading", { name: /Muestra M-2026-0001 registrada/ }),
  ).toBeVisible();
  await success.getByRole("button", { name: "Cerrar" }).first().click();

  const detail = page.getByRole("dialog");
  await expect(
    detail.getByRole("heading", { name: /Muestra M-2026-0001/ }),
  ).toBeVisible();
  return detail;
}

test("rango manual para analito sin rango: captura, estado, guardado y persistencia", async ({
  page,
}) => {
  await login(page);
  const detail = await createSample(page);

  // Poner en proceso para habilitar la grilla de resultados.
  await detail.getByRole("button", { name: "Poner en proceso" }).click();
  await expect(detail.getByText("En proceso", { exact: true })).toBeVisible();
  await expect(detail.getByText("Resultados Analíticos")).toBeVisible();

  // ---------- Sin capturar rango: Urea queda SIN_RANGO ----------
  // Urea (analito 3 del mock) no tiene rango en ninguna parte. Con valor
  // cargado, el estado en vivo es "Cargado" (SIN_RANGO no tiene evaluación).
  const ureaRange = detail.getByLabel("Rango de referencia de Urea");
  await expect(ureaRange).toHaveAttribute("placeholder", "Sin rango");
  const ureaValue = detail.getByRole("spinbutton").nth(2); // 3.º analito del panel
  await ureaValue.fill("40");
  await expect(detail.getByText("Cargado", { exact: true })).toBeVisible();

  // ---------- El veterinario captura su rango (min,max) ----------
  // Con 40/70 el valor 40 sigue siendo Normal; al editar el rango el
  // estado en vivo re-evalúa usando el rango manual.
  await ureaRange.fill("30,70");
  await expect(
    detail.getByText("Rango definido por el veterinario"),
  ).toBeVisible();
  await expect(detail.getByText("Normal", { exact: true })).toBeVisible();

  // Guardar: el mock persiste el rango manual en el resultado.
  await detail.getByRole("button", { name: "Guardar resultados" }).click();
  await expect(
    page.getByText(/1 resultado guardado correctamente/),
  ).toBeVisible();

  // ---------- Persistencia en la grilla ----------
  // Tras invalidar la query, el rango guardado se restaura como texto de
  // la celda (re-guardar no lo pierde) y la marca de manual permanece.
  await expect(ureaRange).toHaveValue("30,70");
  await expect(
    detail.getByText("Rango definido por el veterinario").first(),
  ).toBeVisible();

  // ---------- Finalizar: vista de solo lectura con asterisco ----------
  await detail.getByRole("button", { name: "Finalizar muestra" }).click();
  await expect(
    detail.getByText(/Muestra finalizada con 1 resultado/),
  ).toBeVisible();
  await expect(detail.getByText("Resultados (1)")).toBeVisible();

  // La fila de Urea muestra el rango manual con asterisco y estado Normal.
  const ureaRow = detail.getByRole("row").filter({ hasText: "Urea" });
  await expect(ureaRow).toContainText("30 – 70 *");
  await expect(ureaRow).toContainText("Normal");
});

test("editar el rango en vivo: borde del rango manual cambia el estado", async ({
  page,
}) => {
  await login(page);
  const detail = await createSample(page);

  await detail.getByRole("button", { name: "Poner en proceso" }).click();
  await expect(detail.getByText("Resultados Analíticos")).toBeVisible();

  // Urea 80 con rango manual 30,70 → ALTO en vivo (precedencia del manual;
  // el badge en vivo usa la etiqueta sin flecha: "Alto").
  const ureaRange = detail.getByLabel("Rango de referencia de Urea");
  const ureaValue = detail.getByRole("spinbutton").nth(2);
  await ureaValue.fill("80");
  await ureaRange.fill("30,70");
  await expect(detail.getByText("Alto", { exact: true })).toBeVisible();

  // Rango abierto (solo máximo): 80 > 40 sigue ALTO.
  await ureaRange.fill(",40");
  await expect(detail.getByText("Alto", { exact: true })).toBeVisible();

  // Ampliar el máximo: 80 ≤ 100 pasa a Normal sin recargar nada.
  await ureaRange.fill("30,100");
  await expect(detail.getByText("Normal", { exact: true })).toBeVisible();

  // Rango abierto solo-mínimo: 80 ≥ 30 → Normal.
  await ureaRange.fill("30,");
  await expect(detail.getByText("Normal", { exact: true })).toBeVisible();

  // Bajar el mínimo por encima del valor: 80 < 90 → BAJO.
  await ureaRange.fill("90,");
  await expect(detail.getByText("Bajo", { exact: true })).toBeVisible();

  // Limpiar el rango manual: sin catálogo vuelve a "Cargado" (SIN_RANGO).
  await ureaRange.fill("");
  await expect(detail.getByText("Cargado", { exact: true })).toBeVisible();
});

test("el rango manual no altera a los analitos con rango de catálogo", async ({
  page,
}) => {
  await login(page);
  const detail = await createSample(page);

  await detail.getByRole("button", { name: "Poner en proceso" }).click();
  await expect(detail.getByText("Resultados Analíticos")).toBeVisible();

  // Glucosa (analito 1) SÍ tiene rango en el catálogo: el placeholder lo
  // muestra con unidad; no se captura rango manual para esta fila.
  const glucoseRange = detail.getByLabel("Rango de referencia de Glucosa");
  await expect(glucoseRange).toHaveAttribute("placeholder", "70 – 126 mg/dL");

  // La fila de Urea sigue sin rango hasta que el veterinario capture uno.
  await expect(
    detail.getByLabel("Rango de referencia de Urea"),
  ).toHaveAttribute("placeholder", "Sin rango");

  // Capturar rango manual SOLO para Urea y guardar ambas filas.
  await detail.getByLabel("Rango de referencia de Urea").fill("30,70");
  await detail.getByRole("spinbutton").nth(0).fill("95"); // Glucosa → Normal
  await detail.getByRole("spinbutton").nth(2).fill("40"); // Urea con manual
  await detail.getByRole("button", { name: "Guardar resultados" }).click();
  await expect(
    page.getByText(/2 resultados guardados correctamente/),
  ).toBeVisible();

  // Ambas filas quedaron Normal, pero solo Urea conserva el rango manual
  // (Glucosa sigue mostrando el rango del catálogo como placeholder).
  const glucoseRow = detail.getByRole("row").filter({ hasText: "Glucosa" });
  const ureaRow = detail.getByRole("row").filter({ hasText: "Urea" });
  await expect(glucoseRow).toContainText("Normal");
  await expect(ureaRow).toContainText("Normal");
  await expect(
    detail.getByLabel("Rango de referencia de Glucosa"),
  ).toHaveAttribute("placeholder", "70 – 126 mg/dL");
  await expect(
    detail.getByLabel("Rango de referencia de Urea"),
  ).toHaveValue("30,70");
});
