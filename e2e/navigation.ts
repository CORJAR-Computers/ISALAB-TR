import { expect, type Page } from "@playwright/test";

/**
 * Sección del sidebar a la que pertenece cada entrada de navegación.
 *
 * El sidebar arranca con TODAS las secciones colapsadas (ni siquiera la de la
 * vista activa se abre al montar), así que para llegar a una vista hay que
 * expandir su sección primero. Mantener el mapa aquí evita que cada spec
 * duplique la navegación y se rompa cuando cambie la agrupación del menú.
 */
const SECTION_BY_ITEM: Record<string, string> = {
  // Operación
  "Panel de control": "Operación",
  "Recepción de pacientes": "Operación",
  "Agenda de citas": "Operación",
  "Agenda de consultas": "Operación",
  Pacientes: "Operación",
  "Historial Clínico": "Operación",
  // Laboratorio
  "Muestras & Laboratorio": "Laboratorio",
  "Bandeja de trabajo": "Laboratorio",
  "Órdenes de laboratorio": "Laboratorio",
  "Control de calidad (QC)": "Laboratorio",
  // Clínica
  Cirugías: "Clínica",
  Vacunación: "Clínica",
  "Estancia y hospedaje": "Clínica",
  // Estética
  Peluquería: "Estética",
  // Comercio
  "Inventario y productos": "Comercio",
  // Administración
  Facturación: "Administración",
  Caja: "Administración",
  "Reportes PDF": "Administración",
  "Catálogo de servicios": "Administración",
  Usuarios: "Administración",
  Auditoría: "Administración",
  Configuración: "Administración",
};

/**
 * Navega a una vista del sidebar por el nombre de su entrada.
 *
 * Expande la sección contenedora si está cerrada (`aria-expanded`) y hace clic
 * en la entrada. Se acota la búsqueda al `aside` porque algunos nombres
 * ("Configuración", "Pacientes") también aparecen en la TopBar o en el
 * contenido de la página.
 */
export async function gotoView(page: Page, item: string): Promise<void> {
  const section = SECTION_BY_ITEM[item];
  if (!section) {
    throw new Error(
      `Sin sección de sidebar conocida para "${item}"; añádela a SECTION_BY_ITEM.`,
    );
  }

  const sidebar = page.locator("aside").first();
  await expect(sidebar).toBeVisible();

  const header = sidebar.getByRole("button", { name: section, exact: true });
  if ((await header.getAttribute("aria-expanded")) !== "true") {
    await header.click();
  }
  await expect(header).toHaveAttribute("aria-expanded", "true");

  await sidebar.getByRole("button", { name: item, exact: true }).click();
}
