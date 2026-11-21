import AxeBuilder from "@axe-core/playwright";
import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";
import { installIpcMock } from "./mock";
import { gotoView } from "./navigation";

/**
 * Accesibilidad de los flujos que se usan todo el día.
 *
 * La app se opera con teclado en el mostrador y ya hubo una regresión real de
 * interacción (modales apilados atrapando el puntero). Aquí se revisa lo
 * sistemático con axe: contraste, nombres accesibles, landmarks y formularios.
 *
 * Solo fallan las violaciones `critical` y `serious`; las de `moderate`/`minor`
 * se vigilan pero no bloquean, porque suelen depender de la versión de axe.
 */

const BLOCKING = ["critical", "serious"];

/**
 * Reglas excluidas con su motivo. Se documenta la exclusión en el propio test:
 * si alguna deja de aplicar, el allowlist se vacía, no se amplía.
 */
const ALLOWLIST: Array<{ id: string; reason: string }> = [];

/** Analiza la pantalla actual y falla con el detalle de cada violación. */
async function expectNoBlockingViolations(page: Page, where: string) {
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .disableRules(ALLOWLIST.map((rule) => rule.id))
    .analyze();

  const blocking = results.violations.filter((violation) =>
    BLOCKING.includes(violation.impact ?? ""),
  );

  expect(
    blocking.map((violation) => ({
      id: violation.id,
      impact: violation.impact,
      help: violation.help,
      targets: violation.nodes.map((node) => node.target.join(" ")),
      // El HTML del nodo evita tener que adivinar qué control falla.
      html: violation.nodes.map((node) => node.html),
      // axe incluye aquí el ratio y los colores calculados del contraste.
      detail: violation.nodes.map((node) =>
        node.any
          .map((check) => check.message ?? "")
          .filter(Boolean)
          .join(" | "),
      ),
    })),
    `${where}: violaciones ${BLOCKING.join("/")} de accesibilidad`,
  ).toEqual([]);
}

async function login(page: Page) {
  await page.goto("/");
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  await expect(
    page.getByRole("heading", { name: "Panel de control" }).first(),
  ).toBeVisible();
}

test.beforeEach(async ({ page }) => {
  await installIpcMock(page);
});

test("login sin violaciones críticas", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByLabel("Usuario")).toBeVisible();
  await expectNoBlockingViolations(page, "login");
});

test("pantallas principales sin violaciones críticas", async ({ page }) => {
  await login(page);
  await expectNoBlockingViolations(page, "panel de control");

  for (const [item, heading] of [
    ["Recepción de pacientes", "Recepción de pacientes"],
    ["Facturación", "Facturación"],
    ["Caja", "Caja"],
  ] as const) {
    await gotoView(page, item);
    await expect(page.getByRole("heading", { name: heading }).first()).toBeVisible();
    await expectNoBlockingViolations(page, item);
  }
});

test("el formulario de cobro y el modal de factura son accesibles", async ({
  page,
}) => {
  await login(page);

  await gotoView(page, "Facturación");
  await page.getByRole("button", { name: "Nueva factura" }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await expectNoBlockingViolations(page, "nueva factura");

  // El modal atrapa el foco y Escape lo devuelve al botón que lo abrió.
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(
    page.getByRole("button", { name: "Nueva factura" }),
  ).toBeFocused();
});

test("los campos del cobro se recorren con teclado", async ({ page }) => {
  await login(page);

  await gotoView(page, "Caja");
  await expect(
    page.getByRole("heading", { name: "Caja" }).first(),
  ).toBeVisible();

  // La base inicial es el primer control del formulario de apertura: se llega
  // con Tab desde el propio campo y se puede escribir sin ratón.
  const float = page.getByLabel("Base inicial");
  await float.focus();
  // El campo arranca con "0": sin limpiarlo, escribir concatena (050000).
  await float.fill("");
  await page.keyboard.type("50000");
  await expect(float).toHaveValue("50000");

  // Con el foco en un control, Tab avanza al siguiente campo del formulario
  // (notas) y no se pierde fuera del formulario.
  await page.keyboard.press("Tab");
  await expect(page.getByLabel("Notas (opcional)")).toBeFocused();
});
