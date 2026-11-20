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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { useBillingReport, useReports } from "@/hooks/use-queries";
import { api, getErrorMessage } from "@/lib/api";
import { formatCOP, formatDateTime } from "@/lib/utils";
import { GenerateReportDialog } from "./GenerateReportDialog";

function isoDay(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function ReportsPage() {
  const { data: reports, isLoading, isError } = useReports();
  const [genOpen, setGenOpen] = useState(false);
  const today = new Date();
  const [from, setFrom] = useState(
    isoDay(new Date(today.getFullYear(), today.getMonth(), 1)),
  );
  const [to, setTo] = useState(isoDay(today));
  const { data: billing, isLoading: billingLoading } = useBillingReport(
    from,
    to,
  );

  const open = async (path: string) => {
    try {
      try {
        await api.openReportFile(path);
      } catch {
        await openPath(path);
      }
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

      {/* Facturado vs. cobrado: dos fechas distintas, dos cifras distintas. */}
      <Card className="gap-0 p-0">
        <CardHeader className="border-b">
          <CardTitle className="text-base">Facturación del período</CardTitle>
          <CardDescription>
            Facturado por fecha de emisión y cobrado por fecha de recepción. El
            total de facturas pagadas mezclaba ambas cosas.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4 p-4">
          <div className="flex flex-wrap items-end gap-3">
            <div className="space-y-1">
              <Label htmlFor="billing-from" className="text-xs">
                Desde
              </Label>
              <Input
                id="billing-from"
                type="date"
                value={from}
                onChange={(e) => setFrom(e.target.value)}
                className="h-9 w-40"
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="billing-to" className="text-xs">
                Hasta
              </Label>
              <Input
                id="billing-to"
                type="date"
                value={to}
                onChange={(e) => setTo(e.target.value)}
                className="h-9 w-40"
              />
            </div>
            {billing && (
              <p className="text-muted-foreground pb-2 text-xs">
                {billing.billedCount} factura
                {billing.billedCount === 1 ? "" : "s"} ·{" "}
                {billing.collectedCount} recibo
                {billing.collectedCount === 1 ? "" : "s"}
              </p>
            )}
          </div>

          {billingLoading || !billing ? (
            <Skeleton className="h-20 w-full" />
          ) : (
            <>
              <div className="grid gap-3 sm:grid-cols-3">
                <div className="rounded-lg border p-3">
                  <p className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
                    Facturado
                  </p>
                  <p className="mt-1 text-xl font-bold tabular-nums">
                    {formatCOP(billing.billedTotal ?? 0)}
                  </p>
                  <p className="text-muted-foreground text-xs">
                    IVA {formatCOP(billing.billedTax ?? 0)}
                  </p>
                </div>
                <div className="rounded-lg border p-3">
                  <p className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
                    Cobrado
                  </p>
                  <p className="text-success mt-1 text-xl font-bold tabular-nums">
                    {formatCOP(billing.collectedTotal ?? 0)}
                  </p>
                  <p className="text-muted-foreground text-xs">
                    {billing.collectedCount} recepción
                    {billing.collectedCount === 1 ? "" : "es"}
                  </p>
                </div>
                <div className="rounded-lg border p-3">
                  <p className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
                    Por cobrar
                  </p>
                  <p className="text-destructive mt-1 text-xl font-bold tabular-nums">
                    {formatCOP(billing.outstandingTotal ?? 0)}
                  </p>
                  <p className="text-muted-foreground text-xs">
                    saldo de lo facturado en el rango
                  </p>
                </div>
              </div>

              {billing.methods.length > 0 && (
                <div className="overflow-hidden rounded-lg border">
                  <Table>
                    <TableHeader>
                      <TableRow className="hover:bg-transparent">
                        <TableHead>Método de pago</TableHead>
                        <TableHead className="text-right">Cobrado</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {billing.methods.map((m) => (
                        <TableRow key={m.methodCode}>
                          <TableCell>{m.methodName}</TableCell>
                          <TableCell className="text-right tabular-nums">
                            {formatCOP(m.amount ?? 0)}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}
            </>
          )}
        </CardContent>
      </Card>

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
