import { useEffect, useMemo, useState } from "react";
import { ClipboardPlus, DoorOpen, Search } from "lucide-react";
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
import { Skeleton } from "@/components/ui/skeleton";
import {
  useSetVisitStatus,
  useVisits,
} from "@/hooks/use-queries";
import { usePermissions } from "@/hooks/use-permissions";
import { getErrorMessage } from "@/lib/api";
import { VISIT_STATUS } from "@/lib/status";
import { cn, formatCOP } from "@/lib/utils";
import { toast } from "sonner";
import { PatientQuickCreateButton } from "@/features/patients/PatientQuickCreateButton";
import { NewVisitDialog } from "./NewVisitDialog";
import { VisitDetailDialog } from "./VisitDetailDialog";
import type { Patient, VisitListItem } from "@/bindings";

/** Pestañas de la cola: estado + etiqueta. */
const QUEUE_TABS: Array<{ status: string | null; label: string }> = [
  { status: null, label: "Todas" },
  { status: "EN_ESPERA", label: "En espera" },
  { status: "EN_ATENCION", label: "En atención" },
  { status: "FINALIZADA", label: "Finalizadas" },
  { status: "FACTURADA", label: "Facturadas" },
  { status: "CANCELADA", label: "Canceladas" },
];

export function ReceptionPage() {
  const today = useMemo(() => new Date().toISOString().slice(0, 10), []);
  const [dateFilter, setDateFilter] = useState<string>(today);
  const [statusFilter, setStatusFilter] = useState<string | null>("EN_ESPERA");
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [newVisitOpen, setNewVisitOpen] = useState(false);
  const [presetPatient, setPresetPatient] = useState<Patient | null>(null);
  const [detailVisit, setDetailVisit] = useState<VisitListItem | null>(null);
  const { canReceive } = usePermissions();
  const setStatus = useSetVisitStatus();

  useEffect(() => {
    const t = setTimeout(() => setDebouncedSearch(search.trim()), 250);
    return () => clearTimeout(t);
  }, [search]);

  const { data: visits, isLoading } = useVisits(
    dateFilter || null,
    statusFilter,
    debouncedSearch,
    canReceive,
  );

  if (!canReceive) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 py-20">
        <DoorOpen className="size-10 text-muted-foreground" />
        <p className="text-muted-foreground text-sm">
          Solo el personal de recepción, auxiliares, veterinarios y
          administradores pueden gestionar la cola de visitas.
        </p>
      </div>
    );
  }

  const startAttention = async (visit: VisitListItem) => {
    try {
      await setStatus.mutateAsync({ id: visit.id, status: "EN_ATENCION" });
      toast.success("Visita en atención", { description: visit.code });
    } catch (e) {
      toast.error("Operación rechazada", { description: getErrorMessage(e) });
    }
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Recepción de pacientes
          </h2>
          <p className="text-muted-foreground text-sm">
            Check-in de pacientes, cola de espera y cobro de la visita.
          </p>
        </div>
        <Button onClick={() => setNewVisitOpen(true)}>
          <ClipboardPlus className="size-4" />
          Registrar visita
        </Button>
      </div>

      <div className="grid gap-5 lg:grid-cols-3">
        {/* Columna izquierda: acceso rápido */}
        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Acceso rápido</CardTitle>
              <CardDescription>
                Busca al paciente o ve a su registro completo.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="relative">
                <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                <Input
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="Buscar en la cola (código, paciente, propietario)…"
                  className="pl-9"
                />
              </div>
              <div className="grid gap-2 sm:grid-cols-2">
                <Button variant="outline" onClick={() => setNewVisitOpen(true)}>
                  <ClipboardPlus className="size-4" />
                  Nueva visita
                </Button>
                <PatientQuickCreateButton
                  onCreated={(patient) => {
                    toast.success("Paciente registrado", {
                      description: `${patient.name} · listo para su visita`,
                    });
                    // Se queda en Recepción y abre la visita con el paciente
                    // ya seleccionado (sin perder el contexto del módulo).
                    setPresetPatient(patient);
                    setNewVisitOpen(true);
                  }}
                />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">Resumen del día</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 gap-2 text-sm">
                {QUEUE_TABS.slice(1).map((tab) => {
                  const count = (visits ?? []).filter(
                    (v) => v.status === tab.status,
                  ).length;
                  return (
                    <div
                      key={tab.status}
                      className="flex items-center justify-between rounded-lg border px-3 py-2"
                    >
                      <span className="text-muted-foreground">
                        {tab.label}
                      </span>
                      <Badge variant="secondary">{count}</Badge>
                    </div>
                  );
                })}
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Columna derecha: cola de espera */}
        <div className="space-y-4 lg:col-span-2">
          <div className="flex flex-wrap items-center gap-2">
            <div className="flex flex-wrap gap-1 rounded-lg border bg-card p-1">
              {QUEUE_TABS.map((tab) => (
                <button
                  key={tab.label}
                  type="button"
                  onClick={() => setStatusFilter(tab.status)}
                  className={cn(
                    "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                    statusFilter === tab.status
                      ? "bg-primary text-primary-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                  )}
                >
                  {tab.label}
                </button>
              ))}
            </div>
            <Input
              type="date"
              value={dateFilter}
              onChange={(e) => setDateFilter(e.target.value)}
              className="ml-auto w-44"
              aria-label="Filtrar por fecha de llegada"
            />
          </div>

          {isLoading && (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-28 w-full" />
              ))}
            </div>
          )}

          {!isLoading && (visits ?? []).length === 0 && (
            <Card>
              <CardContent className="text-muted-foreground py-10 text-center text-sm">
                No hay visitas con estos filtros. Registra una con "Registrar
                visita".
              </CardContent>
            </Card>
          )}

          <div className="grid gap-3">
            {(visits ?? []).map((visit) => {
              const st = VISIT_STATUS[visit.status];
              return (
                <button
                  key={visit.id}
                  type="button"
                  onClick={() => setDetailVisit(visit)}
                  className={cn(
                    "group w-full rounded-xl border bg-card p-4 text-left transition-all hover:border-primary/40 hover:shadow-md",
                    visit.priority === "URGENTE" &&
                      visit.status !== "FACTURADA" &&
                      visit.status !== "CANCELADA" &&
                      "border-destructive/40",
                  )}
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-mono text-xs text-muted-foreground">
                      {visit.code}
                    </span>
                    <Badge variant={st?.variant ?? "outline"}>
                      {st?.label ?? visit.status}
                    </Badge>
                    {visit.priority === "URGENTE" && (
                      <Badge variant="destructive">Urgente</Badge>
                    )}
                    <span className="text-muted-foreground ml-auto text-xs">
                      {visit.arrivedAt.slice(11, 16)}
                    </span>
                  </div>
                  <p className="mt-1.5 font-semibold">
                    {visit.patientName}
                    <span className="text-muted-foreground ml-2 text-sm font-normal">
                      · {visit.ownerName}
                    </span>
                  </p>
                  <div className="mt-2 flex flex-wrap items-center gap-2">
                    <span className="text-muted-foreground text-sm">
                      {visit.servicesSummary || "Sin servicios"}
                    </span>
                    <span className="ml-auto font-medium tabular-nums">
                      {formatCOP(visit.estimatedTotal ?? 0)}
                    </span>
                  </div>
                  {visit.status === "EN_ESPERA" && (
                    <div className="mt-3 flex justify-end">
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={(e) => {
                          e.stopPropagation();
                          void startAttention(visit);
                        }}
                      >
                        Iniciar atención
                      </Button>
                    </div>
                  )}
                </button>
              );
            })}
          </div>
        </div>
      </div>

      <NewVisitDialog
        open={newVisitOpen}
        onOpenChange={(open) => {
          setNewVisitOpen(open);
          if (!open) setPresetPatient(null);
        }}
        presetPatient={presetPatient}
      />
      <VisitDetailDialog
        visitSummary={detailVisit}
        onClose={() => setDetailVisit(null)}
      />
    </div>
  );
}
