import { useState } from "react";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  useCancelAppointment,
  useSetAppointmentStatus,
} from "@/hooks/queries/use-appointments";
import { getErrorMessage } from "@/lib/api";
import { cn } from "@/lib/utils";
import type { AppointmentListItem } from "@/bindings";
import { formatTime, parseScheduledAt } from "./agenda-week";
import {
  appointmentActions,
  statusLabel,
  type AppointmentAction,
  type AppointmentActionId,
} from "./appointment-actions";

/** Estado al que lleva cada acción de `set_appointment_status`. */
const TARGET_STATUS: Partial<Record<AppointmentActionId, string>> = {
  confirm: "CONFIRMADA",
  "no-show": "NO_ASISTIO",
};

/**
 * Detalle de una cita con las acciones que admite su estado.
 *
 * El clic en la tarjeta de la rejilla abre este diálogo en vez de disparar una
 * acción: la rejilla es estrecha y una tarjeta de 15 min no da para varios
 * botones. Aquí caben los datos completos, así que confirmar, hacer check-in,
 * marcar no asistió y anular se ofrecen solo cuando el backend los acepta (la
 * matriz vive en appointment-actions.ts).
 *
 * Anular pide un segundo clic: libera el horario y no se puede reabrir.
 */
export function AppointmentDetailDialog({
  appointment,
  onOpenChange,
  onCheckIn,
  onOpenHistory,
}: {
  /** Cita seleccionada; `null` cierra el diálogo. */
  appointment: AppointmentListItem | null;
  onOpenChange: (open: boolean) => void;
  /** El check-in lo hace la agenda, que es quien evita dispararlo dos veces. */
  onCheckIn: (appt: AppointmentListItem) => Promise<void>;
  onOpenHistory: (appt: AppointmentListItem) => void;
}) {
  const setStatus = useSetAppointmentStatus();
  const cancel = useCancelAppointment();
  // El id en vez de un booleano: así el estado no sobrevive al cambiar de cita.
  const [askingCancelId, setAskingCancelId] = useState<number | null>(null);

  const busy = setStatus.isPending || cancel.isPending;

  const runAction = async (
    action: AppointmentAction,
    appt: AppointmentListItem,
  ) => {
    try {
      if (action.id === "cancel") {
        await cancel.mutateAsync(appt.id);
        toast.success("Cita anulada", { description: appt.code });
      } else if (action.id === "checkin") {
        await onCheckIn(appt);
        toast.success("Check-in realizado", {
          description: `${appt.code} · visita creada`,
        });
      } else {
        await setStatus.mutateAsync({
          id: appt.id,
          input: { status: TARGET_STATUS[action.id] ?? appt.status, notes: null },
        });
        toast.success(
          action.id === "confirm"
            ? "Cita confirmada"
            : "Cita cerrada como no asistió",
          { description: appt.code },
        );
      }
      setAskingCancelId(null);
    } catch (e) {
      toast.error(
        action.id === "checkin"
          ? "No se pudo hacer el check-in"
          : action.id === "cancel"
            ? "No se pudo anular la cita"
            : "No se pudo actualizar la cita",
        { description: getErrorMessage(e) },
      );
    }
  };

  return (
    <Dialog
      open={appointment !== null}
      onOpenChange={(open) => {
        // Cerrar el diálogo también cancela una anulación a medias.
        setAskingCancelId(null);
        onOpenChange(open);
      }}
    >
      {appointment && (
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Cita {appointment.code}</DialogTitle>
            <DialogDescription>
              Revisa los datos de la cita y actualiza su estado.
            </DialogDescription>
          </DialogHeader>

          <AppointmentData appointment={appointment} />

          {appointmentActions(appointment.status).length === 0 && (
            <p className="text-muted-foreground text-sm">
              La cita está cerrada: no admite más cambios de estado.
            </p>
          )}

          <DialogFooter className="gap-2 sm:items-center">
            {askingCancelId === appointment.id ? (
              <>
                <p className="text-muted-foreground text-sm sm:mr-auto">
                  ¿Anular la cita? El horario queda libre y no se puede reabrir.
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={busy}
                  onClick={() => setAskingCancelId(null)}
                >
                  Volver
                </Button>
                <Button
                  variant="destructive"
                  size="sm"
                  disabled={busy}
                  onClick={() => void runAction(CANCEL_ACTION, appointment)}
                >
                  {busy ? (
                    <Loader2 className="size-3.5 animate-spin" />
                  ) : (
                    "Sí, anular"
                  )}
                </Button>
              </>
            ) : (
              <>
                <Button
                  variant="outline"
                  size="sm"
                  className="sm:mr-auto"
                  onClick={() => onOpenHistory(appointment)}
                >
                  Ver historial clínico
                </Button>
                {appointmentActions(appointment.status).map((action) => (
                  <Button
                    key={action.id}
                    size="sm"
                    variant={
                      action.id === "confirm"
                        ? "default"
                        : action.id === "checkin"
                          ? "secondary"
                          : "outline"
                    }
                    className={cn(
                      action.id === "cancel" && "text-destructive",
                    )}
                    disabled={busy}
                    onClick={() =>
                      action.id === "cancel"
                        ? setAskingCancelId(appointment.id)
                        : void runAction(action, appointment)
                    }
                  >
                    {action.label}
                  </Button>
                ))}
              </>
            )}
          </DialogFooter>
        </DialogContent>
      )}
    </Dialog>
  );
}

const CANCEL_ACTION: AppointmentAction = { id: "cancel", label: "Anular cita" };

function AppointmentData({
  appointment,
}: {
  appointment: AppointmentListItem;
}) {
  const { day, minutes } = parseScheduledAt(appointment.scheduledAt);
  // `scheduledAt` es fecha local sin zona: se arma el Date a mano para no
  // desplazar el día al interpretarlo como UTC.
  const [year, month, date] = day.split("-").map(Number);
  const dayLabel = new Date(year, month - 1, date).toLocaleDateString("es-CO", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });

  return (
    <dl className="grid grid-cols-[6.5rem_1fr] gap-x-3 gap-y-2 text-sm">
      <dt className="text-muted-foreground">Estado</dt>
      <dd className="font-medium">{statusLabel(appointment.status)}</dd>

      <dt className="text-muted-foreground">Paciente</dt>
      <dd>
        {appointment.patientName} · {appointment.ownerName}
      </dd>

      <dt className="text-muted-foreground">Servicio</dt>
      <dd>
        {appointment.serviceName} · {appointment.categoryName}
      </dd>

      <dt className="text-muted-foreground">Profesional</dt>
      <dd>{appointment.professionalName ?? "Sin asignar"}</dd>

      <dt className="text-muted-foreground">Horario</dt>
      <dd>
        {dayLabel} · {formatTime(minutes)} · {appointment.durationMin} min
      </dd>
    </dl>
  );
}
