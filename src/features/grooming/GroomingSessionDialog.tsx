import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import {
  CalendarPlus,
  Camera,
  Loader2,
  Save,
  Scissors,
  Trash2,
} from "lucide-react";
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
  useAddVisitAttachment,
  useDeleteVisitAttachment,
  useFinishGroomingSession,
  useGroomingSession,
  useUpsertGroomingSession,
  useVisitAttachments,
} from "@/hooks/use-queries";
import { useUsers } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { api } from "@/lib/api";
import {
  GROOMING_BEHAVIOR_LABEL,
  GROOMING_BATH_LABEL,
  GROOMING_MATTING,
  VISIT_ATTACHMENT_KIND,
} from "@/lib/status";
import { ROLE_LABEL } from "@/lib/status";
import type { GroomingDayLine } from "@/bindings";

const schema = z.object({
  groomerId: z.number().optional(),
  style: z.string().optional(),
  bathType: z.string().optional(),
  coatCondition: z.string().optional(),
  mattingLevel: z.number().min(0).max(3),
  weightKg: z.number().optional(),
  behavior: z.string().optional(),
  productsUsed: z.string().optional(),
  nextSuggestedAt: z.string().optional(),
  notes: z.string().optional(),
});

type Values = z.infer<typeof schema>;

/** "0" (sin asignar) ↔ undefined para el Select de Radix. */
const NONE = "__none__";

export function GroomingSessionDialog({
  open,
  onOpenChange,
  line,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Línea de peluquería del tablero (identifica la visita y la sesión). */
  line: GroomingDayLine | null;
}) {
  const upsert = useUpsertGroomingSession();
  const finish = useFinishGroomingSession();
  const addAttachment = useAddVisitAttachment();
  const removeAttachment = useDeleteVisitAttachment();
  const { data: users = [] } = useUsers();
  const activeUsers = users.filter((u) => u.active);

  const { data: session } = useGroomingSession(
    open && line ? line.visitServiceId : null,
  );
  const { data: attachments = [] } = useVisitAttachments(
    open && line ? line.visitId : null,
  );

  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      mattingLevel: 0,
    },
  });

  // Prefill con la ficha existente (y el peluquero de la línea si no hay).
  useEffect(() => {
    if (open && session) {
      form.reset({
        groomerId: session.groomerId ?? line?.professionalId ?? undefined,
        style: session.style ?? "",
        bathType: session.bathType ?? "",
        coatCondition: session.coatCondition ?? "",
        mattingLevel: session.mattingLevel,
        weightKg: session.weightKg ?? undefined,
        behavior: session.behavior ?? "",
        productsUsed: session.productsUsed ?? "",
        nextSuggestedAt: session.nextSuggestedAt ?? "",
        notes: session.notes ?? "",
      });
    } else if (open && line) {
      form.reset({
        groomerId: line.professionalId ?? undefined,
        mattingLevel: 0,
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, session?.id, line?.visitServiceId]);

  const hasGroomer =
    (form.watch("groomerId") ?? 0) > 0 || line?.professionalId != null;
  const finalized = session?.finishedAt != null;

  const save = async (values: Values) => {
    if (!line) return;
    try {
      await upsert.mutateAsync({
        visitServiceId: line.visitServiceId,
        groomerId: values.groomerId && values.groomerId > 0 ? values.groomerId : null,
        style: values.style?.trim() || null,
        bathType: values.bathType?.trim() || null,
        coatCondition: values.coatCondition?.trim() || null,
        mattingLevel: values.mattingLevel ?? 0,
        weightKg: values.weightKg && values.weightKg > 0 ? values.weightKg : null,
        behavior: values.behavior?.trim() || null,
        productsUsed: values.productsUsed?.trim() || null,
        startedAt: session?.startedAt ?? null,
        nextSuggestedAt: values.nextSuggestedAt?.trim() || null,
        notes: values.notes?.trim() || null,
      });
      toast.success("Ficha de peluquería guardada");
    } catch (e) {
      toast.error("No se pudo guardar la ficha", {
        description: getErrorMessage(e),
      });
    }
  };

  const doFinish = async () => {
    if (!line) return;
    // Guarda primero la ficha con lo que haya en el formulario.
    const values = form.getValues();
    const nextSuggestedAt = values.nextSuggestedAt?.trim() || null;
    try {
      await save({
        ...values,
        mattingLevel: values.mattingLevel ?? 0,
      });
      const finished = await finish.mutateAsync({
        visitServiceId: line.visitServiceId,
        input: {
          finishedAt: null,
          nextSuggestedAt,
          notes: null,
        },
      });
      toast.success("Sesión finalizada", {
        description: nextSuggestedAt
          ? `Próximo turno sugerido: ${finished.nextSuggestedAt}.`
          : "La línea de servicio quedó completada.",
      });
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo finalizar la sesión", {
        description: getErrorMessage(e),
      });
    }
  };

  /** Agenda el próximo turno como cita del servicio en la fecha sugerida. */
  const scheduleNext = async (date: string) => {
    if (!line) return;
    try {
      const appt = await api.createAppointment({
        patientId: line.patientId,
        serviceId: line.serviceId,
        professionalId: line.professionalId,
        scheduledAt: `${date} 09:00:00`,
        durationMin: null,
        notes: `Turno de peluquería sugerido (visita ${line.visitCode})`,
      });
      toast.success(`Cita ${appt.code} creada`, {
        description: `${date} 09:00 · ${line.serviceName}`,
      });
    } catch (e) {
      toast.error("No se pudo agendar el turno", {
        description: getErrorMessage(e),
      });
    }
  };

  /** Foto antes/después: elige archivo y lo copia a app_data/attachments. */
  const pickPhoto = async (kind: "BEFORE" | "AFTER") => {
    if (!line) return;
    try {
      const selected = await openFileDialog({
        multiple: false,
        directory: false,
        title: kind === "BEFORE" ? "Foto ANTES de la sesión" : "Foto DESPUÉS de la sesión",
        filters: [{ name: "Imágenes", extensions: ["png", "jpg", "jpeg", "webp"] }],
      });
      if (!selected || Array.isArray(selected)) return;
      await addAttachment.mutateAsync({
        sourcePath: selected,
        visitId: line.visitId,
        kind,
      });
      toast.success(
        kind === "BEFORE" ? "Foto de antes adjuntada" : "Foto de después adjuntada",
      );
    } catch (e) {
      toast.error("No se pudo adjuntar la foto", {
        description: getErrorMessage(e),
      });
    }
  };

  const deleteAttachment = async (id: number, fileName: string) => {
    if (!window.confirm(`¿Eliminar el adjunto "${fileName}"?`)) return;
    try {
      await removeAttachment.mutateAsync(id);
      toast.success("Adjunto eliminado");
    } catch (e) {
      toast.error("No se pudo eliminar", { description: getErrorMessage(e) });
    }
  };

  const openAttachment = async (path: string) => {
    try {
      await openPath(path);
    } catch {
      toast.error("No se pudo abrir el archivo");
    }
  };

  if (!line) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Scissors className="text-primary size-5" />
            Sesión de peluquería · {line.patientName}
          </DialogTitle>
          <DialogDescription>
            {line.serviceName} · visita {line.visitCode} · propietario{" "}
            {line.ownerName}
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(save)} className="space-y-4">
            <div className="grid gap-3 sm:grid-cols-2">
              {/* Peluquero */}
              <FormField
                control={form.control}
                name="groomerId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Peluquero</FormLabel>
                    <Select
                      value={field.value ? field.value.toString() : NONE}
                      onValueChange={(v) =>
                        field.onChange(v === NONE ? undefined : Number(v))
                      }
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue placeholder="Sin asignar" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value={NONE}>Sin asignar</SelectItem>
                        {activeUsers.map((u) => (
                          <SelectItem key={u.id} value={u.id.toString()}>
                            {u.fullName} · {ROLE_LABEL[u.role] ?? u.role}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
              {/* Nivel de enredos */}
              <FormField
                control={form.control}
                name="mattingLevel"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Nivel de enredos</FormLabel>
                    <Select
                      value={(field.value ?? 0).toString()}
                      onValueChange={(v) => field.onChange(Number(v))}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {[0, 1, 2, 3].map((lvl) => (
                          <SelectItem key={lvl} value={lvl.toString()}>
                            {lvl} · {GROOMING_MATTING[lvl]}
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
              {/* Estilo */}
              <FormField
                control={form.control}
                name="style"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Estilo</FormLabel>
                    <FormControl>
                      <Input placeholder="Corte de raza, higiénico…" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              {/* Tipo de baño */}
              <FormField
                control={form.control}
                name="bathType"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Tipo de baño</FormLabel>
                    <Select
                      value={
                        field.value && GROOMING_BATH_LABEL[field.value]
                          ? field.value
                          : NONE
                      }
                      onValueChange={(v) => field.onChange(v === NONE ? "" : v)}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue placeholder="Sin especificar" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value={NONE}>Sin especificar</SelectItem>
                        {Object.entries(GROOMING_BATH_LABEL).map(([k, label]) => (
                          <SelectItem key={k} value={k}>
                            {label}
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
              {/* Condición del pelaje */}
              <FormField
                control={form.control}
                name="coatCondition"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Condición del pelaje</FormLabel>
                    <FormControl>
                      <Input placeholder="Pelaje grueso, seco…" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              {/* Peso */}
              <FormField
                control={form.control}
                name="weightKg"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Peso (kg)</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        step="0.1"
                        min="0"
                        value={field.value ?? ""}
                        onChange={(e) =>
                          field.onChange(
                            e.target.value === "" ? undefined : Number(e.target.value),
                          )
                        }
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              {/* Comportamiento */}
              <FormField
                control={form.control}
                name="behavior"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Comportamiento</FormLabel>
                    <Select
                      value={
                        field.value && GROOMING_BEHAVIOR_LABEL[field.value]
                          ? field.value
                          : NONE
                      }
                      onValueChange={(v) => field.onChange(v === NONE ? "" : v)}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue placeholder="Sin especificar" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value={NONE}>Sin especificar</SelectItem>
                        {Object.entries(GROOMING_BEHAVIOR_LABEL).map(([k, label]) => (
                          <SelectItem key={k} value={k}>
                            {label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
              {/* Próximo turno sugerido */}
              <FormField
                control={form.control}
                name="nextSuggestedAt"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Próximo turno sugerido</FormLabel>
                    <FormControl>
                      <Input type="date" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            {/* Productos */}
            <FormField
              control={form.control}
              name="productsUsed"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Productos usados</FormLabel>
                  <FormControl>
                    <Input
                      placeholder="Shampoo medicado, deslanador…"
                      {...field}
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
                  <FormLabel>Notas</FormLabel>
                  <FormControl>
                    <Textarea rows={2} className="resize-none" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Fotos antes/después */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <p className="text-xs font-medium tracking-wide uppercase">
                  Fotos antes / después
                </p>
                <div className="flex gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="gap-1.5"
                    onClick={() => pickPhoto("BEFORE")}
                  >
                    <Camera className="size-3.5" />
                    Antes
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="gap-1.5"
                    onClick={() => pickPhoto("AFTER")}
                  >
                    <Camera className="size-3.5" />
                    Después
                  </Button>
                </div>
              </div>
              {attachments.length === 0 ? (
                <p className="text-muted-foreground text-xs">
                  Aún no hay fotos adjuntas a la visita.
                </p>
              ) : (
                <ul className="grid gap-1 sm:grid-cols-2">
                  {attachments.map((att) => (
                    <li
                      key={att.id}
                      className="bg-muted/40 flex items-center gap-2 rounded-md border px-2.5 py-1.5 text-xs"
                    >
                      <button
                        type="button"
                        onClick={() => openAttachment(att.filePath)}
                        className="min-w-0 flex-1 truncate text-left hover:underline"
                        title={att.fileName}
                      >
                        {att.fileName}
                      </button>
                      <Badge variant="secondary">
                        {VISIT_ATTACHMENT_KIND[att.kind ?? "DOCUMENT"] ??
                          att.kind ??
                          "Adjunto"}
                      </Badge>
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        className="text-destructive size-6"
                        aria-label="Eliminar adjunto"
                        onClick={() => deleteAttachment(att.id, att.fileName)}
                      >
                        <Trash2 className="size-3.5" />
                      </Button>
                    </li>
                  ))}
                </ul>
              )}
            </div>

            <DialogFooter className="flex-col-reverse gap-2 pt-2 sm:flex-row sm:justify-between">
              <Button
                type="button"
                variant="ghost"
                onClick={() => onOpenChange(false)}
              >
                Cerrar
              </Button>
              <div className="flex items-center gap-2">
                {!finalized && line.nextSuggestedAt && (
                  <Button
                    type="button"
                    variant="outline"
                    className="gap-1.5"
                    onClick={() => scheduleNext(line.nextSuggestedAt!)}
                    title="Crear cita con este servicio en la fecha sugerida"
                  >
                    <CalendarPlus className="size-4" />
                    Agendar próximo turno
                  </Button>
                )}
                <Button type="submit" disabled={upsert.isPending}>
                  {upsert.isPending ? (
                    <Loader2 className="animate-spin" />
                  ) : (
                    <Save className="size-4" />
                  )}
                  Guardar ficha
                </Button>
                {!finalized && (
                  <Button
                    type="button"
                    variant="secondary"
                    className="gap-1.5"
                    disabled={!hasGroomer || finish.isPending || upsert.isPending}
                    title={
                      hasGroomer
                        ? "Cierra la sesión y completa la línea de servicio"
                        : "Asigna un peluquero antes de finalizar"
                    }
                    onClick={doFinish}
                  >
                    {finish.isPending ? (
                      <Loader2 className="animate-spin" />
                    ) : null}
                    Finalizar sesión
                  </Button>
                )}
              </div>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
