import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { Loader2 } from "lucide-react";
import { openPath } from "@tauri-apps/plugin-opener";
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
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import {
  useCloseCash,
  useGenerateCashClosePdf,
} from "@/hooks/use-queries";
import { api, getErrorMessage } from "@/lib/api";
import { cn, formatCOP } from "@/lib/utils";
import { CASH_DENOMINATIONS } from "./money";

/** Cierre de caja: arqueo por denominación, diferencia y PDF de cierre. */
export function CloseCashDialog({
  open,
  onOpenChange,
  expectedCash,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  expectedCash: number;
}) {
  const closeCash = useCloseCash();
  const generatePdf = useGenerateCashClosePdf();
  const [quantities, setQuantities] = useState<Record<number, string>>({});
  const [notes, setNotes] = useState("");

  useEffect(() => {
    if (open) {
      setQuantities({});
      setNotes("");
    }
  }, [open]);

  const counted = useMemo(
    () =>
      CASH_DENOMINATIONS.reduce(
        (acc, d) => acc + d * (Number(quantities[d]) || 0),
        0,
      ),
    [quantities],
  );
  const difference = counted - expectedCash;

  const submit = async () => {
    const lines = CASH_DENOMINATIONS.map((denomination) => ({
      denomination,
      quantity: Number(quantities[denomination]) || 0,
    })).filter((l) => l.quantity > 0);

    if (lines.length === 0) {
      toast.error("Registra al menos una denominación en el arqueo");
      return;
    }

    try {
      const summary = await closeCash.mutateAsync({
        countLines: lines,
        notes: notes.trim() || null,
      });
      onOpenChange(false);
      toast.success(`Caja ${summary.session.code} cerrada`, {
        description:
          Math.abs(summary.difference ?? 0) <= 0.01
            ? "El arqueo coincide con el efectivo esperado."
            : `Diferencia de ${formatCOP(summary.difference ?? 0)}.`,
      });

      try {
        const report = await generatePdf.mutateAsync(summary.session.id);
        try {
          await api.openReportFile(report.path);
        } catch {
          await openPath(report.path);
        }
      } catch (e) {
        toast.error("No se pudo generar el PDF de cierre", {
          description: getErrorMessage(e),
        });
      }
    } catch (e) {
      toast.error("No se pudo cerrar la caja", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Cerrar caja</DialogTitle>
          <DialogDescription>
            Cuenta los billetes y monedas. El total contado se compara con el
            efectivo esperado.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-1.5">
          {CASH_DENOMINATIONS.map((denomination) => {
            const quantity = Number(quantities[denomination]) || 0;
            return (
              <div
                key={denomination}
                className="flex items-center gap-3 rounded-md border px-3 py-1.5"
              >
                <span className="w-24 text-sm font-medium tabular-nums">
                  {formatCOP(denomination)}
                </span>
                <Input
                  value={quantities[denomination] ?? ""}
                  onChange={(e) =>
                    setQuantities((prev) => ({
                      ...prev,
                      [denomination]: e.target.value,
                    }))
                  }
                  inputMode="numeric"
                  className="h-8 w-20 text-right tabular-nums"
                  aria-label={`Cantidad de ${formatCOP(denomination)}`}
                />
                <span className="text-muted-foreground ml-auto text-sm tabular-nums">
                  {formatCOP(denomination * quantity)}
                </span>
              </div>
            );
          })}
        </div>

        <Separator />

        <div className="space-y-1 rounded-lg bg-muted/40 p-3 text-sm">
          <div className="flex justify-between">
            <span className="text-muted-foreground">Efectivo esperado</span>
            <span className="tabular-nums">{formatCOP(expectedCash)}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">Total contado</span>
            <span className="font-semibold tabular-nums">
              {formatCOP(counted)}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">Diferencia</span>
            <span
              className={cn(
                "font-semibold tabular-nums",
                Math.abs(difference) <= 0.01
                  ? "text-success"
                  : difference > 0
                    ? "text-amber-600 dark:text-amber-400"
                    : "text-destructive",
              )}
            >
              {formatCOP(difference)}
            </span>
          </div>
        </div>

        <div className="space-y-1">
          <Label htmlFor="close-cash-notes" className="text-xs">
            Notas del cierre (opcional)
          </Label>
          <Textarea
            id="close-cash-notes"
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            className="min-h-14"
            placeholder="Observaciones del arqueo…"
          />
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancelar
          </Button>
          <Button
            onClick={submit}
            disabled={closeCash.isPending || counted <= 0}
          >
            {closeCash.isPending && <Loader2 className="size-4 animate-spin" />}
            Cerrar caja
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
