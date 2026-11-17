import { Calendar, MapPin, Phone, Sparkles, User } from "lucide-react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { formatAge, formatDate, formatDateTime } from "@/lib/utils";
import { SEX_LABEL, VISIT_STATUS } from "@/lib/status";
import type { Owner, Patient, VisitListItem } from "@/bindings";

export function PatientCard({
  patient,
  owner,
  lastVisit,
  nextGroomingAt,
}: {
  patient: Patient;
  owner: Owner | null;
  /** Visita de recepción más reciente del paciente (fase 5). */
  lastVisit?: VisitListItem | null;
  /** Fecha del próximo turno sugerido de estética (fase 5). */
  nextGroomingAt?: string | null;
}) {
  return (
    <Card>
      <CardHeader className="border-b">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="flex items-center gap-4">
            <div className="bg-primary text-primary-foreground flex size-14 shrink-0 items-center justify-center rounded-2xl text-xl font-bold uppercase shadow-sm">
              {patient.name.slice(0, 1)}
            </div>
            <div>
              <CardTitle className="flex flex-wrap items-center gap-2 text-xl">
                {patient.name}
                <Badge variant="secondary">{patient.id}</Badge>
                {!patient.active && <Badge variant="destructive">Inactivo</Badge>}
              </CardTitle>
              <CardDescription className="mt-1">
                {patient.speciesName}
                {patient.breedName ? ` · ${patient.breedName}` : ""} ·{" "}
                {SEX_LABEL[patient.sex] ?? patient.sex}
                {patient.neutered ? " · Esterilizado" : ""} ·{" "}
                {formatAge(patient.birthDate)}
              </CardDescription>
            </div>
          </div>
        </div>
      </CardHeader>

      <CardContent>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <div>
            <p className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
              Nacimiento
            </p>
            <p className="mt-1 flex items-center gap-1.5 text-sm">
              <Calendar className="text-muted-foreground size-3.5" />
              {formatDate(patient.birthDate)}
            </p>
          </div>
          <div>
            <p className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
              Color / señas
            </p>
            <p className="mt-1 text-sm">{patient.color || "—"}</p>
          </div>
          <div>
            <p className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
              Microchip
            </p>
            <p className="mt-1 font-mono text-sm">{patient.microchip || "—"}</p>
          </div>
          <div>
            <p className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
              Propietario
            </p>
            <p className="mt-1 flex items-center gap-1.5 text-sm font-medium">
              <User className="text-muted-foreground size-3.5" />
              {owner?.fullName ?? patient.ownerName}
            </p>
          </div>
        </div>

        {/* Resumen de recepción/estética (fase 5) */}
        {(lastVisit || nextGroomingAt) && (
          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            <div>
              <p className="text-muted-foreground flex items-center gap-1.5 text-xs font-medium tracking-wide uppercase">
                <Calendar className="size-3" />
                Última visita
              </p>
              {lastVisit ? (
                <p className="mt-1 flex flex-wrap items-center gap-2 text-sm">
                  <span className="font-mono text-xs font-semibold">
                    {lastVisit.code}
                  </span>
                  <span className="text-muted-foreground text-xs">
                    {formatDateTime(lastVisit.arrivedAt)}
                  </span>
                  <Badge
                    variant={
                      VISIT_STATUS[lastVisit.status]?.variant ?? "secondary"
                    }
                  >
                    {VISIT_STATUS[lastVisit.status]?.label ?? lastVisit.status}
                  </Badge>
                </p>
              ) : (
                <p className="mt-1 text-sm text-muted-foreground">—</p>
              )}
            </div>
            <div>
              <p className="text-muted-foreground flex items-center gap-1.5 text-xs font-medium tracking-wide uppercase">
                <Sparkles className="size-3" />
                Próximo turno de estética
              </p>
              <p className="mt-1 text-sm">
                {nextGroomingAt ? formatDate(nextGroomingAt) : "—"}
              </p>
            </div>
          </div>
        )}

        {owner && (
          <div className="bg-muted/50 mt-4 flex flex-wrap gap-x-6 gap-y-1 rounded-lg px-3 py-2 text-xs text-muted-foreground">
            <span>
              {owner.documentType} {owner.documentNumber}
            </span>
            {owner.phone && (
              <span className="flex items-center gap-1">
                <Phone className="size-3" /> {owner.phone}
              </span>
            )}
            {owner.email && <span>{owner.email}</span>}
            {(owner.address || owner.city) && (
              <span className="flex items-center gap-1">
                <MapPin className="size-3" />
                {[owner.address, owner.city].filter(Boolean).join(", ")}
              </span>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
