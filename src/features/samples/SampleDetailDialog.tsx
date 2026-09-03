import { useEffect, useRef, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { openPath } from "@tauri-apps/plugin-opener";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { toast } from "sonner";
import {
  AlertTriangle,
  Ban,
  Bot,
  CheckCircle2,
  ExternalLink,
  FileText,
  FlaskConical,
  HeartPulse,
  ImagePlus,
  Loader2,
  MessageCircle,
  Paperclip,
  PlayCircle,
  Plus,
  Printer,
  Siren,
  Trash2,
  X,
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
  useAttachResultFile,
  useDeleteResultAttachment,
  useGenerateReport,
  useGenerateSampleLabels,
  usePanelAnalytes,
  usePanels,
  usePatient,
  useRegisterLabResult,
  useRegisterLabResults,
  useRejectSample,
  useReopenSample,
  useSample,
  useSetSampleQuality,
  useSetSampleStatus,
} from "@/hooks/use-queries";
import type { LabResult, ResultAttachment } from "@/bindings";
import {
  QUALITY_INDEX_LABEL,
  QUALITY_SEVERITY_LABEL,
  RESULT_STATUS,
  SAMPLE_STATUS,
} from "@/lib/status";
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
    .number({ error: "Ingresa un número válido" })
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
  const registerResults = useRegisterLabResults();
  const setStatus = useSetSampleStatus();
  const setQuality = useSetSampleQuality();
  const rejectSample = useRejectSample();
  const reopenSample = useReopenSample();
  const generate = useGenerateReport();
  const generateLabels = useGenerateSampleLabels();
  const attachFile = useAttachResultFile(sampleId);
  const removeAttachment = useDeleteResultAttachment(sampleId);
  const { data: panels = [] } = usePanels();
  const [panelId, setPanelId] = useState<number | null>(null);
  const { data: panelAnalytes = [] } = usePanelAnalytes(panelId);
  const [batchValues, setBatchValues] = useState<Record<number, string>>({});

  const setActivePatient = useUiStore((s) => s.setActivePatient);
  const navigate = useUiStore((s) => s.navigate);

  const { isVetOrAdmin } = usePermissions();
  const [confirmAnular, setConfirmAnular] = useState(false);
  const [showRejectInput, setShowRejectInput] = useState(false);
  const [rejectReason, setRejectReason] = useState("");
  const [criticalAlert, setCriticalAlert] = useState<LabResult[]>([]);
  const [aiInterpretation, setAiInterpretation] = useState<string | null>(null);
  const [interpreting, setInterpreting] = useState(false);
  const aiRef = useRef<HTMLDivElement | null>(null);
  const [previewAttachment, setPreviewAttachment] =
    useState<ResultAttachment | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);

  // Lleva el contenido del diálogo hasta el bloque de interpretación IA en
  // cuanto aparece, para que el usuario vea el resultado sin buscarlo.
  useEffect(() => {
    if (aiInterpretation) {
      aiRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }, [aiInterpretation]);

  const resultForm = useForm<z.input<typeof resultSchema>, unknown, z.output<typeof resultSchema>>({
    resolver: zodResolver(resultSchema),
    defaultValues: {
      analyteId: 0,
      value: 0,
    },
  });

  const resetForm = () => {
    resultForm.reset({ analyteId: 0, value: 0 });
    setConfirmAnular(false);
    setShowRejectInput(false);
    setRejectReason("");
    setCriticalAlert([]);
    setAiInterpretation(null);
    setPreviewAttachment(null);
    setConfirmDelete(false);
  };

  // Panel disponible para esta muestra: específico del tipo o genérico.
  const availablePanels = panels.filter(
    (p) => p.sampleTypeId == null || p.sampleTypeId === sample?.sampleTypeId,
  );
  useEffect(() => {
    if (open && availablePanels.length > 0 && panelId == null) {
      setPanelId(availablePanels[0].id);
    }
  }, [open, availablePanels, panelId]);

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
      if (result.isCritical) {
        setCriticalAlert([result]);
      }
    } catch (err) {
      toast.error("No se pudo registrar el resultado", {
        description: getErrorMessage(err),
      });
    }
  };

  /** Carga en lote los valores de la grilla del panel (no vacíos). */
  const submitBatch = async () => {
    if (!sample) return;
    const entries = Object.entries(batchValues)
      .filter(([, v]) => v.trim() !== "")
      .map(([analyteId, v]) => ({
        sampleId: sample.id,
        analyteId: Number(analyteId),
        value: Number(v.replace(",", ".")),
      }))
      .filter((r) => !Number.isNaN(r.value));
    if (entries.length === 0) {
      toast.error("Ingresa al menos un valor en la grilla");
      return;
    }
    try {
      const results = await registerResults.mutateAsync({
        sampleId: sample.id,
        results: entries,
      });
      toast.success(`${results.length} resultado${results.length === 1 ? "" : "s"} cargados`);
      setBatchValues({});
      const critical = results.filter((r) => r.isCritical);
      if (critical.length > 0) setCriticalAlert(critical);
    } catch (err) {
      toast.error("No se pudieron cargar los resultados", {
        description: getErrorMessage(err),
      });
    }
  };

  /** Rechaza la muestra pidiendo el motivo obligatorio. */
  const doReject = async () => {
    if (!sample) return;
    if (!showRejectInput) {
      setShowRejectInput(true);
      return;
    }
    if (!rejectReason.trim()) {
      toast.error("Indica el motivo del rechazo");
      return;
    }
    try {
      const updated = await rejectSample.mutateAsync({
        id: sample.id,
        reason: rejectReason.trim(),
      });
      toast.success(`Muestra ${updated.code} rechazada`);
      setShowRejectInput(false);
      setRejectReason("");
    } catch (err) {
      toast.error("No se pudo rechazar la muestra", {
        description: getErrorMessage(err),
      });
    }
  };

  const doReopen = async () => {
    if (!sample) return;
    try {
      const updated = await reopenSample.mutateAsync(sample.id);
      toast.success(`Muestra ${updated.code} reabierta (Recibida)`);
    } catch (err) {
      toast.error("No se pudo reabrir la muestra", {
        description: getErrorMessage(err),
      });
    }
  };

  /** Guarda la calidad preanalítica editada en la ficha. */
  const [qualityDraft, setQualityDraft] = useState<{
    index: string | null;
    severity: string | null;
    note: string;
  }>({ index: null, severity: null, note: "" });
  const [qualityDirty, setQualityDirty] = useState(false);
  const openQualityEditor = () => {
    setQualityDraft({
      index: sample?.qualityIndex ?? null,
      severity: sample?.qualitySeverity ?? null,
      note: sample?.qualityNote ?? "",
    });
    setQualityDirty(false);
  };
  const saveQuality = async () => {
    if (!sample) return;
    try {
      await setQuality.mutateAsync({
        id: sample.id,
        qualityIndex: qualityDraft.index,
        qualitySeverity: qualityDraft.severity,
        qualityNote: qualityDraft.note.trim() || null,
      });
      toast.success("Calidad de la muestra actualizada");
      setQualityDirty(false);
    } catch (err) {
      toast.error("No se pudo guardar la calidad", {
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

  const printLabel = async () => {
    if (!sample) return;
    try {
      const report = await generateLabels.mutateAsync([sample.id]);
      toast.success("Etiqueta de muestra generada", {
        description: "Ábrela con el visor de PDF para imprimirla y pegarla al tubo.",
      });
      try {
        await api.openReportFile(report.path);
      } catch {
        await openPath(report.path);
      }
    } catch (err) {
      toast.error("No se pudo generar la etiqueta", {
        description: getErrorMessage(err),
      });
    }
  };

  const pickAndAttach = async (result: LabResult) => {
    let selected: string | string[] | null;
    try {
      selected = await openDialog({
        title: `Adjuntar imagen a ${result.analyteName}`,
        multiple: true,
        filters: [
          {
            name: "Imágenes (placas, frotis, electroforesis)",
            extensions: ["png", "jpg", "jpeg", "webp", "gif"],
          },
        ],
      });
    } catch (err) {
      toast.error("No se pudo abrir el selector de archivos", {
        description: getErrorMessage(err),
      });
      return;
    }
    if (!selected) return;

    // El lote continúa aunque algún archivo falle (formato/tamaño inválido).
    const paths = Array.isArray(selected) ? selected : [selected];
    let ok = 0;
    let failed = 0;
    for (const p of paths) {
      try {
        await attachFile.mutateAsync({ resultId: result.id, sourcePath: p });
        ok += 1;
      } catch {
        failed += 1;
      }
    }
    if (ok > 0) {
      toast.success(
        ok === 1 ? "Adjunto cargado" : `${ok} adjuntos cargados`,
        {
          description:
            "La imagen quedó asociada al resultado como evidencia del diagnóstico.",
        },
      );
    }
    if (failed > 0) {
      toast.error(
        failed === 1 ? "1 archivo no se pudo adjuntar" : `${failed} archivos no se pudieron adjuntar`,
        {
          description:
            "Revisa el formato (PNG, JPG, WebP o GIF) y el tamaño (máx. 20 MB).",
        },
      );
    }
  };

  const handleDeleteAttachment = async () => {
    if (!previewAttachment) return;
    if (!confirmDelete) {
      setConfirmDelete(true);
      return;
    }
    try {
      await removeAttachment.mutateAsync(previewAttachment.id);
      toast.success("Adjunto eliminado", {
        description: "El archivo se borró de la carpeta de datos.",
      });
      setPreviewAttachment(null);
      setConfirmDelete(false);
    } catch (err) {
      toast.error("No se pudo eliminar el adjunto", {
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

  const criticalResults = sample?.results.filter((r) => r.isCritical) ?? [];

  const handleWhatsAppCritical = (results: LabResult[]) => {
    if (!patient?.ownerPhone || !patient?.ownerName || !patient?.name) {
      toast.error("Falta información", {
        description: "El paciente no tiene un número de teléfono del propietario registrado.",
      });
      return;
    }
    const lines = results
      .map(
        (r) =>
          `- *${r.analyteName}*: ${r.value} ${r.unit ?? ""} (${RESULT_STATUS[r.status]?.label ?? r.status})`,
      )
      .join("\n");
    const message = `*⚠ ALERTA: VALOR CRÍTICO DE LABORATORIO*\n\nHola ${patient.ownerName}, le informamos que el resultado de laboratorio de *${patient.name}* presenta un valor crítico que requiere atención inmediata:\n\n${lines}\n\nPor favor, contacte a su veterinario lo antes posible.\n\nISALAB`;
    sendWhatsAppMessage(patient.ownerPhone, message);
    setCriticalAlert([]);
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
  const canReject = sampleStatus === "RECIBIDA" || sampleStatus === "EN_PROCESO";
  const canAddResult = sampleStatus === "RECIBIDA" || sampleStatus === "EN_PROCESO";
  const isRejected = sampleStatus === "RECHAZADA";
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

  const canDeleteAttachments =
    isVetOrAdmin && sampleStatus !== "ANULADA";

  return (
    <>
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) resetForm();
        onOpenChange(o);
      }}
    >
      <DialogContent className="flex max-h-[90vh] flex-col gap-4 sm:max-w-2xl">
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

        {/* El contenido crece con los resultados y la interpretación IA; el
            scroll queda aquí para que el footer con las acciones nunca se
            pierda fuera de pantalla. */}
        <div className="min-h-0 flex-1 space-y-5 overflow-y-auto pr-1">
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
              {sample.analyzerName && (
                <div>
                  <p className="text-muted-foreground text-[11px] font-medium tracking-wide uppercase">
                    Equipo analizador
                  </p>
                  <p>{sample.analyzerName}</p>
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

            {/* Calidad preanalítica (HIL) */}
            {!qualityDirty && (sample.qualityIndex || isRejected) && (
              <div
                className={cn(
                  "flex flex-wrap items-center gap-2 rounded-lg border px-3 py-2 text-sm",
                  isRejected
                    ? "bg-destructive/10 border-destructive/30"
                    : "bg-warning/10 border-warning/30",
                )}
              >
                <AlertTriangle className="size-4 shrink-0 text-warning" />
                <span className="font-medium">
                  {isRejected
                    ? "Muestra rechazada"
                    : QUALITY_INDEX_LABEL[sample.qualityIndex ?? ""] ?? sample.qualityIndex}
                </span>
                {sample.qualitySeverity && !isRejected && (
                  <Badge variant="outline">
                    {QUALITY_SEVERITY_LABEL[sample.qualitySeverity] ?? sample.qualitySeverity}
                  </Badge>
                )}
                {sample.qualityNote && (
                  <span className="text-muted-foreground text-xs">{sample.qualityNote}</span>
                )}
                {isRejected && sample.rejectionReason && (
                  <span className="text-muted-foreground text-xs">
                    Motivo: {sample.rejectionReason}
                    {sample.rejectedBy ? ` · ${sample.rejectedBy}` : ""}
                  </span>
                )}
                {!isRejected && canAddResult && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="ml-auto h-6 px-2 text-xs"
                    onClick={openQualityEditor}
                  >
                    Editar calidad
                  </Button>
                )}
              </div>
            )}

            {/* Editor de calidad preanalítica */}
            {qualityDirty && (
              <div className="space-y-3 rounded-lg border px-3 py-3">
                <div className="grid gap-3 sm:grid-cols-2">
                  <div>
                    <FormLabel>Interferencia</FormLabel>
                    <Select
                      value={qualityDraft.index ?? ""}
                      onValueChange={(v) => {
                        setQualityDraft((d) => ({ ...d, index: v === "" ? null : v }));
                        setQualityDirty(true);
                      }}
                    >
                      <SelectTrigger className="mt-1">
                        <SelectValue placeholder="Sin interferencia" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="">Sin interferencia</SelectItem>
                        {Object.entries(QUALITY_INDEX_LABEL)
                          .filter(([k]) => k !== "NORMAL")
                          .map(([k, v]) => (
                            <SelectItem key={k} value={k}>
                              {v}
                            </SelectItem>
                          ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <FormLabel>Severidad</FormLabel>
                    <Select
                      value={qualityDraft.severity ?? ""}
                      onValueChange={(v) => {
                        setQualityDraft((d) => ({ ...d, severity: v === "" ? null : v }));
                        setQualityDirty(true);
                      }}
                      disabled={!qualityDraft.index}
                    >
                      <SelectTrigger className="mt-1">
                        <SelectValue placeholder="Selecciona…" />
                      </SelectTrigger>
                      <SelectContent>
                        {Object.entries(QUALITY_SEVERITY_LABEL).map(([k, v]) => (
                          <SelectItem key={k} value={k}>
                            {v}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                </div>
                <Input
                  placeholder="Nota sobre la calidad (opcional)…"
                  value={qualityDraft.note}
                  onChange={(e) => {
                    setQualityDraft((d) => ({ ...d, note: e.target.value }));
                    setQualityDirty(true);
                  }}
                />
                <div className="flex justify-end gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setQualityDirty(false)}
                  >
                    Cancelar
                  </Button>
                  <Button size="sm" onClick={saveQuality} disabled={setQuality.isPending}>
                    {setQuality.isPending ? <Loader2 className="size-4 animate-spin" /> : null}
                    Guardar calidad
                  </Button>
                </div>
              </div>
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
                      <TableHead className="text-right">Adjuntos</TableHead>
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
                                r.isCritical && "text-destructive animate-pulse",
                              )}
                            >
                              {r.value}
                            </span>
                            {r.unit && (
                              <span className="text-muted-foreground ml-1 text-xs">
                                {r.unit}
                              </span>
                            )}
                            {r.deltaVariation != null && (
                              <span
                                title="Variación vs. resultado previo (delta check)"
                                className={cn(
                                  "ml-1 text-[11px] font-medium",
                                  Math.abs(r.deltaVariation) >= 50
                                    ? "text-destructive"
                                    : "text-muted-foreground",
                                )}
                              >
                                {r.deltaVariation >= 0 ? "▲" : "▼"}{" "}
                                {Math.abs(r.deltaVariation).toFixed(1)}%
                              </span>
                            )}
                          </TableCell>
                          <TableCell className="text-muted-foreground font-mono text-xs">
                            {range ? `${r.refMin} – ${r.refMax}` : "—"}
                          </TableCell>
                          <TableCell className="text-right">
                            <Badge variant={rs.variant}>{rs.label}</Badge>
                          </TableCell>
                          <TableCell className="text-right">
                            {r.attachments.length > 0 && (
                              <div className="mb-1 flex flex-wrap justify-end gap-1">
                                {r.attachments.map((att) => (
                                  <button
                                    key={att.id}
                                    type="button"
                                    onClick={() => {
                                      setConfirmDelete(false);
                                      setPreviewAttachment(att);
                                    }}
                                    title={`${att.fileName} · abrir vista previa`}
                                    className="group relative overflow-hidden rounded-md border shadow-sm"
                                  >
                                    <img
                                      src={convertFileSrc(att.filePath)}
                                      alt={att.fileName}
                                      className="size-9 object-cover transition-transform group-hover:scale-110"
                                    />
                                  </button>
                                ))}
                              </div>
                            )}
                            {canAddResult && isVetOrAdmin ? (
                              <Button
                                variant="ghost"
                                size="sm"
                                className="text-muted-foreground h-6 gap-1 px-1.5 text-xs hover:text-foreground"
                                onClick={() => pickAndAttach(r)}
                                disabled={attachFile.isPending}
                                title="Adjuntar foto de placa, frotis o electroforesis"
                              >
                                {attachFile.isPending ? (
                                  <Loader2 className="size-3.5 animate-spin" />
                                ) : (
                                  <ImagePlus className="size-3.5" />
                                )}
                                Adjuntar
                              </Button>
                            ) : (
                              r.attachments.length === 0 && (
                                <span className="text-muted-foreground/40 text-xs">
                                  —
                                </span>
                              )
                            )}
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              )}
            </div>

            {/* Carga rápida por panel (grilla) */}
            {canAddResult && availablePanels.length > 0 && (
              <div className="rounded-lg border">
                <div className="bg-muted/60 flex flex-wrap items-center gap-2 border-b px-3 py-2">
                  <p className="text-sm font-semibold">Carga rápida por panel</p>
                  <Select
                    value={panelId?.toString() ?? ""}
                    onValueChange={(v) => setPanelId(Number(v))}
                  >
                    <SelectTrigger className="h-7 w-56 text-xs">
                      <SelectValue placeholder="Selecciona panel…" />
                    </SelectTrigger>
                    <SelectContent>
                      {availablePanels.map((p) => (
                        <SelectItem key={p.id} value={p.id.toString()}>
                          {p.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                {panelAnalytes.length > 0 ? (
                  <div className="grid gap-2 p-3 sm:grid-cols-2">
                    {panelAnalytes.map((pa) => {
                      const existing = sample.results.find((r) => r.analyteId === pa.analyteId);
                      return (
                        <label key={pa.analyteId} className="flex items-center gap-2 text-sm">
                          <span className="min-w-0 flex-1 truncate">
                            {pa.analyteName}
                            {pa.unit ? (
                              <span className="text-muted-foreground text-xs"> ({pa.unit})</span>
                            ) : null}
                          </span>
                          <Input
                            type="number"
                            step="any"
                            inputMode="decimal"
                            placeholder={
                              existing != null ? `Actual: ${existing.value}` : "—"
                            }
                            className="h-8 w-28 font-mono text-xs"
                            value={batchValues[pa.analyteId] ?? ""}
                            onChange={(e) =>
                              setBatchValues((prev) => ({
                                ...prev,
                                [pa.analyteId]: e.target.value,
                              }))
                            }
                          />
                        </label>
                      );
                    })}
                  </div>
                ) : (
                  <p className="text-muted-foreground px-4 py-3 text-sm">
                    El panel no tiene analitos configurados.
                  </p>
                )}
                <div className="flex justify-end border-t px-3 py-2">
                  <Button
                    size="sm"
                    onClick={submitBatch}
                    disabled={registerResults.isPending}
                    className="gap-1.5"
                  >
                    {registerResults.isPending ? (
                      <Loader2 className="size-4 animate-spin" />
                    ) : (
                      <PlayCircle className="size-4" />
                    )}
                    Cargar valores del panel
                  </Button>
                </div>
              </div>
            )}

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
                              value={(field.value as number | undefined) ?? ""}
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
              <div
                ref={aiRef}
                className="bg-primary/5 border border-primary/20 rounded-lg p-4 text-sm space-y-2"
              >
                <div className="flex items-center gap-2 font-semibold text-primary">
                  <Bot className="size-4" />
                  Interpretación IA (Llama 3)
                  <Button
                    variant="ghost"
                    size="sm"
                    className="ml-auto h-6 gap-1 px-2 text-xs text-muted-foreground hover:text-foreground"
                    onClick={() => setAiInterpretation(null)}
                  >
                    <X className="size-3.5" />
                    Ocultar
                  </Button>
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
        </div>

        <DialogFooter className="shrink-0 border-t pt-3 sm:justify-between">
          <div className="flex flex-wrap gap-2">
            {sample && sampleStatus !== "ANULADA" && isVetOrAdmin && (
              <Button
                variant="outline"
                onClick={printLabel}
                disabled={generateLabels.isPending}
              >
                {generateLabels.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <Printer className="size-4" />
                )}
                Etiqueta
              </Button>
            )}
            {canProcess && (
              <Button variant="outline" onClick={markEnProceso} disabled={pending}>
                <PlayCircle className="size-4" />
                Poner en proceso
              </Button>
            )}
            {canReject && (
              <div className="flex items-center gap-2">
                {showRejectInput ? (
                  <>
                    <Input
                      autoFocus
                      value={rejectReason}
                      onChange={(e) => setRejectReason(e.target.value)}
                      onKeyDown={(e) => e.key === "Enter" && doReject()}
                      placeholder="Motivo del rechazo (obligatorio)…"
                      className="h-9 w-52 text-xs"
                    />
                    <Button
                      variant="destructive"
                      onClick={doReject}
                      disabled={rejectSample.isPending}
                      className="h-9"
                    >
                      {rejectSample.isPending ? <Loader2 className="size-4 animate-spin" /> : null}
                      Confirmar
                    </Button>
                  </>
                ) : (
                  <Button
                    variant="outline"
                    className="text-destructive hover:text-destructive"
                    onClick={doReject}
                  >
                    <Ban className="size-4" />
                    Rechazar
                  </Button>
                )}
              </div>
            )}
            {isRejected && (
              <Button
                variant="outline"
                onClick={doReopen}
                disabled={reopenSample.isPending}
              >
                {reopenSample.isPending ? <Loader2 className="size-4 animate-spin" /> : null}
                Reabrir (Recibida)
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
                {criticalResults.length > 0 && (
                  <Button
                    variant="destructive"
                    className="gap-2 animate-pulse"
                    onClick={() => setCriticalAlert(criticalResults)}
                  >
                    <Siren className="size-4" />
                    Notificar valor crítico
                  </Button>
                )}
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

    {/* Vista previa de adjunto (foto de placa, frotis o electroforesis) */}
    <Dialog
      open={previewAttachment != null}
      onOpenChange={(o) => {
        if (!o) {
          setPreviewAttachment(null);
          setConfirmDelete(false);
        }
      }}
    >
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 truncate">
            <Paperclip className="size-4 shrink-0" />
            <span className="truncate">{previewAttachment?.fileName}</span>
          </DialogTitle>
          <DialogDescription>
            {previewAttachment &&
              `Adjunto del resultado · cargado ${formatDateTime(previewAttachment.createdAt)}`}
          </DialogDescription>
        </DialogHeader>
        <div className="bg-muted/40 flex min-h-48 items-center justify-center rounded-lg p-2">
          {previewAttachment && (
            <img
              src={convertFileSrc(previewAttachment.filePath)}
              alt={previewAttachment.fileName}
              className="max-h-[55vh] w-auto rounded-md object-contain"
            />
          )}
        </div>
        <DialogFooter className="sm:justify-between">
          <div className="flex flex-wrap gap-2">
            {previewAttachment && (
              <Button
                variant="outline"
                onClick={() => openPath(previewAttachment.filePath)}
                className="gap-1.5"
              >
                <ExternalLink className="size-4" />
                Abrir original
              </Button>
            )}
            {canDeleteAttachments && previewAttachment && (
              <Button
                variant={confirmDelete ? "destructive" : "outline"}
                onClick={handleDeleteAttachment}
                disabled={removeAttachment.isPending}
                className={
                  confirmDelete ? "" : "gap-1.5 text-destructive hover:text-destructive"
                }
              >
                {removeAttachment.isPending ? (
                  <Loader2 className="size-4 animate-spin" />
                ) : (
                  <Trash2 className="size-4" />
                )}
                {confirmDelete ? "¿Confirmar eliminación?" : "Eliminar"}
              </Button>
            )}
          </div>
          <Button
            variant="ghost"
            onClick={() => {
              setPreviewAttachment(null);
              setConfirmDelete(false);
            }}
          >
            Cerrar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    {/* Alerta de valor crítico: requiere confirmación del analista y ofrece
        notificación prioritaria por WhatsApp. */}
    <Dialog open={criticalAlert.length > 0} onOpenChange={(o) => !o && setCriticalAlert([])}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-destructive">
            <Siren className="size-5 animate-pulse" />
            Valor(es) crítico(s) registrado(s)
          </DialogTitle>
          <DialogDescription>
            Se registró un resultado fuera del umbral crítico. Confirma que lo
            revisaste y, si corresponde, notifica al propietario de inmediato.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-2 rounded-lg border border-destructive/30 bg-destructive/5 p-3">
          {criticalAlert.map((r) => (
            <div key={r.id} className="flex items-center justify-between text-sm">
              <span className="font-medium">{r.analyteName}</span>
              <span className="font-mono font-semibold text-destructive">
                {r.value} {r.unit ?? ""}
                <span className="ml-2 font-normal">
                  {RESULT_STATUS[r.status]?.label ?? r.status}
                </span>
              </span>
            </div>
          ))}
        </div>
        <DialogFooter className="sm:justify-between">
          <Button variant="ghost" onClick={() => setCriticalAlert([])}>
            Entendido
          </Button>
          <Button variant="destructive" onClick={() => handleWhatsAppCritical(criticalAlert)}>
            <MessageCircle className="size-4" />
            Notificar por WhatsApp
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    </>
  );
}
