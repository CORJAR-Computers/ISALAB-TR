import { useEffect, useMemo, useState } from "react";
import { ClipboardList, Plus, Search } from "lucide-react";
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
import { useLabOrderCounts, useLabOrders } from "@/hooks/use-queries";
import { LAB_ORDER_PRIORITY, LAB_ORDER_STATUS } from "@/lib/status";
import { cn, formatDateTime } from "@/lib/utils";
import { usePermissions } from "@/hooks/use-permissions";
import { useUiStore } from "@/stores/ui-store";
import { LabOrderDetailDialog } from "./LabOrderDetailDialog";
import { NewLabOrderDialog } from "./NewLabOrderDialog";

const STATUS_TABS: Array<{ value: string | null; label: string }> = [
  { value: null, label: "Todas" },
  { value: "RECIBIDA", label: "Recibidas" },
  { value: "EN_PROCESO", label: "En proceso" },
  { value: "COMPLETADA", label: "Completadas" },
  { value: "ANULADA", label: "Anuladas" },
];

export function LabOrdersPage() {
  const [status, setStatus] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [newOpen, setNewOpen] = useState(false);
  const [detailId, setDetailId] = useState<number | null>(null);
  const { data: orders, isLoading, isError } = useLabOrders(status, search);
  const { data: all } = useLabOrderCounts();
  const { isVetOrAdmin } = usePermissions();
  const entityRequest = useUiStore((s) => s.entityRequest);
  const consumeEntityRequest = useUiStore((s) => s.consumeEntityRequest);

  // Solicitud externa (desde la ficha de muestra): abre el detalle de la orden.
  useEffect(() => {
    if (entityRequest?.kind === "lab-order") {
      setDetailId(entityRequest.id);
      consumeEntityRequest();
    }
  }, [entityRequest, consumeEntityRequest]);

  // Contadores reales por estado (independientes de filtros/búsqueda).
  const counts = useMemo(() => {
    const c: Record<string, number> = {
      TOTAL: 0,
      RECIBIDA: 0,
      EN_PROCESO: 0,
      COMPLETADA: 0,
      ANULADA: 0,
    };
    for (const row of all ?? []) {
      c.TOTAL += row.count;
      c[row.status] = (c[row.status] ?? 0) + row.count;
    }
    return c;
  }, [all]);

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Órdenes de laboratorio
          </h2>
          <p className="text-muted-foreground text-sm">
            Solicitudes del veterinario: creación, accesionado de muestras y
            seguimiento por estado.
          </p>
        </div>
        {isVetOrAdmin && (
          <Button onClick={() => setNewOpen(true)}>
            <Plus className="size-4" />
            Nueva orden
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
            placeholder="Buscar por código, propietario o paciente…"
            className="pl-9"
          />
        </div>
      </div>

      <Card className="gap-0 p-0">
        <CardHeader className="border-b">
          <CardTitle className="flex items-center gap-2 text-base">
            <ClipboardList className="text-primary size-4" />
            Órdenes
          </CardTitle>
          <CardDescription>
            {isLoading
              ? "Cargando…"
              : `${orders?.length ?? 0} orden${(orders?.length ?? 0) === 1 ? "" : "es"}`}
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Código</TableHead>
                <TableHead>Paciente</TableHead>
                <TableHead>Propietario</TableHead>
                <TableHead>Fecha</TableHead>
                <TableHead className="text-center">Pruebas</TableHead>
                <TableHead>Prioridad</TableHead>
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
                    No se pudieron cargar las órdenes.
                  </TableCell>
                </TableRow>
              )}

              {!isLoading && orders?.length === 0 && (
                <TableRow>
                  <TableCell colSpan={8} className="text-muted-foreground h-24 text-center">
                    {search
                      ? "Ninguna orden coincide con la búsqueda."
                      : "Sin órdenes. Usa “Nueva orden” para crear la primera."}
                  </TableCell>
                </TableRow>
              )}

              {orders?.map((o) => {
                const st = LAB_ORDER_STATUS[o.status] ?? {
                  label: o.status,
                  variant: "secondary" as const,
                };
                const pr = LAB_ORDER_PRIORITY[o.priority];
                return (
                  <TableRow
                    key={o.id}
                    className="cursor-pointer"
                    onClick={() => setDetailId(o.id)}
                  >
                    <TableCell>
                      <span className="font-mono text-sm font-semibold">{o.code}</span>
                    </TableCell>
                    <TableCell>
                      <span className="font-medium">{o.patientName}</span>
                      <span className="text-muted-foreground block text-[11px]">
                        {o.speciesName}
                      </span>
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {o.ownerName}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {formatDateTime(o.requestedAt)}
                    </TableCell>
                    <TableCell className="text-center text-sm tabular-nums">
                      {o.itemCount}
                    </TableCell>
                    <TableCell>
                      {o.priority === "URGENTE" && pr ? (
                        <Badge variant={pr.variant}>{pr.label}</Badge>
                      ) : (
                        <span className="text-muted-foreground text-xs">—</span>
                      )}
                    </TableCell>
                    <TableCell>
                      <Badge variant={st.variant}>{st.label}</Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          setDetailId(o.id);
                        }}
                      >
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

      <NewLabOrderDialog open={newOpen} onOpenChange={setNewOpen} />
      <LabOrderDetailDialog
        orderId={detailId}
        onOpenChange={(open) => !open && setDetailId(null)}
      />
    </div>
  );
}
