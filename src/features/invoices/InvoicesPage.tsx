import { useEffect, useMemo, useState } from "react";
import { Plus, Receipt, Search } from "lucide-react";
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
import { INVOICE_STATUS, PAYMENT_METHOD_LABEL } from "@/lib/status";
import { cn, formatCOP, formatDateTime } from "@/lib/utils";
import { InvoiceDetailDialog } from "./InvoiceDetailDialog";
import { NewInvoiceDialog } from "./NewInvoiceDialog";
import { usePermissions } from "@/hooks/use-permissions";
import { useUiStore } from "@/stores/ui-store";

const STATUS_TABS: Array<{ value: string | null; label: string }> = [
  { value: null, label: "Todas" },
  { value: "EMITIDA", label: "Emitidas" },
  { value: "PAGADA", label: "Pagadas" },
  { value: "ANULADA", label: "Anuladas" },
];

export function InvoicesPage() {
  const [status, setStatus] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [newOpen, setNewOpen] = useState(false);
  const [detailId, setDetailId] = useState<number | null>(null);
  const { data: invoices, isLoading, isError } = useInvoices(status, search);
  const { data: all } = useInvoiceCounts();
  const { isVetOrAdmin } = usePermissions();
  const entityRequest = useUiStore((s) => s.entityRequest);
  const consumeEntityRequest = useUiStore((s) => s.consumeEntityRequest);

  // Solicitud externa (paleta Ctrl+K): abre el detalle de la factura.
  useEffect(() => {
    if (entityRequest?.kind === "invoice") {
      setDetailId(entityRequest.id);
      consumeEntityRequest();
    }
  }, [entityRequest, consumeEntityRequest]);

  // Contadores reales por estado (independientes de filtros/búsqueda).
  const counts = useMemo(() => {
    const c: Record<string, number> = {
      TOTAL: 0,
      EMITIDA: 0,
      PAGADA: 0,
      ANULADA: 0,
    };
    for (const i of all ?? []) {
      c.TOTAL += 1;
      c[i.status] = (c[i.status] ?? 0) + 1;
    }
    return c;
  }, [all]);

  const paidTotal = useMemo(
    () =>
      (all ?? [])
        .filter((i) => i.status === "PAGADA")
        .reduce((acc, i) => acc + (i.total ?? 0), 0),
    [all],
  );

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Facturación
          </h2>
          <p className="text-muted-foreground text-sm">
            Emisión de facturas con IVA, métodos de pago y estados de cobro.
          </p>
        </div>
        {isVetOrAdmin && (
          <Button onClick={() => setNewOpen(true)}>
            <Plus className="size-4" />
            Nueva factura
          </Button>
        )}
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
              : `${invoices?.length ?? 0} factura${(invoices?.length ?? 0) === 1 ? "" : "s"} · ${formatCOP(paidTotal)} cobrados`}
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
                <TableHead>Estado</TableHead>
                <TableHead className="text-right">Acción</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading &&
                Array.from({ length: 5 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell colSpan={8} className="h-12">
                      <Skeleton className="h-8 w-full" />
                    </TableCell>
                  </TableRow>
                ))}

              {!isLoading && isError && (
                <TableRow>
                  <TableCell colSpan={8} className="text-muted-foreground h-16 text-center">
                    No se pudieron cargar las facturas.
                  </TableCell>
                </TableRow>
              )}

              {!isLoading && invoices?.length === 0 && (
                <TableRow>
                  <TableCell colSpan={8} className="text-muted-foreground h-24 text-center">
                    {search
                      ? "Ninguna factura coincide con la búsqueda."
                      : "Sin facturas. Usa “Nueva factura” para emitir la primera."}
                  </TableCell>
                </TableRow>
              )}

              {invoices?.map((i) => {
                const st = INVOICE_STATUS[i.status] ?? {
                  label: i.status,
                  variant: "secondary" as const,
                };
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
                    <TableCell>
                      <div className="flex flex-col gap-0.5">
                        <Badge variant={st.variant}>{st.label}</Badge>
                        {i.paymentMethod && (
                          <span className="text-muted-foreground text-[11px]">
                            {PAYMENT_METHOD_LABEL[i.paymentMethod] ??
                              i.paymentMethod}
                          </span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="text-right">
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
    </div>
  );
}
