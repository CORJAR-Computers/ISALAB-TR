import { useMemo, useState } from "react";
import {
  AlertTriangle,
  Ban,
  CheckCircle2,
  FileText,
  FlaskConical,
  PlayCircle,
  Plus,
  Search,
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
import { useSampleCounts, useSamples } from "@/hooks/use-queries";
import { SAMPLE_STATUS } from "@/lib/status";
import { cn, formatDateTime } from "@/lib/utils";
import { SampleDetailDialog } from "./SampleDetailDialog";
import { NewSampleDialog } from "./NewSampleDialog";

const STATUS_TABS: Array<{ value: string | null; label: string }> = [
  { value: null, label: "Todas" },
  { value: "RECIBIDA", label: "Recibidas" },
  { value: "EN_PROCESO", label: "En proceso" },
  { value: "FINALIZADA", label: "Finalizadas" },
  { value: "ANULADA", label: "Anuladas" },
];

function StatCard({
  icon: Icon,
  label,
  value,
  tone,
  onClick,
  active,
  disabled = false,
}: {
  icon: typeof FlaskConical;
  label: string;
  value: number;
  tone: string;
  onClick: () => void;
  active: boolean;
  /** Tarjeta informativa sin acción de filtro. */
  disabled?: boolean;
}) {
  const inner = (
    <>
      <div
        className={cn(
          "flex size-10 shrink-0 items-center justify-center rounded-lg",
          tone,
        )}
      >
        <Icon className="size-5" />
      </div>
      <div>
        <p className="text-2xl leading-none font-bold tabular-nums">{value}</p>
        <p className="text-muted-foreground text-xs">{label}</p>
      </div>
    </>
  );

  if (disabled) {
    return (
      <div className="flex items-center gap-3 rounded-xl border bg-card px-4 py-3 shadow-sm">
        {inner}
      </div>
    );
  }

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "group flex items-center gap-3 rounded-xl border bg-card px-4 py-3 text-left shadow-sm transition-all hover:-translate-y-0.5 hover:shadow-md",
        active && "ring-2 ring-ring",
      )}
    >
      {inner}
    </button>
  );
}

export function SamplesPage() {
  const [status, setStatus] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [detailId, setDetailId] = useState<number | null>(null);
  const [newSampleOpen, setNewSampleOpen] = useState(false);

  const { data: samples, isLoading, isError } = useSamples(status, search);
  const { data: all, isLoading: loadingCounts } = useSampleCounts();

  const counts = useMemo(() => {
    const c = {
      TOTAL: 0,
      RECIBIDA: 0,
      EN_PROCESO: 0,
      FINALIZADA: 0,
      ANULADA: 0,
      ABNORMAL: 0,
    };
    for (const s of all ?? []) {
      c.TOTAL += 1;
      c[s.status as keyof typeof c] = ((c[s.status as keyof typeof c] ?? 0) as number) + 1;
      if (s.abnormalCount > 0) c.ABNORMAL += 1;
    }
    return c;
  }, [all]);

  return (
    <div className="space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Muestras & Laboratorio
          </h2>
          <p className="text-muted-foreground text-sm">
            Mesa de trabajo: recepción, procesamiento, resultados y trazabilidad
            de muestras en tiempo real.
          </p>
        </div>
        <Button onClick={() => setNewSampleOpen(true)} className="shrink-0 gap-1.5">
          <Plus className="size-4" />
          Nueva toma de muestra
        </Button>
      </div>

      {/* Resumen */}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
        <StatCard
          icon={FlaskConical}
          label="Total"
          value={counts.TOTAL}
          tone="bg-muted text-muted-foreground"
          onClick={() => setStatus(null)}
          active={status === null}
        />
        <StatCard
          icon={PlayCircle}
          label="En proceso"
          value={counts.EN_PROCESO}
          tone="bg-warning/15 text-warning"
          onClick={() => setStatus("EN_PROCESO")}
          active={status === "EN_PROCESO"}
        />
        <StatCard
          icon={CheckCircle2}
          label="Finalizadas"
          value={counts.FINALIZADA}
          tone="bg-success/15 text-success"
          onClick={() => setStatus("FINALIZADA")}
          active={status === "FINALIZADA"}
        />
        <StatCard
          icon={Ban}
          label="Anuladas"
          value={counts.ANULADA}
          tone="bg-destructive/10 text-destructive"
          onClick={() => setStatus("ANULADA")}
          active={status === "ANULADA"}
        />
        <StatCard
          icon={AlertTriangle}
          label="Con valores anormales"
          value={counts.ABNORMAL}
          tone="bg-orange-500/15 text-orange-600 dark:text-orange-400"
          onClick={() => setStatus(null)}
          active={false}
          disabled
        />
      </div>

      {/* Filtros */}
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
                  {counts[tab.value as keyof typeof counts] ?? 0}
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
            placeholder="Buscar por código, paciente o propietario…"
            className="pl-9"
          />
        </div>
      </div>

      <Card className="gap-0 p-0">
        <CardHeader className="border-b">
          <CardTitle className="text-base">Trazabilidad de muestras</CardTitle>
          <CardDescription>
            {isLoading || loadingCounts
              ? "Cargando…"
              : `${samples?.length ?? 0} muestra${(samples?.length ?? 0) === 1 ? "" : "s"}`}
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Código</TableHead>
                <TableHead>Paciente</TableHead>
                <TableHead>Tipo de muestra</TableHead>
                <TableHead>Recibida</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead className="text-center">Resultados</TableHead>
                <TableHead className="text-right">Acción</TableHead>
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
                    No se pudo cargar la mesa de trabajo.
                  </TableCell>
                </TableRow>
              )}

              {!isLoading && samples?.length === 0 && (
                <TableRow>
                  <TableCell colSpan={7} className="text-muted-foreground py-10 text-center">
                    <div className="flex flex-col items-center justify-center gap-3">
                      <p className="text-sm">
                        {search
                          ? "Ninguna muestra coincide con la búsqueda."
                          : "No hay muestras registradas."}
                      </p>
                      <Button
                        size="sm"
                        onClick={() => setNewSampleOpen(true)}
                        className="gap-1.5"
                      >
                        <Plus className="size-4" />
                        Registrar nueva muestra
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              )}

              {samples?.map((s) => {
                const st = SAMPLE_STATUS[s.status] ?? {
                  label: s.status,
                  variant: "secondary" as const,
                };
                const canLoadResult = s.status === "RECIBIDA" || s.status === "EN_PROCESO";
                const isFinalized = s.status === "FINALIZADA";

                return (
                  <TableRow
                    key={s.id}
                    className="cursor-pointer"
                    onClick={() => setDetailId(s.id)}
                  >
                    <TableCell>
                      <span className="font-mono text-sm font-semibold">
                        {s.code}
                      </span>
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-col">
                        <span className="font-medium">{s.patientName}</span>
                        <span className="text-muted-foreground text-xs">
                          {s.speciesName} · {s.ownerName}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell>{s.sampleTypeName}</TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {formatDateTime(s.receivedAt)}
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-col gap-1">
                        <Badge variant={st.variant}>{st.label}</Badge>
                        {s.abnormalCount > 0 && (
                          <Badge
                            variant="outline"
                            className="text-warning w-fit"
                          >
                            <AlertTriangle className="size-3" />
                            {s.abnormalCount} anormal
                            {s.abnormalCount === 1 ? "" : "es"}
                          </Badge>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="text-center">
                      <span className="text-sm tabular-nums font-semibold">
                        {s.resultCount}
                      </span>
                      <span className="text-muted-foreground text-xs">
                        {" "}
                        analito{s.resultCount === 1 ? "" : "s"}
                      </span>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant={canLoadResult ? "default" : "outline"}
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          setDetailId(s.id);
                        }}
                        className="gap-1.5"
                      >
                        {canLoadResult ? (
                          <>
                            <FlaskConical className="size-3.5" />
                            Cargar resultados
                          </>
                        ) : isFinalized ? (
                          <>
                            <FileText className="size-3.5" />
                            Ver / PDF
                          </>
                        ) : (
                          "Detalle"
                        )}
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <SampleDetailDialog
        sampleId={detailId}
        onOpenChange={(open) => !open && setDetailId(null)}
      />

      <NewSampleDialog
        open={newSampleOpen}
        onOpenChange={setNewSampleOpen}
        onCreated={(sample) => setDetailId(sample.id)}
      />
    </div>
  );
}
