import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import {
  FlaskConical,
  Loader2,
  Microscope,
  Pencil,
  Plus,
  Power,
  Trash2,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
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
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Skeleton } from "@/components/ui/skeleton";
import {
  useAnalytes,
  useAnalyzers,
  useCreateAnalyzer,
  useCreateReferenceRange,
  useDeleteAnalyzer,
  useDeleteReferenceRange,
  useReferenceRanges,
  useSetAnalyzerActive,
  useSpecies,
  useUpdateAnalyzer,
  useUpdateReferenceRange,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import type { Analyzer, ReferenceRange } from "@/bindings";

// ============================ Formularios ===================================

const analyzerSchema = z.object({
  code: z
    .string()
    .min(1, "El código es obligatorio")
    .max(20, "Máximo 20 caracteres"),
  name: z.string().min(1, "El nombre es obligatorio"),
  manufacturer: z.string().optional(),
  model: z.string().optional(),
  notes: z.string().optional(),
});
type AnalyzerValues = z.infer<typeof analyzerSchema>;

const rangeSchema = z
  .object({
    analyteId: z.coerce.number().min(1, "Selecciona el analito"),
    speciesId: z.coerce.number().min(1, "Selecciona la especie"),
    sex: z.enum(["BOTH", "M", "F"]),
    ageMin: z.coerce
      .number({ error: "Número requerido" })
      .min(0, "No puede ser negativo"),
    ageMax: z.coerce
      .number({ error: "Número requerido" })
      .min(0, "No puede ser negativo"),
    minValue: z.coerce.number({ error: "Número requerido" }),
    maxValue: z.coerce.number({ error: "Número requerido" }),
    criticalMin: z.string().optional(),
    criticalMax: z.string().optional(),
    notes: z.string().optional(),
  })
  .refine((v) => v.ageMax >= v.ageMin, {
    message: "La edad máxima debe ser mayor o igual a la mínima",
    path: ["ageMax"],
  })
  .refine((v) => v.maxValue >= v.minValue, {
    message: "El valor máximo debe ser mayor o igual al mínimo",
    path: ["maxValue"],
  });
type RangeValues = z.infer<typeof rangeSchema>;

const toOptNumber = (s: string | undefined) => {
  const t = s?.trim();
  return t && !Number.isNaN(Number(t)) ? Number(t) : null;
};

const SEX_LABEL: Record<string, string> = {
  BOTH: "Ambos sexos",
  M: "Macho (M)",
  F: "Hembra (F)",
};

// ======================== Diálogo de equipo =================================

function AnalyzerDialog({
  open,
  onOpenChange,
  analyzer,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  analyzer: Analyzer | null;
}) {
  const createAnalyzer = useCreateAnalyzer();
  const updateAnalyzer = useUpdateAnalyzer();
  const isEdit = analyzer != null;

  const form = useForm<
    z.input<typeof analyzerSchema>,
    unknown,
    z.output<typeof analyzerSchema>
  >({
    resolver: zodResolver(analyzerSchema),
    defaultValues: {
      code: "",
      name: "",
      manufacturer: "",
      model: "",
      notes: "",
    },
  });

  useEffect(() => {
    if (open) {
      form.reset({
        code: analyzer?.code ?? "",
        name: analyzer?.name ?? "",
        manufacturer: analyzer?.manufacturer ?? "",
        model: analyzer?.model ?? "",
        notes: analyzer?.notes ?? "",
      });
    }
  }, [open, analyzer, form]);

  const pending = createAnalyzer.isPending || updateAnalyzer.isPending;

  const onSubmit = async (values: AnalyzerValues) => {
    try {
      const input = {
        code: values.code.trim(),
        name: values.name.trim(),
        manufacturer: values.manufacturer?.trim() || null,
        model: values.model?.trim() || null,
        notes: values.notes?.trim() || null,
      };
      if (isEdit && analyzer) {
        await updateAnalyzer.mutateAsync({ id: analyzer.id, ...input });
        toast.success("Equipo actualizado");
      } else {
        await createAnalyzer.mutateAsync(input);
        toast.success("Equipo creado", {
          description: "Ya aparece en el selector de muestras y en la gestión de rangos.",
        });
      }
      onOpenChange(false);
    } catch (e) {
      toast.error(isEdit ? "No se pudo actualizar" : "No se pudo crear", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Microscope className="size-5 text-primary" />
            {isEdit ? "Editar equipo" : "Nuevo equipo analizador"}
          </DialogTitle>
          <DialogDescription>
            Marca y modelo del equipo (p. ej. MINDRAY B2800). Sus rangos de
            referencia se gestionan aparte.
          </DialogDescription>
        </DialogHeader>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="code"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Código</FormLabel>
                    <FormControl>
                      <Input placeholder="MINDRAY-B2800" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="name"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Nombre comercial</FormLabel>
                    <FormControl>
                      <Input placeholder="MINDRAY B2800" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="manufacturer"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Fabricante (opcional)</FormLabel>
                    <FormControl>
                      <Input placeholder="MINDRAY" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="model"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Modelo (opcional)</FormLabel>
                    <FormControl>
                      <Input placeholder="B2800" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>
            <FormField
              control={form.control}
              name="notes"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Notas (opcional)</FormLabel>
                  <FormControl>
                    <Textarea
                      placeholder="Tipo de análisis, observaciones del equipo…"
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
              <Button type="submit" disabled={pending}>
                {pending ? <Loader2 className="animate-spin" /> : <Plus className="size-4" />}
                {isEdit ? "Guardar cambios" : "Crear equipo"}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}

// ======================= Diálogo de rango ===================================

function RangeDialog({
  open,
  onOpenChange,
  analyzer,
  range,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  analyzer: Analyzer;
  range: ReferenceRange | null;
}) {
  const { data: analytes = [] } = useAnalytes();
  const { data: species = [] } = useSpecies();
  const createRange = useCreateReferenceRange();
  const updateRange = useUpdateReferenceRange();
  const isEdit = range != null;

  const form = useForm<
    z.input<typeof rangeSchema>,
    unknown,
    z.output<typeof rangeSchema>
  >({
    resolver: zodResolver(rangeSchema),
    defaultValues: {
      analyteId: undefined,
      speciesId: undefined,
      sex: "BOTH",
      ageMin: 0,
      ageMax: 2400,
      minValue: undefined,
      maxValue: undefined,
      criticalMin: "",
      criticalMax: "",
      notes: "",
    },
  });

  useEffect(() => {
    if (open) {
      form.reset({
        analyteId: range?.analyteId ?? undefined,
        speciesId: range?.speciesId ?? undefined,
        sex: range?.sex === "M" || range?.sex === "F" ? range.sex : "BOTH",
        ageMin: range?.ageMinMonths ?? 0,
        ageMax: range?.ageMaxMonths ?? 2400,
        minValue: range?.minValue ?? undefined,
        maxValue: range?.maxValue ?? undefined,
        criticalMin: range?.criticalMin != null ? String(range.criticalMin) : "",
        criticalMax: range?.criticalMax != null ? String(range.criticalMax) : "",
        notes: range?.notes ?? "",
      });
    }
  }, [open, range, form]);

  const pending = createRange.isPending || updateRange.isPending;

  const onSubmit = async (values: RangeValues) => {
    const input = {
      analyzerId: analyzer.id,
      analyteId: values.analyteId,
      speciesId: values.speciesId,
      sex: values.sex === "BOTH" ? null : values.sex,
      ageMinMonths: values.ageMin,
      ageMaxMonths: values.ageMax,
      minValue: values.minValue,
      maxValue: values.maxValue,
      criticalMin: toOptNumber(values.criticalMin),
      criticalMax: toOptNumber(values.criticalMax),
      notes: values.notes?.trim() || null,
    };
    try {
      if (isEdit && range) {
        await updateRange.mutateAsync({ id: range.id, input });
        toast.success("Rango actualizado");
      } else {
        await createRange.mutateAsync(input);
        toast.success("Rango creado", {
          description: `Se validará contra el equipo ${analyzer.name}.`,
        });
      }
      onOpenChange(false);
    } catch (e) {
      toast.error(isEdit ? "No se pudo actualizar" : "No se pudo crear", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FlaskConical className="size-5 text-primary" />
            {isEdit ? "Editar rango" : "Nuevo rango de referencia"}
          </DialogTitle>
          <DialogDescription>
            Rango para {analyzer.name}. Si el equipo no tiene rango propio, la
            validación respalda con el perfil General.
          </DialogDescription>
        </DialogHeader>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="analyteId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Analito</FormLabel>
                    <Select
                      value={field.value ? field.value.toString() : ""}
                      onValueChange={(v) => field.onChange(Number(v))}
                    >
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue placeholder="Selecciona…" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {analytes.map((a) => (
                          <SelectItem key={a.id} value={a.id.toString()}>
                            {a.name}
                            {a.unit ? ` (${a.unit})` : ""}
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
                name="speciesId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Especie</FormLabel>
                    <Select
                      value={field.value ? field.value.toString() : ""}
                      onValueChange={(v) => field.onChange(Number(v))}
                    >
                      <FormControl>
                        <SelectTrigger>
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
                name="sex"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Sexo</FormLabel>
                    <Select
                      value={field.value}
                      onValueChange={field.onChange}
                    >
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value="BOTH">Ambos sexos</SelectItem>
                        <SelectItem value="M">Macho (M)</SelectItem>
                        <SelectItem value="F">Hembra (F)</SelectItem>
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <div className="grid grid-cols-2 gap-2">
                <FormField
                  control={form.control}
                  name="ageMin"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Edad mín (meses)</FormLabel>
                      <FormControl>
                        <Input type="number" min={0} {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="ageMax"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Edad máx (meses)</FormLabel>
                      <FormControl>
                        <Input type="number" min={0} {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>
              <FormField
                control={form.control}
                name="minValue"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Valor mínimo</FormLabel>
                    <FormControl>
                      <Input type="number" step="any" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="maxValue"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Valor máximo</FormLabel>
                    <FormControl>
                      <Input type="number" step="any" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="criticalMin"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Crítico mín (opcional)</FormLabel>
                    <FormControl>
                      <Input type="number" step="any" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="criticalMax"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Crítico máx (opcional)</FormLabel>
                    <FormControl>
                      <Input type="number" step="any" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>
            <FormField
              control={form.control}
              name="notes"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Notas (opcional)</FormLabel>
                  <FormControl>
                    <Textarea
                      placeholder="Origen del valor (inserto del fabricante, literatura…)…"
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
              <Button type="submit" disabled={pending}>
                {pending ? <Loader2 className="animate-spin" /> : <Plus className="size-4" />}
                {isEdit ? "Guardar cambios" : "Crear rango"}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}

// ====================== Tarjeta principal ===================================

export function AnalyzerManagementCard() {
  const { data: analyzers = [], isLoading } = useAnalyzers();
  const setActive = useSetAnalyzerActive();
  const deleteAnalyzer = useDeleteAnalyzer();
  const deleteRange = useDeleteReferenceRange();

  const [analyzerDialog, setAnalyzerDialog] = useState(false);
  const [editingAnalyzer, setEditingAnalyzer] = useState<Analyzer | null>(null);
  const [rangeDialog, setRangeDialog] = useState(false);
  const [editingRange, setEditingRange] = useState<ReferenceRange | null>(null);

  // Equipo seleccionado para ver sus rangos (por defecto el primero activo).
  const [selectedId, setSelectedId] = useState<number | null>(null);
  useEffect(() => {
    if (selectedId == null && analyzers.length > 0) {
      const first =
        analyzers.find((a) => a.isActive && a.code !== "GENERAL") ?? analyzers[0];
      setSelectedId(first.id);
    }
  }, [analyzers, selectedId]);

  const selectedAnalyzer =
    analyzers.find((a) => a.id === selectedId) ?? null;
  const { data: ranges = [], isLoading: loadingRanges } = useReferenceRanges(
    selectedAnalyzer?.id ?? null,
  );

  const openNewAnalyzer = () => {
    setEditingAnalyzer(null);
    setAnalyzerDialog(true);
  };
  const openEditAnalyzer = (a: Analyzer) => {
    setEditingAnalyzer(a);
    setAnalyzerDialog(true);
  };
  const openNewRange = () => {
    setEditingRange(null);
    setRangeDialog(true);
  };
  const openEditRange = (r: ReferenceRange) => {
    setEditingRange(r);
    setRangeDialog(true);
  };

  const toggleActive = async (a: Analyzer) => {
    try {
      const updated = await setActive.mutateAsync({
        id: a.id,
        active: !a.isActive,
      });
      toast.success(
        updated.isActive
          ? `${updated.name} activado`
          : `${updated.name} desactivado`,
      );
    } catch (e) {
      toast.error("No se pudo cambiar el estado", {
        description: getErrorMessage(e),
      });
    }
  };

  const removeAnalyzer = async (a: Analyzer) => {
    if (!window.confirm(`¿Eliminar el equipo ${a.name}? Se borrarán sus rangos.`)) return;
    try {
      await deleteAnalyzer.mutateAsync(a.id);
      toast.success("Equipo eliminado");
    } catch (e) {
      toast.error("No se pudo eliminar", { description: getErrorMessage(e) });
    }
  };

  const removeRange = async (r: ReferenceRange) => {
    if (!window.confirm(`¿Eliminar el rango de ${r.analyteName} (${r.speciesName})?`)) return;
    try {
      await deleteRange.mutateAsync(r.id);
      toast.success("Rango eliminado");
    } catch (e) {
      toast.error("No se pudo eliminar", { description: getErrorMessage(e) });
    }
  };

  const fmtAge = (min: number, max: number) =>
    `${min}–${max} meses`;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Microscope className="size-4 text-primary" />
          Equipos de laboratorio
        </CardTitle>
        <CardDescription>
          Marca/modelo de los analizadores y sus valores de referencia. El
          operario elige el equipo al recibir cada muestra; la validación usa
          sus rangos (con respaldo al perfil General).
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Lista de equipos */}
        {isLoading ? (
          <Skeleton className="h-24 w-full" />
        ) : (
          <div className="space-y-2">
            {analyzers.map((a) => (
              <div
                key={a.id}
                className="bg-muted/40 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-lg border px-3 py-2.5"
              >
                <div className="min-w-0 flex-1">
                  <p className="flex flex-wrap items-center gap-2 text-sm font-semibold">
                    {a.name}
                    {!a.isActive && (
                      <Badge variant="outline" className="text-muted-foreground">
                        Inactivo
                      </Badge>
                    )}
                  </p>
                  <p className="text-muted-foreground truncate text-xs">
                    {a.code}
                    {a.manufacturer ? ` · ${a.manufacturer}` : ""}
                    {a.model ? ` · ${a.model}` : ""}
                  </p>
                </div>
                <Badge variant="secondary" className="shrink-0">
                  {a.rangeCount} rango{a.rangeCount === 1 ? "" : "s"}
                </Badge>
                <div className="flex shrink-0 items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-8 px-2"
                    onClick={() => openEditAnalyzer(a)}
                    disabled={a.code === "GENERAL"}
                    title={a.code === "GENERAL" ? "El perfil General no se edita" : "Editar"}
                  >
                    <Pencil className="size-3.5" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-8 px-2"
                    onClick={() => toggleActive(a)}
                    disabled={a.code === "GENERAL" || setActive.isPending}
                    title={a.isActive ? "Desactivar" : "Activar"}
                  >
                    <Power className="size-3.5" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-8 px-2 text-destructive hover:text-destructive"
                    onClick={() => removeAnalyzer(a)}
                    disabled={a.code === "GENERAL" || deleteAnalyzer.isPending}
                    title="Eliminar"
                  >
                    <Trash2 className="size-3.5" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
        <Button type="button" variant="outline" size="sm" onClick={openNewAnalyzer}>
          <Plus className="size-4" />
          Nuevo equipo
        </Button>

        {/* Gestión de rangos del equipo seleccionado */}
        <div className="space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex flex-wrap items-center gap-3">
              <p className="text-sm font-semibold">Rangos de referencia</p>
              {analyzers.length > 0 && (
                <Select
                  value={selectedAnalyzer ? selectedAnalyzer.id.toString() : ""}
                  onValueChange={(v) => setSelectedId(Number(v))}
                >
                  <SelectTrigger className="w-64">
                    <SelectValue placeholder="Selecciona equipo…" />
                  </SelectTrigger>
                  <SelectContent>
                    {analyzers.map((a) => (
                      <SelectItem key={a.id} value={a.id.toString()}>
                        {a.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </div>
            {selectedAnalyzer && selectedAnalyzer.code !== "GENERAL" && (
              <Button type="button" variant="outline" size="sm" onClick={openNewRange}>
                <Plus className="size-4" />
                Nuevo rango
              </Button>
            )}
          </div>

          {selectedAnalyzer?.code === "GENERAL" && (
            <p className="text-muted-foreground rounded-lg border border-dashed px-3 py-2.5 text-xs">
              El perfil <span className="font-medium">General (lectura manual)</span>{" "}
              contiene los rangos estándar por especie, sexo y edad. Crea un
              equipo (p. ej. MINDRAY B2800) para cargar sus propios valores de
              referencia.
            </p>
          )}

          {loadingRanges ? (
            <Skeleton className="h-40 w-full" />
          ) : ranges.length === 0 ? (
            <p className="text-muted-foreground rounded-lg border border-dashed px-3 py-6 text-center text-xs">
              Sin rangos para {selectedAnalyzer?.name ?? "el equipo seleccionado"}.
            </p>
          ) : (
            <div className="overflow-hidden rounded-lg border">
              <Table>
                <TableHeader>
                  <TableRow className="hover:bg-transparent">
                    <TableHead>Analito</TableHead>
                    <TableHead>Especie</TableHead>
                    <TableHead>Sexo</TableHead>
                    <TableHead>Edad (meses)</TableHead>
                    <TableHead>Rango</TableHead>
                    <TableHead>Crítico</TableHead>
                    <TableHead className="text-right">Acciones</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {ranges.map((r) => (
                    <TableRow key={r.id}>
                      <TableCell className="font-medium">{r.analyteName}</TableCell>
                      <TableCell>{r.speciesName}</TableCell>
                      <TableCell className="text-muted-foreground text-xs">
                        {r.sex ? (r.sex === "M" ? "Macho" : "Hembra") : "Ambos"}
                      </TableCell>
                      <TableCell className="text-muted-foreground text-xs">
                        {fmtAge(r.ageMinMonths, r.ageMaxMonths)}
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {r.minValue} – {r.maxValue}
                        {r.unit ? ` ${r.unit}` : ""}
                      </TableCell>
                      <TableCell className="text-muted-foreground font-mono text-xs">
                        {r.criticalMin != null || r.criticalMax != null
                          ? `${r.criticalMin ?? "—"} – ${r.criticalMax ?? "—"}`
                          : "—"}
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 px-2"
                            onClick={() => openEditRange(r)}
                            title="Editar"
                          >
                            <Pencil className="size-3.5" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 px-2 text-destructive hover:text-destructive"
                            onClick={() => removeRange(r)}
                            disabled={deleteRange.isPending}
                            title="Eliminar"
                          >
                            <Trash2 className="size-3.5" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </div>
      </CardContent>

      <AnalyzerDialog
        open={analyzerDialog}
        onOpenChange={setAnalyzerDialog}
        analyzer={editingAnalyzer}
      />
      {selectedAnalyzer && (
        <RangeDialog
          open={rangeDialog}
          onOpenChange={setRangeDialog}
          analyzer={selectedAnalyzer}
          range={editingRange}
        />
      )}
    </Card>
  );
}
