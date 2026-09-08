import { useState } from "react";
import { ClipboardList, FlaskConical, Search, Stethoscope, Syringe } from "lucide-react";
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
import { Badge } from "@/components/ui/badge";
import { useClinicalHistory, usePatientLabOrders, usePatients } from "@/hooks/use-queries";
import { LAB_ORDER_STATUS } from "@/lib/status";
import { formatDateTime } from "@/lib/utils";
import { useUiStore } from "@/stores/ui-store";
import { NewVaccineDialog } from "@/features/vaccines/NewVaccineDialog";
import { PatientCard } from "./PatientCard";
import { ClinicalTimeline } from "./ClinicalTimeline";
import { NewConsultationDialog } from "./NewConsultationDialog";
import { NewSampleDialog } from "./NewSampleDialog";
import { PatientTrendsChart } from "./PatientTrendsChart";
import { usePermissions } from "@/hooks/use-permissions";

export function ClinicalHistoryPage() {
  const activePatientId = useUiStore((s) => s.activePatientId);
  const setActivePatient = useUiStore((s) => s.setActivePatient);

  const [search, setSearch] = useState("");
  const [consultOpen, setConsultOpen] = useState(false);
  const [sampleOpen, setSampleOpen] = useState(false);
  const [vaccineOpen, setVaccineOpen] = useState(false);
  const { isVetOrAdmin } = usePermissions();

  const { data: patients = [], isLoading: loadingPatients } =
    usePatients(search);
  const { data: history, isLoading: loadingHistory } =
    useClinicalHistory(activePatientId);

  // ---- Sin paciente seleccionado: selector compacto ----
  if (!activePatientId) {
    return (
      <div className="space-y-5">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Historial Clínico
          </h2>
          <p className="text-muted-foreground text-sm">
            Selecciona un paciente para ver su ficha y línea de tiempo clínica.
          </p>
        </div>

        <div className="relative max-w-sm">
          <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Buscar paciente…"
            className="pl-9"
            autoFocus
          />
        </div>

        <Card className="gap-0 p-0">
          <CardContent className="p-2">
            {loadingPatients &&
              Array.from({ length: 4 }).map((_, i) => (
                <Skeleton key={i} className="m-1 h-11 w-full" />
              ))}
            {!loadingPatients && patients.length === 0 && (
              <p className="text-muted-foreground px-3 py-8 text-center text-sm">
                {search
                  ? "Sin coincidencias."
                  : "No hay pacientes. Regístralos desde Pacientes → Nuevo paciente."}
              </p>
            )}
            {patients.map((p) => (
              <button
                key={p.id}
                type="button"
                onClick={() => setActivePatient(p.id)}
                className="hover:bg-accent flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors"
              >
                <div className="bg-muted text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-full text-sm font-bold uppercase">
                  {p.name.slice(0, 1)}
                </div>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium">{p.name}</p>
                  <p className="text-muted-foreground truncate text-xs">
                    {p.speciesName}
                    {p.breedName ? ` · ${p.breedName}` : ""} · {p.ownerName}
                  </p>
                </div>
                <Badge variant="secondary">{p.id}</Badge>
              </button>
            ))}
          </CardContent>
        </Card>
      </div>
    );
  }

  if (loadingHistory) {
    return (
      <div className="space-y-5">
        <Skeleton className="h-40 w-full" />
        <Skeleton className="h-80 w-full" />
      </div>
    );
  }

  if (!history) {
    return (
      <p className="text-muted-foreground py-10 text-center">
        No se encontró el historial del paciente.
      </p>
    );
  }

  return (
    <div className="space-y-5">
      {/* Acciones */}
      <div className="flex flex-wrap items-center gap-2">
        {isVetOrAdmin && (
          <Button variant="default" onClick={() => setConsultOpen(true)}>
            <Stethoscope className="size-4" />
            Nueva consulta
          </Button>
        )}
        {isVetOrAdmin && (
          <Button variant="outline" onClick={() => setSampleOpen(true)}>
            <FlaskConical className="size-4" />
            Registrar muestra
          </Button>
        )}
        {isVetOrAdmin && (
          <Button variant="outline" onClick={() => setVaccineOpen(true)}>
            <Syringe className="size-4" />
            Registrar vacuna
          </Button>
        )}
        <Button
          variant="ghost"
          size="sm"
          className="ml-auto"
          onClick={() => setActivePatient(null)}
        >
          ← Cambiar paciente
        </Button>
      </div>

      <PatientCard
        patient={history.patient}
        owner={history.owner}
      />

      <PatientTrendsChart patientId={activePatientId} />

      <PatientLabOrdersSection patientId={activePatientId} />

      <ClinicalTimeline history={history} />

      <NewConsultationDialog
        open={consultOpen}
        onOpenChange={setConsultOpen}
        patientId={activePatientId}
      />
      <NewSampleDialog
        open={sampleOpen}
        onOpenChange={setSampleOpen}
        patientId={activePatientId}
      />
      <NewVaccineDialog
        open={vaccineOpen}
        onOpenChange={setVaccineOpen}
        patientId={activePatientId}
      />
    </div>
  );
}

/** Últimas órdenes de laboratorio del paciente, con foco en las pendientes. */
function PatientLabOrdersSection({ patientId }: { patientId: number }) {
  const { data: orders = [], isLoading } = usePatientLabOrders(patientId);
  const navigate = useUiStore((s) => s.navigate);
  const requestEntity = useUiStore((s) => s.requestEntity);

  // Pendientes primero (RECIBIDA/EN_PROCESO), luego por fecha descendente.
  const sorted = [...orders].sort((a, b) => {
    const pending = (s: string) => (s === "RECIBIDA" || s === "EN_PROCESO" ? 0 : 1);
    if (pending(a.status) !== pending(b.status)) return pending(a.status) - pending(b.status);
    return b.requestedAt.localeCompare(a.requestedAt);
  });
  const shown = sorted.slice(0, 5);
  const pendingCount = orders.filter(
    (o) => o.status === "RECIBIDA" || o.status === "EN_PROCESO",
  ).length;

  return (
    <Card className="gap-0 p-0">
      <CardHeader className="border-b">
        <CardTitle className="flex items-center gap-2 text-base">
          <ClipboardList className="size-4 text-primary" />
          Órdenes de laboratorio
          {pendingCount > 0 && (
            <Badge variant="warning" className="ml-1">
              {pendingCount} pendiente{pendingCount === 1 ? "" : "s"}
            </Badge>
          )}
        </CardTitle>
        <CardDescription>
          Solicitudes del veterinario y su estado de accesionado/proceso.
        </CardDescription>
      </CardHeader>
      <CardContent className="p-3">
        {isLoading && <Skeleton className="h-16 w-full" />}
        {!isLoading && shown.length === 0 && (
          <p className="text-muted-foreground py-3 text-center text-sm">
            Sin órdenes de laboratorio registradas para este paciente.
          </p>
        )}
        <div className="space-y-1.5">
          {shown.map((o) => {
            const st = LAB_ORDER_STATUS[o.status] ?? {
              label: o.status,
              variant: "secondary" as const,
            };
            return (
              <button
                key={o.id}
                type="button"
                onClick={() => {
                  navigate("lab-orders");
                  requestEntity("lab-order", o.id);
                }}
                className="hover:bg-accent flex w-full items-center gap-3 rounded-lg px-2 py-2 text-left transition-colors"
              >
                <span className="font-mono text-sm font-semibold">{o.code}</span>
                <Badge variant={st.variant}>{st.label}</Badge>
                {o.priority === "URGENTE" && <Badge variant="destructive">Urgente</Badge>}
                <span className="text-muted-foreground ml-auto text-xs">
                  {o.itemCount} prueba{o.itemCount === 1 ? "" : "s"} ·{" "}
                  {formatDateTime(o.requestedAt)}
                </span>
              </button>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
