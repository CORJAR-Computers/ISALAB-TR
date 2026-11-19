import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Loader2 } from "lucide-react";
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
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useAddCashMovement } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { formatCOP } from "@/lib/utils";
import { parseAmount } from "./money";

const KINDS = ["GASTO", "RETIRO", "AJUSTE"] as const;

/** Gasto, retiro o ajuste de la caja abierta. El motivo es obligatorio. */
export function CashMovementDialog({
  open,
  onOpenChange,
  initialKind = "GASTO",
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialKind?: (typeof KINDS)[number];
}) {
  const addMovement = useAddCashMovement();
  const [kind, setKind] = useState<string>(initialKind);
  const [amount, setAmount] = useState("");
  const [reason, setReason] = useState("");

  useEffect(() => {
    if (open) {
      setKind(initialKind);
      setAmount("");
      setReason("");
    }
  }, [open, initialKind]);

  const parsed = parseAmount(amount);
  const canSubmit =
    parsed !== 0 &&
    reason.trim().length > 0 &&
    (kind === "AJUSTE" || parsed > 0) &&
    !addMovement.isPending;

  const submit = async () => {
    if (!canSubmit) return;
    try {
      await addMovement.mutateAsync({
        kind,
        amount: parsed,
        reason: reason.trim(),
      });
      toast.success("Movimiento registrado");
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo registrar el movimiento", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Movimiento de caja</DialogTitle>
          <DialogDescription>
            {kind === "GASTO" && "Dinero que sale de la caja por un gasto."}
            {kind === "RETIRO" && "Retiro de efectivo a bóveda o banco."}
            {kind === "AJUSTE" &&
              "Corrección auditada: importe positivo suma y negativo resta."}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          <div className="space-y-1">
            <Label htmlFor="cash-movement-kind" className="text-xs">
              Tipo
            </Label>
            <Select value={kind} onValueChange={setKind}>
              <SelectTrigger id="cash-movement-kind" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {KINDS.map((k) => (
                  <SelectItem key={k} value={k}>
                    {k === "GASTO" ? "Gasto" : k === "RETIRO" ? "Retiro" : "Ajuste"}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1">
            <Label htmlFor="cash-movement-amount" className="text-xs">
              Importe
            </Label>
            <Input
              id="cash-movement-amount"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              inputMode="decimal"
              className="text-right tabular-nums"
              placeholder="0"
            />
          </div>

          <div className="space-y-1">
            <Label htmlFor="cash-movement-reason" className="text-xs">
              Motivo
            </Label>
            <Textarea
              id="cash-movement-reason"
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              className="min-h-14"
              placeholder="Insumos, domicilio, retiro a bóveda…"
            />
          </div>

          {parsed !== 0 && (
            <p className="text-muted-foreground text-xs">
              {kind === "GASTO" || kind === "RETIRO"
                ? `Salida de caja: ${formatCOP(-Math.abs(parsed))}`
                : `Ajuste: ${formatCOP(parsed)}`}
            </p>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancelar
          </Button>
          <Button onClick={submit} disabled={!canSubmit}>
            {addMovement.isPending && <Loader2 className="size-4 animate-spin" />}
            Registrar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
