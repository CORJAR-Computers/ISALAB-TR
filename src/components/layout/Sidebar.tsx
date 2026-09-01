import { cn } from "@/lib/utils";
import { useUiStore, type View } from "@/stores/ui-store";
import { usePermissions } from "@/hooks/use-permissions";
import {
  CalendarClock,
  FlaskConical,
  HeartPulse,
  Info,
  FileText,
  LayoutDashboard,
  ListTodo,
  Receipt,
  Scissors,
  ScrollText,
  Settings,
  Shield,
  Syringe,
  Users,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import logoSidebar from "@/assets/logo_sidebar.png";

const NAV_ITEMS: Array<{ view: View; label: string; icon: typeof Users; adminOnly?: boolean }> = [
  { view: "dashboard", label: "Panel de control", icon: LayoutDashboard },
  { view: "agenda", label: "Agenda de consultas", icon: CalendarClock },
  { view: "patients", label: "Pacientes", icon: Users },
  { view: "clinical-history", label: "Historial Clínico", icon: HeartPulse },
  { view: "samples", label: "Muestras & Laboratorio", icon: FlaskConical },
  { view: "worklist", label: "Bandeja de trabajo", icon: ListTodo },
  { view: "surgeries", label: "Cirugías", icon: Scissors },
  { view: "vaccines", label: "Vacunación", icon: Syringe },
  { view: "invoices", label: "Facturación", icon: Receipt },
  { view: "reports", label: "Reportes PDF", icon: FileText },
  { view: "users", label: "Usuarios", icon: Shield, adminOnly: true },
  { view: "audit-log", label: "Auditoría", icon: ScrollText, adminOnly: true },
  { view: "settings", label: "Configuración", icon: Settings, adminOnly: true },
];

export function Sidebar() {
  const sidebarOpen = useUiStore((s) => s.sidebarOpen);
  const setSidebarOpen = useUiStore((s) => s.setSidebarOpen);
  const setAboutOpen = useUiStore((s) => s.setAboutOpen);
  const view = useUiStore((s) => s.view);
  const navigate = useUiStore((s) => s.navigate);
  const { isAdmin } = usePermissions();
  const items = NAV_ITEMS.filter((item) => !item.adminOnly || isAdmin);

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
              alt="ISALAB · Laboratorio Veterinario"
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

        {/* Navegación */}
        <nav className="flex-1 space-y-1 overflow-y-auto p-3">
          {items.map((item) => {
            const Icon = item.icon;
            const active = view === item.view;
            return (
              <button
                key={item.view}
                type="button"
                onClick={() => navigate(item.view)}
                title={item.label}
                className={cn(
                  "group relative flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition-all duration-200",
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
                CORJAR Computers · v2.0
              </p>
            </div>
            <Info className="size-4 shrink-0 text-muted-foreground group-hover:text-primary transition-colors" />
          </button>
        </div>
      </aside>
    </>
  );
}
