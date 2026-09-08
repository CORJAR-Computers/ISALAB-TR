import { useState } from "react";
import { toast } from "sonner";
import {
  ArrowRight,
  Ban,
  CheckCircle2,
  ClipboardList,
  Loader2,
  PlayCircle,
  TestTube2,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  useAccessionLabOrder,
  useLabOrder,
  useSampleTypes,
  useSetLabOrderStatus,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import {
  LAB_ORDER_PRIORITY,
  LAB_ORDER_STATUS,
  LAB_ORDER_STATUS_TRANSITIONS,
  SAMPLE_STATUS,
} from "@/lib/status";
import { formatDateTime } from "@/lib/utils";

const STATUS_ACTION_LABEL: Record<string, string> = {
  EN_PROCESO: "Tomar en proceso",
  COMPLETADA: "Marcar completada",
  ANULADA: "Anular orden",
};

export function LabOrderDetailDialog({
  orderId,
  onOpenChange,
}: {
  orderId: number | null;
  onOpenChange: (open: boolean) => void;
}) {
  const { data: order, isLoading } = useLabOrder(orderId);
  const setStatus = useSetLabOrderStatus();
  const accession = useAccessionLabOrder();
  const { data: sampleTypes = [] } = useSampleTypes();

  const [sampleTypeId, setSampleTypeId] = useState<string>("");
  const [collectedBy, setCollectedBy] = useState("");

  const changeStatus = async (status: string) => {
    if (!order) return;
    try {
      await setStatus.mutateAsync({ id: order.id, status });
      toast.success(`Orden ${order.code} → ${LAB_ORDER_STATUS[status]?.label ?? status}`);
    } catch (e) {
      toast.error("No se pudo actualizar la orden", {
        description: getErrorMessage(e),
      });
    }
  };

  const handleAccession = async () => {
    if (!order || !sampleTypeId) return;
    try {
      const sample = await accession.mutateAsync({
        orderId: order.id,
        sampleTypeId: Number(sampleTypeId),
        receivedAt: null,
        collectedBy: collectedBy.trim() || null,
        notes: null,
      });
      toast.success(`Muestra ${sample.code} creada`, {
        description: `Accesionada desde la orden ${order.code}`,
        icon: <TestTube2 className="size-4" />,
      });
      setSampleTypeId("");
      setCollectedBy("");
    } catch (e) {
      toast.error("No se pudo accesionar la orden", {
        description: getErrorMessage(e),
      });
    }
  };

  const active = order != null && !isLoading;
  const transitions = active ? (LAB_ORDER_STATUS_TRANSITIONS[order.status] ?? []) : [];
  const canAccession = active && order.status === "RECIBIDA";

  return (
    <Dialog open={orderId != null} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ClipboardList className="text-primary size-4" />
            Orden {order?.code ?? "…"}
          </DialogTitle>
          <DialogDescription>
            {order && (
              <>
                Solicitada el {formatDateTime(order.requestedAt)} ·{" "}
                {order.patientName} ({order.speciesName}) · {order.ownerName}
              </>
            )}
          </DialogDescription>
        </DialogHeader>

        {isLoading && !order && (
          <div className="space-y-3">
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-40 w-full" />
          </div>
        )}

        {order && (
          <div className="space-y-4">
            {/* Metadatos */}
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant={LAB_ORDER_STATUS[order.status]?.variant ?? "secondary"}>
                {LAB_ORDER_STATUS[order.status]?.label ?? order.status}
              </Badge>
              {order.priority === "URGENTE" && (
                <Badge variant={LAB_ORDER_PRIORITY.URGENTE.variant}>
                  {LAB_ORDER_PRIORITY.URGENTE.label}
                </Badge>
              )}
              {order.requestedBy && (
                <span className="text-muted-foreground text-xs">
                  Solicitada por {order.requestedBy}
                </span>
              )}
            </div>

            {/* Pruebas solicitadas */}
            <div className="overflow-hidden rounded-lg border">
              <Table>
                <TableHeader>
                  <TableRow className="hover:bg-transparent">
                    <TableHead>#</TableHead>
                    <TableHead>Prueba</TableHead>
                    <TableHead>Tipo</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {order.items.map((it) => (
                    <TableRow key={it.id}>
                      <TableCell className="text-muted-foreground tabular-nums">
                        {it.seq}
                      </TableCell>
                      <TableCell className="font-medium">
                        {it.panelName ??
                          it.analyteName ??
                          (it.panelId != null || it.analyteId != null ? `#${it.id}` : "—")}
                        {it.analyteName && it.unit && (
                          <span className="text-muted-foreground text-xs"> ({it.unit})</span>
                        )}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">
                          {it.panelName ? "Panel" : "Analito"}
                        </Badge>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>

            {/* Muestras accesionadas */}
            <div>
              <p className="mb-1.5 text-sm font-semibold">Muestras accesionadas</p>
              {order.samples.length === 0 ? (
                <p className="text-muted-foreground text-sm">
                  Todavía no se ha accesionado ninguna muestra para esta orden.
                </p>
              ) : (
                <div className="flex flex-wrap gap-1.5">
                  {order.samples.map((s) => (
                    <Badge key={s.id} variant={SAMPLE_STATUS[s.status]?.variant ?? "secondary"}>
                      <TestTube2 className="size-3" />
                      {s.code} · {s.sampleTypeName}
                    </Badge>
                  ))}
                </div>
              )}
            </div>

            {order.notes && (
              <>
                <Separator />
                <p className="text-muted-foreground text-sm whitespace-pre-wrap">
                  {order.notes}
                </p>
              </>
            )}

            {/* Accesión (solo en RECIBIDA) */}
            {canAccession && (
              <div className="bg-muted/40 space-y-2 rounded-lg border p-3">
                <p className="text-sm font-semibold">Accesionar orden</p>
                <p className="text-muted-foreground text-xs">
                  Crea una muestra ligada a esta orden con el tipo de tubo indicado.
                </p>
                <div className="grid gap-2 sm:grid-cols-[1fr_1fr_auto] sm:items-end">
                  <div className="space-y-1.5">
                    <Label htmlFor="accession-sample-type">Tipo de muestra</Label>
                    <Select value={sampleTypeId} onValueChange={setSampleTypeId}>
                      <SelectTrigger id="accession-sample-type" className="w-full">
                        <SelectValue placeholder="Selecciona…" />
                      </SelectTrigger>
                      <SelectContent>
                        {sampleTypes.map((t) => (
                          <SelectItem key={t.id} value={t.id.toString()}>
                            {t.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="accession-collected-by">Tomada por</Label>
                    <Input
                      id="accession-collected-by"
                      value={collectedBy}
                      onChange={(e) => setCollectedBy(e.target.value)}
                      placeholder="Opcional"
                    />
                  </div>
                  <Button
                    onClick={handleAccession}
                    disabled={!sampleTypeId || accession.isPending}
                  >
                    {accession.isPending ? (
                      <Loader2 className="size-4 animate-spin" />
                    ) : (
                      <TestTube2 className="size-4" />
                    )}
                    Accesionar
                  </Button>
                </div>
              </div>
            )}

            <DialogFooter>
              {transitions.map((next) => (
                <Button
                  key={next}
                  variant={next === "ANULADA" ? "outline" : "default"}
                  className={next === "ANULADA" ? "text-destructive" : undefined}
                  disabled={setStatus.isPending}
                  onClick={() => changeStatus(next)}
                >
                  {setStatus.isPending ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : next === "ANULADA" ? (
                    <Ban className="size-4" />
                  ) : next === "COMPLETADA" ? (
                    <CheckCircle2 className="size-4" />
                  ) : (
                    <PlayCircle className="size-4" />
                  )}
                  {STATUS_ACTION_LABEL[next] ?? next}
                  <ArrowRight className="size-3 opacity-60" />
                </Button>
              ))}
              {transitions.length === 0 && (
                <Button variant="outline" onClick={() => onOpenChange(false)}>
                  Cerrar
                </Button>
              )}
            </DialogFooter>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
