import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Loader2, Save } from "lucide-react";
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
import { useSaveResource } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { RESOURCE_KIND } from "@/lib/status";
import type { Resource } from "@/bindings";

const RESOURCE_KINDS = [
  "CANIL",
  "GATO",
  "RECEPERA",
  "QUIRURGICO",
  "AISLAMIENTO",
  "PERRERA",
] as const;

const formSchema = z.object({
  name: z.string().trim().min(1, "Nombre requerido"),
  kind: z.enum(RESOURCE_KINDS),
  capacity: z.coerce.number().int().min(1, "Mínimo 1").max(99),
  active: z.boolean(),
  notes: z.string().nullable(),
});

type FormValues = z.infer<typeof formSchema>;

export function ResourceDialog({
  open,
  onOpenChange,
  editing,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Recurso a editar, o null para crear. */
  editing: Resource | null;
}) {
  const saveResource = useSaveResource();

  const form = useForm<z.input<typeof formSchema>, unknown, FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      name: "",
      kind: "CANIL",
      capacity: 1,
      active: true,
      notes: null,
    },
  });

  useEffect(() => {
    if (open) {
      form.reset(
        editing
          ? {
              name: editing.name,
              kind: editing.kind as FormValues["kind"],
              capacity: editing.capacity,
              active: editing.active,
              notes: editing.notes,
            }
          : {
              name: "",
              kind: "CANIL",
              capacity: 1,
              active: true,
              notes: null,
            },
      );
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, editing]);

  const onSubmit = async (values: FormValues) => {
    try {
      const resource = await saveResource.mutateAsync({
        id: editing?.id ?? null,
        name: values.name,
        kind: values.kind,
        capacity: values.capacity,
        active: values.active,
        notes: values.notes?.trim() || null,
      });
      toast.success(editing ? "Recurso actualizado" : "Recurso creado", {
        description: resource.name,
      });
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo guardar el recurso", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {editing ? "Editar recurso" : "Nuevo recurso"}
          </DialogTitle>
          <DialogDescription>
            Espacio físico de la clínica: canil, gatera, box de hospitalización…
            La capacidad indica cuántos pacientes caben a la vez.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Nombre *</FormLabel>
                  <FormControl>
                    <Input {...field} placeholder="Canil 3" />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <div className="grid grid-cols-2 gap-3">
              <FormField
                control={form.control}
                name="kind"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Tipo *</FormLabel>
                    <Select value={field.value} onValueChange={field.onChange}>
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {RESOURCE_KINDS.map((k) => (
                          <SelectItem key={k} value={k}>
                            {RESOURCE_KIND[k]}
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
                name="capacity"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Capacidad *</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        min={1}
                        max={99}
                        {...field}
                        value={(field.value as number | undefined) ?? 1}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <FormField
              control={form.control}
              name="active"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Estado</FormLabel>
                  <Select
                    value={field.value ? "active" : "inactive"}
                    onValueChange={(v) => field.onChange(v === "active")}
                  >
                    <FormControl>
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      <SelectItem value="active">Activo</SelectItem>
                      <SelectItem value="inactive">Inactivo</SelectItem>
                    </SelectContent>
                  </Select>
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
                      value={field.value ?? ""}
                      onChange={(e) => field.onChange(e.target.value || null)}
                      placeholder="Ubicación, características…"
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
              <Button type="submit" disabled={saveResource.isPending}>
                {saveResource.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <Save className="size-4" />
                )}
                Guardar
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
