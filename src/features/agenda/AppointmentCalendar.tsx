import { useMemo, useState } from "react";
import { ChevronLeft, ChevronRight, Clock, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { QueryErrorState } from "@/components/query-error-state";
import { useAppointments, useCheckInAppointment } from "@/hooks/use-queries";
import { NewAppointmentDialog } from "./NewAppointmentDialog";
import { getErrorMessage } from "@/lib/api";
import { cn, localDateISO } from "@/lib/utils";
import { toast } from "sonner";
import { useUiStore } from "@/stores/ui-store";
import { usePermissions } from "@/hooks/use-permissions";
import type { AppointmentListItem } from "@/bindings";

/** Franja base de la agenda: primera hora con etiqueta. */
const BASE_START_HOUR = 7;
/** Última hora con etiqueta en la rejilla base (la rejilla llega hasta las 23:00). */
const BASE_END_HOUR = 22;
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

const STATUS_LABEL: Record<string, string> = {
  PROGRAMADA: "Programada",
  CONFIRMADA: "Confirmada",
  EN_ATENCION: "En atención",
  ATENDIDA: "Atendida",
  NO_ASISTIO: "No asistió",
  CANCELADA: "Cancelada",
};

/** Estados en los que el clic hace check-in (crea la visita); el resto abre el historial. */
const CHECK_IN_STATUSES = new Set(["PROGRAMADA", "CONFIRMADA"]);

function addDays(date: Date, days: number) {
  const d = new Date(date);
  d.setDate(d.getDate() + days);
  return d;
}

function startOfDay(date: Date) {
  const d = new Date(date);
  d.setHours(0, 0, 0, 0);
  return d;
}

/**
 * `scheduledAt` llega como `YYYY-MM-DD HH:MM:SS` (hora local, sin zona).
 * Se parsea a mano: `new Date("2026-11-23 08:30:00")` es dependiente del motor
 * (Safari lo rechaza) y el cálculo de la hora debe ser local, no UTC.
 */
function parseScheduledAt(value: string): { day: string; minutes: number } {
  const [day, time = "00:00:00"] = value.split(" ");
  const [hh = 0, mm = 0] = time.split(":").map(Number);
  return { day, minutes: hh * 60 + mm };
}

/** Minutos desde medianoche → `HH:00`. */
function formatHour(hour: number): string {
  return `${String(hour).padStart(2, "0")}:00`;
}

/** Minutos desde medianoche → `HH:MM`. */
function formatTime(minutes: number): string {
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(
    minutes % 60,
  ).padStart(2, "0")}`;
}

export function AppointmentCalendar() {
  const navigate = useUiStore((s) => s.navigate);
  const setActivePatient = useUiStore((s) => s.setActivePatient);
  const { canReceive } = usePermissions();
  const [viewDate, setViewDate] = useState(() => new Date());
  const [dialogOpen, setDialogOpen] = useState(false);

  const checkIn = useCheckInAppointment();

  const {
    data: appointments = [],
    isLoading,
    isError,
    error,
    refetch,
  } = useAppointments(null, null, null, true);

  /**
   * Días de la semana visible (domingo → sábado), citas agrupadas por día y
   * rango horario a pintar.
   *
   * El rango se amplía para incluir cualquier cita de la semana fuera de
   * 07:00–22:00: una cita de 06:30 o de 22:30 se dibuja en su hora real en vez
   * de recortarse a la primera/última franja. Con la agenda dentro de horario
   * el rango es el base.
   */
  const { days, byDay, startHour, endHour } = useMemo(() => {
    const weekStart = startOfDay(addDays(viewDate, -viewDate.getDay()));
    const days = Array.from({ length: 7 }, (_, i) => addDays(weekStart, i));

    const byDay = new Map<string, AppointmentListItem[]>();
    for (const day of days) byDay.set(localDateISO(day), []);

    let firstHour = BASE_START_HOUR;
    let lastHour = BASE_END_HOUR + 1;

    for (const appt of appointments) {
      const { day, minutes } = parseScheduledAt(appt.scheduledAt);
      const bucket = byDay.get(day);
      if (!bucket) continue; // cita de otra semana
      bucket.push(appt);
      firstHour = Math.min(firstHour, Math.floor(minutes / 60));
      lastHour = Math.max(
        lastHour,
        Math.ceil((minutes + appt.durationMin) / 60),
      );
    }

    return {
      days,
      byDay,
      startHour: Math.max(0, firstHour),
      endHour: Math.min(24, lastHour),
    };
  }, [appointments, viewDate]);

  const hours = useMemo(
    () => Array.from({ length: endHour - startHour }, (_, i) => startHour + i),
    [startHour, endHour],
  );
  const gridHeight = hours.length * HOUR_HEIGHT;

  const handleAppointmentClick = async (appt: AppointmentListItem) => {
    if (!CHECK_IN_STATUSES.has(appt.status)) {
      setActivePatient(appt.patientId);
      navigate("clinical-history");
      return;
    }
    try {
      await checkIn.mutateAsync(appt.id);
    } catch (e) {
      toast.error("No se pudo hacer el check-in", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Agenda de citas
          </h2>
          <p className="text-muted-foreground text-sm">
            Citas de cualquier servicio del catálogo. El check-in crea la
            visita vinculada.
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
            {days[0].toLocaleDateString("es-CO", {
              weekday: "short",
              day: "numeric",
            })}{" "}
            -{" "}
            {days[6].toLocaleDateString("es-CO", {
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
              {days.map((day) => (
                <div
                  key={localDateISO(day)}
                  className="border-r border-border p-1 text-center last:border-r-0"
                >
                  <div className="text-[10px] font-medium text-muted-foreground">
                    {day.toLocaleDateString("es-CO", { weekday: "short" })}
                  </div>
                  <div className="text-xs font-semibold">{day.getDate()}</div>
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
              {days.map((day) => (
                <div
                  key={localDateISO(day)}
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
                    (byDay.get(localDateISO(day)) ?? []).map((appt) => {
                      const { minutes } = parseScheduledAt(appt.scheduledAt);
                      const detail = `${formatTime(minutes)} · ${appt.code} · ${appt.patientName} · ${appt.serviceName} · ${STATUS_LABEL[appt.status] ?? appt.status}`;
                      return (
                        <button
                          type="button"
                          key={appt.id}
                          onClick={() => void handleAppointmentClick(appt)}
                          title={detail}
                          aria-label={detail}
                          className={cn(
                            "absolute left-1 right-1 overflow-hidden rounded border px-1.5 py-px text-left shadow-sm transition-colors hover:brightness-95 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none",
                            STATUS_CLASS[appt.status] ??
                              "bg-secondary text-secondary-foreground border-secondary/40",
                          )}
                          style={{
                            top:
                              ((minutes - startHour * 60) / 60) * HOUR_HEIGHT,
                            height: Math.max(
                              (appt.durationMin / 60) * HOUR_HEIGHT,
                              MIN_CARD_HEIGHT,
                            ),
                          }}
                        >
                          {/* Una línea: en columnas de ~100px dos líneas se
                              recortan a media altura en las citas cortas. El
                              resto del detalle va en el tooltip. */}
                          <div className="truncate text-[10px] leading-tight">
                            <span className="font-normal tabular-nums opacity-80">
                              {formatTime(minutes)}
                            </span>{" "}
                            <span className="font-semibold">
                              {appt.patientName}
                            </span>
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
    </div>
  );
}
