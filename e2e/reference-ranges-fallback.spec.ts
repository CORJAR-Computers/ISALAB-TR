import { test, expect, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";

/**
 * Regresión: rangos de referencia para muestras asignadas a un equipo.
 *
 * El catálogo sembrado (migraciones 0020/0021) vive en el perfil GENERAL
 * (ANALYZER_ID = 1). El diálogo de detalle consulta los rangos del equipo de
 * la muestra; si ese equipo no tiene rangos propios (p. ej. MINDRAY B2800
 * recién instalado tras actualizar desde una versión anterior), antes se
 * mostraba "— Sin rango" en todas las filas. Ahora el diálogo rellena los
 * huecos con el catálogo GENERAL.
 *
 * Reproducción con el mock de IPC: todas las gamas sembradas pertenecen al
 * analizador 1 y la muestra se crea asignada al MINDRAY (id 2).
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

test("muestra en MINDRAY muestra los rangos del catálogo GENERAL", async ({
  page,
}) => {
  await login(page);

  // Crear una muestra asignada al MINDRAY B2800 (equipo sin rangos propios).
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

  await newSample.getByRole("combobox", { name: /Equipo analizador/ }).click();
  await page.getByRole("option", { name: /MINDRAY B2800/ }).click();

  await newSample.getByRole("button", { name: "Registrar muestra" }).click();

  // Pantalla de éxito del registro; al cerrarla el padre abre el detalle.
  const success = page.getByRole("dialog");
  await expect(
    success.getByRole("heading", { name: /Muestra M-2026-0001 registrada/ }),
  ).toBeVisible();
  await success.getByRole("button", { name: "Cerrar" }).first().click();

  const detail = page.getByRole("dialog");
  await expect(
    detail.getByRole("heading", { name: /Muestra M-2026-0001/ }),
  ).toBeVisible();

  // El equipo de la muestra aparece en la cabecera del detalle.
  await expect(detail.getByText("MINDRAY B2800")).toBeVisible();

  // Glucosa (canino) cae del catálogo GENERAL aunque la muestra sea del
  // MINDRAY: 70–126 mg/dL como placeholder del rango editable (columna
  // "Rango de Referencia", editable por el veterinario desde 0022).
  await expect(detail.getByText("Resultados Analíticos")).toBeVisible();
  await expect(
    detail.getByLabel("Rango de referencia de Glucosa"),
  ).toHaveAttribute("placeholder", "70 – 126 mg/dL");

  // Urea no tiene rango en ninguna parte del mock: su rango editable sugiere
  // "Sin rango" (única fila sin gama), probando que el fallback no inventa datos.
  await expect(
    detail.getByLabel("Rango de referencia de Urea"),
  ).toHaveAttribute("placeholder", "Sin rango");

  // El estado en vivo también usa la gama caída del GENERAL (95 → Normal).
  const glucoseInput = detail.getByRole("spinbutton").first();
  await glucoseInput.fill("95");
  await expect(detail.getByText("Normal", { exact: true })).toBeVisible();
});
