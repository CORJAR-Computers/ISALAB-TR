import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { installIpcMock } from "./mock";

/**
 * Reglas de dinero que NO son el camino feliz: cobrar sin caja, abonar más que
 * el saldo, anular un pago cuya caja ya se cerró y anular dos veces la misma
 * factura.
 *
 * Se comprueban las dos capas a propósito:
 *   1. La UI avisa antes de enviar (banner, importe bloqueado, toast), para que
 *      el cajero no descubra la regla con un error genérico.
 *   2. El backend del mock rechaza igual (Validation) y no deja rastro: ni un
 *      pago huérfano ni un segundo asiento de auditoría. Si la UI dejara pasar
 *      algo, el rechazo de abajo lo delata.
 */

const VIEW = { width: 1440, height: 900 };

test.beforeEach(async ({ page }) => {
  await installIpcMock(page);
  await page.setViewportSize(VIEW);
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

async function gotoView(page: Page, section: string, item: string) {
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
}

async function invoiceRow(page: Page, number: string) {
  await gotoView(page, "Administración", "Facturación");
  await expect(
    page.getByRole("heading", { name: "Facturación" }).first(),
  ).toBeVisible();
  const row = page.getByRole("row").filter({ hasText: number });
  await expect(row).toBeVisible();
  return row;
}

/** Payload mínimo del cobro; la factura sembrada es FAC-000001 (119.000). */
function paymentPayload(amount: number) {
  return {
    ownerId: 10,
    patientId: 1,
    notes: null,
    lines: [{ methodCode: "EFECTIVO", amount, reference: null }],
    allocations: [{ invoiceId: 1, amount }],
  };
}

/** Ejecuta un comando contra el mock: es el "backend" de los E2E. */
async function invoke(
  page: Page,
  cmd: string,
  args: Record<string, unknown> = {},
) {
  return page.evaluate(
    async ({ cmd, args }) => {
      const internals = (
        window as unknown as {
          __TAURI_INTERNALS__?: {
            invoke: (c: string, a?: unknown) => Promise<unknown>;
          };
        }
      ).__TAURI_INTERNALS__;
      if (!internals) throw new Error("El mock de IPC no está inyectado");
      try {
        const data = await internals.invoke(cmd, args);
        return { ok: true as const, data: data === undefined ? null : data };
      } catch (error) {
        return {
          ok: false as const,
          error: error as { type?: string; data?: string } | undefined,
        };
      }
    },
    { cmd, args },
  );
}

test("cobrar sin caja abierta: no se registra nada y el diálogo permite abrirla", async ({
  page,
}) => {
  await login(page);
  const row = await invoiceRow(page, "FAC-000001");
  await row.getByRole("button", { name: "Cobrar" }).click();

  const dialog = page.getByRole("dialog");
  await expect(
    dialog.getByRole("heading", { name: "Registrar pago" }),
  ).toBeVisible();
  // El aviso llega antes del intento, con la acción para resolverlo al lado.
  await expect(dialog.getByText("No hay una caja abierta")).toBeVisible();
  const submit = dialog.getByRole("button", { name: "Registrar pago" });
  await expect(submit).toBeDisabled();

  // Y el backend tampoco lo acepta: no hay pago huérfano fuera de una caja.
  const direct = await invoke(page, "register_payment", {
    input: paymentPayload(1000),
  });
  expect(direct).toMatchObject({
    ok: false,
    error: { type: "Validation", data: "Abre una caja antes de cobrar" },
  });
  expect(
    await invoke(page, "list_invoice_payments", { invoiceId: 1 }),
  ).toMatchObject({ ok: true, data: [] });

  // Abrir la caja desde el propio diálogo desbloquea el cobro sin reabrirlo.
  await dialog.getByRole("button", { name: "Abrir caja" }).click();
  await expect(page.getByText("Caja abierta", { exact: true })).toBeVisible();
  await expect(dialog.getByText("No hay una caja abierta")).toHaveCount(0);
  await expect(submit).toBeEnabled();
});

test("abono mayor al saldo: la UI bloquea el envío y el backend lo rechaza sin registrar el pago", async ({
  page,
}) => {
  await login(page);
  await openCash(page, "100000");

  const row = await invoiceRow(page, "FAC-000001");
  await row.getByRole("button", { name: "Cobrar" }).click();
  const dialog = page.getByRole("dialog");
  await expect(
    dialog.getByRole("heading", { name: "Registrar pago" }),
  ).toBeVisible();

  // La factura debe 119.000: abonar 200.000 se explica y no se puede enviar.
  const amount = dialog.locator('input[inputmode="decimal"]').first();
  await amount.fill("200000");
  await expect(
    dialog.getByText(/La suma supera el saldo de la factura/),
  ).toBeVisible();
  const submit = dialog.getByRole("button", { name: "Registrar pago" });
  await expect(submit).toBeDisabled();

  const direct = await invoke(page, "register_payment", {
    input: paymentPayload(200000),
  });
  expect(direct).toMatchObject({
    ok: false,
    error: {
      type: "Validation",
      data: "El abono supera el saldo de FAC-000001",
    },
  });
  expect(
    await invoke(page, "list_invoice_payments", { invoiceId: 1 }),
  ).toMatchObject({ ok: true, data: [] });

  // Ajustado al saldo, el mismo formulario cobra y la factura queda saldada.
  await amount.fill("119000");
  await expect(submit).toBeEnabled();
  await submit.click();
  await expect(page.getByText(/Pago REC-000001 registrado/)).toBeVisible();
  await expect(row).toContainText("Pagada");
});

test("anular un pago con la caja ya cerrada: rechazo visible y el pago sigue aplicado", async ({
  page,
}) => {
  await login(page);
  await openCash(page, "100000");

  // Abono parcial en efectivo: el reverso tendría que salir de esta caja.
  const row = await invoiceRow(page, "FAC-000001");
  await row.getByRole("button", { name: "Cobrar" }).click();
  const payment = page.getByRole("dialog");
  await expect(
    payment.getByRole("heading", { name: "Registrar pago" }),
  ).toBeVisible();
  await payment.locator('input[inputmode="decimal"]').first().fill("50000");
  await payment.getByRole("button", { name: "Registrar pago" }).click();
  await expect(page.getByText(/Pago REC-000001 registrado/)).toBeVisible();
  await expect(row).toContainText("Abonada");

  // Se cierra la caja con un arqueo válido (una denominación de 100.000).
  await gotoView(page, "Administración", "Caja");
  await page.getByRole("button", { name: "Cerrar caja" }).click();
  const close = page.getByRole("dialog");
  await expect(close.getByRole("heading", { name: "Cerrar caja" })).toBeVisible();
  await close.locator('input[inputmode="numeric"]').first().fill("1");
  await close.getByRole("button", { name: "Cerrar caja" }).click();
  await expect(page.getByText(/Caja CAJ-00000001 cerrada/)).toBeVisible();

  // Con la caja cerrada, la anulación se rechaza con el motivo real.
  const invoice = await invoiceRow(page, "FAC-000001");
  await expect(invoice).toContainText("Abonada");
  await invoice.getByText("FAC-000001").click();
  const detail = page.getByRole("dialog");
  await expect(
    detail.getByRole("heading", { name: "Factura FAC-000001" }),
  ).toBeVisible();
  await detail.getByRole("button", { name: "Anular" }).click();
  await detail.getByPlaceholder("Motivo de la anulación").fill("Pago duplicado");
  await detail.getByRole("button", { name: "Confirmar", exact: true }).click();

  const failure = page
    .locator("[data-sonner-toast]")
    .filter({ hasText: "No se pudo anular el pago" });
  await expect(failure).toBeVisible();
  await expect(failure).toContainText("La caja del pago ya está cerrada");
  // Nada cambió: ni el pago se anuló ni la factura perdió el abono.
  await expect(page.getByText("Pago anulado", { exact: true })).toHaveCount(0);
  await expect(detail.getByText("Abonada", { exact: true })).toBeVisible();
});

test("anular dos veces la misma factura: la segunda se rechaza y la auditoría registra un solo asiento", async ({
  page,
}) => {
  await login(page);
  const row = await invoiceRow(page, "FAC-000001");
  await row.getByText("FAC-000001").click();
  const detail = page.getByRole("dialog");
  await expect(
    detail.getByRole("heading", { name: "Factura FAC-000001" }),
  ).toBeVisible();

  const footer = detail.locator('[data-slot="dialog-footer"]');
  await footer.getByRole("button", { name: "Anular" }).click();
  await expect(page.getByText("Factura anulada", { exact: true })).toBeVisible();
  await expect(detail.getByText("Anulada", { exact: true })).toBeVisible();
  // La UI ya no ofrece la acción; el backend es el que tiene que rechazarla.
  await expect(footer.getByRole("button", { name: "Anular" })).toHaveCount(0);

  expect(
    await invoke(page, "set_invoice_status", { id: 1, status: "ANULADA" }),
  ).toMatchObject({
    ok: false,
    error: { type: "Validation", data: "La factura ya está anulada" },
  });

  const audit = await invoke(page, "list_audit_log", {
    limit: null,
    offset: null,
    username: null,
    action: "INVOICE_STATUS_CHANGE",
    dateFrom: null,
    dateTo: null,
  });
  // Un solo asiento: el intento repetido no escribió auditoría.
  expect(audit).toMatchObject({
    ok: true,
    data: [{ action: "INVOICE_STATUS_CHANGE" }],
  });
});
