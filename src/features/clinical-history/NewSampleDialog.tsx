import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Loader2 } from "lucide-react";
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
import { useCreateSample, useSampleTypes } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";

const nowLocal = () => {
  const d = new Date();
  d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
  return d.toISOString().slice(0, 16);
};

const schema = z.object({
  sampleTypeId: z.coerce.number().min(1, "Selecciona el tipo de muestra"),
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
  const createSample = useCreateSample();

  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      sampleTypeId: undefined,
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
        receivedAt: date,
        collectedBy: values.collectedBy?.trim() || null,
        notes: values.notes?.trim() || null,
      });
      toast.success(`Muestra ${sample.code} registrada`, {
        description: "Estado: recibida. Quedó en la cadena de custodia.",
      });
      onOpenChange(false);
      form.reset({ receivedAt: nowLocal() });
    } catch (e) {
      toast.error("No se pudo registrar la muestra", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
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
      </DialogContent>
    </Dialog>
  );
}
