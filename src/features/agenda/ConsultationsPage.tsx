import { useMemo, useState } from "react";
import { toast } from "sonner";
import {
  Ban,
  CalendarClock,
  CheckCircle2,
  HeartPulse,
  Loader2,
  Plus,
  Search,
  Stethoscope,
} from "lucide-react";
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
import {
  useConsultationCounts,
  useConsultations,
  useSetConsultationStatus,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { CONSULTATION_STATUS } from "@/lib/status";
import { cn, formatDateTime } from "@/lib/utils";
import { useUiStore } from "@/stores/ui-store";
import { usePermissions } from "@/hooks/use-permissions";
import { NewConsultationDialog } from "@/features/clinical-history/NewConsultationDialog";

const STATUS_TABS: Array<{ value: string | null; label: string }> = [
  { value: null, label: "Todas" },
  { value: "PENDIENTE", label: "Pendientes" },
  { value: "COMPLETADA", label: "Completadas" },
  { value: "CANCELADA", label: "Canceladas" },
];

export function ConsultationsPage() {
  const [status, setStatus] = useState<string | null>("PENDIENTE");
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const navigate = useUiStore((s) => s.navigate);
  const setActivePatient = useUiStore((s) => s.setActivePatient);
  const { isVetOrAdmin } = usePermissions();

  const { data: consultations, isLoading, isError } = useConsultations(
    status,
    search,
  );
  const { data: all } = useConsultationCounts();
  const setStatusMutation = useSetConsultationStatus();

  // Contadores reales por estado (independientes de filtros/búsqueda).
  const counts = useMemo(() => {
    const c: Record<string, number> = {
      TOTAL: 0,
      PENDIENTE: 0,
      COMPLETADA: 0,
      CANCELADA: 0,
    };
    for (const item of all ?? []) {
      c.TOTAL += 1;
      c[item.status] = (c[item.status] ?? 0) + 1;
    }
    return c;
  }, [all]);

  const openHistory = (patientId: number) => {
    setActivePatient(patientId);
    navigate("clinical-history");
  };

  const changeStatus = async (id: number, next: string, label: string) => {
    try {
      await setStatusMutation.mutateAsync({ id, status: next });
      toast.success(`Consulta ${label.toLowerCase()}`);
    } catch (e) {
      toast.error("No se pudo actualizar la consulta", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Agenda de consultas
          </h2>
          <p className="text-muted-foreground text-sm">
            Consultas ambulatorias programadas: completa o cancela las citas
            pendientes y consulta el historial del paciente.
          </p>
        </div>
        {isVetOrAdmin && (
          <Button onClick={() => setDialogOpen(true)}>
            <Plus className="size-4" />
            Nueva consulta
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
            placeholder="Buscar por paciente, propietario o motivo…"
            className="pl-9"
          />
        </div>
      </div>

      <Card className="gap-0 p-0">
        <CardHeader className="border-b">
          <CardTitle className="flex items-center gap-2 text-base">
            <CalendarClock className="size-4 text-primary" />
            Consultas ambulatorias
          </CardTitle>
          <CardDescription>
            {isLoading
              ? "Cargando…"
              : `${consultations?.length ?? 0} consulta${(consultations?.length ?? 0) === 1 ? "" : "s"}`}
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Paciente</TableHead>
                <TableHead>Motivo</TableHead>
                <TableHead>Fecha</TableHead>
                <TableHead>Veterinario</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead className="text-right">Acciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading &&
                Array.from({ length: 5 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell colSpan={6} className="h-12">
                      <Skeleton className="h-8 w-full" />
                    </TableCell>
                  </TableRow>
                ))}

              {!isLoading && isError && (
                <TableRow>
                  <TableCell colSpan={6} className="text-muted-foreground h-16 text-center">
                    No se pudo cargar la agenda de consultas.
                  </TableCell>
                </TableRow>
              )}

              {!isLoading && consultations?.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} className="text-muted-foreground h-24 text-center">
                    <div className="flex flex-col items-center gap-2">
                      <Stethoscope className="size-6 opacity-60" />
                      {search
                        ? "Ninguna consulta coincide con la búsqueda."
                        : status === "PENDIENTE"
                          ? "No hay consultas pendientes. Programa una con “Nueva consulta”."
                          : "No hay consultas en este estado."}
                    </div>
                  </TableCell>
                </TableRow>
              )}

              {consultations?.map((c) => {
                const st = CONSULTATION_STATUS[c.status] ?? {
                  label: c.status,
                  variant: "secondary" as const,
                };
                const busy = setStatusMutation.isPending;
                return (
                  <TableRow key={c.id}>
                    <TableCell>
                      <div className="flex flex-col">
                        <span className="font-medium">{c.patientName}</span>
                        <span className="text-muted-foreground text-xs">
                          {c.speciesName} · {c.ownerName}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell className="max-w-56">
                      <span className="block truncate">{c.reason || "—"}</span>
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {formatDateTime(c.consultationDate)}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {c.veterinarianName ?? "—"}
                    </TableCell>
                    <TableCell>
                      <Badge variant={st.variant}>{st.label}</Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex items-center justify-end gap-1">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => openHistory(c.patientId)}
                          title="Ver historial clínico"
                        >
                          <HeartPulse className="size-3.5" />
                          Historial
                        </Button>
                        {c.status === "PENDIENTE" && (
                          <>
                            <Button
                              variant="outline"
                              size="sm"
                              className="text-success"
                              disabled={busy}
                              onClick={() =>
                                changeStatus(c.id, "COMPLETADA", "Completada")
                              }
                            >
                              {busy ? (
                                <Loader2 className="size-3.5 animate-spin" />
                              ) : (
                                <CheckCircle2 className="size-3.5" />
                              )}
                              Completar
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              className="text-destructive"
                              disabled={busy}
                              onClick={() =>
                                changeStatus(c.id, "CANCELADA", "Cancelada")
                              }
                            >
                              <Ban className="size-3.5" />
                              Cancelar
                            </Button>
                          </>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <NewConsultationDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        patientId={null}
      />
    </div>
  );
}
