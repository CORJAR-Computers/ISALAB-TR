import { test, expect, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";

/**
 * Flujo de dinero completo: apertura de caja, cobro mixto con abono parcial,
 * segundo pago, recibo, gasto y cierre con diferencia de arqueo; además la
 * anulación de un pago (reverso en caja + auditoría) y el bloqueo de la
 * anulación de factura mientras tenga pagos vigentes.
 *
 * La capa IPC de Tauri se mockea con e2e/ipc-mock.script.js, que reproduce las
 * reglas del backend para caja, abonos y saldos (una sola caja abierta, caja
 * obligatoria para cobrar, movimientos solo por la parte en efectivo).
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

/** Cierra la pila de diálogos con su X (Escape solo cierra el superior). */
async function closeDialogs(page: Page) {
  for (let i = 0; i < 4; i += 1) {
    const dialogs = page.getByRole("dialog");
    if ((await dialogs.count()) === 0) return;
    const close = dialogs.last().locator('[data-slot="dialog-close"]');
    if ((await close.count()) > 0) {
      // El diálogo puede estar cerrándose solo (p. ej. tras generar): el clic
      // sobre una X que se desmonta no debe hacer fallar la espera.
      await close.click({ timeout: 2000 }).catch(() => {});
    } else {
      await page.keyboard.press("Escape");
    }
    await page.waitForTimeout(200);
  }
  await expect(page.getByRole("dialog")).toHaveCount(0);
}

/** Navega a una vista: expande su sección del sidebar si está cerrada. */
async function gotoView(page: Page, section: string, item: string) {
  await closeDialogs(page);
  const header = page.getByRole("button", { name: section, exact: true });
  if ((await header.getAttribute("aria-expanded")) !== "true") {
    await header.click();
  }
  await page.getByRole("button", { name: item, exact: true }).click();
}

async function openCash(page: Page, openingFloat: string) {
  await gotoView(page, "Administración", "Caja");
  await expect(page.getByRole("heading", { name: "Caja" })).toBeVisible();
  await page.getByLabel("Base inicial").fill(openingFloat);
  await page.getByRole("button", { name: "Abrir caja" }).click();
  await expect(page.getByText("Caja abierta")).toBeVisible();
  await expect(page.getByText("Efectivo esperado")).toBeVisible();
}

/** Abre Facturación y devuelve la fila de la factura indicada. */
async function invoiceRow(page: Page, number: string) {
  await gotoView(page, "Administración", "Facturación");
  await expect(page.getByRole("heading", { name: "Facturación" })).toBeVisible();
  const row = page.getByRole("row").filter({ hasText: number });
  await expect(row).toBeVisible();
  return row;
}

test("caja, cobro mixto con abono, segundo pago, recibo, gasto y cierre con diferencia", async ({
  page,
}) => {
  await login(page);

  // ---------- 1. Abrir caja con base inicial ----------
  await openCash(page, "100000");
  await expect(page.getByText("CAJ-00000001")).toBeVisible();
  await expect(page.getByText("Todavía no hay cobros en esta sesión.")).toBeVisible();

  // ---------- 2. Cobro mixto con abono parcial ----------
  // Factura sembrada: 100.000 + IVA 19% = 119.000.
  const row = await invoiceRow(page, "FAC-000001");
  await expect(row).toContainText("119.000");
  await expect(row).toContainText("Pendiente");
  await row.getByRole("button", { name: "Cobrar" }).click();

  const payment = page.getByRole("dialog");
  await expect(
    payment.getByRole("heading", { name: "Registrar pago" }),
  ).toBeVisible();
  // El saldo completo viene precargado en efectivo.
  const amounts = payment.locator('input[inputmode="decimal"]');
  await expect(amounts).toHaveCount(1);
  await expect(amounts.first()).toHaveValue("119000");

  // Abono de 80.000: 50.000 en efectivo + 30.000 por transferencia.
  await amounts.first().fill("50000");
  await payment.getByRole("button", { name: "Agregar método" }).click();
  await expect(amounts).toHaveCount(2);
  await amounts.nth(1).fill("30000");
  await payment.getByRole("combobox").nth(1).click();
  await page.getByRole("option", { name: "Transferencia" }).click();
  await expect(payment.getByPlaceholder("Referencia / aprobación")).toBeVisible();

  // Saldo después del pago: 119.000 - 80.000 = 39.000.
  await expect(payment.getByText(/39\.000/).first()).toBeVisible();
  await payment.getByRole("button", { name: "Registrar pago" }).click();

  await expect(page.getByText(/Pago REC-000001 registrado/)).toBeVisible();
  await expect(page.getByText(/Saldo pendiente de la factura/)).toBeVisible();
  await expect(row).toContainText("Abonada");
  await expect(row).toContainText("80.000");
  await expect(row).toContainText("39.000");

  // ---------- 3. Segundo pago: completa el saldo ----------
  await row.getByRole("button", { name: "Cobrar" }).click();
  const second = page.getByRole("dialog");
  await expect(second.getByRole("heading", { name: "Registrar pago" })).toBeVisible();
  const secondAmount = second.locator('input[inputmode="decimal"]');
  await expect(secondAmount.first()).toHaveValue("39000");
  await second.getByRole("button", { name: "Registrar pago" }).click();

  await expect(page.getByText(/Pago REC-000002 registrado/)).toBeVisible();
  await expect(page.getByText("Factura pagada en su totalidad.")).toBeVisible();
  await expect(row).toContainText("Pagada");
  await expect(row).toContainText("119.000");

  // ---------- 4. Recibo de la factura ----------
  await gotoView(page, "Administración", "Reportes PDF");
  await expect(page.getByRole("heading", { name: "Reportes PDF" })).toBeVisible();
  await page.getByRole("button", { name: "Generar informe" }).click();
  const reportDialog = page.getByRole("dialog");
  await expect(
    reportDialog.getByRole("heading", { name: "Generar reporte PDF" }),
  ).toBeVisible();
  await reportDialog.getByRole("tab", { name: "Recibo" }).click();
  await reportDialog
    .getByRole("button")
    .filter({ hasText: "FAC-000001" })
    .first()
    .click();
  await reportDialog.getByRole("button", { name: "Generar recibo" }).click();
  await expect(
    page.getByText("Informe ISALAB_Recibo_FAC-000001.pdf generado"),
  ).toBeVisible();
  // El diálogo se cierra solo al generar el informe.
  await expect(page.getByRole("dialog")).toHaveCount(0);

  // ---------- 5. Gasto de caja ----------
  await gotoView(page, "Administración", "Caja");
  await expect(page.getByText("CAJ-00000001")).toBeVisible();
  // Efectivo esperado: 100.000 + 50.000 + 39.000 = 189.000.
  await expect(page.getByText(/189\.000/).first()).toBeVisible();

  await page.getByRole("button", { name: "Gasto" }).click();
  const movement = page.getByRole("dialog");
  await expect(
    movement.getByRole("heading", { name: "Movimiento de caja" }),
  ).toBeVisible();
  await movement.getByLabel("Importe").fill("5000");
  await movement.getByLabel("Motivo").fill("Insumos de laboratorio");
  await movement.getByRole("button", { name: "Registrar" }).click();

  await expect(page.getByText("Movimiento registrado")).toBeVisible();
  const movementRow = page.getByRole("row").filter({ hasText: "Insumos de laboratorio" });
  await expect(movementRow).toContainText("Gasto");
  await expect(movementRow).toContainText("5.000");
  // Gastos y retiros y efectivo esperado tras el egreso.
  await expect(page.getByText(/184\.000/).first()).toBeVisible();
  await expect(page.getByText("2", { exact: true }).first()).toBeVisible();

  // ---------- 6. Cierre con diferencia ----------
  await page.getByRole("button", { name: "Cerrar caja" }).click();
  const close = page.getByRole("dialog");
  await expect(close.getByRole("heading", { name: "Cerrar caja" })).toBeVisible();
  await expect(close.getByText(/184\.000/).first()).toBeVisible();

  // Arqueo de 180.000: 100.000 + 50.000 + 20.000 + 10.000 → faltan 4.000.
  const quantities = close.locator('input[inputmode="numeric"]');
  await quantities.nth(0).fill("1"); // 100.000
  await quantities.nth(1).fill("1"); // 50.000
  await quantities.nth(2).fill("1"); // 20.000
  await quantities.nth(3).fill("1"); // 10.000
  await expect(close.getByText(/180\.000/).first()).toBeVisible();
  await expect(close.getByText(/-\$\s*4\.000/).first()).toBeVisible();

  await close.getByRole("button", { name: "Cerrar caja" }).click();
  await expect(page.getByText(/Caja CAJ-00000001 cerrada/)).toBeVisible();
  await expect(page.getByText(/Diferencia de/)).toBeVisible();

  // La caja vuelve a estar cerrada y el cierre queda en el historial.
  await expect(page.getByRole("button", { name: "Abrir caja" })).toBeVisible();
  const history = page.getByRole("row").filter({ hasText: "CAJ-00000001" });
  await expect(history).toContainText("180.000");
  await expect(history).toContainText("184.000");
  await expect(history.getByText(/-\$\s*4\.000/)).toBeVisible();
  await expect(history).toContainText("Cerrada");
});

test("anular un pago revierte la caja y la factura; con pagos vigentes no se anula", async ({
  page,
}) => {
  await login(page);

  // ---------- 1. Caja abierta y abono parcial de 50.000 en efectivo ----------
  await openCash(page, "100000");
  const row = await invoiceRow(page, "FAC-000001");
  await row.getByRole("button", { name: "Cobrar" }).click();
  const firstPayment = page.getByRole("dialog");
  await expect(
    firstPayment.getByRole("heading", { name: "Registrar pago" }),
  ).toBeVisible();
  await firstPayment.locator('input[inputmode="decimal"]').first().fill("50000");
  await firstPayment.getByRole("button", { name: "Registrar pago" }).click();
  await expect(page.getByText(/Pago REC-000001 registrado/)).toBeVisible();
  await expect(row).toContainText("Abonada");

  // ---------- 2. Con pagos vigentes la factura no se anula ----------
  await row.getByText("FAC-000001").click();
  const detail = page.getByRole("dialog");
  await expect(
    detail.getByRole("heading", { name: "Factura FAC-000001" }),
  ).toBeVisible();
  // El «Anular» del pie es el de la factura; el del pago vive en su fila.
  const footer = detail.locator('[data-slot="dialog-footer"]');
  await expect(footer.getByRole("button", { name: "Anular" })).toHaveCount(0);
  await expect(
    detail.getByText(
      "Una factura con pagos activos no puede anularse; primero anula los pagos.",
    ),
  ).toBeVisible();
  await expect(detail.getByText("Abonada", { exact: true })).toBeVisible();

  // ---------- 3. Anulación del pago: motivo obligatorio y reverso ----------
  // El único botón «Anular» del diálogo es el de la fila del pago.
  await detail.getByRole("button", { name: "Anular" }).click();
  await detail.getByPlaceholder("Motivo de la anulación").fill("Pago duplicado");
  await detail.getByRole("button", { name: "Confirmar", exact: true }).click();
  await expect(page.getByText("Pago anulado", { exact: true })).toBeVisible();

  // El pago queda como rastro anulado y la factura recupera saldo y estado.
  await expect(detail.getByText("Anulado", { exact: true })).toBeVisible();
  await expect(detail.getByText("Pendiente", { exact: true })).toBeVisible();
  await expect(detail.getByText(/119\.000/).first()).toBeVisible();
  await expect(footer.getByRole("button", { name: "Anular" })).toBeVisible();

  // ---------- 4. Liberada de pagos, la factura sí se anula ----------
  await footer.getByRole("button", { name: "Anular" }).click();
  await expect(page.getByText("Factura anulada", { exact: true })).toBeVisible();
  await expect(detail.getByText("Anulada", { exact: true })).toBeVisible();
  await closeDialogs(page);
  await expect(row).toContainText("Anulada");

  // ---------- 5. Reverso en caja: el efectivo vuelve a la base ----------
  await gotoView(page, "Administración", "Caja");
  await expect(page.getByText("CAJ-00000001")).toBeVisible();
  const expectedCard = page
    .locator("[data-slot='card']")
    .filter({ hasText: "Efectivo esperado" })
    .first();
  await expect(expectedCard).toContainText("100.000");
  const reversal = page
    .getByRole("row")
    .filter({ hasText: "Anulación REC-000001" });
  await expect(reversal).toContainText("Devolución");
  await expect(reversal).toContainText("Pago duplicado");
  await expect(reversal.getByText(/-\$\s*50\.000/)).toBeVisible();

  // ---------- 6. Auditoría: pago anulado y factura anulada ----------
  await gotoView(page, "Administración", "Auditoría");
  await expect(
    page.getByRole("heading", { name: /Registro de Auditoría/ }),
  ).toBeVisible();
  const voidedPayment = page
    .getByRole("row")
    .filter({ hasText: "Pago REC-000001 anulado" });
  await expect(voidedPayment).toContainText("Pago anulado");
  await expect(voidedPayment).toContainText("Pago duplicado");
  const registeredPayment = page
    .getByRole("row")
    .filter({ hasText: "aplicado a 1 factura" });
  await expect(registeredPayment).toContainText("Pago registrado");
  const voidedInvoice = page
    .getByRole("row")
    .filter({ hasText: "Factura 1 → estado ANULADA" });
  await expect(voidedInvoice).toContainText("Estado factura");
});
