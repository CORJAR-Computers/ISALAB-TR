import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { openPath } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import {
  Ban,
  CheckCircle2,
  FileText,
  FlaskConical,
  HeartPulse,
  Loader2,
  PlayCircle,
  Plus,
  MessageCircle,
  Bot,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
  useGenerateReport,
  usePatient,
  useRegisterLabResult,
  useSample,
  useSetSampleStatus,
} from "@/hooks/use-queries";
import { RESULT_STATUS, SAMPLE_STATUS } from "@/lib/status";
import { cn, formatDateTime } from "@/lib/utils";
import { api, getErrorMessage } from "@/lib/api";
import { useUiStore } from "@/stores/ui-store";
import { usePermissions } from "@/hooks/use-permissions";
import { sendWhatsAppMessage } from "@/lib/whatsapp";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

const resultSchema = z.object({
  analyteId: z.coerce.number().min(1, "Selecciona el analito"),
  value: z.coerce
    .number({ invalid_type_error: "Ingresa un número válido" })
    .min(0, "El valor no puede ser negativo"),
});

type ResultValues = z.infer<typeof resultSchema>;

const STATUS_ICON: Record<string, typeof FlaskConical> = {
  RECIBIDA: FlaskConical,
  EN_PROCESO: PlayCircle,
  FINALIZADA: CheckCircle2,
  ANULADA: Ban,
};

export function SampleDetailDialog({
  sampleId,
  onOpenChange,
}: {
  sampleId: number | null;
  onOpenChange: (open: boolean) => void;
}) {
  const open = sampleId != null;
  const { data: sample, isLoading } = useSample(sampleId);
  const { data: patient } = usePatient(sample?.patientId ?? null);
  const { data: analytes = [] } = useAnalytes();

  const registerResult = useRegisterLabResult();
  const setStatus = useSetSampleStatus();
  const generate = useGenerateReport();

  const setActivePatient = useUiStore((s) => s.setActivePatient);
  const navigate = useUiStore((s) => s.navigate);

  const { isVetOrAdmin } = usePermissions();
  const [confirmAnular, setConfirmAnular] = useState(false);
  const [aiInterpretation, setAiInterpretation] = useState<string | null>(null);
  const [interpreting, setInterpreting] = useState(false);

  const resultForm = useForm<ResultValues>({
    resolver: zodResolver(resultSchema),
    defaultValues: {
      analyteId: 0,
      value: 0,
    },
  });

  const resetForm = () => {
    resultForm.reset({ analyteId: 0, value: 0 });
    setConfirmAnular(false);
    setAiInterpretation(null);
  };

  const pending = registerResult.isPending || setStatus.isPending;

  const onSubmitResult = async (values: ResultValues) => {
    if (!sample) return;
    try {
      const result = await registerResult.mutateAsync({
        sampleId: sample.id,
        analyteId: values.analyteId,
        value: values.value,
      });
      toast.success(`Resultado de ${result.analyteName} cargado`, {
        description: `Valor ${result.value} · estado ${RESULT_STATUS[result.status]?.label ?? result.status}.`,
      });
      resultForm.reset({ analyteId: 0, value: 0 });
    } catch (err) {
      toast.error("No se pudo registrar el resultado", {
        description: getErrorMessage(err),
      });
    }
  };

  const markEnProceso = async () => {
    if (!sample) return;
    try {
      const updated = await setStatus.mutateAsync({
        id: sample.id,
        status: "EN_PROCESO",
      });
      toast.success(`Muestra ${updated.code} en proceso`);
    } catch (err) {
      toast.error("No se pudo cambiar el estado", {
        description: getErrorMessage(err),
      });
    }
  };

  const finalizar = async () => {
    if (!sample) return;
    try {
      const updated = await setStatus.mutateAsync({
        id: sample.id,
        status: "FINALIZADA",
      });
      toast.success(`Muestra ${updated.code} finalizada`, {
        description: "Ya puedes generar el informe PDF.",
      });
    } catch (err) {
      toast.error("No se pudo finalizar la muestra", {
        description: getErrorMessage(err),
      });
    }
  };

  const anular = async () => {
    if (!sample) return;
    if (!confirmAnular) {
      setConfirmAnular(true);
      return;
    }
    try {
      await setStatus.mutateAsync({ id: sample.id, status: "ANULADA" });
      toast.success(`Muestra ${sample.code} anulada`);
      setConfirmAnular(false);
    } catch (err) {
      toast.error("No se pudo anular la muestra", {
        description: getErrorMessage(err),
      });
    }
  };

  const generatePdf = async () => {
    if (!sample) return;
    try {
      const report = await generate.mutateAsync({ sampleId: sample.id, overrideLogoPath: null, saveLogoPreference: false });
      toast.success(`Informe ${report.fileName} generado`, {
        description: "Se abrirá con el visor de PDF del sistema.",
      });
      try {
        await api.openReportFile(report.path);
      } catch {
        await openPath(report.path);
      }
    } catch (err) {
      toast.error("No se pudo generar el PDF", {
        description: getErrorMessage(err),
      });
    }
  };

  const goToHistory = () => {
    if (!sample) return;
    setActivePatient(sample.patientId);
    onOpenChange(false);
    navigate("clinical-history");
  };

  const handleWhatsApp = () => {
    if (!patient?.ownerPhone || !patient?.ownerName || !patient?.name) {
      toast.error("Falta información", {
        description: "El paciente no tiene un número de teléfono del propietario registrado.",
      });
      return;
    }
    const message = `Hola ${patient.ownerName},\n\nTe escribimos de ISALAB para informarte que los resultados de laboratorio de tu mascota *${patient.name}* ya están listos.\n\nPor favor, revisa el archivo adjunto.\n\n¡Gracias por confiar en nosotros!`;
    sendWhatsAppMessage(patient.ownerPhone, message);
  };

  const handleInterpretAI = async () => {
    if (!sample) return;
    setInterpreting(true);
    try {
      const interpretation = await api.interpretLabResults(sample.id);
      setAiInterpretation(interpretation);
    } catch (err) {
      toast.error("Error al interpretar con IA", {
        description: getErrorMessage(err),
      });
    } finally {
      setInterpreting(false);
    }
  };

  const sampleStatus = sample?.status ?? "";
  const st = SAMPLE_STATUS[sampleStatus] ?? {
    label: sampleStatus || "—",
    variant: "secondary" as const,
  };
  const StatusIcon = STATUS_ICON[sampleStatus] ?? FlaskConical;
  const canProcess = sampleStatus === "RECIBIDA";
  const canAnular = sampleStatus === "RECIBIDA" || sampleStatus === "EN_PROCESO";
  const canAddResult = sampleStatus === "RECIBIDA" || sampleStatus === "EN_PROCESO";
  const canFinalize =
    (sampleStatus === "RECIBIDA" || sampleStatus === "EN_PROCESO") &&
    (sample?.results.length ?? 0) > 0;
  const canReport = sampleStatus === "FINALIZADA";
  const analyzed = analytes.filter((a) =>
    sample?.results.some((r) => r.analyteId === a.id),
  );
  const availableAnalytes = analytes.filter(
    (a) => !sample?.results.some((r) => r.analyteId === a.id),
  );

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) resetForm();
        onOpenChange(o);
      }}
    >
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <StatusIcon className="size-4" />
            Muestra {sample?.code ?? ""}
            {sample && <Badge variant={st.variant}>{st.label}</Badge>}
          </DialogTitle>
          <DialogDescription>
            {sample
              ? `${sample.sampleTypeName} · recibida ${formatDateTime(sample.receivedAt)}`
              : "Cargando muestra…"}
          </DialogDescription>
        </DialogHeader>

        {isLoading && !sample && (
          <div className="space-y-3">
            <Skeleton className="h-20 w-full" />
            <Skeleton className="h-40 w-full" />
          </div>
        )}

        {!isLoading && !sample && (
          <p className="text-muted-foreground py-8 text-center text-sm">
            No se encontró la muestra.
          </p>
        )}

        {sample && (
          <div className="space-y-5">
            {/* Datos del paciente / muestra */}
            <div className="bg-muted/50 flex flex-wrap items-center gap-x-6 gap-y-2 rounded-lg px-3 py-2.5 text-sm">
              <div>
                <p className="text-muted-foreground text-[11px] font-medium tracking-wide uppercase">
                  Paciente
                </p>
                <p className="font-medium">{patient?.name ?? "…"}</p>
              </div>
              <div>
                <p className="text-muted-foreground text-[11px] font-medium tracking-wide uppercase">
                  Propietario
                </p>
                <p>{patient?.ownerName ?? "…"}</p>
              </div>
              {sample.collectedBy && (
                <div>
                  <p className="text-muted-foreground text-[11px] font-medium tracking-wide uppercase">
                    Recogida por
                  </p>
                  <p>{sample.collectedBy}</p>
                </div>
              )}
              <Button
                variant="ghost"
                size="sm"
                className="ml-auto"
                onClick={goToHistory}
              >
                <HeartPulse className="size-4" />
                Ver historial
              </Button>
            </div>

            {sample.notes && (
              <p className="text-muted-foreground text-sm">{sample.notes}</p>
            )}

            {/* Resultados */}
            <div className="overflow-hidden rounded-lg border">
              <div className="bg-muted/60 flex items-center justify-between border-b px-3 py-2">
                <p className="text-sm font-semibold">
                  Resultados ({sample.results.length})
                </p>
              </div>
              {sample.results.length === 0 ? (
                <p className="text-muted-foreground px-4 py-6 text-center text-sm">
                  Sin resultados cargados.
                </p>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow className="hover:bg-transparent">
                      <TableHead>Analito</TableHead>
                      <TableHead>Resultado</TableHead>
                      <TableHead>Rango de referencia</TableHead>
                      <TableHead className="text-right">Estado</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {sample.results.map((r) => {
                      const rs =
                        RESULT_STATUS[r.status] ?? RESULT_STATUS.SIN_RANGO;
                      const range = r.refMin != null && r.refMax != null;
                      return (
                        <TableRow
                          key={r.id}
                          className={cn(
                            r.status === "ALTO" && "bg-warning/10",
                            r.status === "BAJO" && "bg-destructive/10",
                          )}
                        >
                          <TableCell>
                            <span className="font-medium">{r.analyteName}</span>
                          </TableCell>
                          <TableCell>
                            <span
                              className={cn(
                                "font-mono font-semibold",
                                r.status === "ALTO" && "text-warning",
                                r.status === "BAJO" && "text-destructive",
                              )}
                            >
                              {r.value}
                            </span>
                            {r.unit && (
                              <span className="text-muted-foreground ml-1 text-xs">
                                {r.unit}
                              </span>
                            )}
                          </TableCell>
                          <TableCell className="text-muted-foreground font-mono text-xs">
                            {range ? `${r.refMin} – ${r.refMax}` : "—"}
                          </TableCell>
                          <TableCell className="text-right">
                            <Badge variant={rs.variant}>{rs.label}</Badge>
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              )}
            </div>

            {/* Carga de resultados */}
            {canAddResult && (
              <Form {...resultForm}>
                <form onSubmit={resultForm.handleSubmit(onSubmitResult)} className="space-y-3">
                  <div className="grid gap-3 sm:grid-cols-[1fr_120px_auto]">
                    <FormField
                      control={resultForm.control}
                      name="analyteId"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Analito</FormLabel>
                          <Select
                            value={field.value?.toString() ?? ""}
                            onValueChange={(v) => field.onChange(Number(v))}
                          >
                            <FormControl>
                              <SelectTrigger className="w-full">
                                <SelectValue
                                  placeholder={
                                    availableAnalytes.length === 0
                                      ? "Todos los analitos ya tienen valor (puedes actualizarlo)"
                                      : "Selecciona analito…"
                                  }
                                />
                              </SelectTrigger>
                            </FormControl>
                            <SelectContent>
                              {availableAnalytes.map((a) => (
                                <SelectItem key={a.id} value={a.id.toString()}>
                                  {a.name}
                                  {a.unit ? ` (${a.unit})` : ""}
                                </SelectItem>
                              ))}
                              {analyzed.length > 0 && (
                                <>
                                  <SelectItem value="__sep__" disabled>
                                    ── Actualizar ──
                                  </SelectItem>
                                  {analyzed.map((a) => (
                                    <SelectItem key={a.id} value={a.id.toString()}>
                                      {a.name}
                                      {a.unit ? ` (${a.unit})` : ""}
                                    </SelectItem>
                                  ))}
                                </>
                              )}
                            </SelectContent>
                          </Select>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={resultForm.control}
                      name="value"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Valor</FormLabel>
                          <FormControl>
                            <Input
                              type="number"
                              step="any"
                              inputMode="decimal"
                              placeholder="0.0"
                              className="font-mono"
                              {...field}
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <div className="flex items-end">
                      <Button
                        type="submit"
                        disabled={pending}
                      >
                        {registerResult.isPending ? (
                          <Loader2 className="animate-spin" />
                        ) : (
                          <Plus className="size-4" />
                        )}
                        Cargar
                      </Button>
                    </div>
                  </div>
                  <p className="text-muted-foreground text-xs">
                    El estado clínico (normal/alto/bajo) se calcula contra los
                    rangos de referencia de la especie, sexo y edad del paciente.
                    Al cargar resultados la muestra pasa a{" "}
                    <span className="font-medium">EN PROCESO</span>; al terminar,
                    finalízala para habilitar el informe PDF.
                  </p>
                </form>
              </Form>
            )}

            {sampleStatus === "FINALIZADA" && sample.results.length > 0 && (
              <div className="bg-success/10 text-success flex items-start gap-2 rounded-lg px-3 py-2.5 text-sm">
                <CheckCircle2 className="mt-0.5 size-4 shrink-0" />
                <span>
                  Muestra finalizada con {sample.results.length} resultado
                  {sample.results.length === 1 ? "" : "s"}. Ya puedes generar el
                  informe PDF.
                </span>
              </div>
            )}

            {/* Resultado IA */}
            {aiInterpretation && (
              <div className="bg-primary/5 border border-primary/20 rounded-lg p-4 text-sm space-y-2">
                <div className="flex items-center gap-2 font-semibold text-primary">
                  <Bot className="size-4" />
                  Interpretación IA (Llama 3)
                </div>
                <div className="prose prose-sm prose-neutral dark:prose-invert max-w-none">
                  <ReactMarkdown remarkPlugins={[remarkGfm]}>
                    {aiInterpretation}
                  </ReactMarkdown>
                </div>
              </div>
            )}
          </div>
        )}

        <DialogFooter className="sm:justify-between">
          <div className="flex flex-wrap gap-2">
            {canProcess && (
              <Button variant="outline" onClick={markEnProceso} disabled={pending}>
                <PlayCircle className="size-4" />
                Poner en proceso
              </Button>
            )}
            {canAnular && (
              <Button
                variant={confirmAnular ? "destructive" : "outline"}
                onClick={anular}
                disabled={pending}
              >
                <Ban className="size-4" />
                {confirmAnular ? "¿Confirmar anulación?" : "Anular"}
              </Button>
            )}
            {canFinalize && sample && (
              <Button
                variant="default"
                onClick={finalizar}
                disabled={pending}
              >
                <CheckCircle2 className="size-4" />
                Finalizar muestra
              </Button>
            )}
            {canReport && sample && sample.results.length > 0 && (
              <>
                <Button onClick={generatePdf} disabled={generate.isPending}>
                  {generate.isPending ? (
                    <Loader2 className="animate-spin" />
                  ) : (
                    <FileText className="size-4" />
                  )}
                  Generar PDF
                </Button>
                <Button
                  variant="outline"
                  className="gap-2 bg-green-50 text-green-700 hover:bg-green-100 hover:text-green-800 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/50 dark:hover:bg-green-900/40"
                  onClick={handleWhatsApp}
                >
                  <MessageCircle className="size-4" />
                  Enviar por WhatsApp
                </Button>
                {isVetOrAdmin && (
                  <Button
                    variant="outline"
                    className="gap-2 bg-purple-50 text-purple-700 hover:bg-purple-100 hover:text-purple-800 border-purple-200 dark:bg-purple-900/20 dark:text-purple-400 dark:border-purple-900/50 dark:hover:bg-purple-900/40"
                    onClick={handleInterpretAI}
                    disabled={interpreting}
                  >
                    {interpreting ? <Loader2 className="size-4 animate-spin" /> : <Bot className="size-4" />}
                    Interpretación IA
                  </Button>
                )}
              </>
            )}
          </div>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cerrar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
