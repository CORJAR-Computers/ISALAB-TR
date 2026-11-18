import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Loader2, Plus, Search } from "lucide-react";
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
import { useCreateStay, usePatients, useResourceBoard } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { STAY_KIND } from "@/lib/status";
import { PatientQuickCreateButton } from "@/features/patients/PatientQuickCreateButton";

const formSchema = z.object({
  patientId: z.coerce
    .number({ message: "Selecciona el paciente" })
    .min(1, "Selecciona el paciente"),
  kind: z.enum(["GUARDERIA", "HOSPEDAJE", "HOSPITALIZACION"]),
  resourceId: z.coerce.number().int().min(1).nullable(),
  dailyRate: z.coerce.number().min(0, "No puede ser negativo").nullable(),
  notes: z.string().nullable(),
});

type FormValues = z.infer<typeof formSchema>;

export function NewStayDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const createStay = useCreateStay();
  const [patientSearch, setPatientSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");

  const { data: patients = [] } = usePatients(debouncedSearch);
  const { data: board = [] } = useResourceBoard();

  const form = useForm<z.input<typeof formSchema>, unknown, FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      patientId: 0,
      kind: "GUARDERIA",
      resourceId: null,
      dailyRate: null,
      notes: null,
    },
  });

  useEffect(() => {
    const t = setTimeout(() => setDebouncedSearch(patientSearch.trim()), 200);
    return () => clearTimeout(t);
  }, [patientSearch]);

  useEffect(() => {
    if (open) {
      form.reset({
        patientId: 0,
        kind: "GUARDERIA",
        resourceId: null,
        dailyRate: null,
        notes: null,
      });
      setPatientSearch("");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  // Solo se ofrecen los recursos con sitio libre.
  const freeResources = board.filter(
    (b) => b.occupied < b.resource.capacity,
  );

  const onSubmit = async (values: FormValues) => {
    try {
      const stay = await createStay.mutateAsync({
        patientId: values.patientId,
        visitId: null,
        resourceId: values.resourceId ?? null,
        kind: values.kind,
        checkInAt: null,
        dailyRate: values.dailyRate ?? null,
        notes: values.notes?.trim() || null,
      });
      toast.success("Estancia registrada", {
        description: `${stay.code} · ${stay.patientName}`,
      });
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo registrar la estancia", {
        description: getErrorMessage(e),
      });
    }
  };

  const patientIdValue = Number(form.watch("patientId") ?? 0);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Nueva estancia</DialogTitle>
          <DialogDescription>
            Ingreso a guardería, hospedaje u hospitalización. El paciente queda
            alojado en el recurso asignado (si lo hay) y su bitácora de
            cuidados empieza vacía.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            {/* Paciente */}
            <div className="space-y-2">
              <FormLabel>Paciente *</FormLabel>
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
                <PatientQuickCreateButton
                  label=""
                  className="shrink-0 [&>svg]:mx-auto"
                  onCreated={(patient) => {
                    form.setValue("patientId", patient.id);
                    setPatientSearch("");
                    toast.info("Paciente listo para la estancia", {
                      description: `${patient.name} · ${patient.ownerName}`,
                    });
                  }}
                />
              </div>
            </div>

            {/* Tipo de estancia */}
            <FormField
              control={form.control}
              name="kind"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Tipo de estancia *</FormLabel>
                  <Select value={field.value} onValueChange={field.onChange}>
                    <FormControl>
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      {(["GUARDERIA", "HOSPEDAJE", "HOSPITALIZACION"] as const).map(
                        (k) => (
                          <SelectItem key={k} value={k}>
                            {STAY_KIND[k].label}
                          </SelectItem>
                        ),
                      )}
                    </SelectContent>
                  </Select>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Recurso */}
            <FormField
              control={form.control}
              name="resourceId"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Recurso</FormLabel>
                  <Select
                    value={field.value ? String(field.value) : "none"}
                    onValueChange={(v) =>
                      field.onChange(v === "none" ? null : Number(v))
                    }
                  >
                    <FormControl>
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder="Sin asignar" />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent className="max-h-64">
                      <SelectItem value="none">Sin asignar</SelectItem>
                      {freeResources.map((b) => (
                        <SelectItem
                          key={b.resource.id}
                          value={b.resource.id.toString()}
                        >
                          {b.resource.name} · {b.occupied}/{b.resource.capacity}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Tarifa diaria */}
            <FormField
              control={form.control}
              name="dailyRate"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Tarifa diaria (COP)</FormLabel>
                  <FormControl>
                    <Input
                      type="number"
                      min={0}
                      step="1"
                      placeholder="0 (sin cobro)"
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

            {/* Notas */}
            <FormField
              control={form.control}
              name="notes"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Notas del ingreso</FormLabel>
                  <FormControl>
                    <Textarea
                      value={field.value ?? ""}
                      onChange={(e) => field.onChange(e.target.value || null)}
                      placeholder="Alimentación, medicación, objetos del propietario…"
                      className="min-h-20"
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
              <Button type="submit" disabled={createStay.isPending}>
                {createStay.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <Plus className="size-4" />
                )}
                Registrar ingreso
              </Button>
            </DialogFooter>
          </form>
        </Form>

        {patientIdValue > 0 && (
          <p className="sr-only">
            Paciente seleccionado: {patientIdValue}
          </p>
        )}
      </DialogContent>
    </Dialog>
  );
}
