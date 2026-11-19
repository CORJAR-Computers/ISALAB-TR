import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { AlertTriangle, Banknote, Loader2, Plus, Trash2 } from "lucide-react";
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
import { Textarea } from "@/components/ui/textarea";
import {
  useInvoice,
  useInvoiceBalance,
  useOpenCash,
  useOpenCashSession,
  usePaymentMethods,
  useRegisterPayment,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { formatCOP } from "@/lib/utils";

type LineDraft = {
  key: number;
  methodCode: string;
  amount: string;
  reference: string;
};

const MONEY_EPSILON = 0.01;

/** Convierte "40.000,50" / "40000.50" en 40000.5. */
function parseAmount(value: string): number {
  const cleaned = value.replace(/\s/g, "").replace(/\.(?=\d{3}\b)/g, "");
  const normalized = cleaned.replace(",", ".");
  const parsed = Number(normalized);
  return Number.isFinite(parsed) ? parsed : 0;
}

function draftAmount(n: number): string {
  return n > 0 ? String(Math.round(n * 100) / 100) : "";
}

let nextLineKey = 1;
const newLine = (methodCode = "EFECTIVO", amount = ""): LineDraft => ({
  key: nextLineKey++,
  methodCode,
  amount,
  reference: "",
});

/**
 * Cobro de una factura: abono parcial o pago total con una o varias formas de
 * pago. Exige una caja abierta y aplica el importe al saldo de la factura.
 */
export function PaymentDialog({
  invoiceId,
  onOpenChange,
}: {
  invoiceId: number | null;
  onOpenChange: (open: boolean) => void;
}) {
  const { data: invoice } = useInvoice(invoiceId);
  const { data: balance } = useInvoiceBalance(invoiceId);
  const { data: methods } = usePaymentMethods();
  const { data: cashSession, isLoading: cashLoading } = useOpenCashSession();
  const register = useRegisterPayment();
  const openCash = useOpenCash();

  const [lines, setLines] = useState<LineDraft[]>([newLine()]);
  const [notes, setNotes] = useState("");
  const [openingFloat, setOpeningFloat] = useState("0");

  const target = balance?.balance ?? 0;

  // Al abrir con una factura, precarga una línea por el saldo completo (una
  // sola vez por factura: un refetch del saldo no debe borrar lo escrito).
  const prefilledFor = useRef<number | null>(null);
  useEffect(() => {
    if (invoiceId == null) {
      prefilledFor.current = null;
      return;
    }
    if (!balance || prefilledFor.current === invoiceId) return;
    prefilledFor.current = invoiceId;
    setNotes("");
    setLines([newLine("EFECTIVO", draftAmount(balance.balance))]);
  }, [invoiceId, balance]);

  const lineTotal = useMemo(
    () => lines.reduce((acc, l) => acc + parseAmount(l.amount), 0),
    [lines],
  );
  const remaining = target - lineTotal;
  const overpay = lineTotal > target + MONEY_EPSILON;
  const linesValid =
    lines.length > 0 &&
    lines.every((l) => l.methodCode !== "" && parseAmount(l.amount) > 0);
  const canSubmit =
    invoice != null &&
    cashSession != null &&
    linesValid &&
    !overpay &&
    lineTotal > 0 &&
    !register.isPending;

  const methodByCode = (code: string) => methods?.find((m) => m.code === code);

  const updateLine = (key: number, patch: Partial<LineDraft>) => {
    setLines((prev) =>
      prev.map((l) => (l.key === key ? { ...l, ...patch } : l)),
    );
  };

  const submit = async () => {
    if (!invoice || !canSubmit) return;
    try {
      const payment = await register.mutateAsync({
        ownerId: invoice.ownerId,
        patientId: invoice.patientId,
        notes: notes.trim() || null,
        lines: lines.map((l) => ({
          methodCode: l.methodCode,
          amount: parseAmount(l.amount),
          reference: l.reference.trim() || null,
        })),
        allocations: [{ invoiceId: invoice.id, amount: lineTotal }],
      });
      const restante = target - lineTotal;
      toast.success(`Pago ${payment.code} registrado`, {
        description:
          restante > MONEY_EPSILON
            ? `Saldo pendiente de la factura: ${formatCOP(restante)}`
            : "Factura pagada en su totalidad.",
      });
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo registrar el pago", {
        description: getErrorMessage(e),
      });
    }
  };

  const handleOpenCash = async () => {
    try {
      await openCash.mutateAsync({
        openingFloat: parseAmount(openingFloat),
        notes: null,
      });
      toast.success("Caja abierta");
    } catch (e) {
      toast.error("No se pudo abrir la caja", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={invoiceId != null} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Banknote className="size-4 text-primary" />
            Registrar pago
          </DialogTitle>
          <DialogDescription>
            {invoice
              ? `${invoice.invoiceNumber} · ${invoice.ownerName}${
                  invoice.patientName ? ` · ${invoice.patientName}` : ""
                }`
              : "Cargando factura…"}
          </DialogDescription>
        </DialogHeader>

        {balance && (
          <div className="grid grid-cols-3 gap-2 rounded-lg border p-3 text-sm">
            <div>
              <p className="text-muted-foreground text-xs">Total</p>
              <p className="font-semibold tabular-nums">
                {formatCOP(balance.total)}
              </p>
            </div>
            <div>
              <p className="text-muted-foreground text-xs">Abonado</p>
              <p className="font-semibold tabular-nums">
                {formatCOP(balance.paidAmount)}
              </p>
            </div>
            <div>
              <p className="text-muted-foreground text-xs">Saldo</p>
              <p className="text-primary font-semibold tabular-nums">
                {formatCOP(balance.balance)}
              </p>
            </div>
          </div>
        )}

        {!cashLoading && cashSession == null && (
          <div className="space-y-2 rounded-lg border border-amber-300 bg-amber-50 p-3 text-sm dark:border-amber-900/60 dark:bg-amber-900/20">
            <p className="flex items-center gap-2 font-medium text-amber-800 dark:text-amber-300">
              <AlertTriangle className="size-4" />
              No hay una caja abierta
            </p>
            <p className="text-muted-foreground text-xs">
              Los cobros deben registrarse dentro de una sesión de caja.
            </p>
            <div className="flex items-end gap-2">
              <div className="flex-1 space-y-1">
                <Label htmlFor="opening-float" className="text-xs">
                  Base inicial
                </Label>
                <Input
                  id="opening-float"
                  value={openingFloat}
                  onChange={(e) => setOpeningFloat(e.target.value)}
                  inputMode="decimal"
                />
              </div>
              <Button
                size="sm"
                variant="outline"
                onClick={handleOpenCash}
                disabled={openCash.isPending}
              >
                {openCash.isPending && (
                  <Loader2 className="size-4 animate-spin" />
                )}
                Abrir caja
              </Button>
            </div>
          </div>
        )}

        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label>Formas de pago</Label>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setLines((prev) => [...prev, newLine()])}
            >
              <Plus className="size-3.5" />
              Agregar método
            </Button>
          </div>

          <div className="space-y-2">
            {lines.map((line) => {
              const method = methodByCode(line.methodCode);
              return (
                <div
                  key={line.key}
                  className="space-y-2 rounded-lg border p-2.5"
                >
                  <div className="flex items-end gap-2">
                    <div className="flex-1 space-y-1">
                      <Label className="text-muted-foreground text-xs">
                        Método
                      </Label>
                      <Select
                        value={line.methodCode}
                        onValueChange={(code) =>
                          updateLine(line.key, { methodCode: code })
                        }
                      >
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {(methods ?? []).map((m) => (
                            <SelectItem key={m.code} value={m.code}>
                              {m.name}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="w-32 space-y-1">
                      <Label className="text-muted-foreground text-xs">
                        Importe
                      </Label>
                      <Input
                        value={line.amount}
                        onChange={(e) =>
                          updateLine(line.key, { amount: e.target.value })
                        }
                        inputMode="decimal"
                        className="text-right tabular-nums"
                      />
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="text-muted-foreground"
                      onClick={() =>
                        setLines((prev) =>
                          prev.filter((l) => l.key !== line.key),
                        )
                      }
                      disabled={lines.length === 1}
                      aria-label="Quitar forma de pago"
                    >
                      <Trash2 className="size-4" />
                    </Button>
                  </div>
                  {method?.requiresReference && (
                    <Input
                      value={line.reference}
                      onChange={(e) =>
                        updateLine(line.key, { reference: e.target.value })
                      }
                      placeholder="Referencia / aprobación"
                      className="h-8 text-xs"
                    />
                  )}
                </div>
              );
            })}
          </div>
        </div>

        <div className="space-y-1 rounded-lg bg-muted/40 p-3 text-sm">
          <div className="flex justify-between">
            <span className="text-muted-foreground">Suma de formas de pago</span>
            <span className="font-medium tabular-nums">
              {formatCOP(lineTotal)}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">Saldo después del pago</span>
            <span className="font-semibold tabular-nums">
              {formatCOP(Math.max(remaining, 0))}
            </span>
          </div>
          {overpay && (
            <p className="text-destructive pt-1 text-xs">
              La suma supera el saldo de la factura ({formatCOP(target)}).
              Ajusta los importes.
            </p>
          )}
        </div>

        <div className="space-y-1">
          <Label htmlFor="payment-notes" className="text-xs">
            Notas (opcional)
          </Label>
          <Textarea
            id="payment-notes"
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            className="min-h-14"
            placeholder="Observaciones del cobro…"
          />
        </div>

        <Separator />

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancelar
          </Button>
          <Button onClick={submit} disabled={!canSubmit}>
            {register.isPending && <Loader2 className="size-4 animate-spin" />}
            Registrar pago
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
