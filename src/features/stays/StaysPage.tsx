import { useState } from "react";
import {
  BedDouble,
  LayoutGrid,
  List,
  Pencil,
  Plus,
  Settings2,
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Skeleton } from "@/components/ui/skeleton";
import { QueryErrorState } from "@/components/query-error-state";
import {
  useResourceBoard,
  useResources,
  useStays,
} from "@/hooks/use-queries";
import { usePermissions } from "@/hooks/use-permissions";
import { cn, formatCOP, formatDateTime } from "@/lib/utils";
import { RESOURCE_KIND, STAY_KIND, STAY_STATUS } from "@/lib/status";
import type { Resource } from "@/bindings";
import { NewStayDialog } from "./NewStayDialog";
import { ResourceDialog } from "./ResourceDialog";
import { StayDetailDialog } from "./StayDetailDialog";

type StayFilter = "TODAS" | "ABIERTA" | "CERRADA" | "CANCELADA";

export function StaysPage() {
  const { canStay, isAdmin } = usePermissions();
  const [statusFilter, setStatusFilter] = useState<StayFilter>("TODAS");
  const [kindFilter, setKindFilter] = useState<string>("TODAS");
  const [newStayOpen, setNewStayOpen] = useState(false);
  const [detailId, setDetailId] = useState<number | null>(null);
  const [resourceDialogOpen, setResourceDialogOpen] = useState(false);
  const [editingResource, setEditingResource] = useState<Resource | null>(null);

  const {
    data: board = [],
    isLoading: loadingBoard,
    isError: boardError,
    error: boardErrorObj,
    refetch: refetchBoard,
  } = useResourceBoard();
  const { data: resources = [] } = useResources(true);
  const {
    data: stays = [],
    isLoading: loadingStays,
    isError: staysError,
    error: staysErrorObj,
    refetch: refetchStays,
  } = useStays(
    statusFilter === "TODAS" ? null : statusFilter,
    kindFilter === "TODAS" ? null : kindFilter,
    null,
  );

  const openCount = stays.filter((s) => s.status === "ABIERTA").length;
  const occupiedResources = board.filter((b) => b.occupied > 0).length;

  const openResourceDialog = (resource: Resource | null) => {
    setEditingResource(resource);
    setResourceDialogOpen(true);
  };

  if (!canStay) {
    return (
      <Card>
        <CardContent className="py-10 text-center text-sm text-muted-foreground">
          Tu rol no tiene acceso al módulo de estancia.
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 text-xl font-semibold tracking-tight">
            <BedDouble className="text-primary size-5" />
            Estancia y hospedaje
          </h2>
          <p className="text-muted-foreground text-sm">
            Guardería, hospedaje y hospitalización: ocupación de caniles y
            boxes, bitácora de cuidados y check-out.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {isAdmin && (
            <Button
              type="button"
              variant="outline"
              onClick={() => openResourceDialog(null)}
            >
              <Settings2 className="size-4" />
              Recursos
            </Button>
          )}
          <Button type="button" onClick={() => setNewStayOpen(true)}>
            <Plus className="size-4" />
            Nueva estancia
          </Button>
        </div>
      </div>

      {/* Indicadores */}
      <div className="grid gap-3 sm:grid-cols-3">
        <Card>
          <CardContent className="pt-4">
            <p className="text-muted-foreground text-xs uppercase tracking-wide">
              Estancias abiertas
            </p>
            <p className="text-2xl font-semibold tabular-nums">{openCount}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4">
            <p className="text-muted-foreground text-xs uppercase tracking-wide">
              Recursos ocupados
            </p>
            <p className="text-2xl font-semibold tabular-nums">
              {occupiedResources}
              <span className="text-muted-foreground text-base font-normal">
                {" "}
                / {board.length}
              </span>
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4">
            <p className="text-muted-foreground text-xs uppercase tracking-wide">
              Tarifa abierta estimada
            </p>
            <p className="text-2xl font-semibold tabular-nums">
              {formatCOP(
                stays
                  .filter((s) => s.status === "ABIERTA")
                  .reduce((acc, s) => acc + (s.estimatedTotal ?? 0), 0),
              )}
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Tablero de recursos */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <LayoutGrid className="text-primary size-4" />
            Ocupación de recursos
          </CardTitle>
          <CardDescription>
            Caniles, gateras y boxes con su ocupación actual.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loadingBoard ? (
            <div className="space-y-2">
              <Skeleton className="h-16 w-full" />
              <Skeleton className="h-16 w-full" />
            </div>
          ) : boardError ? (
            <QueryErrorState
              what="la ocupación de recursos"
              compact
              error={boardErrorObj}
              onRetry={() => void refetchBoard()}
            />
          ) : board.length === 0 ? (
            <p className="text-muted-foreground py-6 text-center text-sm">
              No hay recursos registrados. Un administrador puede crearlos
              desde «Recursos».
            </p>
          ) : (
            <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
              {board.map((item) => {
                const full = item.occupied >= item.resource.capacity;
                return (
                  <div
                    key={item.resource.id}
                    className={cn(
                      "flex items-center justify-between gap-3 rounded-lg border px-3 py-2.5",
                      full && "border-warning/50 bg-warning/5",
                    )}
                  >
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium">
                        {item.resource.name}
                      </p>
                      <p className="text-muted-foreground truncate text-xs">
                        {RESOURCE_KIND[item.resource.kind] ?? item.resource.kind}
                        {item.needsVet > 0 && " · requiere valoración"}
                      </p>
                    </div>
                    <div className="flex shrink-0 items-center gap-1.5">
                      <Badge variant={full ? "warning" : "secondary"}>
                        {item.occupied}/{item.resource.capacity}
                      </Badge>
                      {isAdmin && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          onClick={() => openResourceDialog(item.resource)}
                          title="Editar recurso"
                          aria-label={`Editar ${item.resource.name}`}
                        >
                          <Pencil className="size-4" />
                        </Button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Listado de estancias */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-base">
            <List className="text-primary size-4" />
            Estancias
          </CardTitle>
          <CardDescription>
            Historial de ingresos, días alojados y valor estimado.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {/* Filtros */}
          <div className="flex flex-wrap gap-1 rounded-lg border bg-card p-1">
            {(["TODAS", "ABIERTA", "CERRADA", "CANCELADA"] as const).map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => setStatusFilter(s)}
                className={cn(
                  "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                  statusFilter === s
                    ? "bg-primary text-primary-foreground shadow-sm"
                    : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                )}
              >
                {s === "TODAS" ? "Todas" : STAY_STATUS[s].label}
              </button>
            ))}
          </div>

          <div className="flex flex-wrap gap-1 rounded-lg border bg-card p-1">
            {(["TODAS", "GUARDERIA", "HOSPEDAJE", "HOSPITALIZACION"] as const).map(
              (k) => (
                <button
                  key={k}
                  type="button"
                  onClick={() => setKindFilter(k)}
                  className={cn(
                    "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                    kindFilter === k
                      ? "bg-primary text-primary-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                  )}
                >
                  {k === "TODAS" ? "Todos" : STAY_KIND[k].label}
                </button>
              ),
            )}
          </div>

          {loadingStays ? (
            <div className="space-y-2">
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
            </div>
          ) : staysError ? (
            <QueryErrorState
              what="las estancias"
              compact
              error={staysErrorObj}
              onRetry={() => void refetchStays()}
            />
          ) : stays.length === 0 ? (
            <p className="text-muted-foreground py-6 text-center text-sm">
              No hay estancias con esos filtros.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Código</TableHead>
                  <TableHead>Paciente</TableHead>
                  <TableHead>Tipo</TableHead>
                  <TableHead>Recurso</TableHead>
                  <TableHead>Ingreso</TableHead>
                  <TableHead className="text-right">Días</TableHead>
                  <TableHead className="text-right">Estimado</TableHead>
                  <TableHead className="text-center">Estado</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {stays.map((s) => {
                  const statusMeta = STAY_STATUS[s.status] ?? {
                    label: s.status,
                    variant: "secondary" as const,
                  };
                  const kindMeta = STAY_KIND[s.kind] ?? {
                    label: s.kind,
                    variant: "secondary" as const,
                  };
                  return (
                    <TableRow
                      key={s.id}
                      className="cursor-pointer"
                      onClick={() => setDetailId(s.id)}
                    >
                      <TableCell className="font-mono text-xs">{s.code}</TableCell>
                      <TableCell>
                        <span className="font-medium">{s.patientName}</span>
                        <p className="text-muted-foreground text-xs">
                          {s.ownerName}
                        </p>
                      </TableCell>
                      <TableCell>
                        <Badge variant={kindMeta.variant}>{kindMeta.label}</Badge>
                      </TableCell>
                      <TableCell className="text-sm">
                        {s.resourceName ?? "—"}
                      </TableCell>
                      <TableCell className="text-sm">
                        {formatDateTime(s.checkInAt)}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        {s.days}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        {formatCOP(s.estimatedTotal ?? 0)}
                      </TableCell>
                      <TableCell className="text-center">
                        <Badge variant={statusMeta.variant}>
                          {statusMeta.label}
                        </Badge>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Recursos inactivos (solo admin) */}
      {isAdmin && resources.some((r) => !r.active) && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Recursos inactivos</CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="divide-y">
              {resources
                .filter((r) => !r.active)
                .map((r) => (
                  <li
                    key={r.id}
                    className="flex items-center justify-between gap-3 py-2 text-sm"
                  >
                    <span className="text-muted-foreground">
                      {r.name} · {RESOURCE_KIND[r.kind] ?? r.kind}
                    </span>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => openResourceDialog(r)}
                      title="Editar recurso"
                      aria-label={`Editar ${r.name}`}
                    >
                      <Pencil className="size-4" />
                    </Button>
                  </li>
                ))}
            </ul>
          </CardContent>
        </Card>
      )}

      <NewStayDialog open={newStayOpen} onOpenChange={setNewStayOpen} />
      <StayDetailDialog
        open={detailId != null}
        onOpenChange={(open) => !open && setDetailId(null)}
        stayId={detailId}
      />
      <ResourceDialog
        open={resourceDialogOpen}
        onOpenChange={setResourceDialogOpen}
        editing={editingResource}
      />
    </div>
  );
}
