import { useState } from "react";
import { toast } from "sonner";
import { Banknote, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { useCheckoutVisit, useVisit } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { PAYMENT_METHOD_LABEL } from "@/lib/status";
import { formatCOP } from "@/lib/utils";

/**
 * Cobra una visita FINALIZADA: genera la factura con los precios congelados
 * de la visita y la deja lista en Facturación.
 */
export function CheckoutDialog({
  visitId,
  triggerLabel,
}: {
  visitId: number;
  triggerLabel?: string;
}) {
  const [open, setOpen] = useState(false);
  const [paymentMethod, setPaymentMethod] = useState<string>("EFECTIVO");
  const [notes, setNotes] = useState("");

  const { data: visit } = useVisit(open ? visitId : null);
  const checkout = useCheckoutVisit();

  const lines = (visit?.services ?? []).filter(
    (s) => s.status !== "CANCELADA",
  );
  const subtotal = lines.reduce(
    (acc, s) => acc + (s.unitPrice ?? 0) * s.quantity,
    0,
  );

  const onCheckout = async () => {
    try {
      const invoice = await checkout.mutateAsync({
        visitId,
        paymentMethod: paymentMethod || null,
        notes: notes.trim() || null,
      });
      toast.success("Visita cobrada", {
        description: `${invoice.invoiceNumber} · ${formatCOP(invoice.total ?? 0)}`,
      });
      setOpen(false);
    } catch (e) {
      toast.error("No se pudo cobrar la visita", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm">
          <Banknote className="size-4" />
          {triggerLabel ?? "Cobrar visita"}
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Cobrar visita</DialogTitle>
          <DialogDescription>
            Se genera la factura con los precios congelados de la visita. Podrás
            registrar el pago desde Facturación.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-1 rounded-lg border">
          {lines.map((s) => (
            <div
              key={s.id}
              className="flex items-center justify-between border-b px-3 py-2 text-sm last:border-b-0"
            >
              <span className="truncate">
                {s.serviceName}
                {s.quantity > 1 && (
                  <span className="text-muted-foreground"> ×{s.quantity}</span>
                )}
              </span>
              <span className="tabular-nums">
                {formatCOP((s.unitPrice ?? 0) * s.quantity)}
              </span>
            </div>
          ))}
          {lines.length === 0 && (
            <p className="text-muted-foreground px-3 py-3 text-center text-sm">
              No hay servicios activos en la visita.
            </p>
          )}
          <div className="flex items-center justify-between bg-muted/40 px-3 py-2 text-sm font-semibold">
            <span>Subtotal</span>
            <span className="tabular-nums">{formatCOP(subtotal)}</span>
          </div>
          <p className="text-muted-foreground px-3 pb-2 text-xs">
            El IVA se calcula por línea (tarifa del servicio o global de la
            clínica).
          </p>
        </div>

        <div className="space-y-2">
          <Label>Método de pago</Label>
          <Select value={paymentMethod} onValueChange={setPaymentMethod}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {Object.entries(PAYMENT_METHOD_LABEL).map(([key, label]) => (
                <SelectItem key={key} value={key}>
                  {label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-2">
          <Label>Notas (opcional)</Label>
          <Textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            className="min-h-14"
            placeholder="Observaciones del cobro…"
          />
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>
            Cancelar
          </Button>
          <Button
            onClick={onCheckout}
            disabled={checkout.isPending || lines.length === 0}
          >
            {checkout.isPending && <Loader2 className="size-4 animate-spin" />}
            Emitir factura
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
