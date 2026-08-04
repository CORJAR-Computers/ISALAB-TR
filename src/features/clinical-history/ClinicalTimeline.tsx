import { useState, type ReactNode } from "react";
import {
  ChevronDown,
  FlaskConical,
  Stethoscope,
  Syringe,
  MessageCircle,
  type LucideIcon,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";
import { cn, formatDate, formatDateTime } from "@/lib/utils";
import { sendWhatsAppMessage } from "@/lib/whatsapp";
import { CONSULTATION_STATUS, RESULT_STATUS, SAMPLE_STATUS } from "@/lib/status";
import type { ClinicalHistory, Consultation, Sample, Vaccine, Patient, Owner } from "@/bindings";

type TimelineItem =
  | { kind: "consultation"; key: string; date: string; data: Consultation }
  | { kind: "vaccine"; key: string; date: string; data: Vaccine }
  | { kind: "sample"; key: string; date: string; data: Sample };

export function ClinicalTimeline({ history }: { history: ClinicalHistory }) {
  const items: TimelineItem[] = [
    ...history.consultations.map((c) => ({
      kind: "consultation" as const,
      key: `c-${c.id}`,
      date: c.consultationDate,
      data: c,
    })),
    ...history.vaccines.map((v) => ({
      kind: "vaccine" as const,
      key: `v-${v.id}`,
      date: v.administeredAt,
      data: v,
    })),
    ...history.samples.map((s) => ({
      kind: "sample" as const,
      key: `s-${s.id}`,
      date: s.receivedAt,
      data: s,
    })),
  ].sort((a, b) => b.date.localeCompare(a.date));

  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const toggle = (key: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  return (
    <Card className="gap-0 p-0">
      <CardHeader className="border-b">
        <CardTitle className="text-base">Línea de tiempo clínica</CardTitle>
        <p className="text-muted-foreground text-sm">
          {items.length} registro{items.length === 1 ? "" : "s"} · orden
          cronológico
        </p>
      </CardHeader>
      <CardContent className="p-6">
        {items.length === 0 ? (
          <p className="text-muted-foreground py-10 text-center text-sm">
            Sin actividad clínica registrada. Crea la primera consulta.
          </p>
        ) : (
          <ol className="relative space-y-0 border-l pl-6">
            {items.map((item) => (
              <li key={item.key} className="relative pb-8 last:pb-0">
                {/* Nodo */}
                <span
                  className={cn(
                    "absolute top-1 -left-7.75 flex size-5 items-center justify-center rounded-full border-2 bg-background",
                    item.kind === "consultation" && "border-primary",
                    item.kind === "vaccine" && "border-success",
                    item.kind === "sample" && "border-warning",
                  )}
                >
                  <TimelineDot kind={item.kind} />
                </span>

                {item.kind === "consultation" && (
                  <ConsultationEntry item={item} patient={history.patient} owner={history.owner} />
                )}
                {item.kind === "vaccine" && (
                  <VaccineEntry item={item} patient={history.patient} owner={history.owner} />
                )}
                {item.kind === "sample" && (
                  <SampleEntry
                    item={item}
                    expanded={expanded.has(item.key)}
                    onToggle={() => toggle(item.key)}
                  />
                )}
              </li>
            ))}
          </ol>
        )}
      </CardContent>
    </Card>
  );
}

function TimelineDot({ kind }: { kind: TimelineItem["kind"] }) {
  const Icon: LucideIcon =
    kind === "consultation"
      ? Stethoscope
      : kind === "vaccine"
        ? Syringe
        : FlaskConical;
  const color =
    kind === "consultation"
      ? "text-primary"
      : kind === "vaccine"
        ? "text-success"
        : "text-warning";
  return <Icon className={cn("size-3", color)} />;
}

function EntryShell({
  icon,
  title,
  date,
  badge,
  children,
}: {
  icon: LucideIcon;
  title: string;
  date: string;
  badge?: ReactNode;
  actions?: ReactNode;
  children?: ReactNode;
}) {
  const Icon = icon;
  return (
    <div className="group">
      <div className="flex flex-wrap items-center gap-2">
        <div className="bg-muted flex size-8 items-center justify-center rounded-lg">
          <Icon className="text-muted-foreground size-4" />
        </div>
        <p className="text-sm font-semibold">{title}</p>
        <span className="text-muted-foreground text-xs">
          {formatDateTime(date)}
        </span>
        {badge}
        {actions && (
          <div className="ml-auto flex items-center gap-2">
            {actions}
          </div>
        )}
      </div>
      {children && (
        <div className="text-muted-foreground mt-1.5 space-y-1 text-sm">
          {children}
        </div>
      )}
    </div>
  );
}

function ConsultationEntry({
  item,
  patient,
  owner,
}: {
  item: Extract<TimelineItem, { kind: "consultation" }>;
  patient: Patient;
  owner: Owner | null;
}) {
  const c = item.data;
  const status = CONSULTATION_STATUS[c.status] ?? {
    label: c.status,
    variant: "secondary" as const,
  };

  const handleWhatsApp = () => {
    if (!owner?.phone || !owner?.fullName) {
      toast.error("Falta información", {
        description: "El propietario no tiene un número de teléfono registrado.",
      });
      return;
    }
    const message = `Hola ${owner.fullName},\n\nTe compartimos desde ISALAB la fórmula médica / resumen de la consulta de tu mascota *${patient.name}*.\n\nPor favor, revisa el archivo adjunto.\n\n¡Gracias por confiar en nosotros!`;
    sendWhatsAppMessage(owner.phone, message);
  };

  return (
    <EntryShell
      icon={Stethoscope}
      title={c.reason || "Consulta"}
      date={c.consultationDate}
      badge={<Badge variant={status.variant}>{status.label}</Badge>}
      actions={
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1.5 text-green-600 hover:bg-green-50 hover:text-green-700 dark:text-green-400 dark:hover:bg-green-900/30"
          onClick={handleWhatsApp}
        >
          <MessageCircle className="size-3.5" />
          WhatsApp
        </Button>
      }
    >
      {c.anamnesis && (
        <p>
          <span className="font-medium">Anamnesis:</span> {c.anamnesis}
        </p>
      )}
      {c.physicalExam && (
        <p>
          <span className="font-medium">Examen físico:</span> {c.physicalExam}
        </p>
      )}
      {c.diagnosis && (
        <p>
          <span className="font-medium">Diagnóstico:</span> {c.diagnosis}
        </p>
      )}
      {c.treatmentPlan && (
        <p>
          <span className="font-medium">Plan:</span> {c.treatmentPlan}
        </p>
      )}
      {c.veterinarianName && (
        <p className="text-xs italic">
          Atendió: {c.veterinarianName}
        </p>
      )}
    </EntryShell>
  );
}

function VaccineEntry({
  item,
  patient,
  owner,
}: {
  item: Extract<TimelineItem, { kind: "vaccine" }>;
  patient: Patient;
  owner: Owner | null;
}) {
  const v = item.data;

  const handleWhatsApp = () => {
    if (!owner?.phone || !owner?.fullName) {
      toast.error("Falta información", {
        description: "El propietario no tiene un número de teléfono registrado.",
      });
      return;
    }
    const message = `Hola ${owner.fullName},\n\nTe compartimos desde ISALAB el certificado de vacunación de tu mascota *${patient.name}* (${v.vaccineName}).\n\nPor favor, revisa el archivo adjunto.\n\n¡Gracias por confiar en nosotros!`;
    sendWhatsAppMessage(owner.phone, message);
  };

  return (
    <EntryShell
      icon={Syringe}
      title={`Vacuna · ${v.vaccineName}`}
      date={v.administeredAt}
      badge={
        v.nextDoseAt ? (
          <Badge variant="warning">
            Refuerzo: {formatDate(v.nextDoseAt)}
          </Badge>
        ) : undefined
      }
      actions={
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1.5 text-green-600 hover:bg-green-50 hover:text-green-700 dark:text-green-400 dark:hover:bg-green-900/30"
          onClick={handleWhatsApp}
        >
          <MessageCircle className="size-3.5" />
          WhatsApp
        </Button>
      }
    >
      {v.lot && (
        <p>
          <span className="font-medium">Lote:</span> {v.lot}
          {v.manufacturer ? ` · ${v.manufacturer}` : ""}
        </p>
      )}
      {v.dose && (
        <p>
          <span className="font-medium">Dosis:</span> {v.dose}
        </p>
      )}
    </EntryShell>
  );
}

function SampleEntry({
  item,
  expanded,
  onToggle,
}: {
  item: Extract<TimelineItem, { kind: "sample" }>;
  expanded: boolean;
  onToggle: () => void;
}) {
  const s = item.data;
  const status = SAMPLE_STATUS[s.status] ?? {
    label: s.status,
    variant: "secondary" as const,
  };
  const pendingResults = s.results.length === 0;

  return (
    <div className="group">
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full flex-wrap items-center gap-2 text-left"
      >
        <div className="bg-muted flex size-8 items-center justify-center rounded-lg">
          <FlaskConical className="text-muted-foreground size-4" />
        </div>
        <p className="text-sm font-semibold">
          Muestra {s.code} · {s.sampleTypeName}
        </p>
        <span className="text-muted-foreground text-xs">
          {formatDateTime(s.receivedAt)}
        </span>
        <Badge variant={status.variant}>{status.label}</Badge>
        {pendingResults && (
          <Badge variant="outline" className="text-muted-foreground">
            Sin resultados
          </Badge>
        )}
        <ChevronDown
          className={cn(
            "text-muted-foreground ml-auto size-4 transition-transform",
            expanded && "rotate-180",
          )}
        />
      </button>

      {s.collectedBy && (
        <p className="text-muted-foreground mt-1 text-xs">
          Recogida por: {s.collectedBy}
          {s.notes ? ` · ${s.notes}` : ""}
        </p>
      )}

      {expanded && (
        <div className="mt-3 overflow-hidden rounded-lg border">
          {s.results.length === 0 ? (
            <p className="text-muted-foreground px-4 py-5 text-center text-sm">
              Aún no se han cargado resultados para esta muestra.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow className="hover:bg-transparent">
                  <TableHead>Analito</TableHead>
                  <TableHead>Resultado</TableHead>
                  <TableHead>Rango de referencia</TableHead>
                  <TableHead className="text-right">Estado</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {s.results.map((r) => {
                  const rs = RESULT_STATUS[r.status] ?? RESULT_STATUS.SIN_RANGO;
                  const range = r.refMin != null && r.refMax != null;
                  return (
                    <TableRow
                      key={r.id}
                      className={cn(
                        r.status === "ALTO" && "bg-warning/10",
                        r.status === "BAJO" && "bg-destructive/10",
                      )}
                    >
                      <TableCell>
                        <span className="font-medium">{r.analyteName}</span>
                        {r.unit && (
                          <span className="text-muted-foreground text-xs">
                            {" "}
                            ({r.unit})
                          </span>
                        )}
                      </TableCell>
                      <TableCell>
                        <span
                          className={cn(
                            "font-mono font-semibold",
                            r.status === "ALTO" && "text-warning",
                            r.status === "BAJO" && "text-destructive",
                          )}
                        >
                          {r.value}
                        </span>
                        {r.unit && (
                          <span className="text-muted-foreground ml-1 text-xs">
                            {r.unit}
                          </span>
                        )}
                      </TableCell>
                      <TableCell className="text-muted-foreground font-mono text-xs">
                        {range ? `${r.refMin} – ${r.refMax}` : "—"}
                      </TableCell>
                      <TableCell className="text-right">
                        <Badge variant={rs.variant}>{rs.label}</Badge>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </div>
      )}
    </div>
  );
}
