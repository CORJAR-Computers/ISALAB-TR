import { useEffect, useState } from "react";
import { useFieldArray, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { ClipboardList, FlaskConical, Loader2, Plus, Search, Trash2 } from "lucide-react";
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
import {
  useAnalytes,
  useCreateLabOrder,
  usePanels,
  usePatients,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { useSessionStore } from "@/stores/session-store";

const itemSchema = z
  .object({
    panelId: z.number().nullable(),
    analyteId: z.number().nullable(),
  })
  .refine((it) => it.panelId != null || it.analyteId != null, {
    message: "Elige un panel o un analito",
    path: ["panelId"],
  });

const orderSchema = z.object({
  patientId: z.coerce.number().min(1, "Selecciona el paciente"),
  priority: z.enum(["NORMAL", "URGENTE"]),
  notes: z.string().nullable(),
  items: z.array(itemSchema).min(1, "Agrega al menos una prueba"),
});

type OrderValues = z.infer<typeof orderSchema>;

/** Opción "sin selección" para los combos de panel/analito (Radix no admite value=""). */
const NONE = "__none__";

export function NewLabOrderDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const createOrder = useCreateLabOrder();
  const { data: panels = [] } = usePanels();
  const { data: analytes = [] } = useAnalytes();
  const session = useSessionStore((s) => s.session);

  const [patientSearch, setPatientSearch] = useState("");

  const { data: patients = [], isLoading: loadingPatients } =
    usePatients(patientSearch);

  const form = useForm<z.input<typeof orderSchema>, unknown, z.output<typeof orderSchema>>({
    resolver: zodResolver(orderSchema),
    defaultValues: {
      patientId: 0,
      priority: "NORMAL",
      notes: null,
      items: [{ panelId: null, analyteId: null }],
    },
  });

  const { fields, append, remove } = useFieldArray({
    control: form.control,
    name: "items",
  });

  // Reinicia el formulario al abrir (solo en la transición open=false→true).
  useEffect(() => {
    if (open) {
      form.reset({
        patientId: 0,
        priority: "NORMAL",
        notes: null,
        items: [{ panelId: null, analyteId: null }],
      });
      setPatientSearch("");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const onSubmit = async (values: OrderValues) => {
    try {
      const order = await createOrder.mutateAsync({
        patientId: values.patientId,
        consultationId: null,
        requestedBy: session?.fullName ?? null,
        priority: values.priority,
        notes: values.notes?.trim() || null,
        requestedAt: null,
        items: values.items.map((it) => ({
          panelId: it.panelId ?? null,
          analyteId: it.analyteId ?? null,
        })),
      });
      toast.success(`Orden ${order.code} creada`, {
        description: `${order.items.length} prueba${order.items.length === 1 ? "" : "s"} solicitada${order.items.length === 1 ? "" : "s"}`,
        icon: <ClipboardList className="size-4" />,
      });
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo crear la orden", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Nueva orden de laboratorio</DialogTitle>
          <DialogDescription>
            Solicita paneles y/o analitos para un paciente. Después podrás
            accesionar la orden para generar las muestras correspondientes.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            {/* Paciente */}
            <FormField
              control={form.control}
              name="patientId"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Paciente *</FormLabel>
                  <div className="relative">
                    <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                    <Input
                      value={patientSearch}
                      onChange={(e) => {
                        setPatientSearch(e.target.value);
                        field.onChange(0);
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
                          <SelectValue placeholder="Selecciona…" />
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

            {/* Prioridad */}
            <FormField
              control={form.control}
              name="priority"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Prioridad</FormLabel>
                  <Select value={field.value} onValueChange={field.onChange}>
                    <FormControl>
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      <SelectItem value="NORMAL">Normal</SelectItem>
                      <SelectItem value="URGENTE">Urgente</SelectItem>
                    </SelectContent>
                  </Select>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Pruebas solicitadas */}
            <div className="space-y-2">
              <Label>Pruebas solicitadas *</Label>
              <div className="space-y-2">
                {fields.map((field, index) => (
                  <div key={field.id} className="flex items-center gap-2">
                    <FormField
                      control={form.control}
                      name={`items.${index}.panelId`}
                      render={({ field: f }) => (
                        <Select
                          value={f.value != null ? f.value.toString() : NONE}
                          onValueChange={(v) => {
                            if (v === NONE) {
                              f.onChange(null);
                              // Un panel y un analito son excluyentes en un ítem.
                              form.setValue(`items.${index}.analyteId`, null);
                            } else {
                              f.onChange(Number(v));
                              form.setValue(`items.${index}.analyteId`, null);
                            }
                          }}
                        >
                          <SelectTrigger className="flex-1" aria-label="Panel">
                            <SelectValue placeholder="Panel…" />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value={NONE}>— Sin panel —</SelectItem>
                            {panels
                              .filter((p) => p.isActive)
                              .map((p) => (
                                <SelectItem key={p.id} value={p.id.toString()}>
                                  {p.name} ({p.analyteCount} analitos)
                                </SelectItem>
                              ))}
                          </SelectContent>
                        </Select>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name={`items.${index}.analyteId`}
                      render={({ field: f }) => (
                        <Select
                          value={f.value != null ? f.value.toString() : NONE}
                          onValueChange={(v) => {
                            if (v === NONE) {
                              f.onChange(null);
                            } else {
                              f.onChange(Number(v));
                              form.setValue(`items.${index}.panelId`, null);
                            }
                          }}
                        >
                          <SelectTrigger className="flex-1" aria-label="Analito">
                            <SelectValue placeholder="Analito… (opcional)" />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value={NONE}>— Sin analito —</SelectItem>
                            {analytes.map((a) => (
                              <SelectItem key={a.id} value={a.id.toString()}>
                                {a.name}
                                {a.unit ? ` (${a.unit})` : ""}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      )}
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      className="shrink-0"
                      onClick={() => remove(index)}
                      disabled={fields.length === 1}
                      aria-label="Eliminar prueba"
                    >
                      <Trash2 className="text-destructive size-4" />
                    </Button>
                  </div>
                ))}
              </div>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => append({ panelId: null, analyteId: null })}
              >
                <Plus className="size-3.5" />
                Agregar prueba
              </Button>
              {form.formState.errors.items?.message && (
                <p className="text-destructive text-sm">
                  {form.formState.errors.items.message}
                </p>
              )}
            </div>

            {/* Notas */}
            <FormField
              control={form.control}
              name="notes"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Notas clínicas</FormLabel>
                  <FormControl>
                    <Textarea
                      value={field.value ?? ""}
                      onChange={(e) => field.onChange(e.target.value || null)}
                      placeholder="Signos clínicos, sospecha diagnóstica, indicaciones…"
                      className="min-h-14"
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                Cancelar
              </Button>
              <Button type="submit" disabled={createOrder.isPending}>
                {createOrder.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <FlaskConical className="size-4" />
                )}
                Crear orden
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
