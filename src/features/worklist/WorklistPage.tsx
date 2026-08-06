import { useState } from "react";
import {
  AlertTriangle,
  CalendarDays,
  CheckCircle2,
  Clock,
  FlaskConical,
  ListTodo,
  RefreshCw,
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
import { useWorklist } from "@/hooks/use-queries";
import { SAMPLE_STATUS } from "@/lib/status";
import { cn, formatDateTime } from "@/lib/utils";
import type { WorklistGroup } from "@/bindings";
import { SampleDetailDialog } from "@/features/samples/SampleDetailDialog";

/** Formatea minutos transcurridos como "45m", "3h 25m" o "2d 4h". */
function formatElapsed(minutes: number): string {
  const m = Math.max(0, Math.round(minutes));
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return h % 60 === 0 ? `${h}h` : `${h}h ${m % 60}m`;
  const d = Math.floor(h / 24);
  const rh = h % 24;
  return rh === 0 ? `${d}d` : `${d}d ${rh}h`;
}

/** Color por urgencia: <2h normal, 2-6h aviso, >6h o de días anteriores crítica. */
function elapsedTone(minutes: number, overdue: boolean): string {
  if (overdue || minutes >= 360) return "font-semibold text-destructive";
  if (minutes >= 120) return "font-medium text-warning";
  return "text-muted-foreground";
}

/** "2026-08-06" → "miércoles, 6 de agosto de 2026" (es-CO). */
function formatDateLabel(isoDate: string): string {
  const [y, m, d] = isoDate.split("-").map(Number);
  if (!y || !m || !d) return isoDate;
  return new Intl.DateTimeFormat("es-CO", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  }).format(new Date(y, m - 1, d));
}

function StatChip({
  icon: Icon,
  label,
  value,
  tone,
  alert = false,
}: {
  icon: typeof FlaskConical;
  label: string;
  value: number;
  tone: string;
  /** Resalta el borde cuando hay trabajo pendiente (p. ej. retrasados). */
  alert?: boolean;
}) {
  return (
    <div
      className={cn(
        "flex items-center gap-3 rounded-xl border bg-card px-4 py-3 shadow-sm transition-shadow",
        alert && value > 0 && "border-warning/40 shadow-warning/5",
      )}
    >
      <div
        className={cn(
          "flex size-10 shrink-0 items-center justify-center rounded-lg",
          tone,
        )}
      >
        <Icon className="size-5" />
      </div>
      <div>
        <p className="text-2xl leading-none font-bold tabular-nums">{value}</p>
        <p className="text-muted-foreground text-xs">{label}</p>
      </div>
    </div>
  );
}

function GroupCard({
  group,
  overdue = false,
  onOpenSample,
}: {
  group: WorklistGroup;
  overdue?: boolean;
  onOpenSample: (id: number) => void;
}) {
  return (
    <Card className="gap-0 overflow-hidden p-0">
      <CardHeader className="bg-muted/40 border-b px-4 py-3">
        <div className="flex items-center gap-3">
          <div
            className={cn(
              "flex size-9 shrink-0 items-center justify-center rounded-lg",
              overdue
                ? "bg-warning/15 text-warning"
                : "bg-primary/10 text-primary",
            )}
          >
            <FlaskConical className="size-4.5" />
          </div>
          <div className="min-w-0 flex-1">
            <CardTitle className="truncate text-sm">
              {group.sampleTypeName}
            </CardTitle>
            <CardDescription className="text-xs">
              {group.count} muestra{group.count === 1 ? "" : "s"} · la más
              antigua hace {formatElapsed(group.maxElapsedMinutes)}
            </CardDescription>
          </div>
          <Badge
            variant={overdue ? "warning" : "secondary"}
            className="shrink-0 tabular-nums"
          >
            {group.count}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="p-0">
        <ul className="divide-y">
          {group.samples.map((s) => {
            const st = SAMPLE_STATUS[s.status] ?? {
              label: s.status,
              variant: "secondary" as const,
            };
            return (
              <li key={s.id}>
                <button
                  type="button"
                  onClick={() => onOpenSample(s.id)}
                  title={`Abrir muestra ${s.code}`}
                  className="flex w-full items-center gap-3 px-4 py-2.5 text-left transition-colors hover:bg-accent/50"
                >
                  <div className="w-28 shrink-0">
                    <span className="font-mono text-xs font-semibold">
                      {s.code}
                    </span>
                    <p
                      className="text-muted-foreground truncate text-[11px]"
                      title={s.receivedAt}
                    >
                      {formatDateTime(s.receivedAt)}
                    </p>
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium">
                      {s.patientName}
                    </p>
                    <p className="text-muted-foreground truncate text-xs">
                      {s.speciesName} · {s.ownerName}
                    </p>
                  </div>
                  <span
                    className={cn(
                      "flex shrink-0 items-center gap-1 text-sm tabular-nums",
                      elapsedTone(s.elapsedMinutes, overdue),
                    )}
                    title="Tiempo transcurrido desde la recepción"
                  >
                    <Clock className="size-3.5" />
                    {formatElapsed(s.elapsedMinutes)}
                  </span>
                  <Badge
                    variant={st.variant}
                    className="hidden shrink-0 sm:inline-flex"
                  >
                    {st.label}
                  </Badge>
                  <span
                    className="text-muted-foreground w-12 shrink-0 text-right text-xs tabular-nums"
                    title="Resultados cargados"
                  >
                    <span className="font-semibold text-foreground">
                      {s.resultCount}
                    </span>{" "}
                    res.
                  </span>
                  {s.abnormalCount > 0 && (
                    <Badge
                      variant="outline"
                      className="text-warning shrink-0 gap-1"
                      title={`${s.abnormalCount} resultado(s) fuera de rango`}
                    >
                      <AlertTriangle className="size-3" />
                      {s.abnormalCount}
                    </Badge>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      </CardContent>
    </Card>
  );
}

export function WorklistPage() {
  const { data, isLoading, isError, refetch, isFetching } = useWorklist();
  const [detailId, setDetailId] = useState<number | null>(null);

  const todayCount = (data?.today ?? []).reduce(
    (acc, g) => acc + g.count,
    0,
  );
  const overdueCount = (data?.overdue ?? []).reduce(
    (acc, g) => acc + g.count,
    0,
  );

  return (
    <div className="space-y-5">
      {/* Encabezado */}
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="flex items-center gap-2 text-xl font-semibold tracking-tight">
            <ListTodo className="text-primary size-5" />
            Bandeja de trabajo
          </h2>
          <p className="text-muted-foreground text-sm">
            Muestras pendientes agrupadas por tipo, con el tiempo transcurrido
            desde la recepción para priorizar el procesamiento.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant="outline" className="gap-1.5 px-3 py-1.5 capitalize">
            <CalendarDays className="size-3.5" />
            {data ? formatDateLabel(data.date) : "Cargando…"}
          </Badge>
          <Button
            variant="outline"
            size="sm"
            onClick={() => refetch()}
            disabled={isFetching}
            className="gap-1.5"
          >
            <RefreshCw
              className={cn("size-3.5", isFetching && "animate-spin")}
            />
            Actualizar
          </Button>
        </div>
      </div>

      {/* Resumen */}
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <StatChip
          icon={FlaskConical}
          label="Recibidas hoy"
          value={todayCount}
          tone="bg-primary/10 text-primary"
        />
        <StatChip
          icon={AlertTriangle}
          label="Pendientes de días anteriores"
          value={overdueCount}
          tone="bg-warning/15 text-warning"
          alert
        />
        <StatChip
          icon={Clock}
          label="Total en bandeja"
          value={data?.totalPending ?? 0}
          tone="bg-muted text-muted-foreground"
        />
      </div>

      {isLoading && (
        <div className="grid gap-3 lg:grid-cols-2">
          {Array.from({ length: 4 }).map((_, i) => (
            <Card key={i} className="gap-0 p-0">
              <CardHeader className="px-4 py-3">
                <Skeleton className="h-5 w-40" />
              </CardHeader>
              <CardContent className="space-y-2 p-4">
                <Skeleton className="h-8 w-full" />
                <Skeleton className="h-8 w-full" />
                <Skeleton className="h-8 w-2/3" />
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {!isLoading && isError && (
        <div className="flex flex-col items-center justify-center gap-3 rounded-xl border bg-card py-16 text-center">
          <p className="text-sm font-semibold">No se pudo cargar la bandeja</p>
          <p className="text-muted-foreground text-xs">
            Verifica la conexión con la base de datos e inténtalo de nuevo.
          </p>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            Reintentar
          </Button>
        </div>
      )}

      {!isLoading && !isError && data && data.totalPending === 0 && (
        <div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed bg-card py-16 text-center">
          <div className="bg-success/15 flex size-14 items-center justify-center rounded-2xl text-success">
            <CheckCircle2 className="size-7" />
          </div>
          <p className="text-sm font-semibold">¡Bandeja al día!</p>
          <p className="text-muted-foreground max-w-sm text-xs">
            No hay muestras pendientes. Las nuevas recepciones aparecerán aquí
            agrupadas por tipo de muestra, con su tiempo de espera.
          </p>
        </div>
      )}

      {data && data.totalPending > 0 && (
        <>
          {data.today.length > 0 && (
            <div className="space-y-3">
              <div className="flex items-baseline gap-2">
                <h3 className="text-sm font-semibold">Recibidas hoy</h3>
                <Badge variant="secondary" className="tabular-nums">
                  {todayCount}
                </Badge>
                <p className="text-muted-foreground text-xs">
                  Ordenadas por la más antigua primero.
                </p>
              </div>
              <div className="grid gap-3 lg:grid-cols-2">
                {data.today.map((g) => (
                  <GroupCard
                    key={g.sampleTypeId}
                    group={g}
                    onOpenSample={setDetailId}
                  />
                ))}
              </div>
            </div>
          )}

          {data.overdue.length > 0 && (
            <div className="space-y-3">
              <div className="flex items-baseline gap-2">
                <h3 className="text-warning text-sm font-semibold">
                  Pendientes de días anteriores
                </h3>
                <Badge variant="warning" className="tabular-nums">
                  {overdueCount}
                </Badge>
                <p className="text-muted-foreground text-xs">
                  Recibidas en jornadas previas que aún no se procesan.
                </p>
              </div>
              <div className="grid gap-3 lg:grid-cols-2">
                {data.overdue.map((g) => (
                  <GroupCard
                    key={g.sampleTypeId}
                    group={g}
                    overdue
                    onOpenSample={setDetailId}
                  />
                ))}
              </div>
            </div>
          )}
        </>
      )}

      <SampleDetailDialog
        sampleId={detailId}
        onOpenChange={(open) => !open && setDetailId(null)}
      />
    </div>
  );
}
