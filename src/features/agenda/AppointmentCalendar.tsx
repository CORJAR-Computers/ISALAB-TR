import { useMemo, useState } from "react";
import { ChevronLeft, ChevronRight, Clock, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { QueryErrorState } from "@/components/query-error-state";
import {
  useAppointments,
  useCheckInAppointment,
} from "@/hooks/use-queries";
import {
  NewAppointmentDialog,
} from "./NewAppointmentDialog";
import { getErrorMessage } from "@/lib/api";
import { cn, localDateISO } from "@/lib/utils";
import { toast } from "sonner";
import { useUiStore } from "@/stores/ui-store";
import { usePermissions } from "@/hooks/use-permissions";

const HOURS = Array.from({ length: 16 }, (_, i) => i + 7); // 07:00 → 22:00

const STATUS_BADGE: Record<string, { label: string; variant: string }> = {
  PROGRAMADA: { label: "Programada", variant: "secondary" },
  CONFIRMADA: { label: "Confirmada", variant: "primary" },
  EN_ATENCION: { label: "En atención", variant: "primary" },
  ATENDIDA: { label: "Atendida", variant: "success" },
  NO_ASISTIO: { label: "No asistió", variant: "destructive" },
  CANCELADA: { label: "Cancelada", variant: "ghost" },
};

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

function formatHour(h: number): string {
  return h.toString().padStart(2, "0") + ":00";
}

function fmtDay(date: Date): string {
  // Fecha local (toISOString da la fecha UTC: de noche apunta al día siguiente).
  return localDateISO(date);
}

export function AppointmentCalendar() {
  const navigate = useUiStore((s) => s.navigate);
  const setActivePatient = useUiStore((s) => s.setActivePatient);
  const { canReceive } = usePermissions();
  const [viewDate, setViewDate] = useState(new Date());
  const [dialogOpen, setDialogOpen] = useState(false);

  const weekStart = startOfDay(addDays(viewDate, -viewDate.getDay()));
  const days = Array.from({ length: 7 }, (_, i) => addDays(weekStart, i));

  const checkIn = useCheckInAppointment();

  const {
    data: appointments = [],
    isLoading,
    isError,
    error,
    refetch,
  } = useAppointments(null, null, null, true);

  // Citas que caen en la semana visible.
  const weekAppts = useMemo(() => {
    const byDay: Record<string, typeof appointments> = {};
    for (const d of days) byDay[fmtDay(d)] = [];
    for (const a of appointments) {
      const day = a.scheduledAt.slice(0, 10);
      if (byDay[day]) byDay[day].push(a);
    }
    return byDay;
  }, [appointments, days]);

  const prevWeek = () => setViewDate(addDays(viewDate, -7));
  const nextWeek = () => setViewDate(addDays(viewDate, 7));

  const handleCheckIn = async (id: number) => {
    try {
      await checkIn.mutateAsync(id);
    } catch (e) {
      toast.error("No se pudo hacer el check-in", {
        description: getErrorMessage(e),
      });
    }
  };

  const openPatientHistory = (patientId: number) => {
    setActivePatient(patientId);
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
            Citas de cualquier servicio del catálogo. El check-in crea la
            visita vinculada.
          </p>
        </div>
        <div className="flex items-end gap-2">
          <button
            type="button"
            onClick={prevWeek}
            className="btn btn-outline btn-sm"
          >
            <ChevronLeft className="size-4" />
          </button>
          <span className="text-sm font-medium mr-2">
            {days[0].toLocaleDateString("es-CO", { weekday: "short", day: "numeric" })} -
            {days[6].toLocaleDateString("es-CO", { weekday: "short", day: "numeric", month: "short" })}
          </span>
          <button
            type="button"
            onClick={nextWeek}
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
        <CardContent className="p-0">
          <div className="relative">
            {/* Columnas de horas */}
            <div className="absolute left-0 top-0 bottom-0 w-20 border-r border-border">
              <div className="h-full grid grid-rows-16">
                {HOURS.map((h) => (
                  <div
                    key={h}
                    className="border-b border-border flex items-center justify-end px-2 text-[10px] text-muted-foreground"
                  >
                    {formatHour(h)}
                  </div>
                ))}
              </div>
            </div>

            {/* Días de la semana */}
            <div className="absolute left-20 right-0 top-0 bottom-0 grid grid-cols-7 border-r border-border">
              {days.map((day) => (
                <div
                  key={fmtDay(day)}
                  className="border-b border-border p-1 text-center"
                >
                  <div className="text-[10px] font-medium text-muted-foreground">
                    {day.toLocaleDateString("es-CO", { weekday: "short" })}
                  </div>
                  <div className="text-xs font-semibold">
                    {day.getDate()}
                  </div>
                </div>
              ))}
            </div>

            {/* Grid de citas */}
            <div className="absolute left-20 right-0 top-0 bottom-0 grid grid-cols-7">
              {days.map((day) => (
                <div
                  key={fmtDay(day)}
                  className="border-b border-border h-full relative"
                >
                  {isLoading ? (
                    <Skeleton className="h-16 w-full mx-2 mt-1" />
                  ) : (
                    <>
                      {weekAppts[fmtDay(day)].map((appt) => {
                        const start = new Date(appt.scheduledAt);
                        const startHour = start.getHours() + start.getMinutes() / 60;
                        const top = (startHour - 7) * 30;
                        const height = Math.max(
                          (appt.durationMin / 60) * 30,
                          30,
                        );
                        const st =
                          STATUS_BADGE[appt.status] ?? {
                            label: appt.status,
                            variant: "secondary",
                          };
                        return (
                          <div
                            key={appt.id}
                            className={cn(
                              "absolute left-2 right-2 rounded-md px-2 py-1 text-xs font-medium shadow-sm cursor-pointer border border-white/60",
                              st.variant === "success"
                                ? "bg-success text-success-foreground border-success/30"
                                : st.variant === "destructive"
                                  ? "bg-destructive text-destructive-foreground border-destructive/30"
                                  : st.variant === "primary"
                                    ? "bg-primary text-primary-foreground border-primary/30"
                                    : st.variant === "ghost"
                                      ? "bg-muted text-muted-foreground border-muted/30"
                                      : "bg-secondary text-secondary-foreground border-secondary/30",
                            )}
                            style={{ top, height }}
                            onClick={() => {
                              if (appt.status === "PROGRAMADA" || appt.status === "CONFIRMADA") {
                                handleCheckIn(appt.id);
                              } else {
                                openPatientHistory(appt.patientId);
                              }
                            }}
                            title={appt.code + " · " + appt.patientName + " · " + appt.serviceName}
                          >
                            <div className="font-semibold truncate">
                              {appt.code}
                            </div>
                            <div className="truncate text-[9px] opacity-90">
                              {appt.patientName}
                            </div>
                            <div className="truncate text-[9px] opacity-80">
                              {appt.serviceName}
                            </div>
                            <Badge
                              variant={st.variant as
                                | "default"
                                | "secondary"
                                | "destructive"
                                | "success"
                                | "warning"
                                | "outline"}
                              className="mt-1"
                            >
                              {st.label}
                            </Badge>
                          </div>
                        );
                      })}
                    </>
                  )}
                </div>
              ))}
            </div>
          </div>
        </CardContent>
      </Card>

      <NewAppointmentDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onCreated={() => {}}
      />
    </div>
  );
}
