import { useEffect, useMemo, useRef, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { toast } from "sonner";
import {
  AlertTriangle,
  Ban,
  Bot,
  CheckCircle2,
  ClipboardList,
  ExternalLink,
  FileText,
  FlaskConical,
  HeartPulse,
  History,
  ImagePlus,
  Loader2,
  Mail,
  MessageCircle,
  Paperclip,
  PlayCircle,
  Plus,
  Printer,
  Save,
  Siren,
  Trash2,
  X,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
  useDeleteLabResult,
  useDeleteResultAttachment,
  useOrderForSample,
  useGenerateReport,
  useGenerateSampleLabels,
  usePanelAnalytes,
  usePanels,
  usePatient,
  useReferenceRanges,
  useRegisterLabResults,
  useAcknowledgeCritical,
  useRejectSample,
  useReopenSample,
  useSample,
  useSampleEvents,
  useSampleNotifications,
  useSendCriticalEmail,
  useSetSampleQuality,
  useSetSampleStatus,
} from "@/hooks/use-queries";
import type {
  LabResult,
  NotificationLogEntry,
  ResultAttachment,
  SampleEvent,
} from "@/bindings";
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

  const registerResults = useRegisterLabResults();
  const setStatus = useSetSampleStatus();
  const setQuality = useSetSampleQuality();
  const rejectSample = useRejectSample();
  const reopenSample = useReopenSample();
  const acknowledgeCritical = useAcknowledgeCritical();
  const sendCriticalEmail = useSendCriticalEmail();
  const generate = useGenerateReport();
  const generateLabels = useGenerateSampleLabels();
  const attachFile = useAttachResultFile(sampleId);
  const removeAttachment = useDeleteResultAttachment(sampleId);
  const deleteResult = useDeleteLabResult();
  const { data: referenceRanges = [] } = useReferenceRanges(sample?.analyzerId ?? 1);
  const { data: panels = [] } = usePanels();
  const [panelId, setPanelId] = useState<number | null>(null);
  const { data: panelAnalytes = [] } = usePanelAnalytes(panelId);
  const [batchValues, setBatchValues] = useState<Record<number, string>>({});
  const [extraAnalyteIds, setExtraAnalyteIds] = useState<number[]>([]);

  const setActivePatient = useUiStore((s) => s.setActivePatient);
  const navigate = useUiStore((s) => s.navigate);
  const requestEntity = useUiStore((s) => s.requestEntity);

  // Orden de laboratorio de la que proviene la muestra (null si es espontánea).
  const { data: originOrder } = useOrderForSample(open ? sampleId : null);

  const { isVetOrAdmin } = usePermissions();
  const [confirmAnular, setConfirmAnular] = useState(false);
  const [showRejectInput, setShowRejectInput] = useState(false);
  const [rejectReason, setRejectReason] = useState("");
  const [showEvents, setShowEvents] = useState(false);
  // El historial se consulta solo cuando el usuario abre la vista.
  const { data: events = [], isLoading: loadingEvents } = useSampleEvents(
    showEvents ? sampleId : null,
  );
  const [showNotifications, setShowNotifications] = useState(false);
  const { data: notifications = [], isLoading: loadingNotifications } =
    useSampleNotifications(showNotifications ? sampleId : null);
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

  const resetForm = () => {
    setConfirmAnular(false);
    setShowRejectInput(false);
    setRejectReason("");
    setShowEvents(false);
    setShowNotifications(false);
    setCriticalAlert([]);
    setAiInterpretation(null);
    setPreviewAttachment(null);
    setConfirmDelete(false);
    setExtraAnalyteIds([]);
  };

  // Inicializar valores de la grilla con los resultados existentes al abrir la muestra
  useEffect(() => {
    if (open && sample) {
      const initial: Record<number, string> = {};
      for (const r of sample.results) {
        initial[r.analyteId] = r.value != null ? r.value.toString() : "";
      }
      setBatchValues(initial);
      setExtraAnalyteIds([]);
    }
  }, [open, sample?.id]);

  // Paneles disponibles para esta muestra: específicos del tipo o genéricos
  const availablePanels = useMemo(() => {
    return panels.filter(
      (p) => p.sampleTypeId == null || p.sampleTypeId === sample?.sampleTypeId,
    );
  }, [panels, sample?.sampleTypeId]);

  // Selección automática del panel prioritario para la muestra
  useEffect(() => {
    if (open && availablePanels.length > 0) {
      const specific = availablePanels.find((p) => p.sampleTypeId === sample?.sampleTypeId);
      const target = specific ?? availablePanels[0];
      if (panelId == null || !availablePanels.some((p) => p.id === panelId)) {
        setPanelId(target.id);
      }
    }
  }, [open, availablePanels, sample?.sampleTypeId, panelId]);

  // Lista combinada de analitos a mostrar en la tabla interactiva
  const displayedAnalytes = useMemo(() => {
    const list: { id: number; name: string; unit: string | null }[] = [];
    const seen = new Set<number>();

    // 1. Analitos del panel seleccionado
    for (const pa of panelAnalytes) {
      if (!seen.has(pa.analyteId)) {
        seen.add(pa.analyteId);
        list.push({ id: pa.analyteId, name: pa.analyteName, unit: pa.unit });
      }
    }

    // 2. Analitos con resultado ya registrado en la muestra
    if (sample?.results) {
      for (const r of sample.results) {
        if (!seen.has(r.analyteId)) {
          seen.add(r.analyteId);
          list.push({ id: r.analyteId, name: r.analyteName, unit: r.unit });
        }
      }
    }

    // 3. Analitos adicionales agregados manualmente
    for (const id of extraAnalyteIds) {
      if (!seen.has(id)) {
        const found = analytes.find((a) => a.id === id);
        if (found) {
          seen.add(id);
          list.push({ id: found.id, name: found.name, unit: found.unit });
        }
      }
    }

    return list;
  }, [panelAnalytes, sample?.results, extraAnalyteIds, analytes]);

  // Analitos del catálogo no incluidos actualmente en la tabla
  const unusedAnalytes = useMemo(() => {
    const displayedIds = new Set(displayedAnalytes.map((a) => a.id));
    return analytes.filter((a) => !displayedIds.has(a.id));
  }, [analytes, displayedAnalytes]);

  // Rango de referencia específico para el analito y la especie del paciente
  const getAnalyteRefRange = (analyteId: number) => {
    const existing = sample?.results.find((r) => r.analyteId === analyteId);
    if (existing?.refMin != null && existing?.refMax != null) {
      return {
        min: existing.refMin,
        max: existing.refMax,
        criticalMin: null,
        criticalMax: null,
      };
    }

    const spId = patient?.speciesId;
    const sex = patient?.sex;
    const match =
      referenceRanges.find(
        (rr) =>
          rr.analyteId === analyteId &&
          (spId == null || rr.speciesId === spId) &&
          (rr.sex == null || sex == null || rr.sex === sex),
      ) ??
      referenceRanges.find(
        (rr) =>
          rr.analyteId === analyteId &&
          (spId == null || rr.speciesId === spId),
      ) ??
      referenceRanges.find((rr) => rr.analyteId === analyteId);

    if (match && match.minValue != null && match.maxValue != null) {
      return {
        min: match.minValue,
        max: match.maxValue,
        criticalMin: match.criticalMin,
        criticalMax: match.criticalMax,
      };
    }

    return null;
  };

  // Evaluación en tiempo real del valor ingresado contra el rango de la especie
  const evaluateLiveStatus = (
    valStr: string | undefined,
    refRange: ReturnType<typeof getAnalyteRefRange>,
  ) => {
    if (!valStr || valStr.trim() === "") return null;
    const val = Number(valStr.replace(",", "."));
    if (Number.isNaN(val)) return null;

    if (!refRange) {
      return { label: "Cargado", variant: "secondary" as const, isCritical: false };
    }

    if (refRange.criticalMin != null && val <= refRange.criticalMin) {
      return { label: "Crítico Bajo", variant: "destructive" as const, isCritical: true };
    }
    if (refRange.criticalMax != null && val >= refRange.criticalMax) {
      return { label: "Crítico Alto", variant: "destructive" as const, isCritical: true };
    }
    if (val < refRange.min) {
      return { label: "Bajo", variant: "destructive" as const, isCritical: false };
    }
    if (val > refRange.max) {
      return { label: "Alto", variant: "warning" as const, isCritical: false };
    }
    return { label: "Normal", variant: "success" as const, isCritical: false };
  };

  const pending = registerResults.isPending || deleteResult.isPending || setStatus.isPending;

  /** Guarda todos los resultados ingresados en la grilla y elimina los vaciados. */
  const handleSaveAllResults = async () => {
    if (!sample) return;

    // 1. Analitos con valor ingresado
    const toSaveEntries = Object.entries(batchValues)
      .filter(([, v]) => v != null && v.trim() !== "")
      .map(([analyteId, v]) => ({
        sampleId: sample.id,
        analyteId: Number(analyteId),
        value: Number(v.replace(",", ".")),
      }))
      .filter((r) => !Number.isNaN(r.value));

    // 2. Analitos previamente guardados que ahora están en blanco (se eliminan)
    const toDeleteAnalyteIds = sample.results
      .filter((r) => {
        const currentVal = batchValues[r.analyteId];
        return currentVal == null || currentVal.trim() === "";
      })
      .map((r) => r.analyteId);

    if (toSaveEntries.length === 0 && toDeleteAnalyteIds.length === 0) {
      toast.error("Ingresa al menos un valor en la grilla para guardar");
      return;
    }

    try {
      for (const analyteId of toDeleteAnalyteIds) {
        await deleteResult.mutateAsync({ sampleId: sample.id, analyteId });
      }

      let savedResults: LabResult[] = [];
      if (toSaveEntries.length > 0) {
        savedResults = await registerResults.mutateAsync({
          sampleId: sample.id,
          results: toSaveEntries,
        });
      }

      const totalActive = toSaveEntries.length;
      toast.success(
        `${totalActive} resultado${totalActive === 1 ? "" : "s"} guardado${totalActive === 1 ? "" : "s"} correctamente`,
        {
          description: "Solo los analitos con valor ingresado aparecerán en el informe PDF.",
        },
      );

      const critical = savedResults.filter((r) => r.isCritical);
      if (critical.length > 0) setCriticalAlert(critical);
    } catch (err) {
      toast.error("No se pudieron guardar los resultados", {
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

  /** Confirma la revisión de los valores críticos (quedan auditados). */
  const doAcknowledgeCritical = async () => {
    if (!sample) return;
    try {
      await acknowledgeCritical.mutateAsync({
        sampleId: sample.id,
        resultIds: criticalAlert.map((r) => r.id),
      });
      toast.success("Confirmación registrada", {
        description: "La revisión de los valores críticos quedó auditada.",
      });
      setCriticalAlert([]);
    } catch (err) {
      toast.error("No se pudo registrar la confirmación", {
        description: getErrorMessage(err),
      });
    }
  };

  /** Envía el aviso de valor crítico por email al propietario. */
  const doSendCriticalEmail = async () => {
    if (!sample) return;
    try {
      const entries = await sendCriticalEmail.mutateAsync({
        sampleId: sample.id,
        resultIds: criticalAlert.map((r) => r.id),
      });
      const sent = entries.filter((e) => e.status === "SENT").length;
      toast.success("Aviso enviado por email", {
        description:
          sent > 0
            ? `${sent} notificación(es) registrada(s). El envío quedó auditado.`
            : "El envío se registró en el historial.",
      });
      setCriticalAlert([]);
    } catch (err) {
      toast.error("No se pudo enviar el correo", {
        description: getErrorMessage(err),
      });
    }
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
      <DialogContent className="flex max-h-[92vh] flex-col gap-4 sm:max-w-4xl">
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
            {originOrder && (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onOpenChange(false);
                  navigate("lab-orders");
                  requestEntity("lab-order", originOrder.id);
                }}
                className="ml-2 inline-flex cursor-pointer items-center gap-1 rounded-md bg-primary/10 px-1.5 py-0.5 text-xs font-medium text-primary transition-colors hover:bg-primary/20"
                title="Ver la orden de laboratorio de origen"
              >
                <ClipboardList className="size-3" />
                Orden {originOrder.code}
              </button>
            )}
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
                    <Label>Interferencia</Label>
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
                    <Label>Severidad</Label>
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

            {/* Tabla de Resultados Analíticos */}
            {canAddResult ? (
              <div className="overflow-hidden rounded-lg border shadow-xs">
                {/* Cabecera de la grilla */}
                <div className="bg-muted/60 flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3">
                  <div>
                    <div className="flex items-center gap-2">
                      <p className="text-sm font-semibold">
                        Resultados Analíticos
                      </p>
                      <Badge variant="outline" className="text-xs font-normal">
                        {displayedAnalytes.filter((a) => batchValues[a.id]?.trim()).length} con valor / {displayedAnalytes.length} analitos
                      </Badge>
                    </div>
                    <p className="text-muted-foreground text-xs mt-0.5">
                      Especie: <span className="font-medium text-foreground">{patient?.speciesName ?? "—"}</span> · Tipo de muestra: <span className="font-medium text-foreground">{sample.sampleTypeName}</span>
                    </p>
                  </div>

                  <div className="flex flex-wrap items-center gap-2">
                    {/* Selector de panel para esta muestra */}
                    {availablePanels.length > 0 && (
                      <div className="flex items-center gap-1.5 text-xs">
                        <span className="text-muted-foreground font-medium">Panel:</span>
                        <Select
                          value={panelId?.toString() ?? ""}
                          onValueChange={(v) => setPanelId(Number(v))}
                        >
                          <SelectTrigger className="h-8 w-52 text-xs">
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
                    )}

                    {/* Selector para añadir un analito adicional a la tabla */}
                    {unusedAnalytes.length > 0 && (
                      <Select
                        value=""
                        onValueChange={(v) => {
                          if (v) {
                            const id = Number(v);
                            setExtraAnalyteIds((prev) => (prev.includes(id) ? prev : [...prev, id]));
                          }
                        }}
                      >
                        <SelectTrigger className="h-8 text-xs gap-1 w-44">
                          <Plus className="size-3.5" />
                          <span>+ Agregar analito…</span>
                        </SelectTrigger>
                        <SelectContent>
                          {unusedAnalytes.map((a) => (
                            <SelectItem key={a.id} value={a.id.toString()}>
                              {a.name} {a.unit ? `(${a.unit})` : ""}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    )}
                  </div>
                </div>

                {/* Tabla de analitos */}
                <Table>
                  <TableHeader>
                    <TableRow className="hover:bg-transparent text-xs">
                      <TableHead className="w-[30%]">Analito</TableHead>
                      <TableHead className="w-[25%]">
                        Rango de Referencia ({patient?.speciesName ?? "Especie"})
                      </TableHead>
                      <TableHead className="w-[22%]">Resultado (Valor)</TableHead>
                      <TableHead className="w-[13%] text-center">Estado</TableHead>
                      <TableHead className="w-[10%] text-right">Acciones</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {displayedAnalytes.length === 0 ? (
                      <TableRow>
                        <TableCell colSpan={5} className="text-center text-sm text-muted-foreground py-6">
                          No hay analitos configurados para este tipo de muestra. Usa "+ Agregar analito…" para comenzar.
                        </TableCell>
                      </TableRow>
                    ) : (
                      displayedAnalytes.map((a) => {
                        const existing = sample.results.find((r) => r.analyteId === a.id);
                        const refRange = getAnalyteRefRange(a.id);
                        const currentVal = batchValues[a.id] ?? "";
                        const status = evaluateLiveStatus(currentVal, refRange);
                        const isFilled = currentVal.trim() !== "";

                        return (
                          <TableRow
                            key={a.id}
                            className={cn(
                              status?.label === "Alto" && "bg-amber-500/5 dark:bg-amber-500/10",
                              status?.label === "Bajo" && "bg-destructive/5 dark:bg-destructive/10",
                              status?.isCritical && "bg-destructive/10 dark:bg-destructive/20",
                            )}
                          >
                            <TableCell>
                              <div className="flex flex-col">
                                <span className="font-medium text-sm leading-snug">{a.name}</span>
                                {a.unit && (
                                  <span className="text-muted-foreground text-xs">
                                    Unidad: {a.unit}
                                  </span>
                                )}
                              </div>
                            </TableCell>

                            <TableCell className="font-mono text-xs">
                              {refRange ? (
                                <span className="text-foreground font-medium">
                                  {refRange.min} – {refRange.max} {a.unit ?? ""}
                                </span>
                              ) : (
                                <span className="text-muted-foreground/60 italic">— Sin rango</span>
                              )}
                            </TableCell>

                            <TableCell>
                              <div className="flex items-center gap-2">
                                <Input
                                  type="number"
                                  step="any"
                                  inputMode="decimal"
                                  placeholder={existing != null ? `Actual: ${existing.value}` : "—"}
                                  className={cn(
                                    "h-8 w-32 font-mono text-sm",
                                    isFilled && "font-semibold",
                                    status?.label === "Alto" &&
                                      "border-amber-500/50 text-amber-700 dark:text-amber-400 focus-visible:ring-amber-500",
                                    status?.label === "Bajo" &&
                                      "border-destructive/50 text-destructive focus-visible:ring-destructive",
                                  )}
                                  value={currentVal}
                                  onChange={(e) =>
                                    setBatchValues((prev) => ({
                                      ...prev,
                                      [a.id]: e.target.value,
                                    }))
                                  }
                                />
                                {existing?.deltaVariation != null && (
                                  <span
                                    title="Variación vs. resultado previo (delta check)"
                                    className={cn(
                                      "text-[11px] font-medium shrink-0",
                                      Math.abs(existing.deltaVariation) >= 50
                                        ? "text-destructive font-bold"
                                        : "text-muted-foreground",
                                    )}
                                  >
                                    {existing.deltaVariation >= 0 ? "▲" : "▼"}{" "}
                                    {Math.abs(existing.deltaVariation).toFixed(0)}%
                                  </span>
                                )}
                              </div>
                            </TableCell>

                            <TableCell className="text-center">
                              {status ? (
                                <Badge
                                  variant={status.variant}
                                  className={cn(
                                    "text-xs font-medium",
                                    status.label === "Alto" &&
                                      "bg-amber-500/15 text-amber-700 border-amber-500/30 dark:text-amber-300",
                                    status.label === "Normal" &&
                                      "bg-emerald-500/15 text-emerald-700 border-emerald-500/30 dark:text-emerald-300",
                                    status.isCritical && "animate-pulse font-bold",
                                  )}
                                >
                                  {status.label}
                                </Badge>
                              ) : (
                                <span className="text-muted-foreground/40 text-xs">—</span>
                              )}
                            </TableCell>

                            <TableCell className="text-right">
                              <div className="flex items-center justify-end gap-1">
                                {existing && existing.attachments.length > 0 && (
                                  <button
                                    type="button"
                                    onClick={() => {
                                      setConfirmDelete(false);
                                      setPreviewAttachment(existing.attachments[0]);
                                    }}
                                    title={`${existing.attachments.length} foto(s) adjunta(s)`}
                                    className="p-1 text-primary hover:text-primary/80"
                                  >
                                    <Paperclip className="size-4" />
                                  </button>
                                )}
                                {existing && isVetOrAdmin && (
                                  <Button
                                    variant="ghost"
                                    size="sm"
                                    className="text-muted-foreground h-7 w-7 p-0 hover:text-foreground"
                                    onClick={() => pickAndAttach(existing)}
                                    disabled={attachFile.isPending}
                                    title="Adjuntar foto de placa/frotis"
                                  >
                                    {attachFile.isPending ? (
                                      <Loader2 className="size-3.5 animate-spin" />
                                    ) : (
                                      <ImagePlus className="size-3.5" />
                                    )}
                                  </Button>
                                )}
                                {isFilled && (
                                  <Button
                                    variant="ghost"
                                    size="sm"
                                    className="text-muted-foreground h-7 w-7 p-0 hover:text-destructive"
                                    onClick={() => {
                                      setBatchValues((prev) => {
                                        const copy = { ...prev };
                                        delete copy[a.id];
                                        return copy;
                                      });
                                      if (extraAnalyteIds.includes(a.id)) {
                                        setExtraAnalyteIds((prev) => prev.filter((id) => id !== a.id));
                                      }
                                    }}
                                    title="Limpiar valor (no se incluirá en el informe PDF)"
                                  >
                                    <X className="size-3.5" />
                                  </Button>
                                )}
                              </div>
                            </TableCell>
                          </TableRow>
                        );
                      })
                    )}
                  </TableBody>
                </Table>

                {/* Barra de guardado de la tabla */}
                <div className="bg-muted/40 flex flex-wrap items-center justify-between gap-3 border-t px-4 py-3">
                  <div className="text-xs text-muted-foreground">
                    <p className="font-medium text-foreground">
                      {displayedAnalytes.filter((a) => batchValues[a.id]?.trim()).length} analitos con valor ingresado
                    </p>
                    <p>
                      Solo los analitos con valor se guardarán y saldrán en el informe PDF. Los analitos vacíos no saldrán.
                    </p>
                  </div>

                  <Button
                    onClick={handleSaveAllResults}
                    disabled={registerResults.isPending || deleteResult.isPending}
                    className="gap-2 shadow-xs font-semibold"
                  >
                    {registerResults.isPending || deleteResult.isPending ? (
                      <Loader2 className="size-4 animate-spin" />
                    ) : (
                      <Save className="size-4" />
                    )}
                    Guardar resultados
                  </Button>
                </div>
              </div>
            ) : (
              /* Vista de resultados sólo lectura (muestra FINALIZADA o ANULADA) */
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
                                      className="group relative overflow-hidden rounded-md border shadow-xs"
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
                            </TableCell>
                          </TableRow>
                        );
                      })}
                    </TableBody>
                  </Table>
                )}
              </div>
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
            {sample && (
              <Button variant="outline" onClick={() => setShowEvents(true)}>
                <History className="size-4" />
                Historial
              </Button>
            )}
            {sample && (
              <Button variant="outline" onClick={() => setShowNotifications(true)}>
                <Mail className="size-4" />
                Notificaciones
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
        <DialogFooter className="flex-col gap-2 sm:flex-row sm:justify-between">
          <Button
            variant="outline"
            onClick={doAcknowledgeCritical}
            disabled={acknowledgeCritical.isPending}
          >
            {acknowledgeCritical.isPending ? (
              <Loader2 className="size-4 animate-spin" />
            ) : (
              <CheckCircle2 className="size-4" />
            )}
            Confirmar revisión
          </Button>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              onClick={doSendCriticalEmail}
              disabled={sendCriticalEmail.isPending}
            >
              {sendCriticalEmail.isPending ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Mail className="size-4" />
              )}
              Enviar por email
            </Button>
            <Button
              variant="destructive"
              onClick={() => handleWhatsAppCritical(criticalAlert)}
            >
              <MessageCircle className="size-4" />
              WhatsApp
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    {/* Historial de la muestra: rechazos y reaperturas (quién, cuándo, motivo). */}
    <Dialog open={showEvents} onOpenChange={(o) => !o && setShowEvents(false)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <History className="size-4" />
            Historial de la muestra
          </DialogTitle>
          <DialogDescription>
            Rechazos y reaperturas registrados para {sample?.code ?? "esta muestra"}
            (quién, cuándo y motivo de cada evento).
          </DialogDescription>
        </DialogHeader>
        <div className="max-h-72 space-y-2 overflow-y-auto pr-1">
          {loadingEvents ? (
            <div className="flex items-center justify-center py-6">
              <Loader2 className="size-5 animate-spin text-muted-foreground" />
            </div>
          ) : events.length === 0 ? (
            <p className="text-muted-foreground py-6 text-center text-sm">
              Sin eventos registrados para esta muestra.
            </p>
          ) : (
            events.map((ev) => <EventRow key={ev.id} ev={ev} />)
          )}
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => setShowEvents(false)}>
            Cerrar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    {/* Notificaciones de la muestra: envíos y confirmaciones de valores críticos. */}
    <Dialog
      open={showNotifications}
      onOpenChange={(o) => !o && setShowNotifications(false)}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Mail className="size-4" />
            Notificaciones de la muestra
          </DialogTitle>
          <DialogDescription>
            Envíos por email y confirmaciones de valores críticos registrados
            para {sample?.code ?? "esta muestra"}.
          </DialogDescription>
        </DialogHeader>
        <div className="max-h-72 space-y-2 overflow-y-auto pr-1">
          {loadingNotifications ? (
            <div className="flex items-center justify-center py-6">
              <Loader2 className="size-5 animate-spin text-muted-foreground" />
            </div>
          ) : notifications.length === 0 ? (
            <p className="text-muted-foreground py-6 text-center text-sm">
              Sin notificaciones registradas para esta muestra.
            </p>
          ) : (
            notifications.map((n) => <NotificationRow key={n.id} n={n} />)
          )}
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => setShowNotifications(false)}>
            Cerrar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    </>
  );
}

function NotificationRow({ n }: { n: NotificationLogEntry }) {
  const isAck = n.channel === "MANUAL";
  const isSent = n.status === "SENT";
  const label = isAck
    ? "Confirmación"
    : n.channel === "EMAIL"
      ? "Email"
      : n.channel;
  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-lg border px-3 py-2.5",
        isAck
          ? "border-border bg-muted/30"
          : isSent
            ? "border-emerald-500/30 bg-emerald-500/5"
            : "border-destructive/30 bg-destructive/5",
      )}
    >
      <Badge
        variant={
          isAck ? "outline" : isSent ? "default" : "destructive"
        }
        className="mt-0.5 shrink-0"
      >
        {label}
      </Badge>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-x-2 text-sm">
          <span className="font-medium">
            {isAck ? (n.ackedBy ?? "—") : (n.recipientName ?? "—")}
          </span>
          <span className="text-muted-foreground text-xs">
            {isAck
              ? formatDateTime(n.ackedAt ?? n.createdAt)
              : formatDateTime(n.sentAt ?? n.createdAt)}
          </span>
        </div>
        {!isAck && n.recipientAddress && (
          <p className="text-muted-foreground mt-0.5 font-mono text-xs">
            {n.recipientAddress}
          </p>
        )}
        {isAck ? (
          <p className="text-muted-foreground mt-0.5 text-xs">
            Valor crítico revisado
          </p>
        ) : n.status === "FAILED" ? (
          <p className="text-muted-foreground mt-0.5 text-xs">
            Fallo en el envío
          </p>
        ) : (
          <p className="text-muted-foreground mt-0.5 text-xs">
            Enviado correctamente
          </p>
        )}
      </div>
    </div>
  );
}

function EventRow({ ev }: { ev: SampleEvent }) {
  const isRejected = ev.eventType === "REJECTED";
  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-lg border px-3 py-2.5",
        isRejected
          ? "border-destructive/30 bg-destructive/5"
          : "border-border bg-muted/30",
      )}
    >
      <Badge variant={isRejected ? "destructive" : "outline"} className="mt-0.5 shrink-0">
        {isRejected ? "Rechazo" : "Reapertura"}
      </Badge>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-x-2 text-sm">
          <span className="font-medium">{ev.username}</span>
          <span className="text-muted-foreground text-xs">
            {formatDateTime(ev.createdAt)}
          </span>
        </div>
        {isRejected && ev.reason && (
          <p className="text-muted-foreground mt-0.5 text-xs">
            Motivo: {ev.reason}
          </p>
        )}
      </div>
    </div>
  );
}
