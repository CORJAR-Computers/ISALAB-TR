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

// Regression: hacer clic fuera de un modal no debe inhabilitar sus botones.
// El overlay captura el clic, el modal se queda abierto y Guardar/Cancelar
// siguen respondiendo (bug de pointer-events atrapado en el body con Radix).
test("clic fuera del modal no inhabilita sus botones", async ({ page }) => {
  await login(page);

  await page.getByRole("button", { name: "Muestras & Laboratorio" }).click();
  await page.getByRole("button", { name: "Nueva toma de muestra" }).click();

  const dialog = page.getByRole("dialog");
  await expect(
    dialog.getByRole("heading", { name: "Nueva toma de muestra" }),
  ).toBeVisible();

  // Clic fuera del modal (sobre el overlay, no sobre un control del diálogo).
  await page.mouse.click(20, 400);

  // El modal sigue abierto…
  await expect(
    dialog.getByRole("heading", { name: "Nueva toma de muestra" }),
  ).toBeVisible();
  // …y sus controles siguen respondiendo (el input recibe texto).
  const patientInput = dialog.getByPlaceholder(/Buscar paciente/);
  await patientInput.fill("Rocky");
  await expect(patientInput).toHaveValue("Rocky");

  // Cerrar con Escape (la X queda fuera del viewport por la altura del modal)
  // y verificar que la app sigue usable.
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("button", { name: "Nueva toma de muestra" }),
  ).toBeVisible();
});

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

  // ---------- Cargar un resultado analítico (grilla de panel) ----------
  // La grilla trae los analitos del panel "Química básica" (Glucosa,
  // Hematocrito, Urea); se llena solo la fila de Glucosa.
  await expect(
    detail.getByText("Resultados Analíticos"),
  ).toBeVisible();
  const glucoseInput = detail.getByRole("spinbutton").first();
  await glucoseInput.fill("95");
  // El estado se evalúa en vivo contra el rango de referencia (70–126 → Normal).
  await expect(detail.getByText("Normal", { exact: true })).toBeVisible();
  await detail
    .getByRole("button", { name: "Guardar resultados" })
    .click();
  // El toast de sonner vive fuera del diálogo (region "Notifications").
  await expect(
    page.getByText(/1 resultado guardado correctamente/),
  ).toBeVisible();
  // Badge de la grilla: 1 de 3 analitos con valor.
  await expect(detail.getByText(/1 con valor \/ 3 analitos/)).toBeVisible();

  // ---------- Finalizar la muestra ----------
  await detail.getByRole("button", { name: "Finalizar muestra" }).click();
  await expect(
    detail.getByText(/Muestra finalizada con 1 resultado/),
  ).toBeVisible();
  // Al finalizar, la grilla editable pasa a la vista de solo lectura.
  await expect(detail.getByText("Resultados (1)")).toBeVisible();
  await expect(
    detail.getByRole("button", { name: "Generar PDF" }),
  ).toBeVisible();
  await expect(detail.getByText("Finalizada", { exact: true })).toBeVisible();

  // ---------- Historial y notificaciones (diálogos apilados) ----------
  // Se abren sobre el detalle y se cierran con su propio botón Cerrar; así se
  // ejercita la red de seguridad de pointer-events con modales apilados.
  // exact: true → "Historial" también coincide con "Ver historial" (tarjeta
  // del paciente) y "Notificaciones" con otros textos del detalle.
  await detail
    .getByRole("button", { name: "Historial", exact: true })
    .click();
  const eventsDialog = page.getByRole("dialog").filter({
    hasText: "Historial de la muestra",
  });
  await expect(
    eventsDialog.getByText("Tubo sin etiquetar"),
  ).toBeVisible();
  // .first() → el botón del footer (la X del diálogo también se llama "Cerrar").
  await eventsDialog
    .getByRole("button", { name: "Cerrar" })
    .first()
    .click();

  await detail
    .getByRole("button", { name: "Notificaciones", exact: true })
    .click();
  const notifDialog = page.getByRole("dialog").filter({
    hasText: "Notificaciones de la muestra",
  });
  await expect(notifDialog.getByText("juan.perez@example.com")).toBeVisible();
  await expect(notifDialog.getByText("Dra. Ana Pérez")).toBeVisible();
  await notifDialog
    .getByRole("button", { name: "Cerrar" })
    .first()
    .click();
  // Los diálogos permanecen montados durante la animación de salida; esperar
  // a que se desmonten evita que el Cerrar del detalle resuelva contra ellos.
  await expect(
    page
      .getByRole("dialog")
      .filter({ hasText: "Notificaciones de la muestra" }),
  ).toHaveCount(0);

  // ---------- Cerrar y verificar la trazabilidad en la tabla ----------
  // .first() → el botón del footer (la X del diálogo también se llama "Cerrar").
  await detail
    .getByRole("button", { name: "Cerrar" })
    .first()
    .click();
  await expect(
    detail.getByRole("heading", { name: /Muestra M-2026-0001/ }),
  ).toBeHidden();
  // exact: true → evita los toasts de sonner ("Muestra M-2026-0001 finalizada"…).
  await expect(page.getByText("M-2026-0001", { exact: true })).toBeVisible();
  await expect(page.getByText("Finalizada", { exact: true }).first()).toBeVisible();

  // Tras apilar y cerrar tres diálogos, la app sigue respondiendo (el body no
  // queda con pointer-events: none atrapado).
  await page.getByRole("button", { name: "Nueva toma de muestra" }).click();
  await expect(
    page.getByRole("dialog").getByRole("heading", {
      name: "Nueva toma de muestra",
    }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
});
