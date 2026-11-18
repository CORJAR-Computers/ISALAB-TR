import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Loader2, Save } from "lucide-react";
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
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSaveProduct } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { PRODUCT_CATEGORY, PRODUCT_UNIT } from "@/lib/status";
import type { Product } from "@/bindings";

const CATEGORIES = ["ALIMENTO", "MEDICAMENTO", "ACCESORIO", "INSUMO"] as const;
const UNITS = ["UNIDAD", "KG", "LT", "PAQUETE", "CAJA", "BOLSA"] as const;

const formSchema = z.object({
  code: z.string().trim().min(1, "Código requerido").max(20, "Máximo 20 caracteres"),
  name: z.string().trim().min(1, "Nombre requerido"),
  category: z.enum(CATEGORIES),
  unit: z.enum(UNITS),
  costPrice: z.coerce.number().min(0, "No puede ser negativo"),
  salePrice: z.coerce.number().min(0, "No puede ser negativo"),
  taxRate: z.coerce.number().min(0).max(100).nullable(),
  minStock: z.coerce.number().int().min(0, "No puede ser negativo"),
  active: z.boolean(),
  notes: z.string().nullable(),
});

type FormValues = z.infer<typeof formSchema>;

export function ProductDialog({
  open,
  onOpenChange,
  editing,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Producto a editar, o null para crear. */
  editing: Product | null;
}) {
  const saveProduct = useSaveProduct();

  const form = useForm<z.input<typeof formSchema>, unknown, FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      code: "",
      name: "",
      category: "ALIMENTO",
      unit: "UNIDAD",
      costPrice: 0,
      salePrice: 0,
      taxRate: null,
      minStock: 0,
      active: true,
      notes: null,
    },
  });

  useEffect(() => {
    if (open) {
      form.reset(
        editing
          ? {
              code: editing.code,
              name: editing.name,
              category: editing.category as FormValues["category"],
              unit: editing.unit as FormValues["unit"],
              costPrice: editing.costPrice,
              salePrice: editing.salePrice,
              taxRate: editing.taxRate,
              minStock: editing.minStock,
              active: editing.active,
              notes: editing.notes,
            }
          : {
              code: "",
              name: "",
              category: "ALIMENTO",
              unit: "UNIDAD",
              costPrice: 0,
              salePrice: 0,
              taxRate: null,
              minStock: 0,
              active: true,
              notes: null,
            },
      );
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, editing]);

  const onSubmit = async (values: FormValues) => {
    try {
      const product = await saveProduct.mutateAsync({
        id: editing?.id ?? null,
        code: values.code.toUpperCase(),
        name: values.name,
        category: values.category,
        unit: values.unit,
        costPrice: values.costPrice,
        salePrice: values.salePrice,
        taxRate: values.taxRate,
        minStock: values.minStock,
        active: values.active,
        notes: values.notes?.trim() || null,
      });
      toast.success(editing ? "Producto actualizado" : "Producto creado", {
        description: `${product.code} · ${product.name}`,
      });
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo guardar el producto", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>
            {editing ? "Editar producto" : "Nuevo producto"}
          </DialogTitle>
          <DialogDescription>
            El stock no se edita aquí: se calcula con los movimientos de
            inventario (entradas, salidas, consumos y ajustes).
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <div className="grid gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="code"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Código (SKU) *</FormLabel>
                    <FormControl>
                      <Input {...field} placeholder="ALIM-001" className="uppercase" />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="name"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Nombre *</FormLabel>
                    <FormControl>
                      <Input {...field} placeholder="Alimento adulto 3 kg" />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <FormField
                control={form.control}
                name="category"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Categoría *</FormLabel>
                    <Select value={field.value} onValueChange={field.onChange}>
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {CATEGORIES.map((c) => (
                          <SelectItem key={c} value={c}>
                            {PRODUCT_CATEGORY[c]}
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
                name="unit"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Unidad *</FormLabel>
                    <Select value={field.value} onValueChange={field.onChange}>
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {UNITS.map((u) => (
                          <SelectItem key={u} value={u}>
                            {PRODUCT_UNIT[u]}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
              <FormField
                control={form.control}
                name="costPrice"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Costo (COP) *</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        min={0}
                        step="1"
                        {...field}
                        value={(field.value as number | undefined) ?? 0}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="salePrice"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Precio venta (COP) *</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        min={0}
                        step="1"
                        {...field}
                        value={(field.value as number | undefined) ?? 0}
                      />
                    </FormControl>
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
                        placeholder="Global"
                        value={(field.value as number | null | undefined) ?? ""}
                        onChange={(e) =>
                          field.onChange(
                            e.target.value === "" ? null : e.target.value,
                          )
                        }
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="minStock"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Stock mínimo</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        min={0}
                        {...field}
                        value={(field.value as number | undefined) ?? 0}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <FormField
              control={form.control}
              name="active"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Estado</FormLabel>
                  <Select
                    value={field.value ? "active" : "inactive"}
                    onValueChange={(v) => field.onChange(v === "active")}
                  >
                    <FormControl>
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      <SelectItem value="active">Activo</SelectItem>
                      <SelectItem value="inactive">Inactivo</SelectItem>
                    </SelectContent>
                  </Select>
                  <FormMessage />
                </FormItem>
              )}
            />

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
                      placeholder="Proveedor, presentación, caducidad…"
                      className="min-h-16"
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
              >
                Cancelar
              </Button>
              <Button type="submit" disabled={saveProduct.isPending}>
                {saveProduct.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <Save className="size-4" />
                )}
                Guardar
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
