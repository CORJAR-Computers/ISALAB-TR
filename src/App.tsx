import { useEffect } from "react";
import { AlertTriangle, Loader2, RefreshCw } from "lucide-react";
import { emit } from "@tauri-apps/api/event";
import { Sidebar } from "@/components/layout/Sidebar";
import { TopBar } from "@/components/layout/TopBar";
import { AboutDialog } from "@/components/layout/AboutDialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ChangePasswordDialog } from "@/features/auth/ChangePasswordDialog";
import { useFirebirdEvents } from "@/hooks/use-firebird-events";
import { useDbHealth } from "@/hooks/use-queries";
import { useUiStore } from "@/stores/ui-store";
import { ErrorBoundary } from "react-error-boundary";
import { GlobalErrorFallback } from "@/components/layout/GlobalErrorBoundary";
import { useSessionTimeout } from "@/hooks/use-session-timeout";
import { useSessionStore } from "@/stores/session-store";
import { api } from "@/lib/api";
import { DashboardPage } from "@/features/dashboard/DashboardPage";
import { ConsultationsPage } from "@/features/agenda/ConsultationsPage";
import { PatientsPage } from "@/features/patients/PatientsPage";
import { ClinicalHistoryPage } from "@/features/clinical-history/ClinicalHistoryPage";
import { SamplesPage } from "@/features/samples/SamplesPage";
import { SurgeriesPage } from "@/features/surgeries/SurgeriesPage";
import { VaccinesPage } from "@/features/vaccines/VaccinesPage";
import { InvoicesPage } from "@/features/invoices/InvoicesPage";
import { ReportsPage } from "@/features/reports/ReportsPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { UsersPage } from "@/features/users/UsersPage";
import { AuditLogPage } from "@/features/audit/AuditLogPage";
import { LoginPage } from "@/features/auth/LoginPage";

export default function App() {
  const theme = useUiStore((s) => s.theme);
  const view = useUiStore((s) => s.view);
  const session = useSessionStore((s) => s.session);
  const hydrated = useSessionStore((s) => s.hydrated);
  const changePasswordOpen = useSessionStore((s) => s.changePasswordOpen);
  const closeChangePassword = useSessionStore((s) => s.closeChangePassword);
  const mustChangePassword = session?.mustChangePassword === true;
  const { data: health, refetch: refetchHealth } = useDbHealth();

  useFirebirdEvents();
  useSessionTimeout();

  // Aplica el tema guardado al montar.
  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
  }, [theme]);

  // Restaura la sesión local al abrir la ventana.
  useEffect(() => {
    api
      .getSession()
      .then((user) => useSessionStore.getState().setSession(user))
      .catch(() => useSessionStore.getState().setSession(null))
      .finally(() => useSessionStore.getState().setHydrated(true));
  }, []);

  // Cuando la app está lista, avisa a Rust para cerrar la ventana splash
  // y mostrar la ventana principal.
  useEffect(() => {
    if (!hydrated) return;
    try {
      void emit("app-ready").catch(() => {});
    } catch {
      /* navegador sin runtime Tauri: ignorar */
    }
  }, [hydrated]);

  // Splash mientras se restaura la sesión.
  if (!hydrated) {
    return (
      <div className="flex min-h-svh flex-col items-center justify-center gap-4">
        <div className="bg-primary/10 animate-glow-pulse flex size-16 items-center justify-center rounded-2xl shadow-lg ring-1 ring-primary/20">
          <Loader2 className="size-7 animate-spin text-primary" />
        </div>
        <p className="text-muted-foreground text-sm">Iniciando ISALAB…</p>
      </div>
    );
  }

  // Puerta de autenticación.
  if (!session) {
    return <LoginPage />;
  }

  const firebirdMissing = health && !health.ok && !health.fbclientFound;

  return (
    <ErrorBoundary FallbackComponent={GlobalErrorFallback}>
      <div className="min-h-svh">
        <Sidebar />
        <AboutDialog />
        <ChangePasswordDialog
          open={mustChangePassword || changePasswordOpen}
          onOpenChange={mustChangePassword ? () => {} : closeChangePassword}
          forced={mustChangePassword}
        />

        <div className="lg:pl-64">
          <TopBar />

          {/* Banner de configuración de Firebird Embedded */}
          {firebirdMissing && (
            <div className="border-b bg-warning/10 px-4 py-3 lg:px-6">
              <div className="mx-auto flex max-w-6xl flex-col gap-3 sm:flex-row sm:items-center">
                <div className="flex items-start gap-3">
                  <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-warning/20">
                    <AlertTriangle className="size-4.5 text-warning" />
                  </span>
                  <div>
                    <p className="text-sm font-semibold">
                      Motor Firebird Embedded no encontrado
                    </p>
                    <p className="text-muted-foreground mt-0.5 text-xs leading-relaxed">
                      Copia <code className="font-mono">fbclient.dll</code> de
                      Firebird 5 en{" "}
                      <code className="font-mono">src-tauri/binaries/firebird/</code>{" "}
                      (ver README). La base de datos se creará automáticamente al
                      primer arranque válido.
                    </p>
                  </div>
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  className="shrink-0"
                  onClick={() => refetchHealth()}
                >
                  <RefreshCw className="size-3.5" />
                  Reintentar
                </Button>
              </div>
            </div>
          )}

          {health?.ok && (
            <div className="mx-auto max-w-6xl px-4 pt-4 lg:px-6">
              <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <Badge variant="success" className="gap-1">
                  <span className="relative flex size-1.5">
                    <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-success opacity-60" />
                    <span className="relative inline-flex size-1.5 rounded-full bg-success" />
                  </span>
                  Firebird conectado
                </Badge>
                <Badge variant="secondary">
                  Schema v{health.schemaVersion}
                </Badge>
                <Badge variant="outline" className="font-mono max-w-72 truncate">
                  {health.dbPath}
                </Badge>
              </div>
            </div>
          )}

          <main className="mx-auto max-w-6xl px-4 py-6 lg:px-6">
            <div key={view} className="animate-fade-in-up">
              {view === "dashboard" && <DashboardPage />}
              {view === "agenda" && <ConsultationsPage />}
              {view === "patients" && <PatientsPage />}
              {view === "clinical-history" && <ClinicalHistoryPage />}
              {view === "samples" && <SamplesPage />}
              {view === "surgeries" && <SurgeriesPage />}
              {view === "vaccines" && <VaccinesPage />}
              {view === "invoices" && <InvoicesPage />}
              {view === "reports" && <ReportsPage />}
              {view === "settings" && <SettingsPage />}
              {view === "users" && <UsersPage />}
              {view === "audit-log" && <AuditLogPage />}
            </div>
          </main>
        </div>
      </div>
    </ErrorBoundary>
  );
}