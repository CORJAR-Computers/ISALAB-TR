import { useState } from "react";
import type { ReactNode } from "react";
import type { UseMutationResult } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  ClipboardList,
  FileText,
  HandHeart,
  Loader2,
  Receipt,
  Scissors,
  Search,
  Syringe,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  useConsultations,
  useGenerateCarnetVacunacion,
  useGenerateCertificadoCirugia,
  useGenerateConsentimiento,
  useGenerateFormulaMedica,
  useGenerateReciboInvoice,
  useGenerateReport,
  useInvoices,
  usePatients,
  useSamples,
  useSurgeries,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { formatDateTime } from "@/lib/utils";
import type { ReportFile } from "@/bindings";

type ReportKind =
  | "lab"
  | "formula"
  | "consent"
  | "recibo"
  | "cirugia"
  | "carnet";

const TABS: {
  value: ReportKind;
  label: string;
  icon: typeof FileText;
  description: string;
  emptyText: string;
  placeholder: string;
}[] = [
  {
    value: "lab",
    label: "Laboratorio",
    icon: FileText,
    description:
      "Informe de resultados analíticos de una muestra finalizada.",
    emptyText: "No hay muestras finalizadas que coincidan.",
    placeholder: "Buscar muestra finalizada…",
  },
  {
    value: "formula",
    label: "Fórmula",
    icon: ClipboardList,
    description: "Fórmula médica (receta) de una consulta.",
    emptyText: "No hay consultas que coincidan.",
    placeholder: "Buscar consulta (paciente, motivo)…",
  },
  {
    value: "consent",
    label: "Consentimiento",
    icon: HandHeart,
    description: "Consentimiento informado de una cirugía programada.",
    emptyText: "No hay cirugías que coincidan.",
    placeholder: "Buscar cirugía (paciente, tipo)…",
  },
  {
    value: "recibo",
    label: "Recibo",
    icon: Receipt,
    description: "Comprobante de pago de una factura emitida.",
    emptyText: "No hay facturas que coincidan.",
    placeholder: "Buscar factura (número, propietario)…",
  },
  {
    value: "cirugia",
    label: "Cirugía",
    icon: Scissors,
    description: "Certificado / reporte quirúrgico de una cirugía.",
    emptyText: "No hay cirugías que coincidan.",
    placeholder: "Buscar cirugía (paciente, tipo)…",
  },
  {
    value: "carnet",
    label: "Carnet",
    icon: Syringe,
    description: "Carnet / certificado de vacunación de un paciente.",
    emptyText: "No hay pacientes que coincidan.",
    placeholder: "Buscar paciente (nombre, propietario)…",
  },
];

function SelectableList({
  isLoading,
  items,
  emptyText,
  selectedId,
  onSelect,
}: {
  isLoading: boolean;
  items: { id: number; primary: string; sub: string; right?: string; badge?: ReactNode }[];
  emptyText: string;
  selectedId: number | null;
  onSelect: (id: number) => void;
}) {
  return (
    <div className="max-h-64 space-y-1 overflow-y-auto rounded-lg border p-1">
      {isLoading &&
        Array.from({ length: 4 }).map((_, i) => (
          <Skeleton key={i} className="h-10 w-full" />
        ))}

      {!isLoading && items.length === 0 && (
        <p className="text-muted-foreground px-3 py-8 text-center text-sm">
          {emptyText}
        </p>
      )}

      {items.map((it) => {
        const selected = selectedId === it.id;
        return (
          <button
            key={it.id}
            type="button"
            onClick={() => onSelect(it.id)}
            className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors ${
              selected ? "bg-primary text-primary-foreground" : "hover:bg-accent"
            }`}
          >
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium">{it.primary}</p>
              <p
                className={`truncate text-xs ${
                  selected ? "opacity-80" : "text-muted-foreground"
                }`}
              >
                {it.sub}
              </p>
            </div>
            {it.right && (
              <p
                className={`shrink-0 font-mono text-xs ${
                  selected ? "opacity-90" : "text-muted-foreground"
                }`}
              >
                {it.right}
              </p>
            )}
            {it.badge}
          </button>
        );
      })}
    </div>
  );
}

const statusBadge = (status: string, selected: boolean) => (
  <Badge variant={selected ? "default" : "secondary"} className="shrink-0">
    {status.replace(/_/g, " ")}
  </Badge>
);

export function GenerateReportDialog({
  open,
  onOpenChange,
  onGenerated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onGenerated: (report: ReportFile) => void;
}) {
  const [active, setActive] = useState<ReportKind>("lab");
  const [searches, setSearches] = useState<Record<ReportKind, string>>({
    lab: "",
    formula: "",
    consent: "",
    recibo: "",
    cirugia: "",
    carnet: "",
  });
  const [selected, setSelected] = useState<Record<ReportKind, number | null>>({
    lab: null,
    formula: null,
    consent: null,
    recibo: null,
    cirugia: null,
    carnet: null,
  });

  const lab = useGenerateReport();
  const formula = useGenerateFormulaMedica();
  const consent = useGenerateConsentimiento();
  const recibo = useGenerateReciboInvoice();
  const cirugia = useGenerateCertificadoCirugia();
  const carnet = useGenerateCarnetVacunacion();

  const mutations: Record<ReportKind, UseMutationResult<ReportFile, Error, number>> = {
    lab,
    formula,
    consent,
    recibo,
    cirugia,
    carnet,
  };

  const samples = useSamples("FINALIZADA", searches.lab, active === "lab");
  const consultations = useConsultations(
    null,
    searches.formula,
    active === "formula",
  );
  const consentSurgeries = useSurgeries(
    null,
    searches.consent,
    active === "consent",
  );
  const invoices = useInvoices(null, searches.recibo, active === "recibo");
  const surgeryCerts = useSurgeries(
    null,
    searches.cirugia,
    active === "cirugia",
  );
  const patients = usePatients(searches.carnet, active === "carnet");

  const current = TABS.find((t) => t.value === active)!;
  const CurrentIcon = current.icon;
  const mutation = mutations[active];
  const isLoading =
    (active === "lab" && samples.isLoading) ||
    (active === "formula" && consultations.isLoading) ||
    (active === "consent" && consentSurgeries.isLoading) ||
    (active === "recibo" && invoices.isLoading) ||
    (active === "cirugia" && surgeryCerts.isLoading) ||
    (active === "carnet" && patients.isLoading);

  const submit = async () => {
    const id = selected[active];
    if (id == null) return;
    try {
      const report = await mutation.mutateAsync(id);
      onGenerated(report);
    } catch (e) {
      toast.error(`No se pudo generar ${current.label.toLowerCase()}`, {
        description: getErrorMessage(e),
      });
    }
  };

  const selectAndReset = (kind: ReportKind) => {
    setActive(kind);
    setSelected((s) => ({ ...s, [kind]: null }));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Generar reporte PDF</DialogTitle>
          <DialogDescription>
            El PDF se compone en Rust (printpdf) con los datos de la clínica y
            se guarda en la carpeta de reportes.
          </DialogDescription>
        </DialogHeader>

        <Tabs value={active} onValueChange={(v) => selectAndReset(v as ReportKind)}>
          <TabsList className="flex h-auto w-full flex-wrap justify-start gap-1 py-1">
            {TABS.map((t) => (
              <TabsTrigger
                key={t.value}
                value={t.value}
                className="gap-1.5 px-3"
              >
                <t.icon className="size-3.5" />
                {t.label}
              </TabsTrigger>
            ))}
          </TabsList>

          <div className="mt-4 flex items-start gap-3 rounded-lg border bg-muted/40 px-3 py-2.5">
            <CurrentIcon className="text-muted-foreground mt-0.5 size-4 shrink-0" />
            <p className="text-muted-foreground text-xs leading-relaxed">
              {current.description}
            </p>
          </div>

          <div className="mt-3">
            <TabsContent value={active} forceMount>
              <div className="relative mb-2">
                <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                <Input
                  value={searches[active]}
                  onChange={(e) => {
                    setSearches((s) => ({ ...s, [active]: e.target.value }));
                    setSelected((s) => ({ ...s, [active]: null }));
                  }}
                  placeholder={current.placeholder}
                  className="pl-9"
                  autoFocus
                />
              </div>

              <SelectableList
                isLoading={isLoading}
                emptyText={current.emptyText}
                selectedId={selected[active]}
                onSelect={(id) => setSelected((s) => ({ ...s, [active]: id }))}
                items={
                  active === "lab"
                    ? (samples.data ?? []).map((s) => ({
                        id: s.id,
                        primary: s.patientName,
                        sub: `${s.speciesName} · ${s.ownerName} · ${formatDateTime(s.receivedAt)}`,
                        right: s.code,
                      }))
                    : active === "formula"
                      ? (consultations.data ?? []).map((c) => ({
                          id: c.id,
                          primary: c.patientName,
                          sub: `${c.speciesName} · ${c.reason}`,
                          right: formatDateTime(c.consultationDate),
                          badge: statusBadge(c.status, selected.formula === c.id),
                        }))
                      : active === "consent"
                        ? (consentSurgeries.data ?? [])
                            .filter((s) => s.status !== "CANCELADA")
                            .map((s) => ({
                              id: s.id,
                              primary: s.patientName,
                              sub: `${s.speciesName} · ${s.surgeryType}`,
                              right: formatDateTime(s.scheduledAt),
                              badge: statusBadge(s.status, selected.consent === s.id),
                            }))
                        : active === "recibo"
                          ? (invoices.data ?? [])
                              .filter((i) => i.status !== "ANULADA")
                              .map((i) => ({
                                id: i.id,
                                primary: i.ownerName,
                                sub: i.patientName
                                  ? `Paciente: ${i.patientName} · ${formatDateTime(i.issueDate)}`
                                  : formatDateTime(i.issueDate),
                                right: i.invoiceNumber,
                                badge: statusBadge(i.status, selected.recibo === i.id),
                              }))
                          : active === "cirugia"
                            ? (surgeryCerts.data ?? [])
                                .filter((s) => s.status !== "CANCELADA")
                                .map((s) => ({
                                  id: s.id,
                                  primary: s.patientName,
                                  sub: `${s.speciesName} · ${s.surgeryType}`,
                                  right: formatDateTime(s.scheduledAt),
                                  badge: statusBadge(s.status, selected.cirugia === s.id),
                                }))
                            : (patients.data ?? []).map((p) => ({
                                id: p.id,
                                primary: p.name,
                                sub: `${p.speciesName} · ${p.ownerName}`,
                                right: p.microchip ? `Chip ${p.microchip}` : undefined,
                              }))
                }
              />
            </TabsContent>
          </div>
        </Tabs>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancelar
          </Button>
          <Button
            onClick={submit}
            disabled={selected[active] == null || mutation.isPending}
          >
            {mutation.isPending ? (
              <Loader2 className="animate-spin" />
            ) : (
              <FileText className="size-4" />
            )}
            Generar {current.label.toLowerCase()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
