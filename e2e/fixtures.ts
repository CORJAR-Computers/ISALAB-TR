import { test as base, expect } from "@playwright/test";

/**
 * Base de los specs E2E.
 *
 * Además del `test` normal, al cerrar cada test comprueba que el mock no haya
 * devuelto ninguna respuesta cuyo contenido no cuadre con `src/bindings.ts`
 * (formas de `e2e/ipc-shapes.json`). Así una respuesta con un campo mal
 * nombrado falla en CI aunque la UI se limite a mostrar un estado de error.
 */
export const test = base.extend({
  page: async ({ page }, use) => {
    await use(page);
    if (page.isClosed()) return;
    const violations = await page
      .evaluate(
        () =>
          (window as unknown as { __E2E_IPC_SHAPE_ERRORS__?: string[] })
            .__E2E_IPC_SHAPE_ERRORS__ ?? [],
      )
      .catch(() => []);
    expect(
      violations,
      `El mock IPC devolvió datos que no coinciden con bindings.ts:\n${violations.join("\n\n")}`,
    ).toEqual([]);
  },
});

export { expect };
