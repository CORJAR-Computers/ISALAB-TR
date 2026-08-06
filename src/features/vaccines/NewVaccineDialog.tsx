import { useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Loader2, Search, Syringe } from "lucide-react";
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
import { Skeleton } from "@/components/ui/skeleton";
import {
  useCreateVaccine,
  usePatients,
  useVaccineTypes,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";

const nowLocal = () => {
  const d = new Date();
  d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
  return d.toISOString().slice(0, 16);
};

const today = () => new Date().toISOString().slice(0, 10);

const schema = z.object({
  patientId: z.coerce.number().min(1, "Selecciona el paciente"),
  vaccineTypeId: z.string().optional(),
  vaccineName: z.string().min(2, "Nombre de la vacuna requerido"),
  dose: z.string().optional(),
  administeredAt: z.string().min(1, "Fecha requerida"),
  nextDoseAt: z.string().optional(),
  lot: z.string().optional(),
  manufacturer: z.string().optional(),
  notes: z.string().optional(),
});

type Values = z.infer<typeof schema>;

export function NewVaccineDialog({
  open,
  onOpenChange,
  patientId,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Si se pasa, el paciente queda fijado (p. ej. desde el historial clínico). */
  patientId?: number | null;
}) {
  const createVaccine = useCreateVaccine();
  const { data: vaccineTypes = [] } = useVaccineTypes();
  const [search, setSearch] = useState("");
  // Con paciente fijado (historial clínico) no hace falta buscar pacientes.
  const { data: patients = [], isLoading: loadingPatients } = usePatients(
    patientId ? "" : search,
    !patientId,
  );

  const form = useForm<z.input<typeof schema>, unknown, z.output<typeof schema>>({
    resolver: zodResolver(schema),
    defaultValues: {
      patientId: patientId ?? undefined,
      vaccineTypeId: "",
      vaccineName: "",
      dose: "",
      administeredAt: nowLocal(),
      nextDoseAt: "",
      lot: "",
      manufacturer: "",
      notes: "",
    },
  });

  // Al abrir con un paciente fijado, lo sincroniza con el formulario.
  useEffect(() => {
    if (open && patientId) form.setValue("patientId", patientId);
  }, [open, patientId, form]);

  // Al elegir un tipo del catálogo, autocompleta el nombre si está vacío.
  const vaccineTypeId = form.watch("vaccineTypeId");
  const selectedType = useMemo(
    () => vaccineTypes.find((t) => t.id === Number(vaccineTypeId)),
    [vaccineTypeId, vaccineTypes],
  );

  const onTypeChange = (value: string) => {
    form.setValue("vaccineTypeId", value);
    if (value && !form.getValues("vaccineName").trim()) {
      const t = vaccineTypes.find((x) => x.id === Number(value));
      if (t) form.setValue("vaccineName", t.name);
    }
  };

  const onSubmit = async (values: Values) => {
    try {
      const date = values.administeredAt.replace("T", " ") + ":00";
      const vaccine = await createVaccine.mutateAsync({
        patientId: values.patientId,
        vaccineTypeId: values.vaccineTypeId
          ? Number(values.vaccineTypeId)
          : null,
        vaccineName: values.vaccineName.trim(),
        dose: values.dose?.trim() || null,
        administeredAt: date,
        nextDoseAt: values.nextDoseAt || null,
        lot: values.lot?.trim() || null,
        manufacturer: values.manufacturer?.trim() || null,
        notes: values.notes?.trim() || null,
      });
      toast.success(`Vacuna ${vaccine.vaccineName} registrada`, {
        description: values.nextDoseAt
          ? `Refuerzo programado para ${values.nextDoseAt}.`
          : "Registro guardado en el historial clínico.",
        icon: <Syringe className="size-4" />,
      });
      onOpenChange(false);
      form.reset({
        patientId: patientId ?? undefined,
        vaccineTypeId: "",
        vaccineName: "",
        dose: "",
        administeredAt: nowLocal(),
        nextDoseAt: "",
        lot: "",
        manufacturer: "",
        notes: "",
      });
      setSearch("");
    } catch (e) {
      toast.error("No se pudo registrar la vacuna", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Registrar vacunación</DialogTitle>
          <DialogDescription>
            Control de esquemas de vacunación y desparasitación con fecha de
            refuerzo opcional.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            {!patientId && (
              <FormField
                control={form.control}
                name="patientId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Paciente</FormLabel>
                    <div className="relative">
                      <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                      <Input
                        value={search}
                        onChange={(e) => {
                          setSearch(e.target.value);
                          field.onChange(undefined);
                        }}
                        placeholder="Buscar paciente…"
                        className="pl-9"
                      />
                    </div>
                    {loadingPatients ? (
                      <Skeleton className="h-10 w-full" />
                    ) : (
                      <Select
                        value={field.value?.toString()}
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
                              {p.name} · {p.speciesName} · {p.ownerName}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    )}
                    <FormMessage />
                  </FormItem>
                )}
              />
            )}

            <div className="grid gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="vaccineTypeId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Tipo (catálogo)</FormLabel>
                    <Select
                      value={field.value}
                      onValueChange={onTypeChange}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue placeholder="Selecciona…" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {vaccineTypes.map((t) => (
                          <SelectItem key={t.id} value={t.id.toString()}>
                            {t.name}
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
                name="vaccineName"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Vacuna *</FormLabel>
                    <FormControl>
                      <Input
                        placeholder="Ej. Rabia, Polivalente canina…"
                        {...field}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <div className="grid gap-3 sm:grid-cols-3">
              <FormField
                control={form.control}
                name="administeredAt"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Fecha de aplicación</FormLabel>
                    <FormControl>
                      <Input type="datetime-local" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="nextDoseAt"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Refuerzo</FormLabel>
                    <FormControl>
                      <Input type="date" min={today()} {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="dose"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Dosis</FormLabel>
                    <FormControl>
                      <Input placeholder="Ej. 1 ml" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="lot"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Lote</FormLabel>
                    <FormControl>
                      <Input placeholder="Nº de lote" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="manufacturer"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Laboratorio</FormLabel>
                    <FormControl>
                      <Input placeholder="Fabricante" {...field} />
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
                      placeholder="Reacciones, vía de administración, observaciones…"
                      className="min-h-16"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            {selectedType && (
              <p className="text-muted-foreground text-xs">
                Esquema: <span className="font-medium">{selectedType.name}</span>
              </p>
            )}

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
              >
                Cancelar
              </Button>
              <Button type="submit" disabled={createVaccine.isPending}>
                {createVaccine.isPending && <Loader2 className="animate-spin" />}
                Guardar vacuna
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
