import { useState, useMemo } from "react";
import {
  ChevronLeft,
  ChevronRight,
  ScrollText,
  ShieldAlert,
  Search,
  Download,
  Filter,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
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
import { useAuditLog } from "@/hooks/use-queries";
import { useSessionStore } from "@/stores/session-store";

/** Colores semánticos para los tipos de acción de auditoría. */
const ACTION_STYLES: Record<
  string,
  { variant: "default" | "secondary" | "outline" | "destructive"; label: string }
> = {
  LOGIN: { variant: "default", label: "Login" },
  LOGIN_FAILED: { variant: "destructive", label: "Login fallido" },
  LOGOUT: { variant: "secondary", label: "Logout" },
  USER_CREATED: { variant: "default", label: "Usuario creado" },
  PASSWORD_CHANGED: { variant: "secondary", label: "Contraseña cambiada" },
  SETTINGS_CHANGED: { variant: "secondary", label: "Config cambiada" },
  LOGO_IMPORTED: { variant: "outline", label: "Logo importado" },
  SAMPLE_STATUS_CHANGE: { variant: "secondary", label: "Estado muestra" },
  INVOICE_STATUS_CHANGE: { variant: "secondary", label: "Estado factura" },
  CONSULTATION_STATUS_CHANGE: { variant: "secondary", label: "Estado consulta" },
  SURGERY_STATUS_CHANGE: { variant: "secondary", label: "Estado cirugía" },
};

/** Acciones disponibles para el filtro. */
const ACTION_OPTIONS = [
  { value: "LOGIN", label: "Login" },
  { value: "LOGIN_FAILED", label: "Login fallido" },
  { value: "LOGOUT", label: "Logout" },
  { value: "USER_CREATED", label: "Usuario creado" },
  { value: "PASSWORD_CHANGED", label: "Contraseña cambiada" },
  { value: "SETTINGS_CHANGED", label: "Config cambiada" },
  { value: "LOGO_IMPORTED", label: "Logo importado" },
  { value: "SAMPLE_STATUS_CHANGE", label: "Estado muestra" },
  { value: "INVOICE_STATUS_CHANGE", label: "Estado factura" },
  { value: "CONSULTATION_STATUS_CHANGE", label: "Estado consulta" },
  { value: "SURGERY_STATUS_CHANGE", label: "Estado cirugía" },
];

const PAGE_SIZE = 50;

export function AuditLogPage() {
  const [page, setPage] = useState(0);
  const [showFilters, setShowFilters] = useState(false);
  const [filterUsername, setFilterUsername] = useState("");
  const [filterAction, setFilterAction] = useState("");
  const [filterDateFrom, setFilterDateFrom] = useState("");
  const [filterDateTo, setFilterDateTo] = useState("");

  // Build filter params (only send non-empty values)
  const filterParams = useMemo(() => ({
    username: filterUsername || undefined,
    action: filterAction || undefined,
    dateFrom: filterDateFrom || undefined,
    dateTo: filterDateTo || undefined,
  }), [filterUsername, filterAction, filterDateFrom, filterDateTo]);

  const hasActiveFilters = filterUsername || filterAction || filterDateFrom || filterDateTo;

  const { data: entries, isLoading } = useAuditLog(
    page,
    PAGE_SIZE,
    filterParams.username,
    filterParams.action,
    filterParams.dateFrom,
    filterParams.dateTo,
  );
  const session = useSessionStore((s) => s.session);
  const isAdmin = session?.role === "ADMIN";

  const clearFilters = () => {
    setFilterUsername("");
    setFilterAction("");
    setFilterDateFrom("");
    setFilterDateTo("");
    setPage(0);
  };

  const exportCsv = () => {
    if (!entries || entries.length === 0) return;

    const headers = ["#", "Usuario", "Acción", "Detalles", "Fecha"];
    const rows = entries.map((e) => [
      e.id.toString(),
      e.username,
      ACTION_STYLES[e.action]?.label ?? e.action,
      (e.details ?? "—").replace(/"/g, '""'),
      e.createdAt,
    ]);

    const csv = [
      headers.join(","),
      ...rows.map((r) => r.map((c) => `"${c}"`).join(",")),
    ].join("\n");

    const blob = new Blob(["\uFEFF" + csv], { type: "text/csv;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `auditoria-${new Date().toISOString().slice(0, 10)}.csv`;
    link.click();
    URL.revokeObjectURL(url);
  };

  if (!isAdmin) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 py-20">
        <ShieldAlert className="size-10 text-muted-foreground" />
        <p className="text-muted-foreground text-sm">
          Solo los administradores pueden ver el registro de auditoría.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <div className="flex items-start justify-between">
        <div>
          <h2 className="flex items-center gap-2 text-xl font-semibold tracking-tight">
            <ScrollText className="size-5 text-primary" />
            Registro de Auditoría
          </h2>
          <p className="text-muted-foreground text-sm">
            Historial de acciones del sistema: inicios de sesión, cambios de
            contraseña, transiciones de estado y más.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant={showFilters ? "default" : "outline"}
            size="sm"
            onClick={() => setShowFilters(!showFilters)}
            className="gap-1.5"
          >
            <Filter className="size-3.5" />
            Filtros
            {hasActiveFilters && (
              <Badge variant="secondary" className="ml-1 h-5 px-1.5 text-[10px]">
                {[filterUsername, filterAction, filterDateFrom, filterDateTo].filter(Boolean).length}
              </Badge>
            )}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={exportCsv}
            disabled={!entries || entries.length === 0}
            className="gap-1.5"
          >
            <Download className="size-3.5" />
            CSV
          </Button>
        </div>
      </div>

      {/* Panel de filtros */}
      {showFilters && (
        <div className="rounded-lg border bg-muted/30 p-4 space-y-3">
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
            {/* Filtro por usuario */}
            <div className="space-y-1.5">
              <Label className="text-xs text-muted-foreground">Usuario</Label>
              <div className="relative">
                <Search className="absolute left-2.5 top-2.5 size-3.5 text-muted-foreground" />
                <Input
                  placeholder="Buscar usuario..."
                  value={filterUsername}
                  onChange={(e) => {
                    setFilterUsername(e.target.value);
                    setPage(0);
                  }}
                  className="h-8 pl-8 text-xs"
                />
              </div>
            </div>

            {/* Filtro por acción */}
            <div className="space-y-1.5">
              <Label className="text-xs text-muted-foreground">Acción</Label>
              <Select
                value={filterAction}
                onValueChange={(v) => {
                  setFilterAction(v === "ALL" ? "" : v);
                  setPage(0);
                }}
              >
                <SelectTrigger className="h-8 text-xs">
                  <SelectValue placeholder="Todas las acciones" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="ALL">Todas</SelectItem>
                  {ACTION_OPTIONS.map((opt) => (
                    <SelectItem key={opt.value} value={opt.value}>
                      {opt.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Fecha desde */}
            <div className="space-y-1.5">
              <Label className="text-xs text-muted-foreground">Desde</Label>
              <Input
                type="date"
                value={filterDateFrom}
                onChange={(e) => {
                  setFilterDateFrom(e.target.value);
                  setPage(0);
                }}
                className="h-8 text-xs"
              />
            </div>

            {/* Fecha hasta */}
            <div className="space-y-1.5">
              <Label className="text-xs text-muted-foreground">Hasta</Label>
              <Input
                type="date"
                value={filterDateTo}
                onChange={(e) => {
                  setFilterDateTo(e.target.value);
                  setPage(0);
                }}
                className="h-8 text-xs"
              />
            </div>
          </div>

          {hasActiveFilters && (
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={clearFilters}
                className="gap-1 h-7 text-xs"
              >
                <X className="size-3" />
                Limpiar filtros
              </Button>
            </div>
          )}
        </div>
      )}

      {isLoading && !entries ? (
        <Skeleton className="h-60 w-full" />
      ) : (
        <div className="rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-12">#</TableHead>
                <TableHead>Usuario</TableHead>
                <TableHead>Acción</TableHead>
                <TableHead>Detalles</TableHead>
                <TableHead className="w-40">Fecha</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {entries?.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={5}
                    className="text-muted-foreground text-center"
                  >
                    {hasActiveFilters
                      ? "No hay registros que coincidan con los filtros aplicados."
                      : "No hay registros de auditoría."}
                  </TableCell>
                </TableRow>
              ) : (
                entries?.map((e) => {
                  const style = ACTION_STYLES[e.action] ?? {
                    variant: "outline" as const,
                    label: e.action,
                  };
                  return (
                    <TableRow key={e.id}>
                      <TableCell className="text-muted-foreground font-mono text-xs">
                        {e.id}
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {e.username}
                      </TableCell>
                      <TableCell>
                        <Badge variant={style.variant}>{style.label}</Badge>
                      </TableCell>
                      <TableCell className="text-muted-foreground max-w-xs truncate text-xs">
                        {e.details ?? "—"}
                      </TableCell>
                      <TableCell className="text-muted-foreground text-xs whitespace-nowrap">
                        {e.createdAt}
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </div>
      )}

      {/* Paginación */}
      <div className="flex items-center justify-between">
        <p className="text-muted-foreground text-xs">
          Página {page + 1} · {entries?.length ?? 0} registros{hasActiveFilters ? " (filtrados)" : ""}
        </p>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={page === 0 || isLoading}
            onClick={() => setPage((p) => Math.max(0, p - 1))}
          >
            <ChevronLeft className="size-4" />
            Anterior
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={!entries || entries.length < PAGE_SIZE || isLoading}
            onClick={() => setPage((p) => p + 1)}
          >
            Siguiente
            <ChevronRight className="size-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
