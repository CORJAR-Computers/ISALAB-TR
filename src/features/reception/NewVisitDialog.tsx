import { useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Loader2, Plus, Search, Trash2, Zap } from "lucide-react";
import { Badge } from "@/components/ui/badge";
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
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useCreateVisit,
  usePatients,
  useServiceCategories,
  useServices,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { cn, formatCOP } from "@/lib/utils";
import { PatientQuickCreateButton } from "@/features/patients/PatientQuickCreateButton";
import type { Patient, Service } from "@/bindings";

const formSchema = z.object({
  patientId: z.coerce
    .number({ message: "Selecciona el paciente" })
    .min(1, "Selecciona el paciente"),
  priority: z.enum(["NORMAL", "URGENTE"]),
  notes: z.string().nullable(),
});

type FormValues = z.infer<typeof formSchema>;

/** Línea seleccionada para la visita (del catálogo, precio congelado). */
type SelectedService = {
  serviceId: number;
  name: string;
  price: number;
  quantity: number;
};

export function NewVisitDialog({
  open,
  onOpenChange,
  /** Paciente preseleccionado (p. ej. llegado desde Pacientes). */
  presetPatient,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  presetPatient?: Patient | null;
  onCreated?: (visitCode: string) => void;
}) {
  const createVisit = useCreateVisit();
  const [patientSearch, setPatientSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [catalogServiceId, setCatalogServiceId] = useState("");
  const [selected, setSelected] = useState<SelectedService[]>([]);

  const { data: patients = [] } = usePatients(debouncedSearch);
  const { data: categories = [] } = useServiceCategories();
  const { data: catalogServices = [] } = useServices(null, "");

  const form = useForm<z.input<typeof formSchema>, unknown, FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      patientId: 0,
      priority: "NORMAL",
      notes: null,
    },
  });

  useEffect(() => {
    const t = setTimeout(() => setDebouncedSearch(patientSearch.trim()), 200);
    return () => clearTimeout(t);
  }, [patientSearch]);

  // Reinicia al abrir (con paciente preseleccionado si llega).
  useEffect(() => {
    if (open) {
      form.reset({
        patientId: presetPatient?.id ?? 0,
        priority: "NORMAL",
        notes: null,
      });
      setPatientSearch("");
      setCatalogServiceId("");
      setSelected([]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, presetPatient]);

  const addService = (service: Service) => {
    setSelected((prev) => {
      if (prev.some((s) => s.serviceId === service.id)) {
        toast.info(`${service.name} ya está en la visita`, {
          description: "Ajusta la cantidad en la lista.",
        });
        return prev;
      }
      return [
        ...prev,
        { serviceId: service.id, name: service.name, price: service.price ?? 0, quantity: 1 },
      ];
    });
    setCatalogServiceId("");
  };

  const total = useMemo(
    () => selected.reduce((acc, s) => acc + s.price * s.quantity, 0),
    [selected],
  );

  const setQuantity = (serviceId: number, quantity: number) =>
    setSelected((prev) =>
      prev.map((s) =>
        s.serviceId === serviceId ? { ...s, quantity: Math.max(1, quantity) } : s,
      ),
    );

  const onSubmit = async (values: FormValues) => {
    if (selected.length === 0) {
      toast.error("Agrega al menos un servicio a la visita");
      return;
    }
    try {
      const visit = await createVisit.mutateAsync({
        patientId: values.patientId,
        ownerId: null,
        priority: values.priority,
        notes: values.notes?.trim() || null,
        services: selected.map((s) => ({
          serviceId: s.serviceId,
          professionalId: null,
          quantity: s.quantity,
          notes: null,
        })),
      });
      toast.success("Visita registrada", {
        description: `${visit.code} · ${visit.patientName} · ${formatCOP(total)}`,
      });
      onCreated?.(visit.code);
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo registrar la visita", {
        description: getErrorMessage(e),
      });
    }
  };

  const patientIdValue = form.watch("patientId");
  const chosenPatient = patients.find((p) => p.id === patientIdValue) ?? presetPatient;

  // Servicios agrupados por categoría para el selector.
  const catalogGroups = useMemo(
    () =>
      categories
        .map((cat) => ({
          name: cat.name,
          services: catalogServices.filter((s) => s.categoryId === cat.id),
        }))
        .filter((g) => g.services.length > 0),
    [categories, catalogServices],
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Nueva visita</DialogTitle>
          <DialogDescription>
            Check-in de un paciente: los precios se congelan del catálogo en
            este momento (cambiarlos luego no altera esta visita).
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            {/* Paciente */}
            <div className="space-y-2">
              <FormLabel>Paciente *</FormLabel>
              {presetPatient && patientIdValue === presetPatient.id ? (
                <div className="flex items-center justify-between rounded-lg border bg-muted/40 px-3 py-2">
                  <span className="text-sm font-medium">
                    {presetPatient.name} · {presetPatient.ownerName}
                  </span>
                  <Badge variant="outline">{presetPatient.code}</Badge>
                </div>
              ) : (
                <>
                  <div className="relative">
                    <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                    <Input
                      value={patientSearch}
                      onChange={(e) => setPatientSearch(e.target.value)}
                      placeholder="Buscar paciente por nombre, código o propietario…"
                      className="pl-9"
                    />
                  </div>
                  <div className="flex items-start gap-2">
                    <FormField
                      control={form.control}
                      name="patientId"
                      render={({ field }) => (
                        <FormItem className="flex-1">
                          <Select
                            value={field.value ? String(field.value) : ""}
                            onValueChange={(v) => field.onChange(Number(v))}
                          >
                            <FormControl>
                              <SelectTrigger className="w-full">
                                <SelectValue placeholder="Selecciona el paciente…" />
                              </SelectTrigger>
                            </FormControl>
                            <SelectContent className="max-h-64">
                              {patients.map((p) => (
                                <SelectItem key={p.id} value={p.id.toString()}>
                                  {p.name} · {p.ownerName}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    {/* Alta en línea: el nuevo paciente queda seleccionado
                        sin abandonar el diálogo de la visita. */}
                    <PatientQuickCreateButton
                      label=""
                      className="shrink-0 [&>svg]:mx-auto"
                      onCreated={(patient) => {
                        form.setValue("patientId", patient.id);
                        setPatientSearch("");
                        toast.info("Paciente listo para la visita", {
                          description: `${patient.name} · ${patient.ownerName}`,
                        });
                      }}
                    />
                  </div>
                </>
              )}
            </div>

            {/* Prioridad */}
            <FormField
              control={form.control}
              name="priority"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Prioridad</FormLabel>
                  <div className="flex gap-2">
                    {(["NORMAL", "URGENTE"] as const).map((p) => (
                      <button
                        key={p}
                        type="button"
                        onClick={() => field.onChange(p)}
                        className={cn(
                          "flex flex-1 items-center justify-center gap-2 rounded-lg border px-3 py-2 text-sm font-medium transition-colors",
                          field.value === p
                            ? p === "URGENTE"
                              ? "border-destructive/40 bg-destructive/10 text-destructive"
                              : "border-primary/40 bg-primary/10 text-foreground"
                            : "text-muted-foreground hover:bg-accent",
                        )}
                      >
                        {p === "URGENTE" && <Zap className="size-3.5" />}
                        {p === "NORMAL" ? "Normal" : "Urgente"}
                      </button>
                    ))}
                  </div>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Catálogo */}
            <div className="space-y-2">
              <FormLabel>Servicios del catálogo *</FormLabel>
              <Select value={catalogServiceId} onValueChange={(v) => {
                const service = catalogServices.find((s) => s.id === Number(v));
                if (service) addService(service);
              }}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="Busca un servicio… (consulta, baño, vacuna)" />
                </SelectTrigger>
                <SelectContent className="max-h-64">
                  {catalogGroups.map((group) => (
                    <SelectGroup key={group.name}>
                      <SelectLabel>{group.name}</SelectLabel>
                      {group.services.map((s) => (
                        <SelectItem key={s.id} value={s.id.toString()}>
                          {s.name} · {formatCOP(s.price ?? 0)}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Líneas seleccionadas */}
            {selected.length > 0 && (
              <div className="space-y-1 rounded-lg border">
                {selected.map((s) => (
                  <div
                    key={s.serviceId}
                    className="flex items-center gap-2 border-b px-3 py-2 last:border-b-0"
                  >
                    <span className="flex-1 truncate text-sm">{s.name}</span>
                    <Input
                      type="number"
                      min={1}
                      value={s.quantity}
                      onChange={(e) =>
                        setQuantity(s.serviceId, Number(e.target.value) || 1)
                      }
                      className="w-16 text-center"
                      aria-label={`Cantidad de ${s.name}`}
                    />
                    <span className="w-28 text-right text-sm tabular-nums">
                      {formatCOP(s.price * s.quantity)}
                    </span>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() =>
                        setSelected((prev) =>
                          prev.filter((x) => x.serviceId !== s.serviceId),
                        )
                      }
                      aria-label={`Quitar ${s.name}`}
                    >
                      <Trash2 className="size-4 text-muted-foreground" />
                    </Button>
                  </div>
                ))}
                <div className="flex items-center justify-between bg-muted/40 px-3 py-2 text-sm font-semibold">
                  <span>Total estimado</span>
                  <span className="tabular-nums">{formatCOP(total)}</span>
                </div>
              </div>
            )}

            {/* Notas */}
            <FormField
              control={form.control}
              name="notes"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Motivo / notas</FormLabel>
                  <FormControl>
                    <Textarea
                      value={field.value ?? ""}
                      onChange={(e) => field.onChange(e.target.value || null)}
                      placeholder="Qué reporta el propietario…"
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
              <Button
                type="submit"
                disabled={createVisit.isPending || selected.length === 0}
              >
                {createVisit.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <Plus className="size-4" />
                )}
                Registrar visita
              </Button>
            </DialogFooter>
          </form>
        </Form>
        {chosenPatient && (
          <p className="sr-only">
            Paciente seleccionado: {chosenPatient.name} ({chosenPatient.code})
          </p>
        )}
      </DialogContent>
    </Dialog>
  );
}
