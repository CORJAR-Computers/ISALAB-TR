import { useEffect, useRef, useState } from "react";
import { ChevronDown, Calendar, DoorOpen, Info, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { useUiStore, type View } from "@/stores/ui-store";
import { usePermissions } from "@/hooks/use-permissions";
import {
  Activity,
  Banknote,
  BedDouble,
  CalendarClock,
  FlaskConical,
  HeartPulse,
  FileText,
  LayoutDashboard,
  ListTodo,
  ClipboardList,
  Package,
  Receipt,
  Scissors,
  ScrollText,
  Settings,
  Shield,
  Sparkles,
  Syringe,
  Users,
  LayoutList,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useClinicBranding } from "@/hooks/use-queries";
import { useAppVersion } from "@/hooks/use-app-version";
import logoSidebar from "@/assets/logo_sidebar.png";

type NavItem = {
  view: View;
  label: string;
  icon: typeof Users;
  adminOnly?: boolean;
  /** Capacidad del rol exigida para ver la entrada (fase 6). */
  capability?: "canStay" | "canInventory" | "canGroom" | "canReceive" | "canBill";
};

type NavSection = {
  id: string;
  label: string;
  items: NavItem[];
};

/**
 * Navegación por secciones colapsables: con la clínica integral el menú pasa
 * de 15 a ~20 entradas; agruparlas evita una lista plana interminable.
 * Cada fase del plan de clínica agrega su grupo sin saturar:
 *  - fase 2: Recepción (reception, appointments) → grupo "Recepción"
 *  - fase 4: Peluquería (grooming) → grupo "Estética"
 *  - fase 1: Catálogo de servicios → grupo "Administración"
 *  - fase 6: Estancia (stays) y comercio (products) → sus propios grupos
 */
const SECTIONS: NavSection[] = [
  {
    id: "operacion",
    label: "Operación",
    items: [
      { view: "dashboard", label: "Panel de control", icon: LayoutDashboard },
      { view: "reception", label: "Recepción de pacientes", icon: DoorOpen, capability: "canReceive" },
      { view: "appointments", label: "Agenda de citas", icon: Calendar, capability: "canReceive" },
      { view: "agenda", label: "Agenda de consultas", icon: CalendarClock },
      { view: "patients", label: "Pacientes", icon: Users },
      { view: "clinical-history", label: "Historial Clínico", icon: HeartPulse },
    ],
  },
  {
    id: "laboratorio",
    label: "Laboratorio",
    items: [
      { view: "samples", label: "Muestras & Laboratorio", icon: FlaskConical },
      { view: "worklist", label: "Bandeja de trabajo", icon: ListTodo },
      { view: "lab-orders", label: "Órdenes de laboratorio", icon: ClipboardList },
      { view: "qc", label: "Control de calidad (QC)", icon: Activity },
    ],
  },
  {
    id: "clinica",
    label: "Clínica",
    items: [
      { view: "surgeries", label: "Cirugías", icon: Scissors },
      { view: "vaccines", label: "Vacunación", icon: Syringe },
      { view: "stays", label: "Estancia y hospedaje", icon: BedDouble, capability: "canStay" },
    ],
  },
  {
    id: "estetica",
    label: "Estética",
    items: [
      { view: "grooming", label: "Peluquería", icon: Sparkles, capability: "canGroom" },
    ],
  },
  {
    id: "comercio",
    label: "Comercio",
    items: [
      { view: "products", label: "Inventario y productos", icon: Package, capability: "canInventory" },
    ],
  },
  {
    id: "administracion",
    label: "Administración",
    items: [
      { view: "invoices", label: "Facturación", icon: Receipt },
      { view: "cash", label: "Caja", icon: Banknote, capability: "canBill" },
      { view: "reports", label: "Reportes PDF", icon: FileText },
      { view: "services", label: "Catálogo de servicios", icon: LayoutList, adminOnly: true },
      { view: "users", label: "Usuarios", icon: Shield, adminOnly: true },
      { view: "audit-log", label: "Auditoría", icon: ScrollText, adminOnly: true },
      { view: "settings", label: "Configuración", icon: Settings, adminOnly: true },
    ],
  },
];

/**
 * Estado de las secciones: SIEMPRE colapsadas al montar la app. No se
 * persiste en localStorage para que cada arranque sea determinista (menú
 * limpio, solo títulos de sección); la sección de la vista activa se abre
 * sola únicamente cuando la navegación llega desde otra sección.
 */
const INITIAL_OPEN: string[] = [];

export function Sidebar() {
  const sidebarOpen = useUiStore((s) => s.sidebarOpen);
  const setSidebarOpen = useUiStore((s) => s.setSidebarOpen);
  const setAboutOpen = useUiStore((s) => s.setAboutOpen);
  const view = useUiStore((s) => s.view);
  const navigate = useUiStore((s) => s.navigate);
  const perms = usePermissions();
  const { isAdmin } = perms;
  const { data: branding } = useClinicBranding();
  const tagline = branding?.tagline || "Clínica Veterinaria";
  // Versión real instalada: la misma que muestra el diálogo Acerca de.
  const appVersion = useAppVersion();

  // Mapa de capacidades para filtrar las entradas del menú. Incluye todas
  // las declaradas en `NavItem.capability` para que ninguna quede oculta
  // por error.
  const capabilityMap: Record<string, boolean> = {
    canStay: perms.canStay,
    canInventory: perms.canInventory,
    canGroom: perms.canGroom,
    canReceive: perms.canReceive,
    canBill: perms.canBill,
  };

  const [openSections, setOpenSections] = useState<string[]>(INITIAL_OPEN);

  // Vista con la que se montó la sidebar: el auto-apertura solo reacciona a
  // cambios de vista DESPUÉS del montaje (al llegar desde la paleta Ctrl+K o
  // desde un botón de otra página), nunca al arranque de la app.
  const mountedView = useRef(view);

  // Auto-abre la sección que contiene la vista activa (p. ej. al llegar
  // desde la paleta Ctrl+K o desde un botón de otra página).
  useEffect(() => {
    if (view === mountedView.current) return;
    mountedView.current = view;
    const section = SECTIONS.find((s) =>
      s.items.some((item) => item.view === view),
    );
    if (!section) return;
    setOpenSections((prev) =>
      prev.includes(section.id) ? prev : [...prev, section.id],
    );
  }, [view]);

  const toggleSection = (id: string) =>
    setOpenSections((prev) =>
      prev.includes(id) ? prev.filter((s) => s !== id) : [...prev, id],
    );

  const filtered = SECTIONS.map((section) => ({
    ...section,
    items: section.items.filter((item) => {
      if (item.adminOnly && !isAdmin) return false;
      if (item.capability && !capabilityMap[item.capability]) return false;
      return true;
    }),
  })).filter((section) => section.items.length > 0);

  return (
    <>
      {/* Overlay móvil */}
      {sidebarOpen && (
        <div
          className="bg-black/50 fixed inset-0 z-40 backdrop-blur-sm lg:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      <aside
        className={cn(
          "bg-sidebar text-sidebar-foreground fixed inset-y-0 left-0 z-50 flex w-64 flex-col border-r shadow-lg transition-transform duration-300 lg:translate-x-0",
          "bg-linear-to-b from-sidebar to-sidebar/95",
          sidebarOpen ? "translate-x-0" : "-translate-x-full",
        )}
      >
        {/* Logo */}
        <div className="flex h-20 shrink-0 items-center gap-3 border-b px-4">
          <div className="flex h-12 flex-1 items-center justify-center rounded-xl border border-primary/10 bg-primary/6 px-3 transition-colors hover:bg-primary/10">
            <img
              src={logoSidebar}
              alt={`ISALAB · ${tagline}`}
              className="max-h-full w-full object-contain"
              draggable={false}
            />
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="ml-auto shrink-0 lg:hidden"
            onClick={() => setSidebarOpen(false)}
            aria-label="Cerrar menú"
          >
            <X className="size-4" />
          </Button>
        </div>

        {/* Navegación por secciones */}
        <nav className="flex-1 overflow-y-auto p-3">
          {filtered.map((section) => {
            const open = openSections.includes(section.id);
            const activeInView = section.items.some((item) => item.view === view);
            return (
              <div key={section.id} className="mb-1">
                <button
                  type="button"
                  onClick={() => toggleSection(section.id)}
                  aria-expanded={open}
                  className={cn(
                    "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-[11px] font-semibold uppercase tracking-wider transition-colors",
                    activeInView && !open
                      ? "text-primary"
                      : "text-muted-foreground hover:text-sidebar-accent-foreground",
                  )}
                >
                  <span className="flex-1 text-left">{section.label}</span>
                  <ChevronDown
                    className={cn(
                      "size-3.5 shrink-0 transition-transform duration-200",
                      open && "rotate-180",
                    )}
                  />
                </button>

                {open && (
                  <div className="space-y-0.5 pt-0.5">
                    {section.items.map((item) => {
                      const Icon = item.icon;
                      const active = view === item.view;
                      return (
                        <button
                          key={item.view}
                          type="button"
                          onClick={() => navigate(item.view)}
                          title={item.label}
                          className={cn(
                            "group relative flex w-full items-center gap-3 rounded-xl py-2.5 pr-3 pl-5 text-sm font-medium transition-all duration-200",
                            active
                              ? "bg-linear-to-r from-primary to-primary/90 text-primary-foreground shadow-md"
                              : "text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground hover:translate-x-0.5",
                          )}
                        >
                          {/* Indicador activo */}
                          <span
                            className={cn(
                              "absolute top-1/2 left-0 h-5 w-1 -translate-y-1/2 rounded-r-full bg-foreground/70 transition-opacity",
                              active ? "opacity-100" : "opacity-0",
                            )}
                          />
                          <span
                            className={cn(
                              "flex size-7 items-center justify-center rounded-lg transition-colors",
                              active
                                ? "bg-white/20"
                                : "bg-muted/60 group-hover:bg-background/60",
                            )}
                          >
                            <Icon className="size-4 shrink-0" />
                          </span>
                          <span className="flex-1 text-left">{item.label}</span>
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>
            );
          })}
        </nav>

        {/* Pie */}
        <div className="border-t p-4">
          <button
            type="button"
            onClick={() => setAboutOpen(true)}
            title="Ver información de ISALAB y CORJAR Computers Solutions"
            className="group flex w-full items-center gap-2 rounded-xl bg-sidebar-accent/60 px-3 py-2.5 transition-all hover:bg-sidebar-accent hover:shadow-sm"
          >
            <span className="relative flex size-2 shrink-0">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-success opacity-60" />
              <span className="relative inline-flex size-2 rounded-full bg-success" />
            </span>
            <div className="min-w-0 flex-1 text-left">
              <p className="truncate text-xs font-semibold group-hover:text-primary">
                Acerca de ISALAB
              </p>
              <p className="text-muted-foreground truncate text-[11px]">
                CORJAR Computers{appVersion ? ` · v${appVersion}` : ""}
              </p>
            </div>
            <Info className="size-4 shrink-0 text-muted-foreground group-hover:text-primary transition-colors" />
          </button>
        </div>
      </aside>
    </>
  );
}
