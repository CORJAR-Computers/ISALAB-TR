import { useEffect, useState } from "react";
import { toast } from "sonner";
import { ArrowDownUp, Loader2 } from "lucide-react";
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
import { useAddStockMovement, useProducts } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { PRODUCT_UNIT, STOCK_MOVEMENT_KIND } from "@/lib/status";

const KINDS = [
  "ENTRADA",
  "SALIDA",
  "CONSUMO",
  "DEVOLUCION",
  "AJUSTE",
] as const;

export function StockMovementDialog({
  open,
  onOpenChange,
  /** Producto preseleccionado (opcional). */
  presetProductId,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  presetProductId?: number | null;
}) {
  const addMovement = useAddStockMovement();
  const { data: products = [] } = useProducts("", null, false);

  const [productId, setProductId] = useState<number | null>(null);
  const [kind, setKind] = useState<string>("ENTRADA");
  const [quantity, setQuantity] = useState("");
  const [unitCost, setUnitCost] = useState("");
  const [reason, setReason] = useState("");

  useEffect(() => {
    if (open) {
      setProductId(presetProductId ?? null);
      setKind("ENTRADA");
      setQuantity("");
      setUnitCost("");
      setReason("");
    }
  }, [open, presetProductId]);

  const selected = products.find((p) => p.product.id === productId);
  const isAjuste = kind === "AJUSTE";

  const submit = async () => {
    if (!productId) {
      toast.error("Selecciona el producto");
      return;
    }
    const qty = Number(quantity);
    if (!Number.isFinite(qty) || qty === 0) {
      toast.error("La cantidad debe ser distinta de cero");
      return;
    }
    try {
      const movement = await addMovement.mutateAsync({
        productId,
        // El tipo define el signo, salvo el ajuste (que lo lleva explícito).
        quantity: isAjuste ? qty : Math.abs(qty),
        kind,
        unitCost: unitCost === "" ? null : Number(unitCost),
        reason: reason.trim() || null,
        stayId: null,
        visitId: null,
      });
      toast.success("Movimiento registrado", {
        description: `${movement.productName} · ${movement.quantity > 0 ? "+" : ""}${movement.quantity}`,
      });
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
          <DialogTitle className="flex items-center gap-2">
            <ArrowDownUp className="text-primary size-5" />
            Movimiento de inventario
          </DialogTitle>
          <DialogDescription>
            Entradas, salidas y consumos. El stock es siempre la suma de los
            movimientos: nunca puede quedar en negativo.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="movement-product">Producto *</Label>
            <Select
              value={productId ? String(productId) : ""}
              onValueChange={(v) => setProductId(Number(v))}
            >
              <SelectTrigger id="movement-product" className="w-full">
                <SelectValue placeholder="Selecciona el producto…" />
              </SelectTrigger>
              <SelectContent className="max-h-64">
                {products.map((p) => (
                  <SelectItem key={p.product.id} value={p.product.id.toString()}>
                    {p.product.name} · {p.stock}{" "}
                    {PRODUCT_UNIT[p.product.unit] ?? p.product.unit}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {selected && (
              <p className="text-muted-foreground text-xs">
                Stock actual: {selected.stock}{" "}
                {PRODUCT_UNIT[selected.product.unit] ?? selected.product.unit}
              </p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="movement-kind">Tipo *</Label>
            <Select value={kind} onValueChange={setKind}>
              <SelectTrigger id="movement-kind" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {KINDS.map((k) => (
                  <SelectItem key={k} value={k}>
                    {STOCK_MOVEMENT_KIND[k].label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="movement-qty">Cantidad *</Label>
              <Input
                id="movement-qty"
                type="number"
                step="1"
                value={quantity}
                onChange={(e) => setQuantity(e.target.value)}
                placeholder={isAjuste ? "+5 o -3" : "5"}
              />
              {isAjuste && (
                <p className="text-muted-foreground text-xs">
                  En un ajuste el signo indica la corrección.
                </p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="movement-cost">Costo unitario</Label>
              <Input
                id="movement-cost"
                type="number"
                min={0}
                step="1"
                value={unitCost}
                onChange={(e) => setUnitCost(e.target.value)}
                placeholder="Automático"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="movement-reason">Motivo</Label>
            <Textarea
              id="movement-reason"
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder="Compra, venta en mostrador, consumo en estancia…"
              className="min-h-16"
            />
          </div>
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
          >
            Cancelar
          </Button>
          <Button
            type="button"
            onClick={submit}
            disabled={addMovement.isPending || !productId}
          >
            {addMovement.isPending ? (
              <Loader2 className="animate-spin" />
            ) : (
              <ArrowDownUp className="size-4" />
            )}
            Registrar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
