import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Check, Loader2, Save } from "lucide-react";
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
import {
  useAllServiceCategories,
  useSaveService,
  useSpecies,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { cn } from "@/lib/utils";
import type { Service } from "@/bindings";

const serviceSchema = z.object({
  code: z
    .string()
    .trim()
    .min(1, "Código requerido")
    .max(20, "Máximo 20 caracteres"),
  name: z.string().trim().min(1, "Nombre requerido"),
  categoryId: z.coerce.number().min(1, "Selecciona la categoría"),
  description: z.string().nullable(),
  durationMin: z.coerce
    .number({ message: "Duración requerida" })
    .int("Debe ser entero")
    .min(1, "Mínimo 1 minuto"),
  price: z.coerce
    .number({ message: "Precio requerido" })
    .min(0, "No puede ser negativo"),
  taxRate: z.coerce.number().min(0).max(100).nullable(),
  requiresVet: z.boolean(),
  requiresGroomer: z.boolean(),
  speciesId: z.coerce.number().min(1).nullable(),
  sortOrder: z.coerce.number().int().min(0),
});

type ServiceValues = z.infer<typeof serviceSchema>;

export function ServiceDialog({
  open,
  onOpenChange,
  editing,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Servicio a editar, o null para crear. */
  editing: Service | null;
}) {
  const saveService = useSaveService();

  const form = useForm<z.input<typeof serviceSchema>, unknown, ServiceValues>({
    resolver: zodResolver(serviceSchema),
    defaultValues: {
      code: "",
      name: "",
      categoryId: 0,
      description: null,
      durationMin: 30,
      price: 0,
      taxRate: null,
      requiresVet: false,
      requiresGroomer: false,
      speciesId: null,
      sortOrder: 0,
    },
  });

  // Precarga al abrir (crear vs editar).
  useEffect(() => {
    if (open) {
      form.reset(
        editing
          ? {
              code: editing.code,
              name: editing.name,
              categoryId: editing.categoryId,
              description: editing.description,
              durationMin: editing.durationMin,
              price: editing.price ?? 0,
              taxRate: editing.taxRate,
              requiresVet: editing.requiresVet,
              requiresGroomer: editing.requiresGroomer,
              speciesId: editing.speciesId,
              sortOrder: editing.sortOrder,
            }
          : {
              code: "",
              name: "",
              categoryId: 0,
              description: null,
              durationMin: 30,
              price: 0,
              taxRate: null,
              requiresVet: false,
              requiresGroomer: false,
              speciesId: null,
              sortOrder: 0,
            },
      );
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, editing]);

  const onSubmit = async (values: ServiceValues) => {
    try {
      await saveService.mutateAsync({
        id: editing?.id ?? null,
        code: values.code.toUpperCase(),
        name: values.name,
        categoryId: values.categoryId,
        description: values.description?.trim() || null,
        durationMin: values.durationMin,
        price: values.price,
        taxRate: values.taxRate,
        requiresVet: values.requiresVet,
        requiresGroomer: values.requiresGroomer,
        speciesId: values.speciesId,
        sortOrder: values.sortOrder,
      });
      toast.success(editing ? "Servicio actualizado" : "Servicio creado", {
        description: `${values.code} · ${values.name}`,
      });
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo guardar el servicio", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>
            {editing ? "Editar servicio" : "Nuevo servicio"}
          </DialogTitle>
          <DialogDescription>
            El precio y la duración alimentan la facturación y la agenda. La
            línea de factura guarda una copia del precio: cambiar el catálogo
            no altera facturas ya emitidas.
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
                    <FormLabel>Código *</FormLabel>
                    <FormControl>
                      <Input {...field} placeholder="CONS-GEN" className="uppercase" />
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
                      <Input {...field} placeholder="Consulta general" />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <FormField
              control={form.control}
              name="categoryId"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Categoría *</FormLabel>
                  <CategorySelect
                    value={String(field.value ?? "")}
                    onChange={(v) => field.onChange(Number(v))}
                  />
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="description"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Descripción</FormLabel>
                  <FormControl>
                    <Textarea
                      value={field.value ?? ""}
                      onChange={(e) => field.onChange(e.target.value || null)}
                      placeholder="Qué incluye el servicio…"
                      className="min-h-16"
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
              <FormField
                control={form.control}
                name="durationMin"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Duración (min) *</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        min={1}
                        {...field}
                        value={(field.value as number | undefined) ?? ""}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="price"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Precio (COP) *</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        min={0}
                        step="1"
                        {...field}
                        value={(field.value as number | undefined) ?? ""}
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
            </div>

            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="requiresVet"
                render={({ field }) => (
                  <FormItem>
                    <ToggleRow
                      label="Requiere veterinario"
                      checked={field.value === true}
                      onChecked={(v) => field.onChange(v)}
                    />
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="requiresGroomer"
                render={({ field }) => (
                  <FormItem>
                    <ToggleRow
                      label="Requiere peluquero"
                      checked={field.value === true}
                      onChecked={(v) => field.onChange(v)}
                    />
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="speciesId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Especie (opcional)</FormLabel>
                    <SpeciesSelect
                      value={field.value ? String(field.value) : "all"}
                      onChange={(v) => field.onChange(v === "all" ? null : Number(v))}
                    />
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="sortOrder"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Orden de aparición</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        min={0}
                        {...field}
                        value={(field.value as number | undefined) ?? ""}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
              >
                Cancelar
              </Button>
              <Button type="submit" disabled={saveService.isPending}>
                {saveService.isPending ? (
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

// ============================ SELECTORES COMPARTIDOS ========================

function CategorySelect({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const { data: categories = [] } = useAllServiceCategories();
  return (
    <Select value={value} onValueChange={onChange}>
      <FormControl>
        <SelectTrigger className="w-full">
          <SelectValue placeholder="Selecciona…" />
        </SelectTrigger>
      </FormControl>
      <SelectContent>
        {categories.map((c) => (
          <SelectItem key={c.id} value={c.id.toString()}>
            {c.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

function SpeciesSelect({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const { data: species = [] } = useSpecies();
  return (
    <Select value={value} onValueChange={onChange}>
      <FormControl>
        <SelectTrigger className="w-full">
          <SelectValue placeholder="Todas las especies" />
        </SelectTrigger>
      </FormControl>
      <SelectContent>
        <SelectItem value="all">Todas las especies</SelectItem>
        {species.map((s) => (
          <SelectItem key={s.id} value={s.id.toString()}>
            {s.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

/** Toggle tipo "chip" sin dependencia de Radix checkbox. */
function ToggleRow({
  label,
  checked,
  onChecked,
}: {
  label: string;
  checked: boolean;
  onChecked: (checked: boolean) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onChecked(!checked)}
      aria-pressed={checked}
      className={cn(
        "flex w-full items-center gap-2 rounded-lg border px-3 py-2.5 text-sm transition-colors",
        checked
          ? "border-primary/40 bg-primary/10 text-foreground"
          : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
      )}
    >
      <span
        className={cn(
          "flex size-4.5 items-center justify-center rounded border transition-colors",
          checked
            ? "border-primary bg-primary text-primary-foreground"
            : "border-border",
        )}
      >
        {checked && <Check className="size-3" />}
      </span>
      {label}
    </button>
  );
}
