import { useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  ScrollText,
  ShieldAlert,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
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

const PAGE_SIZE = 50;

export function AuditLogPage() {
  const [page, setPage] = useState(0);
  const { data: entries, isLoading } = useAuditLog(page, PAGE_SIZE);
  const session = useSessionStore((s) => s.session);
  const isAdmin = session?.role === "ADMIN";

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
                    No hay registros de auditoría.
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
          Página {page + 1} · {PAGE_SIZE} registros por página
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
