import { test, expect, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";

/**
 * Smoke test E2E: login → dashboard → flujo completo de una muestra
 * (recepción → en proceso → resultado → finalización).
 *
 * La capa IPC de Tauri se mockea con e2e/ipc-mock.script.js (estado en
 * memoria), inyectado antes de que cargue la app.
 */
test.beforeEach(async ({ page }) => {
  await page.addInitScript({
    path: fileURLToPath(new URL("./ipc-mock.script.js", import.meta.url)),
  });
});

async function login(page: Page) {
  await page.goto("/");
  await expect(page.getByLabel("Usuario")).toBeVisible();
  await page.getByLabel("Usuario").fill("admin");
  // exact: true → evita el botón "Mostrar/Ocultar contraseña" (matching por substring).
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
}

test("login → dashboard → flujo completo de muestra", async ({ page }) => {
  // ---------- Login ----------
  await login(page);

  // ---------- Dashboard ----------
  await expect(
    page.getByRole("heading", { name: "Panel de control" }).first(),
  ).toBeVisible();
  await expect(page.getByText("pacientes activos", { exact: true })).toBeVisible();
  await expect(page.getByText("Muestras en proceso")).toBeVisible();

  // ---------- Mesa de muestras (vacía) ----------
  await page
    .getByRole("button", { name: "Muestras & Laboratorio" })
    .click();
  await expect(
    page.getByRole("heading", { name: "Muestras & Laboratorio" }).first(),
  ).toBeVisible();
  await expect(page.getByText("No hay muestras registradas.")).toBeVisible();

  // ---------- Nueva toma de muestra ----------
  await page.getByRole("button", { name: "Nueva toma de muestra" }).click();
  const dialog = page.getByRole("dialog");
  await expect(
    dialog.getByRole("heading", { name: "Nueva toma de muestra" }),
  ).toBeVisible();

  // Buscar y seleccionar el paciente
  await dialog.getByPlaceholder(/Buscar paciente/).fill("Rocky");
  await dialog.locator("button").filter({ hasText: "Rocky" }).first().click();

  // Tipo de muestra (Radix Select; el diálogo también tiene el combobox
  // "Equipo analizador (opcional)", así que se acota por nombre).
  await dialog.getByRole("combobox", { name: "Tipo de muestra" }).click();
  await page.getByRole("option", { name: "Suero" }).click();

  // Registrar la muestra
  await dialog.getByRole("button", { name: "Registrar muestra" }).click();

  // Pantalla de éxito: genera la etiqueta con código de barras para el tubo
  const success = page.getByRole("dialog");
  await expect(
    success.getByRole("heading", { name: /Muestra M-2026-0001 registrada/ }),
  ).toBeVisible();
  await expect(
    success.getByRole("button", { name: /Generar e imprimir etiqueta/ }),
  ).toBeVisible();

  // Generar la etiqueta y abrirla para imprimirla
  await success
    .getByRole("button", { name: /Generar e imprimir etiqueta/ })
    .click();
  await expect(page.getByText("Etiqueta de muestra generada")).toBeVisible();

  // Se abre el detalle con la muestra RECIBIDA
  const detail = page.getByRole("dialog");
  await expect(
    detail.getByRole("heading", { name: /Muestra M-2026-0001/ }),
  ).toBeVisible();
  await expect(detail.getByText("Recibida", { exact: true })).toBeVisible();

  // ---------- Poner en proceso ----------
  await detail.getByRole("button", { name: "Poner en proceso" }).click();
  await expect(detail.getByText("En proceso", { exact: true })).toBeVisible();

  // ---------- Cargar un resultado analítico ----------
  await detail.getByRole("combobox").click();
  await page.getByRole("option", { name: "Glucosa (mg/dL)" }).click();
  await detail.getByRole("spinbutton").fill("95");
  await detail.getByRole("button", { name: "Cargar" }).click();
  await expect(detail.getByText("Resultados (1)")).toBeVisible();

  // ---------- Finalizar la muestra ----------
  await detail.getByRole("button", { name: "Finalizar muestra" }).click();
  await expect(
    detail.getByText(/Muestra finalizada con 1 resultado/),
  ).toBeVisible();
  await expect(
    detail.getByRole("button", { name: "Generar PDF" }),
  ).toBeVisible();
  await expect(detail.getByText("Finalizada", { exact: true })).toBeVisible();

  // ---------- Cerrar y verificar la trazabilidad en la tabla ----------
  // .last() → el botón del footer (la X del diálogo también se llama "Cerrar").
  await detail.getByRole("button", { name: "Cerrar" }).last().click();
  // exact: true → evita los toasts de sonner ("Muestra M-2026-0001 finalizada"…).
  await expect(page.getByText("M-2026-0001", { exact: true })).toBeVisible();
  await expect(page.getByText("Finalizada", { exact: true }).first()).toBeVisible();
});
