import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { Loader2, Plus, Receipt, Search, Trash2 } from "lucide-react";
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
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import {
  useClinicSettings,
  useCreateInvoice,
  useOwners,
  usePatients,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { PAYMENT_METHOD_LABEL } from "@/lib/status";
import { cn, formatCOP } from "@/lib/utils";

type ItemRow = {
  key: number;
  description: string;
  quantity: string;
  unitPrice: string;
};

let rowKey = 0;
const newRow = (): ItemRow => ({
  key: ++rowKey,
  description: "",
  quantity: "1",
  unitPrice: "",
});

export function NewInvoiceDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const createInvoice = useCreateInvoice();
  const { data: settings } = useClinicSettings();

  const [ownerSearch, setOwnerSearch] = useState("");
  const [ownerId, setOwnerId] = useState<number | null>(null);
  const [patientSearch, setPatientSearch] = useState("");
  const [patientId, setPatientId] = useState<number | null>(null);
  const [paymentMethod, setPaymentMethod] = useState("EFECTIVO");
  const [taxRate, setTaxRate] = useState<string>("");
  const [notes, setNotes] = useState("");
  const [items, setItems] = useState<ItemRow[]>(() => [newRow()]);

  const { data: owners = [], isLoading: loadingOwners } = useOwners(ownerSearch);
  const { data: patients = [], isLoading: loadingPatients } =
    usePatients(patientSearch);

  // IVA por defecto desde la configuración de la clínica.
  useEffect(() => {
    if (open && settings && !taxRate) setTaxRate(String(settings.taxRate));
  }, [open, settings, taxRate]);

  // Reinicia el formulario al abrir.
  useEffect(() => {
    if (open) {
      setOwnerId(null);
      setPatientId(null);
      setPaymentMethod("EFECTIVO");
      setNotes("");
      setOwnerSearch("");
      setPatientSearch("");
      setItems([newRow()]);
    }
  }, [open]);

  const updateItem = (key: number, patch: Partial<ItemRow>) =>
    setItems((prev) =>
      prev.map((it) => (it.key === key ? { ...it, ...patch } : it)),
    );
  const removeItem = (key: number) =>
    setItems((prev) =>
      prev.length > 1 ? prev.filter((it) => it.key !== key) : prev,
    );

  const totals = useMemo(() => {
    const subtotal = items.reduce((acc, it) => {
      const qty = Number(it.quantity) || 0;
      const price = Number(it.unitPrice) || 0;
      return acc + qty * price;
    }, 0);
    const rate = Number(taxRate) || 0;
    const tax = subtotal * (rate / 100);
    return { subtotal, tax, total: subtotal + tax };
  }, [items, taxRate]);

  const submit = async () => {
    const clean = items.filter((it) => it.description.trim());
    if (!ownerId) {
      toast.error("Selecciona el propietario que facturarás");
      return;
    }
    if (clean.length === 0) {
      toast.error("Agrega al menos un concepto con descripción");
      return;
    }
    if (clean.some((it) => !(Number(it.quantity) > 0) || !(Number(it.unitPrice) >= 0))) {
      toast.error("Revisa cantidades y precios de los conceptos");
      return;
    }
    try {
      const invoice = await createInvoice.mutateAsync({
        ownerId,
        patientId,
        consultationId: null,
        taxRate: Number(taxRate) || null,
        paymentMethod: paymentMethod || null,
        notes: notes.trim() || null,
        items: clean.map((it) => ({
          description: it.description.trim(),
          quantity: Number(it.quantity),
          unitPrice: Number(it.unitPrice),
        })),
      });
      toast.success(`Factura ${invoice.invoiceNumber} emitida`, {
        description: `${invoice.items.length} concepto${invoice.items.length === 1 ? "" : "s"} · Total ${formatCOP(invoice.total ?? 0)}`,
        icon: <Receipt className="size-4" />,
      });
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo emitir la factura", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Nueva factura</DialogTitle>
          <DialogDescription>
            Emite la factura con IVA {Number(taxRate) || 0}% (configurable). El
            número se asigna automáticamente (FAC-000001…).
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Cliente y paciente */}
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label>Propietario (a facturar) *</Label>
              <div className="relative">
                <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                <Input
                  value={ownerSearch}
                  onChange={(e) => {
                    setOwnerSearch(e.target.value);
                    setOwnerId(null);
                  }}
                  placeholder="Buscar propietario…"
                  className="pl-9"
                />
              </div>
              {loadingOwners ? (
                <Skeleton className="h-10 w-full" />
              ) : (
                <Select
                  value={ownerId?.toString() ?? ""}
                  onValueChange={(v) => setOwnerId(Number(v))}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Selecciona…" />
                  </SelectTrigger>
                  <SelectContent>
                    {owners.map((o) => (
                      <SelectItem key={o.id} value={o.id.toString()}>
                        {o.fullName} · {o.documentNumber}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </div>

            <div className="space-y-1.5">
              <Label>Paciente (opcional)</Label>
              <div className="relative">
                <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                <Input
                  value={patientSearch}
                  onChange={(e) => {
                    setPatientSearch(e.target.value);
                    setPatientId(null);
                  }}
                  placeholder="Buscar paciente…"
                  className="pl-9"
                />
              </div>
              {loadingPatients ? (
                <Skeleton className="h-10 w-full" />
              ) : (
                <Select
                  value={patientId?.toString() ?? ""}
                  onValueChange={(v) => setPatientId(Number(v))}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Sin paciente" />
                  </SelectTrigger>
                  <SelectContent>
                    {patients.map((p) => (
                      <SelectItem key={p.id} value={p.id.toString()}>
                        {p.name} · {p.ownerName}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </div>
          </div>

          {/* Conceptos */}
          <div className="space-y-2">
            <Label>Conceptos *</Label>
            <div className="space-y-2">
              {items.map((it) => (
                <div key={it.key} className="flex items-center gap-2">
                  <Input
                    value={it.description}
                    onChange={(e) =>
                      updateItem(it.key, { description: e.target.value })
                    }
                    placeholder="Descripción (consulta, vacuna, cirugía…)"
                    className="flex-1"
                  />
                  <Input
                    type="number"
                    min={1}
                    value={it.quantity}
                    onChange={(e) =>
                      updateItem(it.key, { quantity: e.target.value })
                    }
                    placeholder="Cant."
                    className="w-20"
                    aria-label="Cantidad"
                  />
                  <Input
                    type="number"
                    min={0}
                    step="0.01"
                    value={it.unitPrice}
                    onChange={(e) =>
                      updateItem(it.key, { unitPrice: e.target.value })
                    }
                    placeholder="$ 0"
                    className="w-28"
                    aria-label="Precio unitario"
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    className="shrink-0"
                    onClick={() => removeItem(it.key)}
                    disabled={items.length === 1}
                    aria-label="Eliminar concepto"
                  >
                    <Trash2 className="size-4 text-destructive" />
                  </Button>
                </div>
              ))}
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setItems((prev) => [...prev, newRow()])}
            >
              <Plus className="size-3.5" />
              Agregar concepto
            </Button>
          </div>

          {/* Pago y notas */}
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label>Método de pago</Label>
              <Select value={paymentMethod} onValueChange={setPaymentMethod}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {Object.entries(PAYMENT_METHOD_LABEL).map(([value, label]) => (
                    <SelectItem key={value} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label>IVA (%)</Label>
              <Input
                type="number"
                min={0}
                max={100}
                step="0.01"
                value={taxRate}
                onChange={(e) => setTaxRate(e.target.value)}
                placeholder="19"
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label>Notas</Label>
            <Textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Condiciones de pago, observaciones…"
              className="min-h-14"
            />
          </div>

          {/* Totales en vivo */}
          <div className="rounded-lg border bg-muted/40 p-3">
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Subtotal</span>
              <span className="font-medium tabular-nums">
                {formatCOP(totals.subtotal)}
              </span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">
                IVA ({Number(taxRate) || 0}%)
              </span>
              <span className="font-medium tabular-nums">
                {formatCOP(totals.tax)}
              </span>
            </div>
            <Separator className="my-2" />
            <div className="flex justify-between text-base font-semibold">
              <span>Total</span>
              <span className={cn("tabular-nums text-primary")}>
                {formatCOP(totals.total)}
              </span>
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancelar
          </Button>
          <Button onClick={submit} disabled={createInvoice.isPending}>
            {createInvoice.isPending && <Loader2 className="animate-spin" />}
            Emitir factura
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
