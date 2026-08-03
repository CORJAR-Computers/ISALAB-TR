import { useState } from "react";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { FileText, FolderOpen, Plus, ScanSearch } from "lucide-react";
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
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Skeleton } from "@/components/ui/skeleton";
import { useReports } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { formatDateTime } from "@/lib/utils";
import { GenerateReportDialog } from "./GenerateReportDialog";

export function ReportsPage() {
  const { data: reports, isLoading, isError } = useReports();
  const [genOpen, setGenOpen] = useState(false);

  const open = async (path: string) => {
    try {
      await openPath(path);
    } catch (e) {
      toast.error("No se pudo abrir el PDF", {
        description: getErrorMessage(e),
      });
    }
  };

  const reveal = async (path: string) => {
    try {
      await revealItemInDir(path);
    } catch (e) {
      toast.error("No se pudo mostrar el archivo", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Reportes PDF
          </h2>
          <p className="text-muted-foreground text-sm">
            Informes de resultados analíticos generados en Rust (printpdf) y
            guardados en la carpeta de datos de la app.
          </p>
        </div>
        <Button onClick={() => setGenOpen(true)}>
          <Plus className="size-4" />
          Generar informe
        </Button>
      </div>

      <Card className="gap-0 p-0">
        <CardHeader className="border-b">
          <CardTitle className="text-base">Informes generados</CardTitle>
          <CardDescription>
            {isLoading
              ? "Cargando…"
              : `${reports?.length ?? 0} archivo${(reports?.length ?? 0) === 1 ? "" : "s"}`}
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Archivo</TableHead>
                <TableHead>Muestra</TableHead>
                <TableHead>Generado</TableHead>
                <TableHead className="text-right">Acciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading &&
                Array.from({ length: 3 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell colSpan={4} className="h-12">
                      <Skeleton className="h-8 w-full" />
                    </TableCell>
                  </TableRow>
                ))}

              {!isLoading && isError && (
                <TableRow>
                  <TableCell colSpan={4} className="text-muted-foreground h-16 text-center">
                    No se pudo leer la carpeta de reportes.
                  </TableCell>
                </TableRow>
              )}

              {!isLoading && reports?.length === 0 && (
                <TableRow>
                  <TableCell colSpan={4} className="text-muted-foreground h-28 text-center">
                    <div className="flex flex-col items-center gap-2">
                      <ScanSearch className="size-6 opacity-60" />
                      Aún no hay informes. Genera el primero con el botón de
                      arriba (laboratorio, fórmula, consentimiento, recibo,
                      cirugía o carnet de vacunación).
                    </div>
                  </TableCell>
                </TableRow>
              )}

              {reports?.map((r) => (
                <TableRow key={r.path}>
                  <TableCell>
                    <div className="flex items-center gap-2.5">
                      <div className="bg-destructive/10 text-destructive flex size-8 shrink-0 items-center justify-center rounded-lg">
                        <FileText className="size-4" />
                      </div>
                      <span className="font-mono text-sm">{r.fileName}</span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary">{r.sampleCode}</Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground text-xs">
                    {formatDateTime(r.generatedAt)}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => open(r.path)}
                      >
                        <FolderOpen className="size-4" />
                        Abrir
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => reveal(r.path)}
                      >
                        <ScanSearch className="size-4" />
                        Ubicar
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <GenerateReportDialog
        open={genOpen}
        onOpenChange={setGenOpen}
        onGenerated={(report) => {
          setGenOpen(false);
          toast.success(`Informe ${report.fileName} generado`);
        }}
      />
    </div>
  );
}
