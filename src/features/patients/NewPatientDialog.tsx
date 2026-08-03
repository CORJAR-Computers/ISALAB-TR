import { useEffect } from "react";
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
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { useBreeds, useCreatePatient, useSpecies } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";

const DOC_TYPES = [
  { value: "CC", label: "Cédula de Ciudadanía" },
  { value: "TI", label: "Tarjeta de Identidad" },
  { value: "CE", label: "Cédula de Extranjería" },
  { value: "NIT", label: "NIT" },
  { value: "PA", label: "Pasaporte" },
];

const patientSchema = z.object({
  owner: z.object({
    documentType: z.string().min(1, "Selecciona el tipo de documento"),
    documentNumber: z.string().min(3, "Documento inválido"),
    fullName: z.string().min(3, "Nombre del propietario requerido"),
    phone: z.string().optional(),
    email: z.string().email("Correo inválido").or(z.literal("")),
    address: z.string().optional(),
    city: z.string().optional(),
  }),
  name: z.string().min(2, "Nombre del paciente requerido"),
  speciesId: z.coerce.number().min(1, "Selecciona la especie"),
  breedId: z.coerce.number().optional(),
  sex: z.string().min(1, "Selecciona el sexo"),
  birthDate: z.string().optional(),
  neutered: z.boolean(),
  color: z.string().optional(),
  microchip: z.string().optional(),
  notes: z.string().optional(),
});

type PatientFormValues = z.infer<typeof patientSchema>;

export function NewPatientDialog({
  open,
  onOpenChange,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated?: (patientId: number) => void;
}) {
  const { data: species = [] } = useSpecies();
  const createPatient = useCreatePatient();

  const form = useForm<PatientFormValues>({
    resolver: zodResolver(patientSchema),
    defaultValues: {
      owner: {
        documentType: "CC",
        documentNumber: "",
        fullName: "",
        phone: "",
        email: "",
        address: "",
        city: "",
      },
      name: "",
      speciesId: undefined,
      breedId: undefined,
      sex: "",
      birthDate: "",
      neutered: false,
      color: "",
      microchip: "",
      notes: "",
    },
  });

  const currentSpeciesId = form.watch("speciesId");
  const { data: breeds = [] } = useBreeds(currentSpeciesId || null);

  // Al cambiar de especie, limpia la raza si no pertenece.
  useEffect(() => {
    if (currentSpeciesId && form.getValues("breedId")) {
      const ok = breeds.some((b) => b.id === form.getValues("breedId"));
      if (!ok) form.setValue("breedId", undefined);
    }
  }, [currentSpeciesId, breeds, form]);

  const onSubmit = async (values: PatientFormValues) => {
    try {
      const patient = await createPatient.mutateAsync({
        owner: {
          documentType: values.owner.documentType,
          documentNumber: values.owner.documentNumber.trim(),
          fullName: values.owner.fullName.trim(),
          phone: values.owner.phone?.trim() || null,
          email: values.owner.email?.trim() || null,
          address: values.owner.address?.trim() || null,
          city: values.owner.city?.trim() || null,
        },
        name: values.name.trim(),
        speciesId: values.speciesId,
        breedId: values.breedId ?? null,
        sex: values.sex,
        birthDate: values.birthDate || null,
        neutered: values.neutered,
        color: values.color?.trim() || null,
        microchip: values.microchip?.trim() || null,
        notes: values.notes?.trim() || null,
      });
      toast.success(`Paciente ${patient.name} registrado`, {
        description: `#${patient.id} · ${patient.speciesName}`,
      });
      onCreated?.(patient.id);
      form.reset();
    } catch (e) {
      toast.error("No se pudo crear el paciente", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Nuevo paciente</DialogTitle>
          <DialogDescription>
            Registra al propietario y al paciente en un solo paso. El
            propietario se reutiliza si el documento ya existe.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-5">
            <fieldset className="space-y-3">
              <legend className="text-muted-foreground flex w-full items-center gap-2 text-xs font-semibold tracking-wide uppercase">
                Propietario <Separator className="flex-1" />
              </legend>
              <div className="grid gap-3 sm:grid-cols-2">
                <FormField
                  control={form.control}
                  name="owner.documentType"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Tipo de documento</FormLabel>
                      <Select
                        value={field.value}
                        onValueChange={field.onChange}
                      >
                        <FormControl>
                          <SelectTrigger className="w-full">
                            <SelectValue />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          {DOC_TYPES.map((t) => (
                            <SelectItem key={t.value} value={t.value}>
                              {t.label}
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
                  name="owner.documentNumber"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Número de documento</FormLabel>
                      <FormControl>
                        <Input placeholder="1234567890" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>
              <FormField
                control={form.control}
                name="owner.fullName"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Nombre completo</FormLabel>
                    <FormControl>
                      <Input placeholder="Nombre y apellidos" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <div className="grid gap-3 sm:grid-cols-2">
                <FormField
                  control={form.control}
                  name="owner.phone"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Teléfono</FormLabel>
                      <FormControl>
                        <Input placeholder="300 000 0000" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="owner.email"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Correo</FormLabel>
                      <FormControl>
                        <Input type="email" placeholder="correo@ejemplo.co" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                <FormField
                  control={form.control}
                  name="owner.address"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Dirección</FormLabel>
                      <FormControl>
                        <Input placeholder="Calle 12 # 34-56" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="owner.city"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Ciudad</FormLabel>
                      <FormControl>
                        <Input placeholder="Bogotá D.C." {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>
            </fieldset>

            <fieldset className="space-y-3">
              <legend className="text-muted-foreground flex w-full items-center gap-2 text-xs font-semibold tracking-wide uppercase">
                Paciente <Separator className="flex-1" />
              </legend>
              <div className="grid gap-3 sm:grid-cols-3">
                <FormField
                  control={form.control}
                  name="name"
                  render={({ field }) => (
                    <FormItem className="sm:col-span-1">
                      <FormLabel>Nombre</FormLabel>
                      <FormControl>
                        <Input placeholder="Nombre del paciente" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="speciesId"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Especie</FormLabel>
                      <Select value={field.value?.toString()} onValueChange={(v) => field.onChange(Number(v))}>
                        <FormControl>
                          <SelectTrigger className="w-full">
                            <SelectValue placeholder="Selecciona…" />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          {species.map((s) => (
                            <SelectItem key={s.id} value={s.id.toString()}>
                              {s.name}
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
                  name="breedId"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Raza</FormLabel>
                      <Select
                        value={field.value?.toString() ?? ""}
                        onValueChange={(v) =>
                          field.onChange(v ? Number(v) : undefined)
                        }
                        disabled={!currentSpeciesId}
                      >
                        <FormControl>
                          <SelectTrigger className="w-full">
                            <SelectValue placeholder="—" />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          {breeds.map((b) => (
                            <SelectItem key={b.id} value={b.id.toString()}>
                              {b.name}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>

              <div className="grid gap-3 sm:grid-cols-2">
                <FormField
                  control={form.control}
                  name="sex"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Sexo</FormLabel>
                      <Select value={field.value} onValueChange={field.onChange}>
                        <FormControl>
                          <SelectTrigger className="w-full">
                            <SelectValue placeholder="Selecciona…" />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          <SelectItem value="M">Macho</SelectItem>
                          <SelectItem value="F">Hembra</SelectItem>
                        </SelectContent>
                      </Select>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="birthDate"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Fecha de nacimiento</FormLabel>
                      <FormControl>
                        <Input type="date" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>

              <div className="grid gap-3 sm:grid-cols-2">
                <FormField
                  control={form.control}
                  name="color"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Color / señas</FormLabel>
                      <FormControl>
                        <Input placeholder="Pardo claro, mancha blanca" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="microchip"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Microchip</FormLabel>
                      <FormControl>
                        <Input placeholder="985112000000000" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>

              <FormField
                control={form.control}
                name="neutered"
                render={({ field }) => (
                  <FormItem>
                    <div className="flex items-center gap-2">
                      <FormControl>
                        <input
                          type="checkbox"
                          className="accent-primary size-4 rounded"
                          checked={field.value}
                          onChange={field.onChange}
                        />
                      </FormControl>
                      <Label htmlFor="neutered">Esterilizado / castrado</Label>
                    </div>
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
                      <Input placeholder="Alergias, comportamiento, etc." {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </fieldset>

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
              >
                Cancelar
              </Button>
              <Button type="submit" disabled={createPatient.isPending}>
                {createPatient.isPending && (
                  <Loader2 className="animate-spin" />
                )}
                Guardar paciente
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
