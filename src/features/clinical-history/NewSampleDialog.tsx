import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { openPath } from "@tauri-apps/plugin-opener";
import { CheckCircle2, Loader2, Printer } from "lucide-react";
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
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useAnalyzers,
  useCreateSample,
  useGenerateSampleLabels,
  useSampleTypes,
} from "@/hooks/use-queries";
import { api, getErrorMessage } from "@/lib/api";
import type { Sample } from "@/bindings";

const nowLocal = () => {
  const d = new Date();
  d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
  return d.toISOString().slice(0, 16);
};

const schema = z.object({
  sampleTypeId: z.coerce.number().min(1, "Selecciona el tipo de muestra"),
  analyzerId: z.coerce.number().optional(),
  receivedAt: z.string().min(1, "Fecha requerida"),
  collectedBy: z.string().optional(),
  notes: z.string().optional(),
});

type Values = z.infer<typeof schema>;

export function NewSampleDialog({
  open,
  onOpenChange,
  patientId,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  patientId: number;
}) {
  const { data: sampleTypes = [] } = useSampleTypes();
  const { data: analyzers = [] } = useAnalyzers();
  const activeAnalyzers = analyzers.filter((a) => a.isActive);
  const createSample = useCreateSample();
  const generateLabels = useGenerateSampleLabels();
  const [createdSample, setCreatedSample] = useState<Sample | null>(null);

  const form = useForm<z.input<typeof schema>, unknown, z.output<typeof schema>>({
    resolver: zodResolver(schema),
    defaultValues: {
      sampleTypeId: undefined,
      analyzerId: undefined,
      receivedAt: nowLocal(),
      collectedBy: "",
      notes: "",
    },
  });

  const onSubmit = async (values: Values) => {
    try {
      const date = values.receivedAt.replace("T", " ") + ":00";
      const sample = await createSample.mutateAsync({
        patientId,
        sampleTypeId: values.sampleTypeId,
        analyzerId: values.analyzerId ?? null,
        receivedAt: date,
        collectedBy: values.collectedBy?.trim() || null,
        notes: values.notes?.trim() || null,
        qualityIndex: null,
        qualitySeverity: null,
        qualityNote: null,
      });
      setCreatedSample(sample);
    } catch (e) {
      toast.error("No se pudo registrar la muestra", {
        description: getErrorMessage(e),
      });
    }
  };

  /** Cierra el diálogo tras registrar la muestra. */
  const finish = () => {
    onOpenChange(false);
    form.reset({ receivedAt: nowLocal() });
    setCreatedSample(null);
  };

  /** Escape/clic fuera en la pantalla de éxito también cierran vía `finish`,
   *  para que el formulario se reinicie correctamente. */
  const handleOpenChange = (o: boolean) => {
    if (!o && createdSample) {
      finish();
      return;
    }
    onOpenChange(o);
  };

  /** Genera la etiqueta con el código de barras de la muestra y la abre
   *  para imprimirla y pegarla en el tubo. */
  const printLabel = async () => {
    if (!createdSample) return;
    try {
      const report = await generateLabels.mutateAsync([createdSample.id]);
      toast.success("Etiqueta de muestra generada", {
        description: "Se abrirá el PDF para imprimirla y pegarla en el tubo.",
      });
      try {
        await api.openReportFile(report.path);
      } catch {
        await openPath(report.path);
      }
      finish();
    } catch (err) {
      toast.error("No se pudo generar la etiqueta", {
        description: getErrorMessage(err),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent>
        {createdSample ? (
          <>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <CheckCircle2 className="size-5 text-success" />
                Muestra {createdSample.code} registrada
              </DialogTitle>
              <DialogDescription>
                Estado: recibida. Genera la etiqueta con el código de barras e
                imprímela para pegarla en el tubo.
              </DialogDescription>
            </DialogHeader>

            <div className="bg-muted/50 flex items-center justify-between rounded-lg border px-4 py-3">
              <div>
                <p className="text-muted-foreground text-[11px] font-medium tracking-wide uppercase">
                  Código de muestra
                </p>
                <p className="font-mono text-lg font-semibold">
                  {createdSample.code}
                </p>
              </div>
              <Badge variant="secondary">Recibida</Badge>
            </div>

            <DialogFooter className="flex-col-reverse gap-2 pt-2 sm:flex-row sm:justify-between">
              <Button
                variant="ghost"
                onClick={finish}
                disabled={generateLabels.isPending}
              >
                Cerrar
              </Button>
              <Button
                onClick={printLabel}
                disabled={generateLabels.isPending}
                className="gap-1.5"
              >
                {generateLabels.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <Printer className="size-4" />
                )}
                Generar e imprimir etiqueta
              </Button>
            </DialogFooter>
          </>
        ) : (
          <>
            <DialogHeader>
              <DialogTitle>Registrar muestra analítica</DialogTitle>
              <DialogDescription>
                La muestra queda vinculada inequívocamente al paciente con código
                propio y estado "recibida".
              </DialogDescription>
            </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="sampleTypeId"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Tipo de muestra</FormLabel>
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
                      {sampleTypes.map((t) => (
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
              name="analyzerId"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Equipo analizador (opcional)</FormLabel>
                  <Select
                    value={field.value ? field.value.toString() : "0"}
                    onValueChange={(val) =>
                      field.onChange(val === "0" ? undefined : Number(val))
                    }
                  >
                    <FormControl>
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder="Sin equipo (lectura manual / estándar)" />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      <SelectItem value="0">
                        Sin equipo (lectura manual / estándar)
                      </SelectItem>
                      {activeAnalyzers
                        .filter((a) => a.code !== "GENERAL")
                        .map((a) => (
                          <SelectItem key={a.id} value={a.id.toString()}>
                            {a.name}
                            {a.model ? ` · ${a.model}` : ""}
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
              name="receivedAt"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Fecha de recepción</FormLabel>
                  <FormControl>
                    <Input type="datetime-local" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="collectedBy"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Recogida por</FormLabel>
                  <FormControl>
                    <Input placeholder="Nombre de quien toma la muestra" {...field} />
                  </FormControl>
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
                      placeholder="Condiciones de la muestra, observaciones…"
                      className="min-h-16"
                      {...field}
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
              <Button type="submit" disabled={createSample.isPending}>
                {createSample.isPending && <Loader2 className="animate-spin" />}
                Registrar muestra
              </Button>
            </DialogFooter>
          </form>
        </Form>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
