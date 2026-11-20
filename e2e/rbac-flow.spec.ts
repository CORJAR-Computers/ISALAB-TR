import { test, expect, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";
import { gotoView } from "./navigation";

/**
 * RBAC de extremo a extremo: el mock de IPC responde `login` con el rol real
 * del usuario (espejo de USERS), la UI filtra menú y botones con
 * `usePermissions` y los handlers del mock exigen las mismas capacidades que
 * los `require_*` de src-tauri/src/auth.rs.
 *
 * Se cubren las tres capas por rol:
 *   1. Menú: entradas y secciones del sidebar visibles.
 *   2. Botones: acciones de página que dependen de una capacidad.
 *   3. Backend: `invoke` directo para comprobar el 403 de lo no permitido y
 *      que lo permitido no se rechaza (la paridad de la matriz de
 *      capacidades la cubre src/hooks/use-permissions.test.ts).
 */
test.beforeEach(async ({ page }) => {
  await page.addInitScript({
    path: fileURLToPath(new URL("./ipc-mock.script.js", import.meta.url)),
  });
});

type Role = "ADMIN" | "VETERINARIO" | "AUXILIAR" | "RECEPCION" | "PELUQUERO";

const USERS: Record<Role, { username: string; password: string; fullName: string }> = {
  ADMIN: { username: "admin", password: "admin123", fullName: "Administrador" },
  VETERINARIO: { username: "mv.perez", password: "vet123", fullName: "Dra. Ana Pérez" },
  AUXILIAR: { username: "auxiliar", password: "auxiliar123", fullName: "Carlos Ruiz" },
  RECEPCION: { username: "recepcion", password: "recepcion123", fullName: "Laura Gómez" },
  PELUQUERO: { username: "peluquero", password: "peluquero123", fullName: "Diego Santos" },
};

/** Entrada del menú → sección del sidebar que la contiene. */
const ITEMS: Array<{ section: string; label: string }> = [
  { section: "Operación", label: "Panel de control" },
  { section: "Operación", label: "Recepción de pacientes" },
  { section: "Operación", label: "Agenda de citas" },
  { section: "Operación", label: "Agenda de consultas" },
  { section: "Operación", label: "Pacientes" },
  { section: "Operación", label: "Historial Clínico" },
  { section: "Laboratorio", label: "Muestras & Laboratorio" },
  { section: "Laboratorio", label: "Bandeja de trabajo" },
  { section: "Laboratorio", label: "Órdenes de laboratorio" },
  { section: "Laboratorio", label: "Control de calidad (QC)" },
  { section: "Clínica", label: "Cirugías" },
  { section: "Clínica", label: "Vacunación" },
  { section: "Clínica", label: "Estancia y hospedaje" },
  { section: "Estética", label: "Peluquería" },
  { section: "Comercio", label: "Inventario y productos" },
  { section: "Administración", label: "Facturación" },
  { section: "Administración", label: "Caja" },
  { section: "Administración", label: "Reportes PDF" },
  { section: "Administración", label: "Catálogo de servicios" },
  { section: "Administración", label: "Usuarios" },
  { section: "Administración", label: "Auditoría" },
  { section: "Administración", label: "Configuración" },
];

/** Entradas que cada rol NO debe ver (el resto son visibles). */
const HIDDEN_BY_ROLE: Record<Role, string[]> = {
  ADMIN: [],
  VETERINARIO: [
    "Peluquería",
    "Inventario y productos",
    "Catálogo de servicios",
    "Usuarios",
    "Auditoría",
    "Configuración",
  ],
  AUXILIAR: ["Catálogo de servicios", "Usuarios", "Auditoría", "Configuración"],
  RECEPCION: [
    "Peluquería",
    "Catálogo de servicios",
    "Usuarios",
    "Auditoría",
    "Configuración",
  ],
  // Sin canReceive, canStay, canBill, canInventory ni adminOnly: solo
  // Operación/Laboratorio/Clínica generales y Peluquería.
  PELUQUERO: [
    "Recepción de pacientes",
    "Agenda de citas",
    "Estancia y hospedaje",
    "Inventario y productos",
    "Caja",
    "Catálogo de servicios",
    "Usuarios",
    "Auditoría",
    "Configuración",
  ],
};

async function login(page: Page, role: Role) {
  const user = USERS[role];
  await page.goto("/");
  await page.getByLabel("Usuario").fill(user.username);
  await page.getByLabel("Contraseña", { exact: true }).fill(user.password);
  await page.getByRole("button", { name: "Entrar" }).click();
  await expect(
    page.getByRole("heading", { name: "Panel de control" }).first(),
  ).toBeVisible();
}

/** Cabecera de sección del sidebar (existe solo si le quedan entradas). */
function sectionHeader(page: Page, section: string) {
  return page
    .locator("aside")
    .first()
    .getByRole("button", { name: section, exact: true });
}

/**
 * Número de veces que una entrada aparece en el menú. Expande su sección si
 * existe (las secciones arrancan colapsadas y las vacías no se renderizan) y
 * la vuelve a colapsar para dejar el sidebar como estaba.
 */
async function navItemCount(page: Page, section: string, label: string) {
  const header = sectionHeader(page, section);
  if ((await header.count()) === 0) return 0;
  if ((await header.getAttribute("aria-expanded")) !== "true") {
    await header.click();
  }
  const count = await page
    .locator("aside")
    .first()
    .locator(`nav button[title="${label}"]`)
    .count();
  await header.click();
  return count;
}

/** Ejecuta un comando IPC directamente: el mock es el "backend" de los E2E. */
async function invoke(page: Page, cmd: string, args: Record<string, unknown> = {}) {
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

for (const role of Object.keys(USERS) as Role[]) {
  test(`${role}: menú, botones y rechazos del backend`, async ({ page }) => {
    await login(page, role);

    // ---------- 1. Menú por rol ----------
    const hidden = new Set(HIDDEN_BY_ROLE[role]);
    for (const item of ITEMS) {
      const expected = hidden.has(item.label) ? 0 : 1;
      expect(
        await navItemCount(page, item.section, item.label),
        `${role} → "${item.label}"`,
      ).toBe(expected);
    }

    // Secciones que quedan sin entradas no se renderizan.
    const canInventory = role === "ADMIN" || role === "AUXILIAR" || role === "RECEPCION";
    const canGroom = role === "ADMIN" || role === "AUXILIAR" || role === "PELUQUERO";
    await expect(sectionHeader(page, "Comercio")).toHaveCount(
      canInventory ? 1 : 0,
    );
    await expect(sectionHeader(page, "Estética")).toHaveCount(canGroom ? 1 : 0);

    // ---------- 2. Botones que dependen de una capacidad ----------
    const canBill = role !== "PELUQUERO";
    const isVetOrAdmin = role === "ADMIN" || role === "VETERINARIO";

    await gotoView(page, "Facturación");
    await expect(
      page.getByRole("heading", { name: "Facturación" }).first(),
    ).toBeVisible();
    // toHaveCount reintenta: la página monta su cabecera después del TopBar.
    await expect(
      page.getByRole("button", { name: "Nueva factura" }),
      `${role} → "Nueva factura"`,
    ).toHaveCount(canBill ? 1 : 0);

    await gotoView(page, "Muestras & Laboratorio");
    await expect(
      page.getByRole("heading", { name: "Muestras & Laboratorio" }).first(),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Nueva toma de muestra" }),
      `${role} → "Nueva toma de muestra"`,
    ).toHaveCount(isVetOrAdmin ? 1 : 0);

    // ---------- 3. Backend: 403 de lo no permitido ----------
    // list_users / save_clinic_settings exigen ADMIN.
    const listUsers = await invoke(page, "list_users");
    if (role === "ADMIN") {
      expect(listUsers.ok).toBe(true);
    } else {
      expect(listUsers).toMatchObject({ ok: false, error: { type: "Forbidden" } });
    }

    const saveSettings = await invoke(page, "save_clinic_settings");
    if (role === "ADMIN") {
      expect(saveSettings.ok).toBe(true);
    } else {
      expect(saveSettings).toMatchObject({ ok: false, error: { type: "Forbidden" } });
    }

    // Receptor/cobro: create_visit exige canReceive.
    const createVisit = await invoke(page, "create_visit", {
      input: {
        patientId: 1,
        priority: "NORMAL",
        notes: null,
        services: [{ serviceId: 1, quantity: 1 }],
      },
    });
    if (role === "PELUQUERO") {
      expect(createVisit).toMatchObject({ ok: false, error: { type: "Forbidden" } });
    } else {
      expect(createVisit.ok, `${role} debería poder hacer check-in`).toBe(true);
    }

    // register_payment exige canBill; sin caja abierta lo permitido falla por
    // validación (no por permisos), que es justo lo que se comprueba.
    const registerPayment = await invoke(page, "register_payment", {
      input: {
        ownerId: 10,
        patientId: 1,
        notes: null,
        lines: [{ methodCode: "EFECTIVO", amount: 1000, reference: null }],
        allocations: [],
      },
    });
    if (role === "PELUQUERO") {
      expect(registerPayment).toMatchObject({
        ok: false,
        error: { type: "Forbidden" },
      });
    } else {
      expect(registerPayment.ok).toBe(false);
      expect((registerPayment as { error?: { type?: string } }).error?.type).not.toBe(
        "Forbidden",
      );
    }
  });
}
