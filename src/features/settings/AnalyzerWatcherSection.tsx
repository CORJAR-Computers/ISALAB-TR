import { useEffect, useState } from "react";
import { toast } from "sonner";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  FolderCog,
  FolderSync,
  ListChecks,
  Loader2,
  Pause,
  Play,
  Plus,
  RefreshCw,
  Trash2,
  Upload,
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
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Label } from "@/components/ui/label";
import {
  useAnalytes,
  useAnalyzerImportJobs,
  useAnalyzers,
  useAnalyzerSources,
  useDeleteAnalyzerImportJob,
  useDeleteAnalyzerSource,
  usePollAnalyzerSource,
  usePreviewAnalyzerImport,
  useSaveAnalyzerSource,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import type {
  Analyzer,
  AnalyzerImportJob,
  AnalyzerImportMapping,
  AnalyzerSource,
  ImportColumnMapping,
  ImportPreview,
} from "@/bindings";

type ColumnRole =
  | { kind: "skip" }
  | { kind: "code" }
  | { kind: "analyte"; analyteId: number };

// =================== Diálogo de configuración de la fuente ==================

function SourceDialog({
  open,
  onOpenChange,
  analyzer,
  source,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  analyzer: Analyzer;
  source: AnalyzerSource | null;
}) {
  const save = useSaveAnalyzerSource();
  const previewMut = usePreviewAnalyzerImport();
  const { data: analytes = [] } = useAnalytes();

  const [folder, setFolder] = useState("");
  const [enabled, setEnabled] = useState(true);
  // Mapeo guardado (sin editar aún).
  const [mapping, setMapping] = useState<AnalyzerImportMapping | null>(null);
  // Vista previa del CSV de referencia cargado (para re-mapear).
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [roles, setRoles] = useState<ColumnRole[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (open) {
      setFolder(source?.folderPath ?? "");
      setEnabled(source?.enabled ?? true);
      setMapping(source?.mapping ?? null);
      setPreview(null);
      setRoles([]);
    }
  }, [open, source]);

  const pickFolder = async () => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "Carpeta de exportación del analizador",
      });
      if (!selected || Array.isArray(selected)) return;
      setFolder(selected);
    } catch (e) {
      toast.error("No se pudo abrir el selector de carpetas", {
        description: getErrorMessage(e),
      });
    }
  };

  /** Carga un CSV de referencia y sugiere el mapeo columna → analito. */
  const pickReferenceCsv = async () => {
    let selected: string | string[] | null;
    try {
      selected = await openDialog({
        title: "Selecciona un CSV exportado por este analizador",
        filters: [{ name: "CSV", extensions: ["csv", "txt"] }],
      });
    } catch (e) {
      toast.error("No se pudo abrir el selector de archivos", {
        description: getErrorMessage(e),
      });
      return;
    }
    const p = Array.isArray(selected) ? selected[0] : selected;
    if (!p) return;
    setBusy(true);
    try {
      const prev = await previewMut.mutateAsync(p);
      setPreview(prev);
      setRoles(
        prev.headers.map((_, i) => {
          if (i === prev.suggestedSampleCodeColumn)
            return { kind: "code" as const };
          const analyteId = prev.suggestedAnalytes[i];
          if (analyteId != null)
            return { kind: "analyte" as const, analyteId };
          return { kind: "skip" as const };
        }),
      );
      setMapping(null); // se reconstruye al guardar
    } catch (e) {
      toast.error("No se pudo leer el CSV de referencia", {
        description: getErrorMessage(e),
      });
    } finally {
      setBusy(false);
    }
  };

  const setRole = (index: number, role: ColumnRole) => {
    setRoles((prev) => {
      const next = [...prev];
      if (role.kind === "code") {
        for (let i = 0; i < next.length; i++) {
          if (i !== index && next[i]?.kind === "code")
            next[i] = { kind: "skip" };
        }
      }
      next[index] = role;
      return next;
    });
  };

  /** Mapeo efectivo: el editado o el construido desde la vista previa. */
  const effectiveMapping = (): AnalyzerImportMapping | null => {
    if (mapping) return mapping;
    if (!preview) return null;
    const codeColumn = roles.findIndex((r) => r.kind === "code");
    if (codeColumn < 0) return null;
    const columns: ImportColumnMapping[] = roles
      .map((r, i) => ({ role: r, index: i }))
      .filter((c) => c.role.kind === "analyte")
      .map((c) => ({
        columnIndex: c.index,
        analyteId: (c.role as { kind: "analyte"; analyteId: number })
          .analyteId,
      }));
    if (columns.length === 0) return null;
    return { sampleCodeColumn: codeColumn, columns };
  };

  // Con la vigilancia activa se exige mapeo; en pausa se puede guardar solo
  // la carpeta (para configurarla después).
  const canSave =
    folder.trim().length > 0 && (!enabled || effectiveMapping() != null);

  const onSubmit = async () => {
    if (!canSave) return;
    try {
      const saved = await save.mutateAsync({
        analyzerId: analyzer.id,
        sourceType: source?.sourceType ?? null,
        folderPath: folder.trim(),
        enabled,
        mapping: effectiveMapping(),
      });
      toast.success(
        saved
          ? saved.enabled
            ? "Vigilancia activada"
            : "Configuración guardada"
          : "Fuente eliminada",
        {
          description: saved
            ? `${analyzer.name}: ${saved.folderPath ?? "sin carpeta"}`
            : undefined,
        },
      );
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo guardar la configuración", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FolderCog className="size-5 text-primary" />
            Carpeta vigilada — {analyzer.name}
          </DialogTitle>
          <DialogDescription>
            La app importará automáticamente los CSV que el analizador exporte
            a esta carpeta, usando el mapeo de columnas configurado.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-5">
          {/* Carpeta */}
          <div className="space-y-2">
            <Label>Carpeta de exportación</Label>
            <div className="flex gap-2">
              <Input
                value={folder}
                onChange={(e) => setFolder(e.target.value)}
                placeholder="C:\\Exports\\MINDRAY"
                className="font-mono text-xs"
              />
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={pickFolder}
              >
                Examinar…
              </Button>
            </div>
            <p className="text-muted-foreground text-xs">
              Apunta a la carpeta donde el software del equipo guarda los CSV
              (USB o carpeta compartida). Los archivos importados se mueven a
              una subcarpeta <span className="font-mono">importados/</span>.
            </p>
          </div>

          {/* Vigilar automáticamente */}
          <label className="flex cursor-pointer items-start gap-2">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
              className="mt-0.5"
            />
            <span className="text-sm">
              Vigilar la carpeta automáticamente (cada 3 segundos)
              <span className="text-muted-foreground block text-xs">
                Los resultados se cargan a las muestras por código y se validan
                contra los rangos del equipo.
              </span>
            </span>
          </label>

          {/* Mapeo */}
          <div className="space-y-2">
            <Label>Mapeo de columnas del CSV</Label>
            {!preview && !mapping && (
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={pickReferenceCsv}
                disabled={busy || previewMut.isPending}
              >
                {busy || previewMut.isPending ? (
                  <Loader2 className="size-4 animate-spin" />
                ) : (
                  <Upload className="size-4" />
                )}
                Elegir CSV de referencia para el mapeo
              </Button>
            )}
            {mapping && !preview && (
              <div className="bg-muted/40 rounded-lg border px-3 py-2 text-xs">
                <p>
                  Mapeo guardado: columna{" "}
                  <span className="font-mono">{mapping.sampleCodeColumn}</span>{" "}
                  = código de muestra y {mapping.columns.length} analito
                  {mapping.columns.length === 1 ? "" : "s"}.
                </p>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="mt-1 h-6 px-2 text-xs"
                  onClick={pickReferenceCsv}
                  disabled={busy}
                >
                  <RefreshCw className="size-3" /> Cambiar con CSV de referencia
                </Button>
              </div>
            )}

            {preview && (
              <div className="space-y-2">
                <div className="bg-muted/50 flex flex-wrap items-center gap-2 rounded-lg border px-3 py-2 text-xs">
                  <span className="font-medium">{preview.fileName}</span>
                  <Badge variant="outline">{preview.totalRows} filas</Badge>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="ml-auto h-6 gap-1 px-2 text-xs"
                    onClick={pickReferenceCsv}
                  >
                    <RefreshCw className="size-3" /> Otro archivo
                  </Button>
                </div>
                <div className="overflow-x-auto rounded-lg border">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="bg-muted/40 text-muted-foreground text-left text-xs">
                        <th className="px-3 py-2 font-medium">Columna</th>
                        <th className="px-3 py-2 font-medium">Asignación</th>
                      </tr>
                    </thead>
                    <tbody>
                      {preview.headers.map((header, i) => (
                        <tr key={i} className="border-t">
                          <td className="px-3 py-1.5 font-mono text-xs">
                            {header}
                          </td>
                          <td className="px-3 py-1.5">
                            <Select
                              value={
                                roles[i]?.kind === "code"
                                  ? "__code__"
                                  : roles[i]?.kind === "analyte"
                                    ? `a:${(roles[i] as { kind: "analyte"; analyteId: number }).analyteId}`
                                    : "__skip__"
                              }
                              onValueChange={(v) => {
                                if (v === "__code__")
                                  setRole(i, { kind: "code" });
                                else if (v === "__skip__")
                                  setRole(i, { kind: "skip" });
                                else
                                  setRole(i, {
                                    kind: "analyte",
                                    analyteId: Number(v.slice(2)),
                                  });
                              }}
                            >
                              <SelectTrigger className="h-7 w-full text-xs">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="__skip__">
                                  Ignorar
                                </SelectItem>
                                <SelectItem value="__code__">
                                  Código de muestra
                                </SelectItem>
                                {analytes.map((a) => (
                                  <SelectItem
                                    key={a.id}
                                    value={`a:${a.id}`}
                                  >
                                    {a.name}
                                    {a.unit ? ` (${a.unit})` : ""}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {!effectiveMapping() && (
              <p className="text-destructive text-xs">
                {enabled
                  ? "Se necesita un mapeo (columna de código + al menos un analito) para activar la vigilancia."
                  : "Sin mapeo todavía: la fuente quedará en pausa hasta que cargues un CSV de referencia."}
              </p>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="ghost"
            onClick={() => onOpenChange(false)}
          >
            Cancelar
          </Button>
          <Button
            type="button"
            onClick={onSubmit}
            disabled={!canSave || save.isPending}
          >
            {save.isPending ? (
              <Loader2 className="size-4 animate-spin" />
            ) : enabled ? (
              <Play className="size-4" />
            ) : (
              <Pause className="size-4" />
            )}
            Guardar configuración
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ====================== Diálogo de cola de importación ======================

function QueueDialog({
  open,
  onOpenChange,
  source,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  source: AnalyzerSource;
}) {
  const { data: jobs = [], isLoading } = useAnalyzerImportJobs(
    open ? source.id : null,
    50,
  );
  const poll = usePollAnalyzerSource();
  const deleteJob = useDeleteAnalyzerImportJob();

  const retry = async (job: AnalyzerImportJob) => {
    try {
      await deleteJob.mutateAsync(job.id);
      toast.success("Reintento programado", {
        description: `El archivo ${job.fileName} volverá a intentarse en el próximo sondeo.`,
      });
    } catch (e) {
      toast.error("No se pudo reintentar", {
        description: getErrorMessage(e),
      });
    }
  };

  const pollNow = async () => {
    try {
      const newJobs = await poll.mutateAsync(source.id);
      toast.success(
        newJobs.length > 0
          ? `Sondeo completado: ${newJobs.length} archivo(s) procesado(s)`
          : "Sondeo completado: sin archivos nuevos",
      );
    } catch (e) {
      toast.error("El sondeo falló", {
        description: getErrorMessage(e),
      });
    }
  };

  const failed = jobs.filter((j) => j.status === "FALLIDO").length;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ListChecks className="size-5 text-primary" />
            Cola de importación — {source.analyzerName}
          </DialogTitle>
          <DialogDescription>
            Archivos detectados en {source.folderPath ?? "la carpeta vigilada"}.
            Los fallidos pueden reintentarse desde aquí.
          </DialogDescription>
        </DialogHeader>

        {failed > 0 && (
          <Badge variant="destructive" className="w-fit">
            {failed} archivo{failed === 1 ? "" : "s"} fallido
            {failed === 1 ? "" : "s"}
          </Badge>
        )}

        {isLoading ? (
          <Skeleton className="h-32 w-full" />
        ) : jobs.length === 0 ? (
          <div className="border-dashed py-8 text-center text-xs text-muted-foreground rounded-lg border">
            Aún no se ha procesado ningún archivo. Cuando el analizador exporte
            un CSV a la carpeta, aparecerá aquí.
          </div>
        ) : (
          <div className="overflow-hidden rounded-lg border">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-muted/40 text-muted-foreground text-left text-xs">
                  <th className="px-3 py-2 font-medium">Archivo</th>
                  <th className="px-3 py-2 font-medium">Estado</th>
                  <th className="px-3 py-2 text-right font-medium">
                    Muestras
                  </th>
                  <th className="px-3 py-2 text-right font-medium">
                    Resultados
                  </th>
                  <th className="px-3 py-2 font-medium">Procesado</th>
                  <th className="px-3 py-2" />
                </tr>
              </thead>
              <tbody>
                {jobs.map((j) => (
                  <tr key={j.id} className="border-t">
                    <td className="max-w-[180px] truncate px-3 py-2 font-mono text-xs">
                      <span title={j.fileName}>{j.fileName}</span>
                    </td>
                    <td className="px-3 py-2">
                      <Badge
                        variant={
                          j.status === "IMPORTADO" ? "success" : "destructive"
                        }
                      >
                        {j.status === "IMPORTADO" ? "Importado" : "Fallido"}
                      </Badge>
                    </td>
                    <td className="px-3 py-2 text-right font-mono text-xs">
                      {j.samplesUpdated}
                    </td>
                    <td className="px-3 py-2 text-right font-mono text-xs">
                      {j.resultsImported}
                      {j.skippedRows > 0 && (
                        <span className="text-muted-foreground ml-1">
                          ({j.skippedRows} omit.)
                        </span>
                      )}
                    </td>
                    <td className="text-muted-foreground px-3 py-2 text-xs">
                      {j.processedAt}
                    </td>
                    <td className="px-3 py-2 text-right">
                      {j.status === "FALLIDO" && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 px-2"
                          onClick={() => retry(j)}
                          disabled={deleteJob.isPending}
                          title="Reintentar en el próximo sondeo"
                        >
                          <RefreshCw className="size-3.5" />
                        </Button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {jobs.some((j) => j.errorMsg) && (
              <div className="bg-muted/40 border-t px-3 py-2 text-xs">
                {jobs
                  .filter((j) => j.errorMsg)
                  .slice(0, 3)
                  .map((j) => (
                    <p
                      key={j.id}
                      className="text-muted-foreground flex gap-1"
                    >
                      <span className="font-mono">{j.fileName}:</span>
                      <span className="truncate">{j.errorMsg}</span>
                    </p>
                  ))}
              </div>
            )}
          </div>
        )}

        <DialogFooter className="flex items-center gap-2">
          <p className="text-muted-foreground mr-auto text-xs">
            Último sondeo: {source.lastPollAt ?? "nunca"}
          </p>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={pollNow}
            disabled={poll.isPending}
          >
            {poll.isPending ? (
              <Loader2 className="size-4 animate-spin" />
            ) : (
              <RefreshCw className="size-4" />
            )}
            Sondear ahora
          </Button>
          <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
            Cerrar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ============================ Sección principal =============================

/**
 * Importación automática por carpeta vigilada. Lista las fuentes configuradas
 * por analizador y permite crear/editar la carpeta + mapeo, activar/desactivar
 * la vigilancia y revisar la cola de importación por archivo.
 */
export function AnalyzerWatcherSection() {
  const { data: sources = [], isLoading } = useAnalyzerSources();
  const { data: analyzers = [] } = useAnalyzers();
  const deleteSource = useDeleteAnalyzerSource();
  const saveSource = useSaveAnalyzerSource();
  const pollNow = usePollAnalyzerSource();

  const [editing, setEditing] = useState<{
    analyzer: Analyzer;
    source: AnalyzerSource | null;
  } | null>(null);
  const [queueFor, setQueueFor] = useState<AnalyzerSource | null>(null);

  // Analizadores sin fuente configurada (para crear una nueva).
  const configuredIds = new Set(sources.map((s) => s.analyzerId));
  const availableAnalyzers = analyzers.filter(
    (a) => a.isActive && a.code !== "GENERAL" && !configuredIds.has(a.id),
  );
  const [newAnalyzerId, setNewAnalyzerId] = useState<number | null>(null);

  const removeSource = async (s: AnalyzerSource) => {
    if (
      !window.confirm(
        `¿Eliminar la carpeta vigilada de ${s.analyzerName}? Se borrará también su historial de importación.`,
      )
    )
      return;
    try {
      await deleteSource.mutateAsync(s.id);
      toast.success("Fuente eliminada");
    } catch (e) {
      toast.error("No se pudo eliminar", { description: getErrorMessage(e) });
    }
  };

  const toggleSource = async (s: AnalyzerSource) => {
    try {
      await saveSource.mutateAsync({
        analyzerId: s.analyzerId,
        sourceType: s.sourceType,
        folderPath: s.folderPath,
        enabled: !s.enabled,
        mapping: s.mapping,
      });
      toast.success(s.enabled ? "Vigilancia en pausa" : "Vigilancia activada");
    } catch (e) {
      toast.error("No se pudo cambiar el estado", {
        description: getErrorMessage(e),
      });
    }
  };

  const pollSourceNow = async (s: AnalyzerSource) => {
    try {
      const jobs = await pollNow.mutateAsync(s.id);
      toast.success(
        jobs.length > 0
          ? `Sondeo completado: ${jobs.length} archivo(s) procesado(s)`
          : "Sondeo completado: sin archivos nuevos",
      );
    } catch (e) {
      toast.error("El sondeo falló", {
        description: getErrorMessage(e),
      });
    }
  };

  if (isLoading) {
    return <Skeleton className="h-24 w-full" />;
  }

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-sm font-semibold">
            Importación automática (carpeta vigilada)
          </p>
          <p className="text-muted-foreground text-xs">
            El analizador exporta resultados a una carpeta y la app los importa
            solos con el mapeo configurado. Los archivos se archivan tras
            importarse; los fallidos quedan en la cola para reintentar.
          </p>
        </div>
        <Badge variant="secondary">
          {sources.filter((s) => s.enabled).length} activa
          {sources.filter((s) => s.enabled).length === 1 ? "" : "s"} ·{" "}
          {sources.length} configurada{sources.length === 1 ? "" : "s"}
        </Badge>
      </div>

      {sources.length === 0 && availableAnalyzers.length > 0 && (
        <p className="text-muted-foreground rounded-lg border border-dashed px-3 py-6 text-center text-xs">
          Sin fuentes configuradas todavía. Crea la primera con el selector de
          abajo.
        </p>
      )}
      {sources.length > 0 && (
        <div className="space-y-2">
          {sources.map((s) => (
            <div
              key={s.id}
              className="bg-muted/40 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-lg border px-3 py-2.5"
            >
              <div className="min-w-0 flex-1">
                <p className="flex flex-wrap items-center gap-2 text-sm font-semibold">
                  <FolderSync className="text-muted-foreground size-4" />
                  {s.analyzerName}
                  {s.enabled ? (
                    <Badge variant="success">Vigilando</Badge>
                  ) : (
                    <Badge variant="outline" className="text-muted-foreground">
                      En pausa
                    </Badge>
                  )}
                </p>
                <p className="text-muted-foreground truncate text-xs">
                  <span className="font-mono">{s.folderPath}</span>
                  <span> · {s.mappedColumns} analito(s) mapeado(s)</span>
                  {s.lastPollAt ? ` · sondeo: ${s.lastPollAt}` : ""}
                </p>
              </div>
              <div className="flex shrink-0 items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-8 px-2"
                  onClick={() => toggleSource(s)}
                  disabled={
                    saveSource.isPending ||
                    (s.mapping == null && !s.enabled)
                  }
                  title={
                    s.mapping == null && !s.enabled
                      ? "Configura el mapeo antes de activar"
                      : s.enabled
                        ? "Pausar vigilancia"
                        : "Activar vigilancia"
                  }
                >
                  {s.enabled ? (
                    <Pause className="size-3.5" />
                  ) : (
                    <Play className="size-3.5" />
                  )}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-8 px-2"
                  onClick={() => setQueueFor(s)}
                  title="Ver cola de importación"
                >
                  <ListChecks className="size-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-8 px-2"
                  onClick={() =>
                    setEditing({
                      analyzer:
                        analyzers.find((a) => a.id === s.analyzerId) ??
                        ({ id: s.analyzerId, name: s.analyzerName } as Analyzer),
                      source: s,
                    })
                  }
                  title="Editar configuración"
                >
                  <FolderCog className="size-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-8 px-2"
                  onClick={() => pollSourceNow(s)}
                  disabled={
                    pollNow.isPending ||
                    s.mapping == null ||
                    s.folderPath == null
                  }
                  title={
                    s.mapping == null
                      ? "La fuente necesita un mapeo"
                      : "Sondear ahora"
                  }
                >
                  <RefreshCw className="size-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-8 px-2 text-destructive hover:text-destructive"
                  onClick={() => removeSource(s)}
                  disabled={deleteSource.isPending}
                  title="Eliminar fuente"
                >
                  <Trash2 className="size-3.5" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Nueva fuente para un analizador sin configurar */}
      {availableAnalyzers.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <Select
            value={newAnalyzerId ? newAnalyzerId.toString() : ""}
            onValueChange={(v) => setNewAnalyzerId(Number(v))}
          >
            <SelectTrigger className="w-72">
              <SelectValue placeholder="Analizador sin carpeta vigilada…" />
            </SelectTrigger>
            <SelectContent>
              {availableAnalyzers.map((a) => (
                <SelectItem key={a.id} value={a.id.toString()}>
                  {a.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => {
              const analyzer = availableAnalyzers.find(
                (a) => a.id === newAnalyzerId,
              );
              if (analyzer) {
                setEditing({ analyzer, source: null });
                setNewAnalyzerId(null);
              }
            }}
            disabled={newAnalyzerId == null}
          >
            <Plus className="size-4" />
            Configurar carpeta vigilada
          </Button>
        </div>
      )}

      {editing && (
        <SourceDialog
          open
          onOpenChange={(o) => {
            if (!o) setEditing(null);
          }}
          analyzer={editing.analyzer}
          source={editing.source}
        />
      )}
      {queueFor && (
        <QueueDialog
          open
          onOpenChange={(o) => {
            if (!o) setQueueFor(null);
          }}
          source={queueFor}
        />
      )}
    </div>
  );
}
