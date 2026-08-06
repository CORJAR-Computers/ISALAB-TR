import { Info, KeyRound, LogOut, Menu, Moon, Search, Settings, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useUiStore } from "@/stores/ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useLogout } from "@/hooks/use-queries";
import { ROLE_LABEL } from "@/lib/status";
import { cn } from "@/lib/utils";

const TITLES: Record<string, { title: string; subtitle: string }> = {
  dashboard: {
    title: "Panel de control",
    subtitle: "Resumen de la actividad clínica de hoy",
  },
  agenda: {
    title: "Agenda de consultas",
    subtitle: "Citas ambulatorias programadas",
  },
  patients: {
    title: "Pacientes",
    subtitle: "Registro y búsqueda de pacientes",
  },
  "clinical-history": {
    title: "Historial Clínico",
    subtitle: "Ficha y línea de tiempo del paciente",
  },
  samples: {
    title: "Muestras & Laboratorio",
    subtitle: "Mesa de trabajo y trazabilidad",
  },
  surgeries: {
    title: "Cirugías",
    subtitle: "Agenda quirúrgica e intervenciones",
  },
  vaccines: {
    title: "Vacunación",
    subtitle: "Esquemas y refuerzos",
  },
  invoices: {
    title: "Facturación",
    subtitle: "Emisión y cobro de facturas",
  },
  reports: {
    title: "Reportes PDF",
    subtitle: "Informes generados en Rust",
  },
  users: {
    title: "Usuarios",
    subtitle: "Cuentas y roles del sistema",
  },
  settings: {
    title: "Configuración",
    subtitle: "Datos de la clínica y reportes",
  },
};

export function TopBar() {
  const view = useUiStore((s) => s.view);
  const theme = useUiStore((s) => s.theme);
  const toggleTheme = useUiStore((s) => s.toggleTheme);
  const setSidebarOpen = useUiStore((s) => s.setSidebarOpen);
  const setAboutOpen = useUiStore((s) => s.setAboutOpen);
  const openSearch = useUiStore((s) => s.openSearch);
  const navigate = useUiStore((s) => s.navigate);
  const session = useSessionStore((s) => s.session);
  const openChangePassword = useSessionStore((s) => s.openChangePassword);
  const logout = useLogout();

  const current = TITLES[view] ?? {
    title: "ISALAB",
    subtitle: "Sistema de gestión para laboratorios veterinarios",
  };

  return (
    <header
      className={cn(
        "glass sticky top-0 z-30 flex h-16 items-center gap-3 border-b px-4 shadow-sm",
        "lg:px-6",
      )}
    >
      <Button
        variant="ghost"
        size="icon"
        className="lg:hidden"
        onClick={() => setSidebarOpen(true)}
        aria-label="Abrir menú"
      >
        <Menu className="size-5" />
      </Button>

      <div className="min-w-0">
        <h1 className="text-gradient truncate text-base font-semibold tracking-tight">
          {current.title}
        </h1>
        <p className="text-muted-foreground hidden truncate text-xs sm:block">
          {current.subtitle}
        </p>
      </div>

      <div className="ml-auto flex items-center gap-1">
        <Button
          variant="outline"
          onClick={openSearch}
          className="gap-2 px-2.5 text-muted-foreground"
          title="Buscar en todo el sistema (Ctrl+K)"
          aria-label="Buscar (Ctrl+K)"
        >
          <Search className="size-4" />
          <span className="hidden text-sm lg:inline">Buscar…</span>
          <kbd className="rounded border bg-muted hidden px-1.5 py-0.5 text-[10px] font-medium lg:inline">
            Ctrl K
          </kbd>
        </Button>
        {session && (
          <>
            <Button
              variant="ghost"
              size="icon"
              onClick={openChangePassword}
              title="Cambiar contraseña"
              aria-label="Cambiar contraseña"
            >
              <KeyRound className="size-4.5" />
            </Button>
            <div className="bg-secondary/80 text-foreground flex h-9 items-center gap-2 rounded-full py-1 pr-3 pl-1 shadow-sm ring-1 ring-border backdrop-blur-sm transition-shadow hover:shadow-md">
              <span className="bg-linear-to-br from-primary to-emerald-600 text-primary-foreground flex size-7 items-center justify-center rounded-full text-[11px] font-bold uppercase shadow-sm">
                {(session.fullName || session.username).slice(0, 1)}
              </span>
              <span className="hidden text-xs font-medium sm:block">
                {session.fullName || session.username}
              </span>
              <Badge variant="secondary" className="hidden sm:inline-flex">
                {ROLE_LABEL[session.role] ?? session.role}
              </Badge>
            </div>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => logout.mutate()}
              title="Cerrar sesión"
              aria-label="Cerrar sesión"
            >
              <LogOut className="size-4.5" />
            </Button>
          </>
        )}
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleTheme}
          aria-label="Cambiar tema"
          className="transition-transform hover:rotate-12"
        >
          {theme === "light" ? <Moon className="size-4.5" /> : <Sun className="size-4.5" />}
        </Button>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setAboutOpen(true)}
          aria-label="Acerca de ISALAB"
          title="Acerca de ISALAB (CORJAR Computers Solutions)"
        >
          <Info className="size-4.5" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => navigate("settings")}
          aria-label="Configuración"
          title="Configuración de la clínica"
        >
          <Settings className="size-4.5" />
        </Button>
      </div>
    </header>
  );
}
