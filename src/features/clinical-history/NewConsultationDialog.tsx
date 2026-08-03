import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { ArrowLeft, Loader2, Search, Stethoscope } from "lucide-react";
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
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useCreateConsultation,
  usePatients,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import type { Patient } from "@/bindings";

const nowLocal = () => {
  const d = new Date();
  d.setMinutes(d.getMinutes() - d.getTimezoneOffset());
  return d.toISOString().slice(0, 16);
};

const schema = z.object({
  consultationDate: z.string().min(1, "Fecha requerida"),
  reason: z.string().min(3, "Describe el motivo de la consulta"),
  anamnesis: z.string().optional(),
  physicalExam: z.string().optional(),
  diagnosis: z.string().optional(),
  treatmentPlan: z.string().optional(),
  status: z.string(),
});

type Values = z.infer<typeof schema>;

export function NewConsultationDialog({
  open,
  onOpenChange,
  patientId,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Paciente fijado (historial clínico) o null para elegirlo en la agenda. */
  patientId: number | null;
}) {
  const createConsultation = useCreateConsultation();
  const [search, setSearch] = useState("");
  const [picked, setPicked] = useState<Patient | null>(null);

  const { data: patients = [], isLoading: loadingPatients } =
    usePatients(search, patientId == null && open && picked == null);

  // Resetea la selección de paciente al abrir el diálogo.
  const handleOpenChange = (next: boolean) => {
    if (!next) {
      setPicked(null);
      setSearch("");
    }
    onOpenChange(next);
  };

  /** Paciente efectivo: fijado por prop (historial) o elegido en la agenda. */
  const targetPatientId = patientId ?? picked?.id ?? null;

  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      consultationDate: nowLocal(),
      reason: "",
      anamnesis: "",
      physicalExam: "",
      diagnosis: "",
      treatmentPlan: "",
      status: "COMPLETADA",
    },
  });

  const onSubmit = async (values: Values) => {
    if (targetPatientId == null) return;
    try {
      const date = values.consultationDate.replace("T", " ") + ":00";
      await createConsultation.mutateAsync({
        patientId: targetPatientId,
        consultationDate: date,
        reason: values.reason.trim(),
        anamnesis: values.anamnesis?.trim() || null,
        physicalExam: values.physicalExam?.trim() || null,
        diagnosis: values.diagnosis?.trim() || null,
        treatmentPlan: values.treatmentPlan?.trim() || null,
        status: values.status,
      });
      toast.success("Consulta registrada", {
        description: "El historial clínico se actualizó.",
        icon: <Stethoscope className="size-4" />,
      });
      handleOpenChange(false);
      form.reset({ consultationDate: nowLocal() });
    } catch (e) {
      toast.error("No se pudo registrar la consulta", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Nueva consulta</DialogTitle>
          <DialogDescription>
            {patientId == null
              ? "Selecciona el paciente y registra el motivo de la consulta."
              : "Registro clínico estructurado: anamnesis, examen físico, diagnóstico y plan de tratamiento."}
          </DialogDescription>
        </DialogHeader>

        {patientId == null && picked == null && (
          <div className="space-y-3">
            <div className="relative">
              <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
              <Input
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Buscar paciente por nombre o propietario…"
                className="pl-9"
                autoFocus
              />
            </div>
            <div className="max-h-56 space-y-1 overflow-y-auto rounded-lg border p-1">
              {loadingPatients &&
                Array.from({ length: 4 }).map((_, i) => (
                  <Skeleton key={i} className="h-10 w-full" />
                ))}
              {!loadingPatients && patients.length === 0 && (
                <p className="text-muted-foreground px-3 py-8 text-center text-sm">
                  {search
                    ? "Sin coincidencias."
                    : "No hay pacientes registrados."}
                </p>
              )}
              {patients.map((p) => (
                <button
                  key={p.id}
                  type="button"
                  onClick={() => setPicked(p)}
                  className="hover:bg-accent flex w-full items-center gap-3 rounded-md px-3 py-2.5 text-left transition-colors"
                >
                  <div className="bg-muted text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-full text-sm font-bold uppercase">
                    {p.name.slice(0, 1)}
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium">{p.name}</p>
                    <p className="text-muted-foreground truncate text-xs">
                      {p.speciesName}
                      {p.breedName ? ` · ${p.breedName}` : ""} · {p.ownerName}
                    </p>
                  </div>
                  <Badge variant="secondary">#{p.id}</Badge>
                </button>
              ))}
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => handleOpenChange(false)}
              >
                Cancelar
              </Button>
            </DialogFooter>
          </div>
        )}

        {picked != null && (
          <div className="bg-accent/60 flex items-center gap-3 rounded-lg border px-3 py-2.5">
            <div className="bg-primary/10 text-primary flex size-9 shrink-0 items-center justify-center rounded-full text-sm font-bold uppercase">
              {picked.name.slice(0, 1)}
            </div>
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-semibold">{picked.name}</p>
              <p className="text-muted-foreground truncate text-xs">
                {picked.speciesName}
                {picked.breedName ? ` · ${picked.breedName}` : ""} ·{" "}
                {picked.ownerName}
              </p>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setPicked(null)}
            >
              <ArrowLeft className="size-4" />
              Cambiar
            </Button>
          </div>
        )}

        {targetPatientId != null ? (
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <div className="grid gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="consultationDate"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Fecha y hora</FormLabel>
                    <FormControl>
                      <Input type="datetime-local" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="status"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Estado</FormLabel>
                    <Select value={field.value} onValueChange={field.onChange}>
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value="COMPLETADA">Completada</SelectItem>
                        <SelectItem value="PENDIENTE">Pendiente</SelectItem>
                        <SelectItem value="CANCELADA">Cancelada</SelectItem>
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <FormField
              control={form.control}
              name="reason"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Motivo de consulta *</FormLabel>
                  <FormControl>
                    <Input
                      placeholder="Ej. vómito recurrente, control posquirúrgico…"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="anamnesis"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Anamnesis</FormLabel>
                  <FormControl>
                    <Textarea
                      placeholder="Historia del cuadro: inicio, evolución, alimentación…"
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
              name="physicalExam"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Examen físico</FormLabel>
                  <FormControl>
                    <Textarea
                      placeholder="Temperatura, FC, FR, mucosas, hallazgos…"
                      className="min-h-20"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <div className="grid gap-3 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="diagnosis"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Diagnóstico</FormLabel>
                    <FormControl>
                      <Textarea
                        placeholder="Diagnóstico presuntivo / definitivo"
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
                name="treatmentPlan"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Plan de tratamiento</FormLabel>
                    <FormControl>
                      <Textarea
                        placeholder="Medicación, dosis, seguimiento…"
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
                onClick={() => handleOpenChange(false)}
              >
                Cancelar
              </Button>
              <Button
                type="submit"
                disabled={createConsultation.isPending || targetPatientId == null}
              >
                {createConsultation.isPending && (
                  <Loader2 className="animate-spin" />
                )}
                Guardar consulta
              </Button>
            </DialogFooter>
          </form>
        </Form>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
