import { test, expect, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";

/**
 * Test de viewport: portátiles de 14".
 *
 * - 1366x768 físicos a 125 %  ≈ 1093x614 px CSS → por encima del breakpoint
 *   `lg` (1024 px): la sidebar se muestra fija (inline) y el contenido usa
 *   `lg:pl-64`.
 * - 1366x768 físicos a 150 %  ≈ 911x472 px CSS → por debajo de `lg`: la
 *   sidebar pasa a off-canvas y la navegación es vía botón hamburguesa.
 *
 * La ventana nativa ya se recorta al área de trabajo en Rust (lib.rs); aquí
 * se valida que la UI web es completamente usable en ambos regímenes: sin
 * desbordamientos horizontales del documento, barras de acciones accesibles,
 * diálogos sin recortes y todas las páginas montando sin excepciones.
 */

const W_LG = 1093; // 1366 / 1.25
const H_LG = 614; // 768 / 1.25 - barra de tareas
const W_MD = 911; // 1366 / 1.5
const H_MD = 576; // 768 / 1.5 - barra de tareas

test.beforeEach(async ({ page }) => {
  await page.addInitScript({
    path: fileURLToPath(new URL("./ipc-mock.script.js", import.meta.url)),
  });
});

async function login(page: Page, width: number, height: number) {
  await page.setViewportSize({ width, height });
  await page.goto("/");
  await expect(page.getByLabel("Usuario")).toBeVisible();
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  await expect(
    page.getByRole("heading", { name: "Panel de control" }).first(),
  ).toBeVisible();
}

/** Ir a una vista evitando la colisión sidebar/TopBar ("Configuración"). */
async function gotoView(page: Page, nav: string) {
  await page
    .locator("aside")
    .getByRole("button", { name: nav, exact: true })
    .click();
}

/** La página no debe desbordar el documento horizontalmente. */
async function expectNoHorizontalOverflow(page: Page, width: number) {
  await expect
    .poll(
      async () =>
        page.evaluate(() => document.documentElement.scrollWidth),
      { timeout: 5_000 },
    )
    .toBeLessThanOrEqual(width);
}


// ---------------------------------------------------------------------------
// Régimen 125 % (1093x614): sidebar inline
// ---------------------------------------------------------------------------

test.describe(`viewport 1093x614 (14" a 125 %)`, () => {
  test("login usable y sin overflow", async ({ page }) => {
    await page.setViewportSize({ width: W_LG, height: H_LG });
    await page.goto("/");
    await expect(page.getByLabel("Usuario")).toBeVisible();
    await expectNoHorizontalOverflow(page, W_LG);
    await page.getByLabel("Usuario").fill("admin");
    await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
    await page.getByRole("button", { name: "Entrar" }).click();
    await expect(
      page.getByRole("heading", { name: "Panel de control" }).first(),
    ).toBeVisible();
  });

  test("sidebar inline navega sin hamburguesa ni overflow", async ({ page }) => {
    await login(page, W_LG, H_LG);
    // > lg: la sidebar es visible directamente (sin botón hamburguesa).
    await expect(page.getByRole("button", { name: "Abrir menú" })).toHaveCount(0);
    await gotoView(page, "Muestras & Laboratorio");
    await expect(
      page.getByRole("heading", { name: "Muestras & Laboratorio" }).first(),
    ).toBeVisible();
    await expectNoHorizontalOverflow(page, W_LG);
  });

  test("barra de acciones de Muestras accesible sin scroll", async ({ page }) => {
    await login(page, W_LG, H_LG);
    await gotoView(page, "Muestras & Laboratorio");
    const newSample = page.getByRole("button", {
      name: "Nueva toma de muestra",
    });
    await expect(newSample).toBeVisible();
    const inViewport = await newSample.evaluate(
      (el) =>
        el.getBoundingClientRect().top >= 0 &&
        el.getBoundingClientRect().bottom <= window.innerHeight,
    );
    expect(inViewport).toBe(true);
    await expectNoHorizontalOverflow(page, W_LG);
  });

  test("diálogo de nueva muestra cabe con scroll interno", async ({ page }) => {
    await login(page, W_LG, H_LG);
    await gotoView(page, "Muestras & Laboratorio");
    await page.getByRole("button", { name: "Nueva toma de muestra" }).click();

    const dialog = page.getByRole("dialog");
    await expect(
      dialog.getByRole("heading", { name: "Nueva toma de muestra" }),
    ).toBeVisible();

    const fits = await dialog.evaluate((el) => {
      const rect = el.getBoundingClientRect();
      return (
        rect.top >= 0 &&
        rect.bottom <= window.innerHeight + 1 &&
        rect.height <= window.innerHeight
      );
    });
    expect(fits).toBe(true);

    const registrar = dialog.getByRole("button", { name: "Registrar muestra" });
    await registrar.scrollIntoViewIfNeeded();
    await expect(registrar).toBeVisible();
    await expectNoHorizontalOverflow(page, W_LG);
    await page.keyboard.press("Escape");
  });

  test("tabla de muestras con scroll interno propio", async ({ page }) => {
    await login(page, W_LG, H_LG);
    await gotoView(page, "Muestras & Laboratorio");
    await expect(
      page.getByText("No hay muestras registradas."),
    ).toBeVisible();
    await expectNoHorizontalOverflow(page, W_LG);
  });

  test("flujo mínimo de muestra completo", async ({ page }) => {
    await login(page, W_LG, H_LG);
    await gotoView(page, "Muestras & Laboratorio");
    await page.getByRole("button", { name: "Nueva toma de muestra" }).click();

    const dialog = page.getByRole("dialog");
    await dialog.getByPlaceholder(/Buscar paciente/).fill("Rocky");
    await dialog.locator("button").filter({ hasText: "Rocky" }).first().click();
    await dialog.getByRole("combobox", { name: "Tipo de muestra" }).click();
    await page.getByRole("option", { name: "Suero" }).click();
    await dialog.getByRole("button", { name: "Registrar muestra" }).click();

    await expect(
      page.getByRole("dialog").getByRole("heading", {
        name: /Muestra M-2026-0001 registrada/,
      }),
    ).toBeVisible();
  });

  test("recorrido por todas las páginas sin errores ni overflow", async ({
    page,
  }) => {
    const pageErrors: string[] = [];
    page.on("pageerror", (err) =>
      pageErrors.push(err.stack ?? String(err)),
    );

    await login(page, W_LG, H_LG);
    await expectNoHorizontalOverflow(page, W_LG);

    const pages: Array<{ nav: string; heading: RegExp }> = [
      { nav: "Pacientes", heading: /^Pacientes$/ },
      { nav: "Historial Clínico", heading: /Historial Clínico/ },
      { nav: "Muestras & Laboratorio", heading: /Muestras & Laboratorio/ },
      { nav: "Bandeja de trabajo", heading: /Bandeja de trabajo/ },
      { nav: "Órdenes de laboratorio", heading: /Órdenes de laboratorio/ },
      { nav: "Cirugías", heading: /Cirugías/ },
      { nav: "Vacunación", heading: /Vacunación/ },
      { nav: "Facturación", heading: /Facturación/ },
      { nav: "Reportes PDF", heading: /Reportes PDF/ },
      { nav: "Control de calidad (QC)", heading: /Control de calidad \(QC\)/ },
      { nav: "Usuarios", heading: /Usuarios/ },
      { nav: "Auditoría", heading: /Registro de Auditoría/ },
      { nav: "Configuración", heading: /Configuración/ },
      { nav: "Panel de control", heading: /Panel de control/ },
    ];

    for (const { nav, heading } of pages) {
      await gotoView(page, nav);
      await expect(
        page.getByRole("heading", { name: heading }).first(),
      ).toBeVisible({ timeout: 10_000 });
      await expectNoHorizontalOverflow(page, W_LG);
    }

    expect(pageErrors).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Régimen 150 % (911x576): sidebar off-canvas con hamburguesa
// ---------------------------------------------------------------------------

test.describe(`viewport 911x576 (14" a 150 %)`, () => {
  test("sidebar off-canvas: hamburguesa abre, navega y cierra", async ({
    page,
  }) => {
    await login(page, W_MD, H_MD);
    await expectNoHorizontalOverflow(page, W_MD);

    const hamburger = page.getByRole("button", { name: "Abrir menú" });
    await expect(hamburger).toBeVisible();

    await hamburger.click();
    await gotoView(page, "Muestras & Laboratorio");
    await expect(
      page.getByRole("heading", { name: "Muestras & Laboratorio" }).first(),
    ).toBeVisible();
    await expectNoHorizontalOverflow(page, W_MD);

    // Reabrir y cerrar con la X de la sidebar. (La sidebar se oculta con
    // translate-x-full, no con display:none: la comprobación es geométrica.)
    await hamburger.click();
    await page.getByRole("button", { name: "Cerrar menú" }).click();
    await expect
      .poll(async () =>
        page
          .locator("aside")
          .evaluate((el) => el.getBoundingClientRect().left),
      { timeout: 5_000 })
      .toBeLessThanOrEqual(0);
  });

  test("diálogo de nueva muestra cabe con scroll interno", async ({ page }) => {
    await login(page, W_MD, H_MD);
    await page.getByRole("button", { name: "Abrir menú" }).click();
    await gotoView(page, "Muestras & Laboratorio");
    await page.getByRole("button", { name: "Nueva toma de muestra" }).click();

    const dialog = page.getByRole("dialog");
    await expect(
      dialog.getByRole("heading", { name: "Nueva toma de muestra" }),
    ).toBeVisible();

    const fits = await dialog.evaluate((el) => {
      const rect = el.getBoundingClientRect();
      return (
        rect.top >= 0 &&
        rect.bottom <= window.innerHeight + 1 &&
        rect.height <= window.innerHeight
      );
    });
    expect(fits).toBe(true);
    await expectNoHorizontalOverflow(page, W_MD);
    await page.keyboard.press("Escape");
  });
});
