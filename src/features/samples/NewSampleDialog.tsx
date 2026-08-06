import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { FlaskConical, Loader2, Plus, Search } from "lucide-react";
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
  useCreateSample,
  usePatients,
  useSampleTypes,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import type { Sample } from "@/bindings";

const nowLocal = () => {
  const d = new Date();
  d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
  return d.toISOString().slice(0, 16);
};

const formatDbDateTime = (iso: string) => {
  if (!iso) return "";
  const d = new Date(iso);
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:00`;
};

const schema = z.object({
  patientId: z.coerce.number().min(1, "Selecciona un paciente"),
  sampleTypeId: z.coerce.number().min(1, "Selecciona el tipo de muestra"),
  receivedAt: z.string().min(1, "Fecha de recepción requerida"),
  collectedBy: z.string().optional(),
  notes: z.string().optional(),
});

type Values = z.infer<typeof schema>;

export function NewSampleDialog({
  open,
  onOpenChange,
  patientId,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  patientId?: number | null;
  onCreated?: (sample: Sample) => void;
}) {
  const createSample = useCreateSample();
  const { data: sampleTypes = [] } = useSampleTypes();
  const [search, setSearch] = useState("");
  const { data: patients = [], isLoading: loadingPatients } = usePatients(
    patientId ? "" : search,
    !patientId,
  );

  const form = useForm<z.input<typeof schema>, unknown, z.output<typeof schema>>({
    resolver: zodResolver(schema),
    defaultValues: {
      patientId: patientId ?? undefined,
      sampleTypeId: undefined,
      receivedAt: nowLocal(),
      collectedBy: "",
      notes: "",
    },
  });

  useEffect(() => {
    if (open) {
      if (patientId) form.setValue("patientId", patientId);
      form.setValue("receivedAt", nowLocal());
    }
  }, [open, patientId, form]);

  const selectedPatientId = form.watch("patientId");
  const selectedPatient = patientId
    ? patients.find((p) => p.id === patientId)
    : patients.find((p) => p.id === selectedPatientId);

  const onSubmit = async (values: Values) => {
    try {
      const sample = await createSample.mutateAsync({
        patientId: values.patientId,
        sampleTypeId: values.sampleTypeId,
        receivedAt: formatDbDateTime(values.receivedAt),
        collectedBy: values.collectedBy?.trim() || null,
        notes: values.notes?.trim() || null,
      });

      toast.success(`Muestra ${sample.code} registrada`, {
        description: "Ya puedes ingresar a cargar los resultados analíticos.",
      });

      form.reset();
      onOpenChange(false);
      onCreated?.(sample);
    } catch (e) {
      toast.error("No se pudo registrar la muestra", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FlaskConical className="size-5 text-primary" />
            Nueva toma de muestra
          </DialogTitle>
          <DialogDescription>
            Registra una nueva muestra para iniciar el procesamiento y carga de resultados analíticos.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            {/* Selección de paciente */}
            {!patientId ? (
              <FormField
                control={form.control}
                name="patientId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Paciente</FormLabel>
                    <div className="space-y-2">
                      <div className="relative">
                        <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                        <Input
                          value={search}
                          onChange={(e) => setSearch(e.target.value)}
                          placeholder="Buscar paciente (nombre, propietario)…"
                          className="pl-9"
                        />
                      </div>
                      <div className="max-h-40 space-y-1 overflow-y-auto rounded-lg border p-1">
                        {loadingPatients && (
                          <Skeleton className="h-10 w-full" />
                        )}
                        {!loadingPatients && patients.length === 0 && (
                          <p className="text-muted-foreground p-3 text-center text-xs">
                            {search
                              ? "Sin pacientes coincidentes."
                              : "Escribe para buscar un paciente."}
                          </p>
                        )}
                        {patients.map((p) => {
                          const selected = field.value === p.id;
                          return (
                            <button
                              key={p.id}
                              type="button"
                              onClick={() => field.onChange(p.id)}
                              className={`flex w-full items-center justify-between rounded-md px-3 py-2 text-left text-xs transition-colors ${
                                selected
                                  ? "bg-primary text-primary-foreground font-medium"
                                  : "hover:bg-accent"
                              }`}
                            >
                              <div>
                                <p className="font-medium">{p.name}</p>
                                <p
                                  className={
                                    selected
                                      ? "opacity-80"
                                      : "text-muted-foreground"
                                  }
                                >
                                  {p.speciesName} · {p.ownerName}
                                </p>
                              </div>
                            </button>
                          );
                        })}
                      </div>
                    </div>
                    <FormMessage />
                  </FormItem>
                )}
              />
            ) : (
              selectedPatient && (
                <div className="bg-muted/50 rounded-lg p-3 text-sm">
                  <p className="text-muted-foreground text-xs font-medium uppercase tracking-wide">
                    Paciente seleccionado
                  </p>
                  <p className="font-semibold">{selectedPatient.name}</p>
                  <p className="text-muted-foreground text-xs">
                    {selectedPatient.speciesName} · Propietario: {selectedPatient.ownerName}
                  </p>
                </div>
              )
            )}

            {/* Tipo de muestra */}
            <FormField
              control={form.control}
              name="sampleTypeId"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Tipo de muestra</FormLabel>
                  <Select
                    value={field.value ? field.value.toString() : ""}
                    onValueChange={(val) => field.onChange(Number(val))}
                  >
                    <FormControl>
                      <SelectTrigger>
                        <SelectValue placeholder="Selecciona tipo de muestra…" />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      {sampleTypes.map((st) => (
                        <SelectItem key={st.id} value={st.id.toString()}>
                          {st.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Fecha y hora de recepción */}
            <FormField
              control={form.control}
              name="receivedAt"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Fecha y hora de recepción</FormLabel>
                  <FormControl>
                    <Input type="datetime-local" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Recogida por */}
            <FormField
              control={form.control}
              name="collectedBy"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Recogida por / Responsable (opcional)</FormLabel>
                  <FormControl>
                    <Input placeholder="Ej. Dr. Carlos" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Notas */}
            <FormField
              control={form.control}
              name="notes"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Notas / Observaciones (opcional)</FormLabel>
                  <FormControl>
                    <Textarea
                      placeholder="Observaciones de la toma o recepción de muestra…"
                      className="resize-none"
                      rows={2}
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <DialogFooter className="pt-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
              >
                Cancelar
              </Button>
              <Button type="submit" disabled={createSample.isPending}>
                {createSample.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <Plus className="size-4" />
                )}
                Registrar muestra
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
