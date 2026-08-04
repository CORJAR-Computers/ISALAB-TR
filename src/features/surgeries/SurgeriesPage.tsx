import { useMemo, useState } from "react";
import { toast } from "sonner";
import {
  Ban,
  CheckCircle2,
  Loader2,
  PlayCircle,
  Plus,
  Scissors,
  Search,
  MessageCircle,
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
  useSetSurgeryStatus,
  useSurgeryCounts,
  useSurgeries,
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { SURGERY_STATUS } from "@/lib/status";
import { cn, formatDateTime } from "@/lib/utils";
import { sendWhatsAppMessage } from "@/lib/whatsapp";
import { NewSurgeryDialog } from "./NewSurgeryDialog";
import { usePermissions } from "@/hooks/use-permissions";

const STATUS_TABS: Array<{ value: string | null; label: string }> = [
  { value: null, label: "Todas" },
  { value: "PROGRAMADA", label: "Programadas" },
  { value: "EN_CURSO", label: "En curso" },
  { value: "COMPLETADA", label: "Completadas" },
  { value: "CANCELADA", label: "Canceladas" },
];

export function SurgeriesPage() {
  const [status, setStatus] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const { data: surgeries, isLoading, isError } = useSurgeries(status, search);
  const { data: all } = useSurgeryCounts();
  const setStatusMutation = useSetSurgeryStatus();
  const { isVetOrAdmin } = usePermissions();

  // Contadores reales por estado (independientes de filtros/búsqueda).
  const counts = useMemo(() => {
    const c: Record<string, number> = {
      TOTAL: 0,
      PROGRAMADA: 0,
      EN_CURSO: 0,
      COMPLETADA: 0,
      CANCELADA: 0,
    };
    for (const s of all ?? []) {
      c.TOTAL += 1;
      c[s.status] = (c[s.status] ?? 0) + 1;
    }
    return c;
  }, [all]);

  const changeStatus = async (id: number, next: string, label: string) => {
    try {
      await setStatusMutation.mutateAsync({ id, status: next });
      toast.success(`Cirugía ${label.toLowerCase()}`);
    } catch (e) {
      toast.error("No se pudo actualizar el estado", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">Cirugías</h2>
          <p className="text-muted-foreground text-sm">
            Agenda quirúrgica: programación, anestesia y estados de la
            intervención.
          </p>
        </div>
        {isVetOrAdmin && (
          <Button onClick={() => setDialogOpen(true)}>
            <Plus className="size-4" />
            Programar cirugía
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
            placeholder="Buscar por paciente, propietario o tipo…"
            className="pl-9"
          />
        </div>
      </div>

      <Card className="gap-0 p-0">
        <CardHeader className="border-b">
          <CardTitle className="flex items-center gap-2 text-base">
            <Scissors className="size-4 text-primary" />
            Agenda quirúrgica
          </CardTitle>
          <CardDescription>
            {isLoading
              ? "Cargando…"
              : `${surgeries?.length ?? 0} cirugía${(surgeries?.length ?? 0) === 1 ? "" : "s"}`}
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Paciente</TableHead>
                <TableHead>Tipo</TableHead>
                <TableHead>Fecha programada</TableHead>
                <TableHead>Anestesia</TableHead>
                <TableHead>Veterinario</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead className="text-right">Acciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading &&
                Array.from({ length: 5 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell colSpan={7} className="h-12">
                      <Skeleton className="h-8 w-full" />
                    </TableCell>
                  </TableRow>
                ))}

              {!isLoading && isError && (
                <TableRow>
                  <TableCell colSpan={7} className="text-muted-foreground h-16 text-center">
                    No se pudo cargar la agenda quirúrgica.
                  </TableCell>
                </TableRow>
              )}

              {!isLoading && surgeries?.length === 0 && (
                <TableRow>
                  <TableCell colSpan={7} className="text-muted-foreground h-24 text-center">
                    {search
                      ? "Ninguna cirugía coincide con la búsqueda."
                      : "Sin cirugías programadas. Usa “Programar cirugía”."}
                  </TableCell>
                </TableRow>
              )}

              {surgeries?.map((s) => {
                const st = SURGERY_STATUS[s.status] ?? {
                  label: s.status,
                  variant: "secondary" as const,
                };
                const busy = setStatusMutation.isPending;
                return (
                  <TableRow key={s.id}>
                    <TableCell>
                      <div className="flex flex-col">
                        <span className="font-medium">{s.patientName}</span>
                        <span className="text-muted-foreground text-xs">
                          {s.speciesName} · {s.ownerName}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell className="font-medium">
                      {s.surgeryType}
                      {s.preoperativeNotes && (
                        <span className="text-muted-foreground block max-w-52 truncate text-xs">
                          {s.preoperativeNotes}
                        </span>
                      )}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {formatDateTime(s.scheduledAt)}
                    </TableCell>
                    <TableCell className="text-xs">
                      {s.anesthesiaType ?? "—"}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {s.veterinarianName ?? "—"}
                    </TableCell>
                    <TableCell>
                      <Badge variant={st.variant}>{st.label}</Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      {isVetOrAdmin && (
                        <>
                          {s.status === "PROGRAMADA" && (
                            <Button
                              variant="outline"
                              size="sm"
                              disabled={busy}
                              onClick={() =>
                                changeStatus(s.id, "EN_CURSO", "En curso")
                              }
                            >
                              {busy ? (
                                <Loader2 className="size-3.5 animate-spin" />
                              ) : (
                                <PlayCircle className="size-3.5" />
                              )}
                              Iniciar
                            </Button>
                          )}
                          {s.status === "EN_CURSO" && (
                            <div className="flex justify-end gap-1">
                              <Button
                                variant="outline"
                                size="sm"
                                className="text-success"
                                disabled={busy}
                                onClick={() =>
                                  changeStatus(s.id, "COMPLETADA", "Completada")
                                }
                              >
                                <CheckCircle2 className="size-3.5" />
                                Completar
                              </Button>
                            </div>
                          )}
                          {(s.status === "PROGRAMADA" || s.status === "EN_CURSO") && (
                            <Button
                              variant="ghost"
                              size="sm"
                              className="text-destructive"
                              disabled={busy}
                              onClick={() =>
                                changeStatus(s.id, "CANCELADA", "Cancelada")
                              }
                            >
                              <Ban className="size-3.5" />
                              Cancelar
                            </Button>
                          )}
                        </>
                      )}
                      
                      {s.status === "PROGRAMADA" && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 gap-1.5 text-green-600 hover:bg-green-50 hover:text-green-700 dark:text-green-400 dark:hover:bg-green-900/30"
                          onClick={() => {
                            if (!s.ownerPhone) {
                              toast.error("Falta información", { description: "El propietario no tiene teléfono." });
                              return;
                            }
                            sendWhatsAppMessage(s.ownerPhone, `Hola ${s.ownerName},\n\nTe compartimos desde ISALAB el consentimiento informado para la cirugía programada de *${s.patientName}*.\n\nPor favor, revísalo y fírmalo.`);
                          }}
                        >
                          <MessageCircle className="size-3.5" />
                          Consent.
                        </Button>
                      )}

                      {s.status === "COMPLETADA" && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 gap-1.5 text-green-600 hover:bg-green-50 hover:text-green-700 dark:text-green-400 dark:hover:bg-green-900/30"
                          onClick={() => {
                            if (!s.ownerPhone) {
                              toast.error("Falta información", { description: "El propietario no tiene teléfono." });
                              return;
                            }
                            sendWhatsAppMessage(s.ownerPhone, `Hola ${s.ownerName},\n\nTe compartimos desde ISALAB el certificado/reporte quirúrgico de *${s.patientName}*.\n\nPor favor, revisa el archivo adjunto.\n\n¡Pronta recuperación!`);
                          }}
                        >
                          <MessageCircle className="size-3.5" />
                          Reporte
                        </Button>
                      )}
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <NewSurgeryDialog open={dialogOpen} onOpenChange={setDialogOpen} />
    </div>
  );
}
