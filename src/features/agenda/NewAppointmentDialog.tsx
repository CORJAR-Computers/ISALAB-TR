import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Calendar, Clock, Loader2, Search } from "lucide-react";
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
import { PatientQuickCreateButton } from "@/features/patients/PatientQuickCreateButton";
import {
  useCreateAppointment,
  usePatients,
  useServices,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { formatCOP } from "@/lib/utils";
import type { CreateAppointmentInput } from "@/bindings";

const appointmentSchema = z.object({
  patientId: z.coerce.number().min(1, "Selecciona el paciente"),
  serviceId: z.coerce.number().min(1, "Selecciona el servicio"),
  professionalId: z.coerce.number().optional().nullable(),
  scheduledAt: z.string().min(1, "Selecciona la fecha y hora"),
  durationMin: z.coerce.number().min(1, "La duración debe ser mayor a 0"),
  notes: z.string().nullable(),
});

export function NewAppointmentDialog({
  open,
  onOpenChange,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated?: () => void;
}) {
  const createAppointment = useCreateAppointment();
  const [patientSearch, setPatientSearch] = useState("");
  const [serviceSearch, setServiceSearch] = useState("");
  const { data: patients = [], isLoading: loadingPatients } = usePatients(
    patientSearch,
  );
  const {
    data: services = [],
    isLoading: loadingServices,
  } = useServices(null, "", false, true);

  const form = useForm<z.input<typeof appointmentSchema>, unknown, z.output<typeof appointmentSchema>>({
    resolver: zodResolver(appointmentSchema),
    defaultValues: {
      patientId: undefined,
      serviceId: undefined,
      professionalId: null,
      scheduledAt: "",
      durationMin: 30,
      notes: "",
    },
  });

  const serviceIdValue = form.getValues("serviceId");
  const service = services.find((s) => s.id === serviceIdValue);
  const defaultDuration = service?.durationMin ?? 30;

  const onSubmit = async (values: z.infer<typeof appointmentSchema>) => {
    try {
      const input: CreateAppointmentInput = {
        patientId: values.patientId,
        serviceId: values.serviceId,
        professionalId: values.professionalId ?? null,
        scheduledAt: values.scheduledAt,
        durationMin: values.durationMin,
        notes: values.notes?.trim() || null,
      };
      await createAppointment.mutateAsync(input);
      toast.success("Cita programada", {
        description: `${service?.name ?? ""} · ${values.scheduledAt}`,
      });
      onOpenChange(false);
      onCreated?.();
    } catch (e) {
      toast.error("No se pudo programar la cita", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Nueva cita</DialogTitle>
          <DialogDescription>
            Programa una cita para cualquier servicio del catálogo. La
            duración se toma del servicio y se puede ajustar.
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
                  <div className="flex items-start gap-2">
                    <div className="relative flex-1">
                      <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                      <Input
                        value={patientSearch}
                        onChange={(e) => {
                          setPatientSearch(e.target.value);
                          field.onChange(undefined);
                        }}
                        placeholder="Buscar paciente por nombre o propietario…"
                        className="pl-9"
                      />
                    </div>
                    <PatientQuickCreateButton
                      label=""
                      className="shrink-0 [&>svg]:mx-auto"
                      onCreated={(patient) => {
                        field.onChange(patient.id);
                        setPatientSearch("");
                        toast.info("Paciente listo para la cita", {
                          description: `${patient.name} · ${patient.ownerName}`,
                        });
                      }}
                    />
                  </div>
                  {loadingPatients ? (
                    <Skeleton className="h-10 w-full" />
                  ) : (
                    <Select
                      value={field.value ? String(field.value) : ""}
                      onValueChange={(v) => field.onChange(Number(v))}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue placeholder="Selecciona…" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent className="max-h-64">
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

            {/* Servicio */}
            <FormField
              control={form.control}
              name="serviceId"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Servicio *</FormLabel>
                  <div className="relative">
                    <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                    <Input
                      value={serviceSearch}
                      onChange={(e) => setServiceSearch(e.target.value)}
                      placeholder="Buscar servicio…"
                      className="pl-9"
                    />
                  </div>
                  {loadingServices ? (
                    <Skeleton className="h-10 w-full" />
                  ) : (
                    <Select
                      value={field.value ? String(field.value) : ""}
                      onValueChange={(v) => {
                        field.onChange(Number(v));
                        form.setValue("durationMin", defaultDuration);
                      }}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue placeholder="Selecciona…" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent className="max-h-64">
                        {services
                          .filter(
                            (s) =>
                              !serviceSearch ||
                              s.name.toLowerCase().includes(
                                serviceSearch.toLowerCase(),
                              ),
                          )
                          .map((s) => (
                            <SelectItem
                              key={s.id}
                              value={s.id.toString()}
                            >
                              {s.name} ({s.durationMin} min) ·
                              {formatCOP(s.price ?? 0)}
                            </SelectItem>
                          ))}
                      </SelectContent>
                    </Select>
                  )}
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Profesional */}
            <FormField
              control={form.control}
              name="professionalId"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Profesional (opcional)</FormLabel>
                  <Select
                    value={field.value ? String(field.value) : ""}
                    onValueChange={(v) => field.onChange(Number(v) || null)}
                  >
                    <FormControl>
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder="Sin asignar" />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      <SelectItem value="">Sin asignar</SelectItem>
                    </SelectContent>
                  </Select>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Fecha y hora */}
            <FormField
              control={form.control}
              name="scheduledAt"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Fecha y hora *</FormLabel>
                  <div className="flex items-center gap-2">
                    <Calendar className="size-4 text-muted-foreground" />
                    <Input
                      type="datetime-local"
                      value={field.value}
                      onChange={(e) => field.onChange(e.target.value)}
                      className="flex-1"
                    />
                  </div>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Duración */}
            <FormField
              control={form.control}
              name="durationMin"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Duración (minutos)</FormLabel>
                  <div className="flex items-center gap-2">
                    <Clock className="size-4 text-muted-foreground" />
                    <Input
                      type="number"
                      value={String(field.value ?? defaultDuration)}
                      onChange={(e) =>
                        field.onChange(Number(e.target.value) || defaultDuration)
                      }
                      min="5"
                      step="5"
                      className="w-24"
                    />
                    {service?.durationMin && (
                      <span className="text-xs text-muted-foreground">
                        Catálogo: {service.durationMin} min
                      </span>
                    )}
                  </div>
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
                  <FormLabel>Notas</FormLabel>
                  <Textarea
                    value={field.value ?? ""}
                    onChange={(e) => field.onChange(e.target.value)}
                    placeholder="Observaciones de la cita…"
                    className="min-h-[80px]"
                  />
                  <FormMessage />
                </FormItem>
              )}
            />

            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                Cancelar
              </Button>
              <Button type="submit" disabled={createAppointment.isPending}>
                {createAppointment.isPending ? (
                  <Loader2 className="size-3.5 animate-spin" />
                ) : (
                  "Programar cita"
                )}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
