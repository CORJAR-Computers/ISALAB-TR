import { useState } from "react";
import { FlaskConical, Search, Stethoscope, Syringe } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { useClinicalHistory, usePatients } from "@/hooks/use-queries";
import { useUiStore } from "@/stores/ui-store";
import { NewVaccineDialog } from "@/features/vaccines/NewVaccineDialog";
import { PatientCard } from "./PatientCard";
import { ClinicalTimeline } from "./ClinicalTimeline";
import { NewConsultationDialog } from "./NewConsultationDialog";
import { NewSampleDialog } from "./NewSampleDialog";
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
        <Button variant="outline" onClick={() => setSampleOpen(true)}>
          <FlaskConical className="size-4" />
          Registrar muestra
        </Button>
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
