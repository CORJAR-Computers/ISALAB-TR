import { useState } from "react";
import { toast } from "sonner";
import {
  Ban,
  Banknote,
  Loader2,
  MessageCircle,
  Receipt,
  Undo2,
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
  useInvoice,
  useInvoiceBalance,
  useInvoicePayments,
  useSetInvoiceStatus,
  useVoidPayment,
} from "@/hooks/use-queries";
import { usePermissions } from "@/hooks/use-permissions";
import { getErrorMessage } from "@/lib/api";
import { PAYMENT_STATUS } from "@/lib/status";
import { formatCOP, formatDateTime } from "@/lib/utils";
import { sendWhatsAppMessage } from "@/lib/whatsapp";
import { PaymentDialog } from "./PaymentDialog";

export function InvoiceDetailDialog({
  invoiceId,
  onOpenChange,
}: {
  invoiceId: number | null;
  onOpenChange: (open: boolean) => void;
}) {
  const { data: invoice, isLoading } = useInvoice(invoiceId);
  const { data: balance } = useInvoiceBalance(invoiceId);
  const { data: payments } = useInvoicePayments(invoiceId);
  const setStatus = useSetInvoiceStatus();
  const voidPayment = useVoidPayment();
  const { canBill } = usePermissions();
  const [paying, setPaying] = useState(false);
  const [voidingId, setVoidingId] = useState<number | null>(null);
  const [voidReason, setVoidReason] = useState("");

  const activePayments = (payments ?? []).filter(
    (p) => p.status === "APLICADO",
  );
  const hasPayments = activePayments.length > 0;
  const paymentStatus = balance?.paymentStatus ?? invoice?.status ?? "";
  const statusInfo = PAYMENT_STATUS[paymentStatus] ?? {
    label: paymentStatus,
    variant: "secondary" as const,
  };
  const canCharge =
    canBill &&
    (paymentStatus === "PENDIENTE" || paymentStatus === "ABONADA");
  const canAnulate = paymentStatus !== "ANULADA" && !hasPayments;

  const anulate = async () => {
    if (!invoice) return;
    try {
      await setStatus.mutateAsync({ id: invoice.id, status: "ANULADA" });
      toast.success("Factura anulada");
    } catch (e) {
      toast.error("No se pudo anular la factura", {
        description: getErrorMessage(e),
      });
    }
  };

  const confirmVoid = async (paymentId: number) => {
    try {
      await voidPayment.mutateAsync({
        paymentId,
        reason: voidReason.trim(),
      });
      toast.success("Pago anulado", {
        description:
          "El efectivo volvió a la caja y la factura recuperó su saldo. Puedes anularla ahora si hace falta.",
      });
      setVoidingId(null);
      setVoidReason("");
    } catch (e) {
      toast.error("No se pudo anular el pago", {
        description: getErrorMessage(e),
      });
    }
  };

  const handleWhatsApp = () => {
    if (!invoice?.ownerPhone || !invoice?.ownerName) {
      toast.error("Falta información", {
        description:
          "La factura no tiene un número de teléfono del propietario registrado.",
      });
      return;
    }
    const message = `Hola ${invoice.ownerName},\n\nTe compartimos desde ISALAB la factura correspondiente a tu última visita${invoice.patientName ? ` con ${invoice.patientName}` : ""}.\n\nPor favor, revisa el archivo adjunto.\n\n¡Gracias por confiar en nosotros!`;
    sendWhatsAppMessage(invoice.ownerPhone, message);
  };

  return (
    <>
      <Dialog open={invoiceId != null} onOpenChange={onOpenChange}>
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Receipt className="size-4 text-primary" />
              Factura {invoice?.invoiceNumber ?? "…"}
            </DialogTitle>
            <DialogDescription>
              {invoice && (
                <>
                  Emitida el {formatDateTime(invoice.issueDate)} ·{" "}
                  {invoice.patientName ? `Paciente ${invoice.patientName} · ` : ""}
                  {invoice.ownerName}
                </>
              )}
            </DialogDescription>
          </DialogHeader>

          {isLoading && !invoice && (
            <div className="space-y-3">
              <Skeleton className="h-32 w-full" />
              <Skeleton className="h-24 w-full" />
            </div>
          )}

          {invoice && (
            <div className="space-y-4">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant={statusInfo.variant}>{statusInfo.label}</Badge>
                {invoice.notes && (
                  <span className="text-muted-foreground text-xs">
                    {invoice.notes}
                  </span>
                )}
              </div>

              <div className="overflow-hidden rounded-lg border">
                <Table>
                  <TableHeader>
                    <TableRow className="hover:bg-transparent">
                      <TableHead>Concepto</TableHead>
                      <TableHead className="text-center">Cant.</TableHead>
                      <TableHead className="text-right">P. unitario</TableHead>
                      <TableHead className="text-right">Subtotal</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {invoice.items.map((it) => (
                      <TableRow key={it.id}>
                        <TableCell>{it.description}</TableCell>
                        <TableCell className="text-center tabular-nums">
                          {it.quantity}
                        </TableCell>
                        <TableCell className="text-right tabular-nums">
                          {formatCOP(it.unitPrice ?? 0)}
                        </TableCell>
                        <TableCell className="text-right font-medium tabular-nums">
                          {formatCOP(it.lineTotal ?? 0)}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>

              <div className="ml-auto w-full max-w-64 space-y-1 text-sm">
                <div className="flex justify-between text-muted-foreground">
                  <span>Subtotal</span>
                  <span className="tabular-nums">
                    {formatCOP(invoice.subtotal ?? 0)}
                  </span>
                </div>
                <div className="flex justify-between text-muted-foreground">
                  <span>IVA ({invoice.taxRate ?? 0}%)</span>
                  <span className="tabular-nums">
                    {formatCOP(invoice.taxAmount ?? 0)}
                  </span>
                </div>
                <Separator />
                <div className="flex justify-between text-base font-semibold">
                  <span>Total</span>
                  <span className="text-primary tabular-nums">
                    {formatCOP(invoice.total ?? 0)}
                  </span>
                </div>
                {balance && (
                  <>
                    <div className="flex justify-between text-muted-foreground">
                      <span>Abonado</span>
                      <span className="tabular-nums">
                        {formatCOP(balance.paidAmount)}
                      </span>
                    </div>
                    <div className="flex justify-between font-semibold">
                      <span>Saldo</span>
                      <span className="tabular-nums">
                        {formatCOP(balance.balance)}
                      </span>
                    </div>
                  </>
                )}
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <h4 className="text-sm font-semibold">Pagos</h4>
                  {activePayments.length > 0 && (
                    <span className="text-muted-foreground text-xs">
                      {activePayments.length} recepci
                      {activePayments.length === 1 ? "ón" : "ones"}
                    </span>
                  )}
                </div>
                {!payments || payments.length === 0 ? (
                  <p className="text-muted-foreground rounded-lg border px-3 py-3 text-center text-sm">
                    Esta factura no tiene abonos registrados.
                  </p>
                ) : (
                  <div className="divide-y rounded-lg border">
                    {payments.map((p) => (
                      <div
                        key={p.id}
                        className="flex flex-wrap items-center justify-between gap-2 px-3 py-2 text-sm"
                      >
                        <div className="min-w-0">
                          <p className="font-mono text-xs font-semibold">
                            {p.code}
                            {p.status === "ANULADO" && (
                              <Badge
                                variant="destructive"
                                className="ml-2 text-[10px]"
                              >
                                Anulado
                              </Badge>
                            )}
                          </p>
                          <p className="text-muted-foreground truncate text-xs">
                            {formatDateTime(p.receivedAt)} ·{" "}
                            {p.lines
                              .map((l) => l.methodName)
                              .join(" + ") || "—"}
                            {p.createdByName ? ` · ${p.createdByName}` : ""}
                          </p>
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="font-semibold tabular-nums">
                            {formatCOP(p.total)}
                          </span>
                          {canBill && p.status === "APLICADO" && (
                            <Button
                              variant="ghost"
                              size="sm"
                              className="text-destructive h-7 px-2 text-xs"
                              onClick={() => {
                                setVoidingId(p.id);
                                setVoidReason("");
                              }}
                            >
                              <Undo2 className="size-3" />
                              Anular
                            </Button>
                          )}
                        </div>
                        {voidingId === p.id && (
                          <div className="flex w-full flex-wrap items-center gap-2">
                            <Input
                              autoFocus
                              value={voidReason}
                              onChange={(e) => setVoidReason(e.target.value)}
                              placeholder="Motivo de la anulación"
                              className="h-8 flex-1 text-xs"
                              maxLength={500}
                            />
                            <Button
                              size="sm"
                              variant="destructive"
                              className="h-8"
                              disabled={
                                voidPayment.isPending || !voidReason.trim()
                              }
                              onClick={() => confirmVoid(p.id)}
                            >
                              {voidPayment.isPending ? (
                                <Loader2 className="size-3.5 animate-spin" />
                              ) : (
                                "Confirmar"
                              )}
                            </Button>
                            <Button
                              size="sm"
                              variant="ghost"
                              className="h-8"
                              onClick={() => setVoidingId(null)}
                            >
                              Cancelar
                            </Button>
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}
                {hasPayments && (
                  <p className="text-muted-foreground text-xs">
                    Una factura con pagos activos no puede anularse; primero
                    anula los pagos.
                  </p>
                )}
              </div>

              <DialogFooter>
                {canAnulate && (
                  <Button
                    variant="outline"
                    className="text-destructive"
                    disabled={setStatus.isPending}
                    onClick={anulate}
                  >
                    {setStatus.isPending ? (
                      <Loader2 className="size-4 animate-spin" />
                    ) : (
                      <Ban className="size-4" />
                    )}
                    Anular
                  </Button>
                )}
                {canCharge && (
                  <Button onClick={() => setPaying(true)}>
                    <Banknote className="size-4" />
                    Registrar pago
                  </Button>
                )}
                {paymentStatus !== "ANULADA" && (
                  <Button
                    variant="outline"
                    className="gap-2 border-green-200 bg-green-50 text-green-700 hover:bg-green-100 hover:text-green-800 dark:border-green-900/50 dark:bg-green-900/20 dark:text-green-400 dark:hover:bg-green-900/40"
                    onClick={handleWhatsApp}
                  >
                    <MessageCircle className="size-4" />
                    Enviar por WhatsApp
                  </Button>
                )}
                {paymentStatus === "ANULADA" && (
                  <Button variant="outline" onClick={() => onOpenChange(false)}>
                    Cerrar
                  </Button>
                )}
              </DialogFooter>
            </div>
          )}
        </DialogContent>
      </Dialog>

      <PaymentDialog
        invoiceId={paying ? invoiceId : null}
        onOpenChange={(open) => !open && setPaying(false)}
      />
    </>
  );
}
