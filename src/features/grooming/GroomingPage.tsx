import { useState } from "react";
import {
  CalendarClock,
  ChevronLeft,
  ChevronRight,
  Scissors,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  useAppointments,
  useGroomingDay,
  useGroomingReminders,
} from "@/hooks/use-queries";
import { useAllServiceCategories } from "@/hooks/use-queries";
import { usePermissions } from "@/hooks/use-permissions";
import {
  GROOMING_SESSION_STATE,
  VISIT_SERVICE_STATUS,
} from "@/lib/status";
import { cn } from "@/lib/utils";
import type { GroomingDayLine } from "@/bindings";
import { GroomingSessionDialog } from "./GroomingSessionDialog";

function fmtDay(date: Date): string {
  const d = new Date(date);
  d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
  return d.toISOString().slice(0, 10);
}

function addDays(date: Date, days: number) {
  const d = new Date(date);
  d.setDate(d.getDate() + days);
  return d;
}

export function GroomingPage() {
  const { canGroom } = usePermissions();
  const [viewDate, setViewDate] = useState(new Date());
  const date = fmtDay(viewDate);
  const [selectedLine, setSelectedLine] = useState<GroomingDayLine | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  const { data: categories = [] } = useAllServiceCategories();
  // Servicios de la categoría PELUQUERIA (para filtrar las citas del día).
  const groomingCategory = categories.find(
    (c) => c.code.toUpperCase() === "PELUQUERIA",
  );

  const { data: lines = [], isLoading: loadingLines } = useGroomingDay(date);
  const { data: appointments = [], isLoading: loadingAppts } = useAppointments(
    date,
    null,
    null,
  );
  const { data: reminders = [] } = useGroomingReminders();

  const groomingAppointments = appointments.filter(
    (a) => groomingCategory != null && a.categoryId === groomingCategory.id,
  );

  const prevDay = () => setViewDate(addDays(viewDate, -1));
  const nextDay = () => setViewDate(addDays(viewDate, 1));

  const openSession = (line: GroomingDayLine) => {
    if (!canGroom) return;
    setSelectedLine(line);
    setDialogOpen(true);
  };

  const pendingCount = lines.filter(
    (l) => l.sessionState === "SIN_SESION" || l.sessionState === "EN_PROCESO",
  ).length;

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 text-xl font-semibold tracking-tight">
            <Scissors className="text-primary size-5" />
            Peluquería
          </h2>
          <p className="text-muted-foreground text-sm">
            Tablero del día: sesiones de peluquería, fichas y próximos turnos
            sugeridos.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button type="button" variant="outline" size="icon" onClick={prevDay} aria-label="Día anterior">
            <ChevronLeft className="size-4" />
          </Button>
          <span className="text-sm font-medium">
            {viewDate.toLocaleDateString("es-CO", {
              weekday: "long",
              day: "numeric",
              month: "long",
            })}
          </span>
          <Button type="button" variant="outline" size="icon" onClick={nextDay} aria-label="Día siguiente">
            <ChevronRight className="size-4" />
          </Button>
        </div>
      </div>

      {/* Próximos turnos sugeridos */}
      {reminders.length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-base">
              <CalendarClock className="text-primary size-4" />
              Próximos turnos sugeridos
            </CardTitle>
          </CardHeader>
          <CardContent className="grid gap-1.5 sm:grid-cols-2">
            {reminders.map((r) => (
              <div
                key={r.sessionId}
                className="bg-muted/40 flex items-center justify-between gap-2 rounded-md border px-3 py-2 text-sm"
              >
                <div className="min-w-0">
                  <p className="truncate font-medium">{r.patientName}</p>
                  <p className="text-muted-foreground truncate text-xs">
                    {r.serviceName} · {r.ownerName}
                  </p>
                </div>
                <Badge variant="default">{r.nextSuggestedAt}</Badge>
              </div>
            ))}
          </CardContent>
        </Card>
      )}

      {/* Sesiones del día */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center justify-between text-base">
            <span>Sesiones del día</span>
            {pendingCount > 0 && (
              <Badge variant="warning">{pendingCount} pendiente{pendingCount === 1 ? "" : "s"}</Badge>
            )}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {loadingLines ? (
            <div className="space-y-2">
              <Skeleton className="h-12 w-full" />
              <Skeleton className="h-12 w-full" />
            </div>
          ) : lines.length === 0 ? (
            <p className="text-muted-foreground py-6 text-center text-sm">
              No hay sesiones de peluquería para este día.
            </p>
          ) : (
            <ul className="divide-y">
              {lines.map((line) => {
                const sessionMeta = GROOMING_SESSION_STATE[line.sessionState] ?? {
                  label: line.sessionState,
                  variant: "secondary" as const,
                };
                const lineMeta = VISIT_SERVICE_STATUS[line.lineStatus] ?? {
                  label: line.lineStatus,
                  variant: "secondary" as const,
                };
                const clickable = canGroom && line.sessionState !== "FINALIZADA";
                return (
                  <li key={line.visitServiceId}>
                    <button
                      type="button"
                      disabled={!clickable}
                      onClick={() => openSession(line)}
                      className={cn(
                        "flex w-full items-center justify-between gap-3 px-1 py-2.5 text-left text-sm transition-colors",
                        clickable && "hover:bg-accent rounded-md",
                        !clickable && "cursor-default",
                      )}
                    >
                      <div className="min-w-0">
                        <p className="truncate font-medium">
                          {line.patientName}
                          <span className="text-muted-foreground font-normal">
                            {" "}
                            · {line.ownerName}
                          </span>
                        </p>
                        <p className="text-muted-foreground truncate text-xs">
                          {line.visitCode} · {line.serviceName} ·{" "}
                          {line.arrivedAt.slice(11, 16)}
                          {line.professionalName
                            ? ` · ${line.professionalName}`
                            : ""}
                        </p>
                      </div>
                      <div className="flex shrink-0 items-center gap-1.5">
                        {line.nextSuggestedAt && (
                          <Badge variant="outline" className="gap-1">
                            <CalendarClock className="size-3" />
                            {line.nextSuggestedAt}
                          </Badge>
                        )}
                        <Badge variant={lineMeta.variant}>
                          {lineMeta.label}
                        </Badge>
                        <Badge variant={sessionMeta.variant}>
                          {sessionMeta.label}
                        </Badge>
                      </div>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </CardContent>
      </Card>

      {/* Citas de peluquería del día (aún sin check-in) */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">Citas de peluquería del día</CardTitle>
        </CardHeader>
        <CardContent>
          {loadingAppts ? (
            <Skeleton className="h-12 w-full" />
          ) : groomingAppointments.length === 0 ? (
            <p className="text-muted-foreground py-4 text-center text-sm">
              Sin citas de peluquería para este día.
            </p>
          ) : (
            <ul className="divide-y">
              {groomingAppointments.map((a) => (
                <li
                  key={a.id}
                  className="flex items-center justify-between gap-3 py-2 text-sm"
                >
                  <div className="min-w-0">
                    <p className="truncate font-medium">
                      {a.patientName}
                      <span className="text-muted-foreground font-normal">
                        {" "}
                        · {a.ownerName}
                      </span>
                    </p>
                    <p className="text-muted-foreground truncate text-xs">
                      {a.scheduledAt.slice(11, 16)} · {a.serviceName}
                      {a.professionalName ? ` · ${a.professionalName}` : ""}
                    </p>
                  </div>
                  <Badge variant="secondary">{a.status}</Badge>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <GroomingSessionDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        line={selectedLine}
      />
    </div>
  );
}
