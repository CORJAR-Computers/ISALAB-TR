import { expect, type Page } from "@playwright/test";
import { installIpcMock } from "./mock";
import { gotoView } from "./navigation";

/** Fecha local de hoy: la agenda abre en la semana que la contiene. */
export function todayISO(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Login, navegación a la agenda y espera a que la rejilla resuelva su consulta. */
export async function openAgenda(page: Page) {
  await installIpcMock(page);
  await page.goto("/");
  await page.getByLabel("Usuario").fill("admin");
  await page.getByLabel("Contraseña", { exact: true }).fill("admin123");
  await page.getByRole("button", { name: "Entrar" }).click();
  await gotoView(page, "Agenda de citas");
  await expect(
    page.getByRole("heading", { name: "Agenda de citas" }).first(),
  ).toBeVisible();
  // La rejilla lleva `aria-busy` mientras la consulta está pendiente.
  await expect(page.locator("[aria-busy]")).toHaveAttribute(
    "aria-busy",
    "false",
  );
}
