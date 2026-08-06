import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Loader2, Scissors, Search } from "lucide-react";
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
import { useCreateSurgery, usePatients } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { ANESTHESIA_OPTIONS } from "@/lib/status";

const nowLocal = () => {
  const d = new Date();
  d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
  return d.toISOString().slice(0, 16);
};

const schema = z.object({
  patientId: z.coerce.number().min(1, "Selecciona el paciente"),
  surgeryType: z.string().min(3, "Describe el tipo de cirugía"),
  scheduledAt: z.string().min(1, "Fecha requerida"),
  anesthesiaType: z.string().optional(),
  preoperativeNotes: z.string().optional(),
  postoperativeNotes: z.string().optional(),
});

type Values = z.infer<typeof schema>;

export function NewSurgeryDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const createSurgery = useCreateSurgery();
  const [search, setSearch] = useState("");
  const { data: patients = [], isLoading: loadingPatients } =
    usePatients(search);

  const form = useForm<z.input<typeof schema>, unknown, z.output<typeof schema>>({
    resolver: zodResolver(schema),
    defaultValues: {
      patientId: undefined,
      surgeryType: "",
      scheduledAt: nowLocal(),
      anesthesiaType: "",
      preoperativeNotes: "",
      postoperativeNotes: "",
    },
  });

  const onSubmit = async (values: Values) => {
    try {
      const date = values.scheduledAt.replace("T", " ") + ":00";
      await createSurgery.mutateAsync({
        patientId: values.patientId,
        surgeryType: values.surgeryType.trim(),
        scheduledAt: date,
        anesthesiaType: values.anesthesiaType?.trim() || null,
        preoperativeNotes: values.preoperativeNotes?.trim() || null,
        postoperativeNotes: values.postoperativeNotes?.trim() || null,
      });
      toast.success("Cirugía programada", {
        description: "La agenda quirúrgica se actualizó.",
        icon: <Scissors className="size-4" />,
      });
      onOpenChange(false);
      form.reset({ scheduledAt: nowLocal() });
      setSearch("");
    } catch (e) {
      toast.error("No se pudo programar la cirugía", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Programar cirugía</DialogTitle>
          <DialogDescription>
            Registra la intervención con tipo, anestesia y notas pre y
            postoperatorias. Quedará en estado "programada".
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
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

            <div className="grid gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="surgeryType"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Tipo de cirugía *</FormLabel>
                    <FormControl>
                      <Input
                        placeholder="Ej. Esterilización, cesárea, extracción dental…"
                        {...field}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="anesthesiaType"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Anestesia</FormLabel>
                    <Select
                      value={field.value}
                      onValueChange={field.onChange}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue placeholder="Selecciona…" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {ANESTHESIA_OPTIONS.map((a) => (
                          <SelectItem key={a} value={a}>
                            {a}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <FormField
              control={form.control}
              name="scheduledAt"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Fecha y hora de la intervención</FormLabel>
                  <FormControl>
                    <Input type="datetime-local" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <div className="grid gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="preoperativeNotes"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Notas preoperatorias</FormLabel>
                    <FormControl>
                      <Textarea
                        placeholder="Ayuno, exámenes prequirúrgicos, ayuno…"
                        className="min-h-20"
                        {...field}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="postoperativeNotes"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Protocolo postoperatorio</FormLabel>
                    <FormControl>
                      <Textarea
                        placeholder="Analgesia, curación, control…"
                        className="min-h-20"
                        {...field}
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
              <Button type="submit" disabled={createSurgery.isPending}>
                {createSurgery.isPending && (
                  <Loader2 className="animate-spin" />
                )}
                Programar cirugía
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
