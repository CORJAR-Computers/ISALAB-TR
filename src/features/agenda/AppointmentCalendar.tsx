import { useMemo, useRef, useState } from "react";
import { ChevronLeft, ChevronRight, Clock, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { QueryErrorState } from "@/components/query-error-state";
import { useAppointments, useCheckInAppointment } from "@/hooks/use-queries";
import { NewAppointmentDialog } from "./NewAppointmentDialog";
import { AppointmentDetailDialog } from "./AppointmentDetailDialog";
import { statusLabel } from "./appointment-actions";
import { cn } from "@/lib/utils";
import {
  addDays,
  buildWeek,
  formatHour,
  formatTime,
  parseScheduledAt,
} from "./agenda-week";
import { toast } from "sonner";
import { useUiStore } from "@/stores/ui-store";
import { usePermissions } from "@/hooks/use-permissions";
import type { AppointmentListItem } from "@/bindings";

/** Alto de una hora en la rejilla. */
const HOUR_HEIGHT = 48;
/** Alto mínimo de una tarjeta: una línea de texto, para citas de 15 min. */
const MIN_CARD_HEIGHT = 20;

/** Clases por estado: el color es la señal de estado en la rejilla. */
const STATUS_CLASS: Record<string, string> = {
  PROGRAMADA: "bg-secondary text-secondary-foreground border-secondary/40",
  CONFIRMADA: "bg-primary text-primary-foreground border-primary/30",
  EN_ATENCION: "bg-primary text-primary-foreground border-primary/30",
  ATENDIDA: "bg-success text-success-foreground border-success/30",
  NO_ASISTIO: "bg-destructive text-destructive-foreground border-destructive/30",
  CANCELADA: "bg-muted text-muted-foreground border-muted/40",
};

export function AppointmentCalendar() {
  const navigate = useUiStore((s) => s.navigate);
  const setActivePatient = useUiStore((s) => s.setActivePatient);
  const { canReceive } = usePermissions();
  const [viewDate, setViewDate] = useState(() => new Date());
  const [dialogOpen, setDialogOpen] = useState(false);
  /** Cita abierta en el diálogo de detalle; `null` lo cierra. */
  const [selectedId, setSelectedId] = useState<number | null>(null);
  // El check-in crea una visita, así que solo se dispara una vez por cita
  // mientras el listado no refleje el estado nuevo. Un `isPending` de la
  // mutación no sirve: con dos clics seguidos la primera petición puede
  // terminar antes del segundo y volvería a lanzarse. Si falla, se libera para
  // permitir reintentar.
  const checkedIn = useRef(new Set<number>());

  const checkIn = useCheckInAppointment();

  const {
    data: appointments = [],
    isLoading,
    isError,
    error,
    refetch,
  } = useAppointments(null, null, null, true);

  // La semana (días, rango horario, agrupación y carriles) se calcula en
  // agenda-week.ts, que tiene sus propios tests.
  const { days, byDay, startHour, endHour } = useMemo(
    () => buildWeek(viewDate, appointments),
    [appointments, viewDate],
  );

  const hours = useMemo(
    () => Array.from({ length: endHour - startHour }, (_, i) => startHour + i),
    [startHour, endHour],
  );
  const gridHeight = hours.length * HOUR_HEIGHT;

  // La cita se busca en el listado (y no se guarda en el estado) para que el
  // diálogo muestre el estado nuevo en cuanto la mutación invalida la consulta.
  const selected =
    selectedId === null
      ? null
      : (appointments.find((appt) => appt.id === selectedId) ?? null);

  const handleCheckIn = async (appt: AppointmentListItem) => {
    if (checkedIn.current.has(appt.id)) return;
    checkedIn.current.add(appt.id);
    try {
      await checkIn.mutateAsync(appt.id);
    } catch (e) {
      checkedIn.current.delete(appt.id);
      // El diálogo es quien informa del error; aquí solo se libera el guardián.
      throw e;
    }
  };

  const openHistory = (appt: AppointmentListItem) => {
    setSelectedId(null);
    setActivePatient(appt.patientId);
    navigate("clinical-history");
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Agenda de citas
          </h2>
          <p className="text-muted-foreground text-sm">
            Citas de cualquier servicio del catálogo. Haz clic en una cita para
            ver su detalle, confirmarla o anularla.
          </p>
        </div>
        <div className="flex items-end gap-2">
          <button
            type="button"
            onClick={() => setViewDate((d) => addDays(d, -7))}
            aria-label="Semana anterior"
            className="btn btn-outline btn-sm"
          >
            <ChevronLeft className="size-4" />
          </button>
          <span className="mr-2 text-sm font-medium">
            {days[0].date.toLocaleDateString("es-CO", {
              weekday: "short",
              day: "numeric",
            })}{" "}
            -{" "}
            {days[6].date.toLocaleDateString("es-CO", {
              weekday: "short",
              day: "numeric",
              month: "short",
            })}
          </span>
          <button
            type="button"
            onClick={() => setViewDate((d) => addDays(d, 7))}
            aria-label="Semana siguiente"
            className="btn btn-outline btn-sm"
          >
            <ChevronRight className="size-4" />
          </button>
          {canReceive && (
            <Button onClick={() => setDialogOpen(true)}>
              <Plus className="size-4" />
              Nueva cita
            </Button>
          )}
        </div>
      </div>

      {isError && (
        <QueryErrorState
          what="la agenda de citas"
          error={error}
          onRetry={() => void refetch()}
        />
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Clock className="size-4" />
            Semana
          </CardTitle>
        </CardHeader>
        {/* La rejilla (días, horas y líneas) no depende de los datos: se pinta
            siempre y al cargar solo se sustituyen las citas por un esqueleto,
            así el calendario no cambia de tamaño al llegar los datos. */}
        <CardContent className="p-0" aria-busy={isLoading}>
          {/* Cabecera: esquina vacía + días de la semana */}
          <div className="flex border-b border-border">
            <div className="w-20 shrink-0 border-r border-border" />
            <div className="grid flex-1 grid-cols-7">
              {days.map(({ date, key }) => (
                <div
                  key={key}
                  className="border-r border-border p-1 text-center last:border-r-0"
                >
                  <div className="text-[10px] font-medium text-muted-foreground">
                    {date.toLocaleDateString("es-CO", { weekday: "short" })}
                  </div>
                  <div className="text-xs font-semibold">{date.getDate()}</div>
                </div>
              ))}
            </div>
          </div>

          {/* Cuerpo: columna de horas + rejilla de la semana */}
          <div className="flex">
            <div className="w-20 shrink-0 border-r border-border">
              {hours.map((h) => (
                <div
                  key={h}
                  className="border-b border-border pr-2 pt-0.5 text-right text-[10px] text-muted-foreground"
                  style={{ height: HOUR_HEIGHT }}
                >
                  {formatHour(h)}
                </div>
              ))}
            </div>

            <div className="grid flex-1 grid-cols-7">
              {days.map(({ key }) => (
                <div
                  key={key}
                  data-testid="agenda-day-column"
                  className="relative border-r border-border last:border-r-0"
                  style={{ height: gridHeight }}
                >
                  {/* Líneas horarias de fondo */}
                  {hours.map((h) => (
                    <div
                      key={h}
                      className="border-b border-border/60"
                      style={{ height: HOUR_HEIGHT }}
                    />
                  ))}

                  {isLoading ? (
                    <Skeleton className="absolute top-24 left-1 right-1 h-12" />
                  ) : (
                    (byDay.get(key) ?? []).map(({ appt, lane, lanes, span }) => {
                      const { minutes } = parseScheduledAt(appt.scheduledAt);
                      const detail = `${formatTime(minutes)} · ${appt.code} · ${appt.patientName} · ${appt.serviceName} · ${statusLabel(appt.status)}`;
                      const top =
                        ((minutes - startHour * 60) / 60) * HOUR_HEIGHT;
                      // Ancho del carril: las citas del mismo clúster se
                      // reparten el ancho de la columna en partes iguales, y
                      // cada tarjeta mide `span` carriles (los que quedaron
                      // libres a su derecha durante toda su franja).
                      const laneWidth = 100 / lanes;
                      // Con tarjetas estrechas la hora se trunca y deja de
                      // leerse el nombre: ahí manda el nombre (la hora ya la da
                      // la posición vertical) y el resto va en el tooltip.
                      const compact = span / lanes <= 0.5;
                      // Una cita que termina después de la última hora de la
                      // rejilla (p. ej. 23:45 + 60 min) se recorta al borde
                      // inferior en vez de salirse del calendario.
                      const height = Math.max(
                        Math.min(
                          Math.max(
                            (Math.max(appt.durationMin, 0) / 60) * HOUR_HEIGHT,
                            MIN_CARD_HEIGHT,
                          ),
                          gridHeight - top,
                        ),
                        8,
                      );
                      return (
                        <button
                          type="button"
                          key={appt.id}
                          onClick={() => setSelectedId(appt.id)}
                          title={detail}
                          aria-label={detail}
                          aria-haspopup="dialog"
                          className={cn(
                            "absolute overflow-hidden rounded border py-px text-left shadow-sm transition-colors hover:brightness-95 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none",
                            compact ? "px-1" : "px-1.5",
                            STATUS_CLASS[appt.status] ??
                              "bg-secondary text-secondary-foreground border-secondary/40",
                          )}
                          style={{
                            top,
                            height,
                            left: `calc(${lane * laneWidth}% + 2px)`,
                            width: `calc(${span * laneWidth}% - 4px)`,
                          }}
                        >
                          {/* Una sola línea: en columnas de ~100px dos líneas se
                              recortan a media altura en las citas cortas. El
                              resto del detalle va en el tooltip. */}
                          <div
                            className={cn(
                              "truncate leading-tight",
                              compact ? "text-[9px]" : "text-[10px]",
                            )}
                          >
                            {compact ? (
                              <span className="font-semibold">
                                {appt.patientName}
                              </span>
                            ) : (
                              <>
                                <span className="font-normal tabular-nums opacity-80">
                                  {formatTime(minutes)}
                                </span>{" "}
                                <span className="font-semibold">
                                  {appt.patientName}
                                </span>
                              </>
                            )}
                          </div>
                        </button>
                      );
                    })
                  )}
                </div>
              ))}
            </div>
          </div>
        </CardContent>
      </Card>

      <NewAppointmentDialog open={dialogOpen} onOpenChange={setDialogOpen} />

      <AppointmentDetailDialog
        appointment={selected}
        onOpenChange={(open) => {
          if (!open) setSelectedId(null);
        }}
        onCheckIn={handleCheckIn}
        onOpenHistory={openHistory}
      />
    </div>
  );
}
