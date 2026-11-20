import { test, expect, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";
import { entityCode, entityCodePattern } from "./dates";

/**
 * Flujo de recepción: check-in de una visita con servicios del catálogo,
 * atención, finalización y cobro desde el checkout (que emite la factura y
 * abre el registro de pago).
 *
 * El mock de IPC reproduce las reglas del backend: la visita congela precios
 * del catálogo, solo se cobra FINALIZADA y el cobro exige una caja abierta.
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
 * Cierra la pila de diálogos: Escape cierra el superior y, si ese diálogo no
 * reacciona (o la capa está en animación de salida y el clic no llega), su X.
 * Se espera al desmontaje con aserciones que reintentan, nunca con una pausa
 * fija (`playwright/no-wait-for-timeout` lo prohíbe).
 */
async function closeDialogs(page: Page) {
  const dialogs = page.getByRole("dialog");
  for (let i = 0; i < 6 && (await dialogs.count()) > 0; i += 1) {
    const open = await dialogs.count();
    await page.keyboard.press("Escape");
    await expect
      .poll(() => dialogs.count(), { timeout: 3_000 })
      .toBeLessThan(open)
      .catch(async () => {
        const close = dialogs.last().locator('[data-slot="dialog-close"]');
        if ((await close.count()) > 0) {
          await close.first().click({ timeout: 2_000 });
        }
      });
  }
  await expect(dialogs).toHaveCount(0);
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

test("check-in de visita y cobro desde el checkout", async ({ page }) => {
  await login(page);

  // ---------- 1. Check-in ----------
  await gotoView(page, "Operación", "Recepción de pacientes");
  await expect(
    page.getByRole("heading", { name: "Recepción de pacientes" }),
  ).toBeVisible();
  await expect(page.getByText(/No hay visitas con estos filtros/)).toBeVisible();

  await page.getByRole("button", { name: "Registrar visita" }).click();
  const newVisit = page.getByRole("dialog");
  await expect(newVisit.getByRole("heading", { name: "Nueva visita" })).toBeVisible();

  // Paciente del directorio (Rocky, de Juan Pérez).
  await newVisit.getByPlaceholder(/Buscar paciente/).fill("Rocky");
  await newVisit.getByRole("combobox").first().click();
  await page.getByRole("option", { name: /Rocky/ }).click();

  // Servicio del catálogo con precio congelado (Consulta general: 60.000).
  await newVisit.getByRole("combobox").nth(1).click();
  await page.getByRole("option", { name: /Consulta general/ }).click();
  await expect(newVisit.getByText("Total estimado")).toBeVisible();
  // El importe aparece en la línea seleccionada y en el total estimado.
  await expect(newVisit.getByText(/60\.000/).last()).toBeVisible();

  await newVisit.getByRole("button", { name: "Registrar visita" }).click();
  await expect(page.getByText("Visita registrada")).toBeVisible();
  await expect(page.getByText(entityCodePattern("V", 1)).first()).toBeVisible();

  // ---------- 2. La visita aparece en la cola de espera ----------
  const card = page
    .getByRole("button")
    .filter({ hasText: entityCode("V", 1) })
    .first();
  await expect(card).toContainText("Rocky");
  await expect(card).toContainText("En espera");
  await expect(card).toContainText("Consulta general");
  await card.click();

  // ---------- 3. Atención y finalización ----------
  const detail = page.getByRole("dialog");
  await expect(
    detail.getByRole("heading", { name: entityCodePattern("V", 1) }),
  ).toBeVisible();
  await detail.getByRole("button", { name: "Iniciar atención" }).click();
  await expect(page.getByText("Visita en atención")).toBeVisible();
  await expect(detail.getByText("En atención", { exact: true })).toBeVisible();

  await detail.getByRole("button", { name: "Finalizar visita" }).click();
  await expect(page.getByText("Visita finalizada")).toBeVisible();

  // ---------- 4. Cobro desde el checkout ----------
  const checkoutButton = detail.getByRole("button", { name: /^Cobrar/ });
  await expect(checkoutButton).toBeVisible();
  await expect(checkoutButton).toContainText("60.000");
  await checkoutButton.click();

  const checkout = page.getByRole("dialog");
  await expect(checkout.getByRole("heading", { name: "Cobrar visita" })).toBeVisible();
  await expect(checkout.getByText(/60\.000/).first()).toBeVisible();
  await checkout.getByRole("button", { name: "Emitir factura" }).click();

  // El cobro exige caja abierta: el diálogo de pago la abre en línea.
  const payment = page.getByRole("dialog");
  await expect(
    payment.getByRole("heading", { name: "Registrar pago" }),
  ).toBeVisible();
  await expect(payment.getByText("No hay una caja abierta")).toBeVisible();
  await payment.getByLabel("Base inicial").fill("20000");
  await payment.getByRole("button", { name: "Abrir caja" }).click();
  await expect(page.getByText("Caja abierta")).toBeVisible();

  // Factura de la visita: 60.000 + 19% IVA = 71.400, precargada completa.
  const amount = payment.locator('input[inputmode="decimal"]');
  await expect(amount.first()).toHaveValue("71400");
  await payment.getByRole("button", { name: "Registrar pago" }).click();

  await expect(page.getByText(/Pago REC-000001 registrado/)).toBeVisible();
  await expect(page.getByText("Factura pagada en su totalidad.")).toBeVisible();

  // ---------- 5. La visita queda facturada y la factura pagada ----------
  await expect(
    page.getByText(/Visita cobrada: los pagos y la factura se gestionan/),
  ).toBeVisible();
  await closeDialogs(page);

  await gotoView(page, "Administración", "Facturación");
  await expect(page.getByRole("heading", { name: "Facturación" })).toBeVisible();
  const invoiceRow = page.getByRole("row").filter({ hasText: "FAC-000002" });
  await expect(invoiceRow).toContainText("71.400");
  await expect(invoiceRow).toContainText("Pagada");
  await expect(invoiceRow).toContainText("0");

  // El pago quedó dentro de la sesión de caja abierta desde el checkout.
  await gotoView(page, "Administración", "Caja");
  await expect(page.getByText("CAJ-00000001")).toBeVisible();
  await expect(page.getByText(/71\.400/).first()).toBeVisible();
  await expect(page.getByText("Recibos")).toBeVisible();
});
