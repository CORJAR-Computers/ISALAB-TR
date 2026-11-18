import { useState } from "react";
import { toast } from "sonner";
import { ClipboardList, DoorOpen, Loader2, LogOut, Plus, Stethoscope } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useAddStayEvent,
  useCancelStay,
  useCheckOutStay,
  useStay,
  useStayEvents,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { STAY_EVENT_KIND, STAY_KIND, STAY_STATUS } from "@/lib/status";
import { formatCOP, formatDateTime } from "@/lib/utils";

const EVENT_KINDS = [
  "COMIDA",
  "MEDICACION",
  "PASEO",
  "LIMPIEZA",
  "NOTA",
  "VETERINARIO",
] as const;

export function StayDetailDialog({
  open,
  onOpenChange,
  stayId,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  stayId: number | null;
}) {
  const { data: stay } = useStay(open ? stayId : null);
  const { data: events = [] } = useStayEvents(open ? stayId : null);
  const checkOut = useCheckOutStay();
  const cancel = useCancelStay();
  const addEvent = useAddStayEvent();

  const [eventKind, setEventKind] = useState<string>("COMIDA");
  const [eventNotes, setEventNotes] = useState("");
  const [checkoutNotes, setCheckoutNotes] = useState("");

  const isOpen = stay?.status === "ABIERTA";

  const submitEvent = async () => {
    if (!stay) return;
    try {
      await addEvent.mutateAsync({
        stayId: stay.id,
        input: {
          kind: eventKind,
          occurredAt: null,
          notes: eventNotes.trim() || null,
        },
      });
      setEventNotes("");
      toast.success("Evento registrado en la bitácora");
    } catch (e) {
      toast.error("No se pudo registrar el evento", {
        description: getErrorMessage(e),
      });
    }
  };

  const doCheckOut = async () => {
    if (!stay) return;
    try {
      await checkOut.mutateAsync({
        id: stay.id,
        input: { checkOutAt: null, notes: checkoutNotes.trim() || null },
      });
      setCheckoutNotes("");
      toast.success("Check-out registrado", {
        description: `${stay.code} · ${stay.patientName}`,
      });
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo cerrar la estancia", {
        description: getErrorMessage(e),
      });
    }
  };

  const doCancel = async () => {
    if (!stay) return;
    try {
      await cancel.mutateAsync(stay.id);
      toast.success("Estancia cancelada", {
        description: `${stay.code} · ${stay.patientName}`,
      });
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo cancelar la estancia", {
        description: getErrorMessage(e),
      });
    }
  };

  const statusMeta = stay ? STAY_STATUS[stay.status] : undefined;
  const kindMeta = stay ? STAY_KIND[stay.kind] : undefined;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <DoorOpen className="text-primary size-5" />
            {stay ? stay.code : "Estancia"}
          </DialogTitle>
          <DialogDescription>
            {stay
              ? `${stay.patientName} · ${stay.ownerName} · ${kindMeta?.label ?? stay.kind}`
              : "Cargando los datos de la estancia…"}
          </DialogDescription>
        </DialogHeader>

        {stay && (
          <div className="space-y-4">
            {/* Ficha */}
            <div className="grid gap-3 rounded-lg border p-3 text-sm sm:grid-cols-2">
              <div className="flex items-center gap-2">
                <Badge variant={statusMeta?.variant ?? "secondary"}>
                  {statusMeta?.label ?? stay.status}
                </Badge>
                <Badge variant={kindMeta?.variant ?? "secondary"}>
                  {kindMeta?.label ?? stay.kind}
                </Badge>
              </div>
              <p className="text-muted-foreground sm:text-right">
                Recurso: {stay.resourceName ?? "sin asignar"}
              </p>
              <p>
                <span className="text-muted-foreground">Ingreso:</span>{" "}
                {formatDateTime(stay.checkInAt)}
              </p>
              <p>
                <span className="text-muted-foreground">Salida:</span>{" "}
                {stay.checkOutAt ? formatDateTime(stay.checkOutAt) : "—"}
              </p>
              <p>
                <span className="text-muted-foreground">Días:</span> {stay.days}
              </p>
              <p className="text-right tabular-nums">
                <span className="text-muted-foreground">Estimado:</span>{" "}
                {formatCOP(stay.estimatedTotal ?? 0)}
              </p>
              {stay.ownerPhone && (
                <p className="sm:col-span-2">
                  <span className="text-muted-foreground">Teléfono:</span>{" "}
                  {stay.ownerPhone}
                </p>
              )}
              {stay.notes && (
                <p className="text-muted-foreground sm:col-span-2">
                  {stay.notes}
                </p>
              )}
            </div>

            {/* Bitácora de cuidados */}
            <div className="space-y-2">
              <p className="flex items-center gap-2 text-sm font-semibold">
                <ClipboardList className="text-primary size-4" />
                Bitácora de cuidados
              </p>

              {isOpen && (
                <div className="flex flex-col gap-2 rounded-lg border p-3 sm:flex-row sm:items-start">
                  <div className="flex flex-1 flex-col gap-2">
                    <Select value={eventKind} onValueChange={setEventKind}>
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {EVENT_KINDS.map((k) => (
                          <SelectItem key={k} value={k}>
                            {STAY_EVENT_KIND[k]}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <Textarea
                      value={eventNotes}
                      onChange={(e) => setEventNotes(e.target.value)}
                      placeholder="Detalle del evento…"
                      className="min-h-14"
                    />
                  </div>
                  <Button
                    type="button"
                    onClick={submitEvent}
                    disabled={addEvent.isPending}
                    className="sm:self-end"
                  >
                    {addEvent.isPending ? (
                      <Loader2 className="animate-spin" />
                    ) : (
                      <Plus className="size-4" />
                    )}
                    Registrar
                  </Button>
                </div>
              )}

              {events.length === 0 ? (
                <p className="text-muted-foreground rounded-lg border border-dashed py-4 text-center text-sm">
                  Sin eventos registrados.
                </p>
              ) : (
                <ul className="divide-y rounded-lg border">
                  {events.map((ev) => (
                    <li key={ev.id} className="flex items-start gap-3 px-3 py-2">
                      <Stethoscope className="text-muted-foreground mt-0.5 size-4 shrink-0" />
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium">
                          {STAY_EVENT_KIND[ev.kind] ?? ev.kind}
                          {ev.authorName && (
                            <span className="text-muted-foreground font-normal">
                              {" "}
                              · {ev.authorName}
                            </span>
                          )}
                        </p>
                        {ev.notes && (
                          <p className="text-muted-foreground text-xs">{ev.notes}</p>
                        )}
                      </div>
                      <span className="text-muted-foreground shrink-0 text-xs">
                        {formatDateTime(ev.occurredAt)}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </div>

            {/* Check-out */}
            {isOpen && (
              <div className="space-y-2 rounded-lg border p-3">
                <Label htmlFor="stay-checkout-notes">Notas de salida</Label>
                <Textarea
                  id="stay-checkout-notes"
                  value={checkoutNotes}
                  onChange={(e) => setCheckoutNotes(e.target.value)}
                  placeholder="Estado del paciente al salir, objetos entregados…"
                  className="min-h-16"
                />
              </div>
            )}
          </div>
        )}

        <DialogFooter className="flex-wrap gap-2">
          {stay && isOpen && (
            <>
              <Button
                type="button"
                variant="outline"
                onClick={doCancel}
                disabled={cancel.isPending}
                className="text-destructive"
              >
                {cancel.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <LogOut className="size-4" />
                )}
                Cancelar estancia
              </Button>
              <Button
                type="button"
                onClick={doCheckOut}
                disabled={checkOut.isPending}
              >
                {checkOut.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <LogOut className="size-4" />
                )}
                Registrar salida
              </Button>
            </>
          )}
          <Button
            type="button"
            variant="ghost"
            onClick={() => onOpenChange(false)}
          >
            Cerrar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
