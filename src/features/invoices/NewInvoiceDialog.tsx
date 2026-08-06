import { useEffect, useMemo, useState } from "react";
import { useForm, useFieldArray } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
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
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
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

const invoiceItemSchema = z.object({
  description: z.string().min(1, "Descripción requerida"),
  quantity: z.coerce.number().int().min(1, "Cantidad mínima: 1"),
  unitPrice: z.coerce.number().min(0, "Precio no puede ser negativo"),
});

const invoiceSchema = z.object({
  ownerId: z.coerce.number().min(1, "Selecciona el propietario"),
  patientId: z.coerce.number().nullable(),
  taxRate: z.coerce.number().min(0).max(100).nullable(),
  paymentMethod: z.string().nullable(),
  notes: z.string().nullable(),
  items: z
    .array(invoiceItemSchema)
    .min(1, "Agrega al menos un concepto"),
});

type InvoiceValues = z.infer<typeof invoiceSchema>;

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
  const [patientSearch, setPatientSearch] = useState("");

  const { data: owners = [], isLoading: loadingOwners } = useOwners(ownerSearch);
  const { data: patients = [], isLoading: loadingPatients } =
    usePatients(patientSearch);

  const form = useForm<z.input<typeof invoiceSchema>, unknown, z.output<typeof invoiceSchema>>({
    resolver: zodResolver(invoiceSchema),
    defaultValues: {
      ownerId: 0,
      patientId: null,
      taxRate: null,
      paymentMethod: "EFECTIVO",
      notes: null,
      items: [{ description: "", quantity: 1, unitPrice: 0 }],
    },
  });

  const { fields, append, remove } = useFieldArray({
    control: form.control,
    name: "items",
  });

  // IVA por defecto desde la configuración de la clínica.
  useEffect(() => {
    if (open && settings && !form.getValues("taxRate")) {
      form.setValue("taxRate", settings.taxRate);
    }
  }, [open, settings, form]);

  // Reinicia el formulario al abrir.
  useEffect(() => {
    if (open) {
      form.reset({
        ownerId: 0,
        patientId: null,
        taxRate: settings?.taxRate ?? 19,
        paymentMethod: "EFECTIVO",
        notes: null,
        items: [{ description: "", quantity: 1, unitPrice: 0 }],
      });
      setOwnerSearch("");
      setPatientSearch("");
    }
  }, [open, settings, form]);

  const watchedItems = form.watch("items");
  const watchedTaxRate = form.watch("taxRate");

  const totals = useMemo(() => {
    const subtotal = watchedItems.reduce((acc, it) => {
      const qty = Number(it.quantity) || 0;
      const price = Number(it.unitPrice) || 0;
      return acc + qty * price;
    }, 0);
    const rate = Number(watchedTaxRate) || 0;
    const tax = subtotal * (rate / 100);
    return { subtotal, tax, total: subtotal + tax };
  }, [watchedItems, watchedTaxRate]);

  const onSubmit = async (values: InvoiceValues) => {
    try {
      const invoice = await createInvoice.mutateAsync({
        ownerId: values.ownerId,
        patientId: values.patientId || null,
        consultationId: null,
        taxRate: values.taxRate,
        paymentMethod: values.paymentMethod || null,
        notes: values.notes?.trim() || null,
        items: values.items.map((it) => ({
          description: it.description.trim(),
          quantity: it.quantity,
          unitPrice: it.unitPrice,
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
            Emite la factura con IVA {Number(watchedTaxRate) || 0}% (configurable). El
            número se asigna automáticamente (FAC-000001…).
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            {/* Cliente y paciente */}
            <div className="grid gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="ownerId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Propietario (a facturar) *</FormLabel>
                    <div className="relative">
                      <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                      <Input
                        value={ownerSearch}
                        onChange={(e) => {
                          setOwnerSearch(e.target.value);
                          field.onChange(0);
                        }}
                        placeholder="Buscar propietario…"
                        className="pl-9"
                      />
                    </div>
                    {loadingOwners ? (
                      <Skeleton className="h-10 w-full" />
                    ) : (
                      <Select
                        value={field.value?.toString() ?? ""}
                        onValueChange={(v) => field.onChange(Number(v))}
                      >
                        <FormControl>
                          <SelectTrigger className="w-full">
                            <SelectValue placeholder="Selecciona…" />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          {owners.map((o) => (
                            <SelectItem key={o.id} value={o.id.toString()}>
                              {o.fullName} · {o.documentNumber}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    )}
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="patientId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Paciente (opcional)</FormLabel>
                    <div className="relative">
                      <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                      <Input
                        value={patientSearch}
                        onChange={(e) => {
                          setPatientSearch(e.target.value);
                          field.onChange(null);
                        }}
                        placeholder="Buscar paciente…"
                        className="pl-9"
                      />
                    </div>
                    {loadingPatients ? (
                      <Skeleton className="h-10 w-full" />
                    ) : (
                      <Select
                        value={field.value?.toString() ?? ""}
                        onValueChange={(v) => field.onChange(Number(v))}
                      >
                        <FormControl>
                          <SelectTrigger className="w-full">
                            <SelectValue placeholder="Sin paciente" />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          {patients.map((p) => (
                            <SelectItem key={p.id} value={p.id.toString()}>
                              {p.name} · {p.ownerName}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    )}
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            {/* Conceptos */}
            <div className="space-y-2">
              <Label>Conceptos *</Label>
              <div className="space-y-2">
                {fields.map((field, index) => (
                  <div key={field.id} className="flex items-center gap-2">
                    <FormField
                      control={form.control}
                      name={`items.${index}.description`}
                      render={({ field: f }) => (
                        <Input
                          {...f}
                          placeholder="Descripción (consulta, vacuna, cirugía…)"
                          className="flex-1"
                        />
                      )}
                    />
                    <FormField
                      control={form.control}
                      name={`items.${index}.quantity`}
                      render={({ field: f }) => (
                        <Input
                          {...f}
                          type="number"
                          min={1}
                          placeholder="Cant."
                          className="w-20"
                          aria-label="Cantidad"
                          value={(f.value as number | undefined) ?? ""}
                        />
                      )}
                    />
                    <FormField
                      control={form.control}
                      name={`items.${index}.unitPrice`}
                      render={({ field: f }) => (
                        <Input
                          {...f}
                          type="number"
                          min={0}
                          step="0.01"
                          placeholder="$ 0"
                          className="w-28"
                          aria-label="Precio unitario"
                          value={(f.value as number | undefined) ?? ""}
                        />
                      )}
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="shrink-0"
                      onClick={() => remove(index)}
                      disabled={fields.length === 1}
                      aria-label="Eliminar concepto"
                    >
                      <Trash2 className="size-4 text-destructive" />
                    </Button>
                  </div>
                ))}
              </div>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => append({ description: "", quantity: 1, unitPrice: 0 })}
              >
                <Plus className="size-3.5" />
                Agregar concepto
              </Button>
              {form.formState.errors.items?.message && (
                <p className="text-sm text-destructive">
                  {form.formState.errors.items.message}
                </p>
              )}
            </div>

            {/* Pago y notas */}
            <div className="grid gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="paymentMethod"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Método de pago</FormLabel>
                    <Select value={field.value ?? ""} onValueChange={field.onChange}>
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {Object.entries(PAYMENT_METHOD_LABEL).map(([value, label]) => (
                          <SelectItem key={value} value={value}>
                            {label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="taxRate"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>IVA (%)</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        min={0}
                        max={100}
                        step="0.01"
                        value={(field.value as number | null | undefined) ?? ""}
                        onChange={(e) => field.onChange(e.target.value ? Number(e.target.value) : null)}
                        placeholder="19"
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <FormField
              control={form.control}
              name="notes"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Notas</FormLabel>
                  <FormControl>
                    <Textarea
                      value={field.value ?? ""}
                      onChange={(e) => field.onChange(e.target.value || null)}
                      placeholder="Condiciones de pago, observaciones…"
                      className="min-h-14"
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

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
                  IVA ({Number(watchedTaxRate) || 0}%)
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

            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                Cancelar
              </Button>
              <Button type="submit" disabled={createInvoice.isPending}>
                {createInvoice.isPending && <Loader2 className="animate-spin" />}
                Emitir factura
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
