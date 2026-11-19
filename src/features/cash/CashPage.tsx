import { useState } from "react";
import { toast } from "sonner";
import {
  ArrowDownCircle,
  ArrowUpCircle,
  Banknote,
  FileDown,
  Loader2,
  LockKeyhole,
  RotateCcw,
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import {
  useCashSessions,
  useCashSummary,
  useGenerateCashClosePdf,
  useOpenCash,
  useOpenCashSession,
} from "@/hooks/use-queries";
import { api, getErrorMessage } from "@/lib/api";
import { CASH_MOVEMENT_KIND, CASH_SESSION_STATUS } from "@/lib/status";
import { cn, formatCOP, formatDateTime } from "@/lib/utils";
import { openPath } from "@tauri-apps/plugin-opener";
import { CashMovementDialog } from "./CashMovementDialog";
import { CloseCashDialog } from "./CloseCashDialog";
import { parseAmount } from "./money";

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <Card className="gap-0 p-3">
      <p className="text-muted-foreground text-xs">{label}</p>
      <p className="text-lg font-semibold tabular-nums">{value}</p>
    </Card>
  );
}

/** Vista Caja: apertura, movimientos, arqueo, cierre e historial. */
export function CashPage() {
  const { data: openSession, isLoading: sessionLoading } = useOpenCashSession();
  const hasSession = openSession != null;
  const { data: summary, isLoading: summaryLoading } = useCashSummary(
    null,
    hasSession,
  );
  const { data: sessions } = useCashSessions(20);
  const openCash = useOpenCash();
  const generatePdf = useGenerateCashClosePdf();

  const [openingFloat, setOpeningFloat] = useState("0");
  const [openingNotes, setOpeningNotes] = useState("");
  const [movementKind, setMovementKind] = useState<"GASTO" | "RETIRO" | "AJUSTE">(
    "GASTO",
  );
  const [movementOpen, setMovementOpen] = useState(false);
  const [closeOpen, setCloseOpen] = useState(false);

  const closedSessions = (sessions ?? []).filter((s) => s.status === "CERRADA");

  const handleOpenCash = async () => {
    try {
      await openCash.mutateAsync({
        openingFloat: parseAmount(openingFloat),
        notes: openingNotes.trim() || null,
      });
      toast.success("Caja abierta");
      setOpeningNotes("");
    } catch (e) {
      toast.error("No se pudo abrir la caja", {
        description: getErrorMessage(e),
      });
    }
  };

  const openMovement = (kind: "GASTO" | "RETIRO" | "AJUSTE") => {
    setMovementKind(kind);
    setMovementOpen(true);
  };

  const printClose = async (sessionId: number) => {
    try {
      const report = await generatePdf.mutateAsync(sessionId);
      try {
        await api.openReportFile(report.path);
      } catch {
        await openPath(report.path);
      }
    } catch (e) {
      toast.error("No se pudo generar el PDF de cierre", {
        description: getErrorMessage(e),
      });
    }
  };

  if (sessionLoading) {
    return (
      <div className="space-y-3">
        <Skeleton className="h-9 w-48" />
        <Skeleton className="h-40 w-full" />
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">Caja</h2>
          <p className="text-muted-foreground text-sm">
            Apertura, gastos y retiros, arqueo por denominación y cierre de
            sesión.
          </p>
        </div>
        {hasSession && (
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" onClick={() => openMovement("GASTO")}>
              <ArrowDownCircle className="size-4" />
              Gasto
            </Button>
            <Button variant="outline" onClick={() => openMovement("RETIRO")}>
              <ArrowUpCircle className="size-4" />
              Retiro
            </Button>
            <Button variant="outline" onClick={() => openMovement("AJUSTE")}>
              <RotateCcw className="size-4" />
              Ajuste
            </Button>
            <Button
              onClick={() => setCloseOpen(true)}
              disabled={summary == null}
            >
              <LockKeyhole className="size-4" />
              Cerrar caja
            </Button>
          </div>
        )}
      </div>

      {!hasSession && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <Banknote className="size-4 text-primary" />
              Abrir caja
            </CardTitle>
            <CardDescription>
              Los cobros exigen una sesión abierta. Registra la base inicial en
              efectivo.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-1">
                <Label htmlFor="cash-opening-float" className="text-xs">
                  Base inicial
                </Label>
                <Input
                  id="cash-opening-float"
                  value={openingFloat}
                  onChange={(e) => setOpeningFloat(e.target.value)}
                  inputMode="decimal"
                  className="text-right tabular-nums"
                />
              </div>
              <div className="space-y-1">
                <Label htmlFor="cash-opening-notes" className="text-xs">
                  Notas (opcional)
                </Label>
                <Textarea
                  id="cash-opening-notes"
                  value={openingNotes}
                  onChange={(e) => setOpeningNotes(e.target.value)}
                  className="min-h-10"
                  placeholder="Turno, responsable…"
                />
              </div>
            </div>
            <Button onClick={handleOpenCash} disabled={openCash.isPending}>
              {openCash.isPending && (
                <Loader2 className="size-4 animate-spin" />
              )}
              Abrir caja
            </Button>
          </CardContent>
        </Card>
      )}

      {hasSession && summary && (
        <>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
            <StatCard label="Base inicial" value={formatCOP(summary.openingFloat)} />
            <StatCard
              label="Efectivo esperado"
              value={formatCOP(summary.expectedCash)}
            />
            <StatCard
              label="Cobrado"
              value={formatCOP(summary.totalCollected)}
            />
            <StatCard
              label="Gastos y retiros"
              value={formatCOP(summary.cashOut)}
            />
            <StatCard label="Recibos" value={String(summary.paymentsCount)} />
          </div>

          <Card className="gap-0 p-0">
            <CardHeader className="border-b">
              <CardTitle className="text-base">
                Sesión {summary.session.code}
              </CardTitle>
              <CardDescription>
                Abierta {formatDateTime(summary.session.openedAt)}
                {summary.session.openedByName
                  ? ` por ${summary.session.openedByName}`
                  : ""}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 p-4">
              <div>
                <p className="mb-2 text-sm font-medium">Cobrado por método</p>
                {summary.methods.length === 0 ? (
                  <p className="text-muted-foreground text-sm">
                    Todavía no hay cobros en esta sesión.
                  </p>
                ) : (
                  <div className="flex flex-wrap gap-2">
                    {summary.methods.map((m) => (
                      <Badge key={m.methodCode} variant="outline" className="gap-2">
                        {m.methodName}
                        <span className="tabular-nums">
                          {formatCOP(m.amount)}
                        </span>
                      </Badge>
                    ))}
                  </div>
                )}
              </div>

              <div>
                <p className="mb-2 text-sm font-medium">Movimientos de caja</p>
                <Table>
                  <TableHeader>
                    <TableRow className="hover:bg-transparent">
                      <TableHead>Fecha</TableHead>
                      <TableHead>Tipo</TableHead>
                      <TableHead>Motivo</TableHead>
                      <TableHead>Usuario</TableHead>
                      <TableHead className="text-right">Importe</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {summaryLoading && (
                      <TableRow>
                        <TableCell colSpan={5} className="h-12">
                          <Skeleton className="h-8 w-full" />
                        </TableCell>
                      </TableRow>
                    )}
                    {!summaryLoading && summary.movements.length === 0 && (
                      <TableRow>
                        <TableCell
                          colSpan={5}
                          className="text-muted-foreground h-20 text-center"
                        >
                          Sin movimientos todavía.
                        </TableCell>
                      </TableRow>
                    )}
                    {summary.movements.map((m) => (
                      <TableRow key={m.id}>
                        <TableCell className="text-muted-foreground text-xs">
                          {formatDateTime(m.createdAt)}
                        </TableCell>
                        <TableCell>
                          {CASH_MOVEMENT_KIND[m.kind] ?? m.kind}
                        </TableCell>
                        <TableCell className="text-muted-foreground text-sm">
                          {m.reason ?? "—"}
                        </TableCell>
                        <TableCell className="text-muted-foreground text-xs">
                          {m.createdByName ?? "—"}
                        </TableCell>
                        <TableCell
                          className={cn(
                            "text-right font-medium tabular-nums",
                            m.amount < 0
                              ? "text-destructive"
                              : "text-success",
                          )}
                        >
                          {formatCOP(m.amount)}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </CardContent>
          </Card>
        </>
      )}

      <Card className="gap-0 p-0">
        <CardHeader className="border-b">
          <CardTitle className="text-base">Historial de cierres</CardTitle>
          <CardDescription>
            Sesiones cerradas con su arqueo y diferencia.
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Caja</TableHead>
                <TableHead>Apertura</TableHead>
                <TableHead>Cierre</TableHead>
                <TableHead className="text-right">Esperado</TableHead>
                <TableHead className="text-right">Contado</TableHead>
                <TableHead className="text-right">Diferencia</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead className="text-right">PDF</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {closedSessions.length === 0 && (
                <TableRow>
                  <TableCell
                    colSpan={8}
                    className="text-muted-foreground h-20 text-center"
                  >
                    Aún no hay cierres de caja.
                  </TableCell>
                </TableRow>
              )}
              {closedSessions.map((s) => {
                const status = CASH_SESSION_STATUS[s.status] ?? {
                  label: s.status,
                  variant: "secondary" as const,
                };
                const difference = s.difference ?? 0;
                return (
                  <TableRow key={s.id}>
                    <TableCell className="font-mono text-sm font-semibold">
                      {s.code}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {formatDateTime(s.openedAt)}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {s.closedAt ? formatDateTime(s.closedAt) : "—"}
                    </TableCell>
                    <TableCell className="text-right tabular-nums">
                      {formatCOP(s.expectedCash ?? 0)}
                    </TableCell>
                    <TableCell className="text-right tabular-nums">
                      {formatCOP(s.countedCash ?? 0)}
                    </TableCell>
                    <TableCell
                      className={cn(
                        "text-right font-medium tabular-nums",
                        Math.abs(difference) <= 0.01
                          ? "text-success"
                          : difference > 0
                            ? "text-amber-600 dark:text-amber-400"
                            : "text-destructive",
                      )}
                    >
                      {formatCOP(difference)}
                    </TableCell>
                    <TableCell>
                      <Badge variant={status.variant}>{status.label}</Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => printClose(s.id)}
                        disabled={generatePdf.isPending}
                      >
                        <FileDown className="size-3.5" />
                        Cierre
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <CashMovementDialog
        open={movementOpen}
        onOpenChange={setMovementOpen}
        initialKind={movementKind}
      />
      {summary && (
        <CloseCashDialog
          open={closeOpen}
          onOpenChange={setCloseOpen}
          expectedCash={summary.expectedCash}
        />
      )}
    </div>
  );
}
