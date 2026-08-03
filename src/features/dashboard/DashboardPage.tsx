import { useMemo } from "react";
import { toast } from "sonner";
import {
  Activity,
  Ban,
  CalendarClock,
  CheckCircle2,
  FlaskConical,
  Loader2,
  PawPrint,
  Plus,
  Receipt,
  Scissors,
  Stethoscope,
  Syringe,
  Wallet,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  useDashboardStats,
  useSetConsultationStatus,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { cn, formatCOP, formatDate, formatDateTime } from "@/lib/utils";
import {
  CONSULTATION_STATUS,
  INVOICE_STATUS,
  SAMPLE_STATUS,
  SURGERY_STATUS,
} from "@/lib/status";
import { useUiStore } from "@/stores/ui-store";

function StatCard({
  icon: Icon,
  label,
  value,
  tone,
}: {
  icon: typeof FlaskConical;
  label: string;
  value: number | string;
  tone: string;
}) {
  return (
    <div className="group relative flex items-center gap-3 overflow-hidden rounded-xl border bg-card px-4 py-3 shadow-sm transition-all duration-300 hover:-translate-y-1 hover:shadow-lg">
      {/* Brillo al hover */}
      <div className="bg-primary/5 absolute inset-0 opacity-0 transition-opacity duration-300 group-hover:opacity-100" />
      <div
        className={cn(
          "flex size-11 shrink-0 items-center justify-center rounded-xl shadow-sm ring-1 ring-black/5 transition-transform duration-300 group-hover:scale-110",
          tone,
        )}
      >
        <Icon className="size-5" />
      </div>
      <div className="relative min-w-0">
        <p className="text-2xl leading-none font-bold tabular-nums">
          {value}
        </p>
        <p className="text-muted-foreground mt-1 truncate text-xs">{label}</p>
      </div>
    </div>
  );
}

function AgendaList({
  title,
  icon: Icon,
  empty,
  items,
  children,
}: {
  title: string;
  icon: typeof CalendarClock;
  empty: string;
  items: unknown[];
  children: React.ReactNode;
}) {
  return (
    <Card className="gap-0 p-0">
      <CardHeader className="border-b bg-gradient-to-r from-transparent via-transparent to-primary/[0.04]">
        <CardTitle className="flex items-center gap-2 text-base">
          <span className="bg-primary/10 text-primary flex size-7 items-center justify-center rounded-lg">
            <Icon className="size-4" />
          </span>
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent className="p-2">
        {items.length > 0 ? (
          children
        ) : (
          <p className="text-muted-foreground px-4 py-8 text-center text-sm">
            {empty}
          </p>
        )}
      </CardContent>
    </Card>
  );
}

export function DashboardPage() {
  const { data: stats, isLoading } = useDashboardStats();
  const navigate = useUiStore((s) => s.navigate);
  const requestNewPatient = useUiStore((s) => s.requestNewPatient);
  const setConsultationStatus = useSetConsultationStatus();

  const sampleBreakdown = useMemo(() => {
    if (!stats) return [];
    const total = stats.samplesTotal || 1;
    const finished = Math.max(0, stats.samplesFinished);
    const inProgress = Math.max(0, stats.samplesInProgress);
    const cancelled = Math.max(0, stats.samplesCancelled);
    const received = Math.max(0, total - finished - inProgress - cancelled);
    return [
      { label: "Recibidas", value: received, color: "bg-muted-foreground/40" },
      { label: "En proceso", value: inProgress, color: "bg-warning" },
      { label: "Finalizadas", value: finished, color: "bg-success" },
    ];
  }, [stats]);

  const changeConsultationStatus = async (
    id: number,
    status: string,
    label: string,
  ) => {
    try {
      await setConsultationStatus.mutateAsync({ id, status });
      toast.success(`Consulta ${label.toLowerCase()}`);
    } catch (e) {
      toast.error("No se pudo actualizar la consulta", {
        description: getErrorMessage(e),
      });
    }
  };

  if (isLoading || !stats) {
    return (
      <div className="space-y-5">
        <Skeleton className="h-28 w-full" />
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {Array.from({ length: 8 }).map((_, i) => (
            <Skeleton key={i} className="h-20 w-full" />
          ))}
        </div>
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  return (
    <div className="animate-fade-in-up space-y-5">
      {/* Hero */}
      <div className="from-primary via-primary/95 to-emerald-700 relative overflow-hidden rounded-2xl bg-gradient-to-br p-6 text-primary-foreground shadow-lg">
        <div className="bg-white/10 absolute -top-16 -right-10 size-56 rounded-full blur-2xl animate-float" />
        <div className="bg-white/10 absolute -bottom-24 right-32 size-40 rounded-full blur-2xl animate-float [animation-delay:2s]" />
        <div className="relative flex flex-wrap items-center justify-between gap-4">
          <div>
            <p className="text-white/70 text-xs font-medium tracking-wide uppercase">
              {new Date().toLocaleDateString("es-CO", {
                weekday: "long",
                day: "numeric",
                month: "long",
              })}
            </p>
            <h2 className="mt-1 text-2xl font-bold tracking-tight">
              Panel de control
            </h2>
            <p className="text-white/80 mt-1 max-w-lg text-sm">
              Resumen clínico, agenda de próximas citas y actividad del
              laboratorio.
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-3 rounded-xl bg-white/10 px-4 py-3 backdrop-blur-sm ring-1 ring-white/15">
              <PawPrint className="size-8" />
              <div>
                <p className="text-2xl leading-none font-bold tabular-nums">
                  {stats.patientsActive}
                </p>
                <p className="text-white/70 text-xs">
                  pacientes activos
                </p>
              </div>
            </div>
            <Button
              variant="secondary"
              onClick={() => {
                requestNewPatient();
                navigate("patients");
              }}
              className="bg-white text-primary shadow-lg ring-1 ring-white/30 hover:bg-white/90 hover:-translate-y-px"
            >
              <Plus className="size-4" />
              Nuevo paciente
            </Button>
          </div>
        </div>
      </div>

      {/* Tarjetas de métricas */}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
        <StatCard
          icon={PawPrint}
          label={`Pacientes activos (${stats.patientsTotal} total)`}
          value={stats.patientsActive}
          tone="bg-primary/10 text-primary"
        />
        <StatCard
          icon={FlaskConical}
          label="Muestras en proceso"
          value={stats.samplesInProgress}
          tone="bg-warning/15 text-warning"
        />
        <StatCard
          icon={CheckCircle2}
          label="Muestras finalizadas"
          value={stats.samplesFinished}
          tone="bg-success/15 text-success"
        />
        <StatCard
          icon={Stethoscope}
          label="Consultas pendientes"
          value={stats.consultationsPending}
          tone="bg-sky-500/15 text-sky-600 dark:text-sky-400"
        />
        <StatCard
          icon={Scissors}
          label="Cirugías programadas"
          value={stats.surgeriesProgrammed}
          tone="bg-violet-500/15 text-violet-600 dark:text-violet-400"
        />
        <StatCard
          icon={Syringe}
          label="Refuerzos vencidos"
          value={stats.vaccinesDue}
          tone="bg-orange-500/15 text-orange-600 dark:text-orange-400"
        />
        <StatCard
          icon={Receipt}
          label="Facturas sin pagar"
          value={stats.invoicesUnpaid}
          tone="bg-destructive/10 text-destructive"
        />
        <StatCard
          icon={Wallet}
          label="Ingresos (COP)"
          value={formatCOP(stats.revenueTotal ?? 0)}
          tone="bg-emerald-500/15 text-emerald-600 dark:text-emerald-400"
        />
      </div>

      {/* Progreso del laboratorio */}
      <Card className="gap-0 p-0">
        <CardHeader className="border-b bg-gradient-to-r from-transparent via-transparent to-primary/[0.04]">
          <CardTitle className="flex items-center gap-2 text-base">
            <span className="bg-primary/10 text-primary flex size-7 items-center justify-center rounded-lg">
              <Activity className="size-4" />
            </span>
            Laboratorio · {stats.samplesTotal} muestras
          </CardTitle>
          <CardDescription>
            {stats.abnormalResults} resultado
            {stats.abnormalResults === 1 ? "" : "s"} fuera de rango de
            referencia
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-2 p-4">
          <div className="flex h-3 w-full overflow-hidden rounded-full bg-muted shadow-inner">
            {sampleBreakdown.map((s) =>
              s.value > 0 ? (
                <div
                  key={s.label}
                  className={cn("h-full transition-all duration-500", s.color)}
                  style={{
                    width: `${(s.value / (stats.samplesTotal || 1)) * 100}%`,
                  }}
                  title={`${s.label}: ${s.value}`}
                />
              ) : null,
            )}
          </div>
          <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
            {sampleBreakdown.map((s) => (
              <span key={s.label} className="flex items-center gap-1.5">
                <span className={cn("size-2 rounded-full", s.color)} />
                {s.label}: {s.value}
              </span>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Agenda */}
      <div className="grid gap-4 lg:grid-cols-2">
        <AgendaList
          title="Próximas consultas"
          icon={Stethoscope}
          empty="Sin consultas pendientes en la agenda."
          items={stats.upcomingConsultations}
        >
          {stats.upcomingConsultations.map((c) => {
            const st = CONSULTATION_STATUS[c.status] ?? {
              label: c.status,
              variant: "secondary" as const,
            };
            const busy = setConsultationStatus.isPending;
            return (
              <div
                key={c.id}
                className="hover:bg-accent group flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-all duration-200 hover:translate-x-0.5"
              >
                <div className="bg-primary/10 text-primary flex size-9 shrink-0 items-center justify-center rounded-full text-xs font-bold">
                  {c.consultationDate.slice(8, 10)}
                </div>
                <div
                  className="min-w-0 flex-1 cursor-pointer"
                  onClick={() => navigate("agenda")}
                >
                  <p className="truncate text-sm font-medium">
                    {c.patientName}
                  </p>
                  <p className="text-muted-foreground truncate text-xs">
                    {c.reason || "Consulta"} · {c.ownerName}
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-xs font-semibold tabular-nums">
                    {formatDateTime(c.consultationDate)}
                  </p>
                  <div className="mt-0.5 flex items-center justify-end gap-1">
                    <Badge variant={st.variant}>{st.label}</Badge>
                    {c.status === "PENDIENTE" && (
                      <>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-6 px-1.5 text-xs text-success"
                          disabled={busy}
                          onClick={(e) => {
                            e.stopPropagation();
                            changeConsultationStatus(c.id, "COMPLETADA", "Completada");
                          }}
                          title="Marcar como completada"
                        >
                          {busy ? (
                            <Loader2 className="size-3 animate-spin" />
                          ) : (
                            <CheckCircle2 className="size-3" />
                          )}
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-6 px-1.5 text-xs text-destructive"
                          disabled={busy}
                          onClick={(e) => {
                            e.stopPropagation();
                            changeConsultationStatus(c.id, "CANCELADA", "Cancelada");
                          }}
                          title="Cancelar consulta"
                        >
                          <Ban className="size-3" />
                        </Button>
                      </>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </AgendaList>

        <AgendaList
          title="Próximas cirugías"
          icon={Scissors}
          empty="Sin cirugías programadas."
          items={stats.upcomingSurgeries}
        >
          {stats.upcomingSurgeries.map((s) => {
            const st = SURGERY_STATUS[s.status] ?? {
              label: s.status,
              variant: "secondary" as const,
            };
            return (
              <button
                key={s.id}
                type="button"
                onClick={() => navigate("surgeries")}
                className="hover:bg-accent flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-all duration-200 hover:translate-x-0.5"
              >
                <div className="bg-violet-500/10 text-violet-600 flex size-9 shrink-0 items-center justify-center rounded-full text-xs font-bold dark:text-violet-400">
                  {s.scheduledAt.slice(8, 10)}
                </div>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium">
                    {s.surgeryType}
                  </p>
                  <p className="text-muted-foreground truncate text-xs">
                    {s.patientName} · {s.ownerName}
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-xs font-semibold tabular-nums">
                    {formatDateTime(s.scheduledAt)}
                  </p>
                  <Badge variant={st.variant} className="mt-0.5">
                    {st.label}
                  </Badge>
                </div>
              </button>
            );
          })}
        </AgendaList>

        <AgendaList
          title="Próximos refuerzos de vacunación"
          icon={Syringe}
          empty="Sin refuerzos próximos."
          items={stats.upcomingVaccines}
        >
          {stats.upcomingVaccines.map((v) => (
            <button
              key={v.id}
              type="button"
              onClick={() => navigate("vaccines")}
              className="hover:bg-accent flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-all duration-200 hover:translate-x-0.5"
            >
              <div className="bg-orange-500/10 text-orange-600 flex size-9 shrink-0 items-center justify-center rounded-full text-xs font-bold dark:text-orange-400">
                {v.nextDoseAt ? v.nextDoseAt.slice(8, 10) : "—"}
              </div>
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">{v.vaccineName}</p>
                <p className="text-muted-foreground truncate text-xs">
                  {v.patientName} · {v.ownerName}
                </p>
              </div>
              <div className="text-right">
                <p className="text-xs font-semibold tabular-nums">
                  {formatDate(v.nextDoseAt)}
                </p>
                <Badge variant="warning" className="mt-0.5">
                  Refuerzo
                </Badge>
              </div>
            </button>
          ))}
        </AgendaList>

        <AgendaList
          title="Últimas muestras recibidas"
          icon={FlaskConical}
          empty="Sin muestras registradas."
          items={stats.recentSamples}
        >
          {stats.recentSamples.map((s) => {
            const st = SAMPLE_STATUS[s.status] ?? {
              label: s.status,
              variant: "secondary" as const,
            };
            return (
              <button
                key={s.id}
                type="button"
                onClick={() => navigate("samples")}
                className="hover:bg-accent flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-all duration-200 hover:translate-x-0.5"
              >
                <div className="bg-warning/10 text-warning flex size-9 shrink-0 items-center justify-center rounded-full font-mono text-xs font-bold">
                  {s.code.slice(-2)}
                </div>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium">
                    {s.patientName}
                  </p>
                  <p className="text-muted-foreground truncate text-xs">
                    {s.sampleTypeName} · {s.ownerName}
                  </p>
                </div>
                <div className="text-right">
                  <p className="font-mono text-xs font-semibold">{s.code}</p>
                  <Badge
                    variant={st.variant}
                    className="mt-0.5"
                  >
                    {st.label}
                  </Badge>
                </div>
              </button>
            );
          })}
        </AgendaList>
      </div>

      {/* Accesos rápidos a facturación */}
      {stats.invoicesUnpaid > 0 && (
        <Card className="gap-0 border-dashed p-0">
          <CardContent className="flex flex-wrap items-center gap-3 p-4">
            <div className="bg-destructive/10 text-destructive flex size-9 shrink-0 items-center justify-center rounded-lg">
              <Receipt className="size-4" />
            </div>
            <div className="min-w-0 flex-1">
              <p className="text-sm font-semibold">
                {stats.invoicesUnpaid} factura
                {stats.invoicesUnpaid === 1 ? "" : "s"} pendiente
                {stats.invoicesUnpaid === 1 ? "" : "s"} de cobro
              </p>
              <p className="text-muted-foreground text-xs">
                Revisa las facturas emitidas para registrarlas como pagadas.
              </p>
            </div>
            <Badge variant="outline" className="text-success">
              <Wallet className="size-3" />
              {formatCOP(stats.revenueTotal ?? 0)} cobrados
            </Badge>
            <Badge variant={INVOICE_STATUS.EMITIDA.variant}>
              {INVOICE_STATUS.EMITIDA.label}
            </Badge>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
