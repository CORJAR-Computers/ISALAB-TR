import { useEffect, useMemo, useRef, useState } from "react";
import { Banknote, Plus, Receipt, Search } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
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
import { useInvoiceCounts, useInvoices } from "@/hooks/use-queries";
import { PAYMENT_STATUS } from "@/lib/status";
import { cn, formatCOP, formatDateTime } from "@/lib/utils";
import { InvoiceDetailDialog } from "./InvoiceDetailDialog";
import { NewInvoiceDialog } from "./NewInvoiceDialog";
import { PaymentDialog } from "./PaymentDialog";
import { usePermissions } from "@/hooks/use-permissions";
import { useUiStore } from "@/stores/ui-store";

const STATUS_TABS: Array<{ value: string | null; label: string }> = [
  { value: null, label: "Todas" },
  { value: "PENDIENTE", label: "Pendientes" },
  { value: "ABONADA", label: "Abonadas" },
  { value: "PAGADA", label: "Pagadas" },
  { value: "ANULADA", label: "Anuladas" },
];

const COLUMN_COUNT = 10;

export function InvoicesPage() {
  const [status, setStatus] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [newOpen, setNewOpen] = useState(false);
  // El diálogo se abre por estado (no con DialogTrigger) y Radix devolvía el
  // foco al menú lateral. Aquí se guarda el botón real para recuperarlo al
  // cerrar: teclado y lector de pantalla siguen donde estaban.
  const newInvoiceButton = useRef<HTMLButtonElement>(null);
  const wasNewOpen = useRef(false);
  useEffect(() => {
    if (wasNewOpen.current && !newOpen) {
      const button = newInvoiceButton.current;
      const timer = window.setTimeout(() => button?.focus(), 300);
      wasNewOpen.current = newOpen;
      return () => window.clearTimeout(timer);
    }
    wasNewOpen.current = newOpen;
  }, [newOpen]);
  const [detailId, setDetailId] = useState<number | null>(null);
  const [payingId, setPayingId] = useState<number | null>(null);
  const { data: invoices, isLoading, isError } = useInvoices(status, search);
  const { data: all } = useInvoiceCounts();
  const { canBill } = usePermissions();
  const entityRequest = useUiStore((s) => s.entityRequest);
  const consumeEntityRequest = useUiStore((s) => s.consumeEntityRequest);

  // Solicitud externa (paleta Ctrl+K): abre el detalle de la factura.
  useEffect(() => {
    if (entityRequest?.kind === "invoice") {
      setDetailId(entityRequest.id);
      consumeEntityRequest();
    }
  }, [entityRequest, consumeEntityRequest]);

  // Contadores reales por estado de cobro (independientes de filtros/búsqueda).
  const counts = useMemo(() => {
    const c: Record<string, number> = {
      TOTAL: 0,
      PENDIENTE: 0,
      ABONADA: 0,
      PAGADA: 0,
      ANULADA: 0,
    };
    for (const i of all ?? []) {
      c.TOTAL += i.count;
      c[i.status] = (c[i.status] ?? 0) + i.count;
    }
    return c;
  }, [all]);

  // Totales del listado visible: facturado (sin anuladas), cobrado y saldo.
  const totals = useMemo(() => {
    let billed = 0;
    let collected = 0;
    let pending = 0;
    for (const i of invoices ?? []) {
      if (i.paymentStatus === "ANULADA") continue;
      billed += i.total ?? 0;
      collected += i.paidAmount ?? 0;
      pending += i.balance ?? 0;
    }
    return { billed, collected, pending };
  }, [invoices]);

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">Facturación</h2>
          <p className="text-muted-foreground text-sm">
            Emisión de facturas con IVA, abonos, métodos de pago y saldos.
          </p>
        </div>
        {canBill && (
          <Button ref={newInvoiceButton} onClick={() => setNewOpen(true)}>
            <Plus className="size-4" />
            Nueva factura
          </Button>
        )}
      </div>

      <div className="grid gap-3 sm:grid-cols-3">
        <Card className="gap-0 p-3">
          <p className="text-muted-foreground text-xs">Facturado (visible)</p>
          <p className="text-lg font-semibold tabular-nums">
            {formatCOP(totals.billed)}
          </p>
        </Card>
        <Card className="gap-0 p-3">
          <p className="text-muted-foreground text-xs">Cobrado</p>
          <p className="text-lg font-semibold tabular-nums">
            {formatCOP(totals.collected)}
          </p>
        </Card>
        <Card className="gap-0 p-3">
          <p className="text-muted-foreground text-xs">Saldo pendiente</p>
          <p className="text-lg font-semibold tabular-nums">
            {formatCOP(totals.pending)}
          </p>
        </Card>
      </div>

      <div className="flex flex-col gap-3 lg:flex-row lg:items-center">
        <div className="flex flex-wrap gap-1 rounded-lg border bg-card p-1">
          {STATUS_TABS.map((tab) => (
            <button
              key={tab.value ?? "all"}
              type="button"
              onClick={() => setStatus(tab.value)}
              className={cn(
                "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                status === tab.value
                  ? "bg-primary text-primary-foreground shadow-sm"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
              )}
            >
              {tab.label}
              {tab.value && (
                <span className="ml-1.5 text-xs opacity-80">
                  {counts[tab.value] ?? 0}
                </span>
              )}
            </button>
          ))}
        </div>

        <div className="relative lg:ml-auto lg:w-72">
          <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Buscar por número, propietario o paciente…"
            className="pl-9"
          />
        </div>
      </div>

      <Card className="gap-0 p-0">
        <CardHeader className="border-b">
          <CardTitle className="flex items-center gap-2 text-base">
            <Receipt className="size-4 text-primary" />
            Facturas
          </CardTitle>
          <CardDescription>
            {isLoading
              ? "Cargando…"
              : `${invoices?.length ?? 0} factura${(invoices?.length ?? 0) === 1 ? "" : "s"} · ${formatCOP(totals.pending)} por cobrar`}
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Nº</TableHead>
                <TableHead>Cliente</TableHead>
                <TableHead>Paciente</TableHead>
                <TableHead>Fecha</TableHead>
                <TableHead className="text-center">Conceptos</TableHead>
                <TableHead className="text-right">Total</TableHead>
                <TableHead className="text-right">Abonado</TableHead>
                <TableHead className="text-right">Saldo</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead className="text-right">Acción</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading &&
                Array.from({ length: 5 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell colSpan={COLUMN_COUNT} className="h-12">
                      <Skeleton className="h-8 w-full" />
                    </TableCell>
                  </TableRow>
                ))}

              {!isLoading && isError && (
                <TableRow>
                  <TableCell
                    colSpan={COLUMN_COUNT}
                    className="text-muted-foreground h-16 text-center"
                  >
                    No se pudieron cargar las facturas.
                  </TableCell>
                </TableRow>
              )}

              {!isLoading && invoices?.length === 0 && (
                <TableRow>
                  <TableCell
                    colSpan={COLUMN_COUNT}
                    className="text-muted-foreground h-24 text-center"
                  >
                    {search
                      ? "Ninguna factura coincide con la búsqueda."
                      : "Sin facturas. Usa “Nueva factura” para emitir la primera."}
                  </TableCell>
                </TableRow>
              )}

              {invoices?.map((i) => {
                const st = PAYMENT_STATUS[i.paymentStatus] ?? {
                  label: i.paymentStatus,
                  variant: "secondary" as const,
                };
                const hasBalance =
                  i.paymentStatus !== "ANULADA" && (i.balance ?? 0) > 0.01;
                return (
                  <TableRow
                    key={i.id}
                    className="cursor-pointer"
                    onClick={() => setDetailId(i.id)}
                  >
                    <TableCell>
                      <span className="font-mono text-sm font-semibold">
                        {i.invoiceNumber}
                      </span>
                    </TableCell>
                    <TableCell>
                      <span className="font-medium">{i.ownerName}</span>
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {i.patientName ?? "—"}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {formatDateTime(i.issueDate)}
                    </TableCell>
                    <TableCell className="text-center text-sm tabular-nums">
                      {i.itemCount}
                    </TableCell>
                    <TableCell className="text-right font-semibold tabular-nums">
                      {formatCOP(i.total ?? 0)}
                    </TableCell>
                    <TableCell className="text-right text-muted-foreground tabular-nums">
                      {formatCOP(i.paidAmount ?? 0)}
                    </TableCell>
                    <TableCell className="text-right tabular-nums">
                      <span
                        className={cn(
                          hasBalance && "text-destructive font-medium",
                        )}
                      >
                        {formatCOP(i.balance ?? 0)}
                      </span>
                    </TableCell>
                    <TableCell>
                      <Badge variant={st.variant}>{st.label}</Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      {hasBalance && canBill ? (
                        <Button
                          size="sm"
                          onClick={(e) => {
                            e.stopPropagation();
                            setPayingId(i.id);
                          }}
                        >
                          <Banknote className="size-3.5" />
                          Cobrar
                        </Button>
                      ) : (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={(e) => {
                            e.stopPropagation();
                            setDetailId(i.id);
                          }}
                        >
                          <Receipt className="size-3.5" />
                          Ver
                        </Button>
                      )}
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <NewInvoiceDialog open={newOpen} onOpenChange={setNewOpen} />
      <InvoiceDetailDialog
        invoiceId={detailId}
        onOpenChange={(open) => !open && setDetailId(null)}
      />
      <PaymentDialog
        invoiceId={payingId}
        onOpenChange={(open) => !open && setPayingId(null)}
      />
    </div>
  );
}
