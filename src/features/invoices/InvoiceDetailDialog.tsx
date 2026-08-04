import { toast } from "sonner";
import { Ban, CheckCircle2, Loader2, Receipt, MessageCircle } from "lucide-react";
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
import { useInvoice, useSetInvoiceStatus } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { INVOICE_STATUS, PAYMENT_METHOD_LABEL } from "@/lib/status";
import { formatCOP, formatDateTime } from "@/lib/utils";
import { sendWhatsAppMessage } from "@/lib/whatsapp";

export function InvoiceDetailDialog({
  invoiceId,
  onOpenChange,
}: {
  invoiceId: number | null;
  onOpenChange: (open: boolean) => void;
}) {
  const { data: invoice, isLoading } = useInvoice(invoiceId);
  const setStatus = useSetInvoiceStatus();

  const changeStatus = async (status: string, label: string) => {
    if (!invoice) return;
    try {
      await setStatus.mutateAsync({ id: invoice.id, status });
      toast.success(`Factura ${label.toLowerCase()}`);
    } catch (e) {
      toast.error("No se pudo actualizar la factura", {
        description: getErrorMessage(e),
      });
    }
  };

  const handleWhatsApp = () => {
    if (!invoice?.ownerPhone || !invoice?.ownerName) {
      toast.error("Falta información", {
        description: "La factura no tiene un número de teléfono del propietario registrado.",
      });
      return;
    }
    const message = `Hola ${invoice.ownerName},\n\nTe compartimos desde ISALAB la factura correspondiente a tu última visita${invoice.patientName ? ` con ${invoice.patientName}` : ""}.\n\nPor favor, revisa el archivo adjunto.\n\n¡Gracias por confiar en nosotros!`;
    sendWhatsAppMessage(invoice.ownerPhone, message);
  };

  return (
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
              <Badge
                variant={
                  INVOICE_STATUS[invoice.status]?.variant ?? "secondary"
                }
              >
                {INVOICE_STATUS[invoice.status]?.label ?? invoice.status}
              </Badge>
              {invoice.paymentMethod && (
                <Badge variant="outline">
                  {PAYMENT_METHOD_LABEL[invoice.paymentMethod] ??
                    invoice.paymentMethod}
                </Badge>
              )}
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
            </div>

            <DialogFooter>
              {invoice.status === "EMITIDA" && (
                <Button
                  variant="outline"
                  className="text-destructive"
                  disabled={setStatus.isPending}
                  onClick={() => changeStatus("ANULADA", "Anulada")}
                >
                  {setStatus.isPending ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : (
                    <Ban className="size-4" />
                  )}
                  Anular
                </Button>
              )}
              {invoice.status === "PAGADA" && (
                <Button
                  variant="ghost"
                  className="text-destructive"
                  disabled={setStatus.isPending}
                  onClick={() => changeStatus("ANULADA", "Anulada")}
                >
                  {setStatus.isPending ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : (
                    <Ban className="size-4" />
                  )}
                  Anular
                </Button>
              )}
              {invoice.status !== "PAGADA" && invoice.status !== "ANULADA" && (
                <Button
                  className="text-success"
                  disabled={setStatus.isPending}
                  onClick={() => changeStatus("PAGADA", "Pagada")}
                >
                  {setStatus.isPending ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : (
                    <CheckCircle2 className="size-4" />
                  )}
                  Marcar como pagada
                </Button>
              )}
              {invoice.status !== "ANULADA" && (
                <Button
                  variant="outline"
                  className="gap-2 bg-green-50 text-green-700 hover:bg-green-100 hover:text-green-800 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/50 dark:hover:bg-green-900/40"
                  onClick={handleWhatsApp}
                >
                  <MessageCircle className="size-4" />
                  Enviar por WhatsApp
                </Button>
              )}
              {invoice.status === "ANULADA" && (
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
