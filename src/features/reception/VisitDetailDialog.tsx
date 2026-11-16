import { useMemo } from "react";
import { toast } from "sonner";
import {
  Banknote,
  Check,
  CirclePlay,
  Clock,
  Plus,
  Trash2,
  X,
  Zap,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import {
  useAddVisitService,
  useRemoveVisitService,
  useServices,
  useSetVisitStatus,
  useUpdateVisitServiceStatus,
  useVisit,
} from "@/hooks/use-queries";
import { usePermissions } from "@/hooks/use-permissions";
import { getErrorMessage } from "@/lib/api";
import { VISIT_SERVICE_STATUS, VISIT_STATUS } from "@/lib/status";
import { cn, formatCOP } from "@/lib/utils";
import { CheckoutDialog } from "./CheckoutDialog";
import type { VisitListItem } from "@/bindings";

export function VisitDetailDialog({
  visitSummary,
  onClose,
}: {
  /** Fila del listado (id, código, paciente…); null = cerrado. */
  visitSummary: VisitListItem | null;
  onClose: () => void;
}) {
  const open = visitSummary != null;
  const visitId = visitSummary?.id ?? null;
  const { data: visit, isLoading } = useVisit(open ? visitId : null);
  const { data: catalogServices = [] } = useServices(null, "");
  const setStatus = useSetVisitStatus();
  const addService = useAddVisitService();
  const removeService = useRemoveVisitService();
  const setLineStatus = useUpdateVisitServiceStatus();
  const { canReceive, canBill } = usePermissions();

  const busy =
    setStatus.isPending ||
    addService.isPending ||
    removeService.isPending ||
    setLineStatus.isPending;

  const estimatedTotal = useMemo(
    () =>
      (visit?.services ?? [])
        .filter((s) => s.status !== "CANCELADA")
        .reduce((acc, s) => acc + (s.unitPrice ?? 0) * s.quantity, 0),
    [visit],
  );

  if (!open) return null;

  const visitStatus = visit?.status ?? visitSummary.status;
  const canEditLines =
    canReceive &&
    (visitStatus === "EN_ESPERA" || visitStatus === "EN_ATENCION");
  const canChangeStatus = canReceive && visitStatus !== "FACTURADA" && visitStatus !== "CANCELADA";

  const act = async (fn: () => Promise<unknown>, okMsg: string) => {
    try {
      await fn();
      toast.success(okMsg);
    } catch (e) {
      toast.error("Operación rechazada", { description: getErrorMessage(e) });
    }
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        {isLoading || !visit ? (
          <div className="space-y-3">
            <Skeleton className="h-8 w-1/2" />
            <Skeleton className="h-24 w-full" />
            <Skeleton className="h-40 w-full" />
          </div>
        ) : (
          <>
            <DialogHeader>
              <DialogTitle className="flex flex-wrap items-center gap-2">
                {visit.code}
                <Badge variant={VISIT_STATUS[visitStatus]?.variant ?? "outline"}>
                  {VISIT_STATUS[visitStatus]?.label ?? visitStatus}
                </Badge>
                {visit.priority === "URGENTE" && (
                  <Badge variant="destructive" className="gap-1">
                    <Zap className="size-3" /> Urgente
                  </Badge>
                )}
              </DialogTitle>
              <DialogDescription>
                {visit.patientName} · {visit.ownerName} · llegó {visit.arrivedAt}
              </DialogDescription>
            </DialogHeader>

            {/* Acciones de estado */}
            <div className="flex flex-wrap gap-2">
              {visitStatus === "EN_ESPERA" && canReceive && (
                <Button
                  size="sm"
                  disabled={busy}
                  onClick={() =>
                    act(
                      () => setStatus.mutateAsync({ id: visit.id, status: "EN_ATENCION" }),
                      "Visita en atención",
                    )
                  }
                >
                  <CirclePlay className="size-4" />
                  Iniciar atención
                </Button>
              )}
              {visitStatus === "EN_ATENCION" && canReceive && (
                <Button
                  size="sm"
                  disabled={busy}
                  onClick={() =>
                    act(
                      () => setStatus.mutateAsync({ id: visit.id, status: "FINALIZADA" }),
                      "Visita finalizada",
                    )
                  }
                >
                  <Check className="size-4" />
                  Finalizar visita
                </Button>
              )}
              {visitStatus === "FINALIZADA" && canBill && (
                <CheckoutDialog
                  visitId={visit.id}
                  triggerLabel={`Cobrar ${formatCOP(estimatedTotal)}`}
                />
              )}
              {canChangeStatus && (
                <Button
                  size="sm"
                  variant="outline"
                  disabled={busy}
                  onClick={() =>
                    act(
                      () => setStatus.mutateAsync({ id: visit.id, status: "CANCELADA" }),
                      "Visita cancelada",
                    )
                  }
                >
                  <X className="size-4" />
                  Cancelar visita
                </Button>
              )}
            </div>

            {visit.notes && (
              <p className="text-muted-foreground rounded-lg bg-muted/40 px-3 py-2 text-sm">
                {visit.notes}
              </p>
            )}

            {/* Líneas de servicio */}
            <div className="space-y-1 rounded-lg border">
              <div className="grid grid-cols-[1fr_auto_auto_auto] items-center gap-2 bg-muted/40 px-3 py-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                <span>Servicio</span>
                <span className="w-16 text-center">Cant.</span>
                <span className="w-24 text-right">Precio</span>
                <span className="w-20" />
              </div>
              {visit.services.map((line) => {
                const st = VISIT_SERVICE_STATUS[line.status];
                return (
                  <div
                    key={line.id}
                    className={cn(
                      "grid grid-cols-[1fr_auto_auto_auto] items-center gap-2 border-b px-3 py-2 last:border-b-0",
                      line.status === "CANCELADA" && "opacity-50 line-through",
                    )}
                  >
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium">
                        {line.serviceName}
                      </p>
                      <div className="flex items-center gap-2">
                        <Badge variant={st?.variant ?? "outline"} className="h-5 px-1.5 text-[10px]">
                          {st?.label ?? line.status}
                        </Badge>
                        {line.professionalName && (
                          <span className="text-muted-foreground truncate text-xs">
                            {line.professionalName}
                          </span>
                        )}
                      </div>
                    </div>
                    <span className="w-16 text-center text-sm tabular-nums">
                      {line.quantity}
                    </span>
                    <span className="w-24 text-right text-sm tabular-nums">
                      {formatCOP(line.unitPrice ?? 0)}
                    </span>
                    <div className="flex w-20 justify-end gap-0.5">
                      {canEditLines && line.status === "PENDIENTE" && (
                        <>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="size-7"
                            disabled={busy}
                            title="Iniciar línea"
                            onClick={() =>
                              act(
                                () =>
                                  setLineStatus.mutateAsync({
                                    visitId: visit.id,
                                    lineId: line.id,
                                    input: { status: "EN_PROCESO", professionalId: null },
                                  }),
                                "Línea en proceso",
                              )
                            }
                          >
                            <Clock className="size-3.5" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="size-7"
                            disabled={busy}
                            title="Quitar línea"
                            onClick={() =>
                              act(
                                () =>
                                  removeService.mutateAsync({
                                    visitId: visit.id,
                                    lineId: line.id,
                                  }),
                                "Línea quitada",
                              )
                            }
                          >
                            <Trash2 className="size-3.5 text-muted-foreground" />
                          </Button>
                        </>
                      )}
                      {canEditLines &&
                        (line.status === "PENDIENTE" || line.status === "EN_PROCESO") && (
                          <>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="size-7"
                              disabled={busy}
                              title={
                                line.status === "PENDIENTE"
                                  ? "Iniciar y completar línea"
                                  : "Completar línea"
                              }
                              onClick={() =>
                                act(
                                  () =>
                                    setLineStatus.mutateAsync({
                                      visitId: visit.id,
                                      lineId: line.id,
                                      input: {
                                        status:
                                          line.status === "PENDIENTE"
                                            ? "EN_PROCESO"
                                            : "COMPLETADA",
                                        professionalId: null,
                                      },
                                    }),
                                "Línea actualizada",
                              )
                            }
                          >
                            <Check className="size-3.5 text-success" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="size-7"
                            disabled={busy}
                            title="Cancelar línea"
                            onClick={() =>
                              act(
                                () =>
                                  setLineStatus.mutateAsync({
                                    visitId: visit.id,
                                    lineId: line.id,
                                    input: { status: "CANCELADA", professionalId: null },
                                  }),
                                "Línea cancelada",
                              )
                            }
                          >
                            <X className="size-3.5 text-destructive" />
                          </Button>
                        </>
                      )}
                    </div>
                  </div>
                );
              })}
              <div className="flex items-center justify-between bg-muted/40 px-3 py-2 text-sm font-semibold">
                <span>Total estimado</span>
                <span className="tabular-nums">{formatCOP(estimatedTotal)}</span>
              </div>
            </div>

            {/* Agregar servicio */}
            {canEditLines && (
              <div className="flex items-center gap-2">
                <Select
                  value=""
                  onValueChange={(v) => {
                    const service = catalogServices.find((s) => s.id === Number(v));
                    if (!service) return;
                    act(
                      () =>
                        addService.mutateAsync({
                          visitId: visit.id,
                          input: {
                            serviceId: service.id,
                            professionalId: null,
                            quantity: 1,
                            notes: null,
                          },
                        }),
                      `${service.name} agregado a la visita`,
                    );
                  }}
                >
                  <SelectTrigger className="w-full" disabled={busy}>
                    <SelectValue placeholder={<span className="flex items-center gap-2"><Plus className="size-3.5" /> Agregar servicio del catálogo…</span>} />
                  </SelectTrigger>
                  <SelectContent className="max-h-64">
                    {catalogServices.map((s) => (
                      <SelectItem key={s.id} value={s.id.toString()}>
                        {s.name} · {formatCOP(s.price ?? 0)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}

            {visitStatus === "FACTURADA" && (
              <p className="text-muted-foreground flex items-center gap-2 text-sm">
                <Banknote className="size-4" />
                Visita cobrada: los pagos y la factura se gestionan en
                Facturación.
              </p>
            )}
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
