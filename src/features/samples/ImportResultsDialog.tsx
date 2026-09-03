import { useEffect, useState } from "react";
import { toast } from "sonner";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  FileSpreadsheet,
  Loader2,
  Upload,
  X,
} from "lucide-react";
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
import { Badge } from "@/components/ui/badge";
import {
  useAnalytes,
  useImportAnalyzerResults,
  usePreviewAnalyzerImport,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import type { ImportPreview } from "@/bindings";

type ColumnRole =
  | { kind: "skip" }
  | { kind: "code" }
  | { kind: "analyte"; analyteId: number };

/**
 * Importa resultados desde el archivo CSV exportado por un analizador.
 * El usuario elige el archivo, revisa el mapeo automático columna → analito
 * (con vista previa) y confirma la importación.
 */
export function ImportResultsDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { data: analytes = [] } = useAnalytes();
  const previewMut = usePreviewAnalyzerImport();
  const importMut = useImportAnalyzerResults();

  const [path, setPath] = useState<string | null>(null);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [roles, setRoles] = useState<ColumnRole[]>([]);
  const [loadingFile, setLoadingFile] = useState(false);

  useEffect(() => {
    if (open) {
      setPath(null);
      setPreview(null);
      setRoles([]);
    }
  }, [open]);

  const pickFile = async () => {
    let selected: string | string[] | null;
    try {
      selected = await openDialog({
        title: "Selecciona el CSV exportado por el analizador",
        filters: [
          { name: "CSV (analizador)", extensions: ["csv", "txt"] },
          { name: "Todos los archivos", extensions: ["*"] },
        ],
      });
    } catch (err) {
      toast.error("No se pudo abrir el selector de archivos", {
        description: getErrorMessage(err),
      });
      return;
    }
    const p = Array.isArray(selected) ? selected[0] : selected;
    if (!p) return;

    setLoadingFile(true);
    try {
      const prev = await previewMut.mutateAsync(p);
      setPath(p);
      setPreview(prev);
      // Mapeo inicial: sugerencias del backend.
      setRoles(
        prev.headers.map((_, i) => {
          if (i === prev.suggestedSampleCodeColumn) return { kind: "code" as const };
          const analyteId = prev.suggestedAnalytes[i];
          if (analyteId != null) return { kind: "analyte" as const, analyteId };
          return { kind: "skip" as const };
        }),
      );
    } catch (err) {
      toast.error("No se pudo leer el archivo", {
        description: getErrorMessage(err),
      });
    } finally {
      setLoadingFile(false);
    }
  };

  const setRole = (index: number, role: ColumnRole) => {
    setRoles((prev) => {
      const next = [...prev];
      // Solo una columna puede ser el código de muestra.
      if (role.kind === "code") {
        for (let i = 0; i < next.length; i++) {
          if (i !== index && next[i]?.kind === "code") next[i] = { kind: "skip" };
        }
      }
      next[index] = role;
      return next;
    });
  };

  const canImport =
    preview != null &&
    roles.some((r) => r.kind === "code") &&
    roles.some((r) => r.kind === "analyte");

  const doImport = async () => {
    if (!path || !preview || !canImport) return;
    const codeColumn = roles.findIndex((r) => r.kind === "code");
    const columns = roles
      .map((r, i) => ({ role: r, index: i }))
      .filter((c) => c.role.kind === "analyte")
      .map((c) => ({
        columnIndex: c.index,
        analyteId: (c.role as { kind: "analyte"; analyteId: number }).analyteId,
      }));
    try {
      const summary = await importMut.mutateAsync({ path, mapping: { sampleCodeColumn: codeColumn, columns } });
      toast.success(
        `Importación completada: ${summary.resultsImported} resultados en ${summary.samplesUpdated} muestras`,
        {
          description:
            summary.skipped.length > 0
              ? `${summary.skipped.length} fila(s) omitida(s). Revisa el detalle en el log de auditoría.`
              : "Todas las filas se procesaron correctamente.",
        },
      );
      onOpenChange(false);
    } catch (err) {
      toast.error("No se pudo importar el archivo", {
        description: getErrorMessage(err),
      });
    }
  };

  const analyteName = (id: number) => analytes.find((a) => a.id === id)?.name ?? `Analito ${id}`;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileSpreadsheet className="size-5 text-primary" />
            Importar resultados del analizador (CSV)
          </DialogTitle>
          <DialogDescription>
            Elige el archivo exportado por el equipo (USB/software). Asocia cada
            columna a un analito y la columna del código de muestra, revisa la
            vista previa y confirma. Los resultados se validan contra los rangos
            de referencia de cada paciente.
          </DialogDescription>
        </DialogHeader>

        {!preview && (
          <div className="flex flex-col items-center gap-3 py-8">
            <Button onClick={pickFile} disabled={loadingFile || previewMut.isPending}>
              {loadingFile || previewMut.isPending ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Upload className="size-4" />
              )}
              {previewMut.isPending ? "Leyendo archivo…" : "Seleccionar archivo CSV"}
            </Button>
            {loadingFile && (
              <p className="text-muted-foreground text-xs">
                Analizando encabezados y sugiriendo el mapeo…
              </p>
            )}
          </div>
        )}

        {preview && (
          <div className="space-y-4">
            <div className="bg-muted/50 flex flex-wrap items-center gap-2 rounded-lg border px-3 py-2 text-xs">
              <span className="font-medium">{preview.fileName}</span>
              <Badge variant="outline">Delimitador: {preview.delimiter}</Badge>
              <Badge variant="outline">{preview.totalRows} filas de datos</Badge>
              <Button
                variant="ghost"
                size="sm"
                className="ml-auto h-6 gap-1 px-2 text-xs"
                onClick={pickFile}
              >
                <X className="size-3" />
                Cambiar archivo
              </Button>
            </div>

            <div className="overflow-x-auto rounded-lg border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Columna</TableHead>
                    <TableHead className="min-w-44">Asignación</TableHead>
                    <TableHead>Vista previa</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {preview.headers.map((header, i) => (
                    <TableRow key={i}>
                      <TableCell className="font-mono text-xs">{header}</TableCell>
                      <TableCell>
                        <Select
                          value={
                            roles[i]?.kind === "code"
                              ? "__code__"
                              : roles[i]?.kind === "analyte"
                                ? `a:${(roles[i] as { kind: "analyte"; analyteId: number }).analyteId}`
                                : "__skip__"
                          }
                          onValueChange={(v) => {
                            if (v === "__code__") setRole(i, { kind: "code" });
                            else if (v === "__skip__") setRole(i, { kind: "skip" });
                            else setRole(i, { kind: "analyte", analyteId: Number(v.slice(2)) });
                          }}
                        >
                          <SelectTrigger className="h-8 w-full text-xs">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="__skip__">Ignorar</SelectItem>
                            <SelectItem value="__code__">Código de muestra</SelectItem>
                            {analytes.map((a) => (
                              <SelectItem key={a.id} value={`a:${a.id}`}>
                                {a.name}
                                {a.unit ? ` (${a.unit})` : ""}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </TableCell>
                      <TableCell className="font-mono text-xs text-muted-foreground">
                        {preview.sampleRows.slice(0, 3).map((row, ri) => (
                          <div key={ri}>
                            {row[i] ?? ""}
                          </div>
                        ))}
                        {preview.totalRows > 3 && (
                          <div className="text-muted-foreground/50">…</div>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>

            <p className="text-muted-foreground text-xs">
              Los analitos se sugieren automáticamente por coincidencia de nombre
              (p. ej. <span className="font-mono">Glucosa</span> → Glucosa).
              Si tu analizador usa nombres distintos, asígnalos manualmente con
              el selector de cada columna.
            </p>
          </div>
        )}

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancelar
          </Button>
          <Button onClick={doImport} disabled={!canImport || importMut.isPending}>
            {importMut.isPending ? <Loader2 className="size-4 animate-spin" /> : <Upload className="size-4" />}
            Importar resultados
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}