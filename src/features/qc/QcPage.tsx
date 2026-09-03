import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  FlaskConical,
  Loader2,
  Plus,
  Trash2,
  X,
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { Skeleton } from "@/components/ui/skeleton";
import {
  useAnalyzers,
  useAnalytes,
} from "@/hooks/use-queries";
import {
  useDeleteQcMaterial,
  useDeleteQcRun,
  useQcAnalyzerStatus,
  useQcChart,
  useQcMaterials,
  useQcRuns,
  useQcTargets,
  useRecordQcRun,
  useSaveQcMaterial,
} from "@/hooks/queries/use-qc";
import { cn, formatDateTime } from "@/lib/utils";
import { getErrorMessage } from "@/lib/api";
import { usePermissions } from "@/hooks/use-permissions";
import type { QcChartData, QcMaterialInput, QcRunInput } from "@/bindings";

/** Gráfico Levey-Jennings en SVG (sin dependencias): media, bandas ±1/2/3 SD
 *  y los puntos de control coloreados según las reglas Westgard violadas. */
function LeveyJenningsChart({ data }: { data: QcChartData }) {
  const { mean, sd, points, analyteName, unit } = data;
  const W = 640;
  const H = 240;
  const PAD = { top: 16, right: 16, bottom: 24, left: 44 };

  const yFor = (z: number) => {
    const yMid = PAD.top + (H - PAD.top - PAD.bottom) / 2;
    return yMid - (z / 3.5) * ((H - PAD.top - PAD.bottom) / 2);
  };
  const xFor = (i: number, n: number) =>
    PAD.left + (n <= 1 ? 0 : (i * (W - PAD.left - PAD.right)) / Math.max(n - 1, 1));

  const n = points.length;
  const bands = [
    { z: 1, color: "#3b82f6" },
    { z: 2, color: "#f59e0b" },
    { z: 3, color: "#ef4444" },
  ];

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="w-full">
      {bands.map((b) => (
        <g key={b.z}>
          <line
            x1={PAD.left}
            x2={W - PAD.right}
            y1={yFor(b.z)}
            y2={yFor(b.z)}
            stroke={b.color}
            strokeWidth={b.z === 3 ? 1.2 : 0.8}
            strokeDasharray={b.z === 1 ? "none" : "4 3"}
            opacity={0.7}
          />
          <line
            x1={PAD.left}
            x2={W - PAD.right}
            y1={yFor(-b.z)}
            y2={yFor(-b.z)}
            stroke={b.color}
            strokeWidth={b.z === 3 ? 1.2 : 0.8}
            strokeDasharray={b.z === 1 ? "none" : "4 3"}
            opacity={0.7}
          />
        </g>
      ))}
      <line x1={PAD.left} x2={W - PAD.right} y1={yFor(0)} y2={yFor(0)} stroke="#71717a" strokeWidth={1.4} />

      {points.map((p, i) => {
        const cx = xFor(i, n);
        const cy = yFor(p.zScore);
        const isViolated = p.violation != null;
        return (
          <g key={p.runId}>
            <line x1={cx} x2={cx} y1={cy} y2={PAD.top} stroke="#a1a1aa" strokeWidth={0.5} opacity={0.4} />
            <circle
              cx={cx}
              cy={cy}
              r={isViolated ? 5.5 : 4}
              fill={isViolated ? "#ef4444" : "#3b82f6"}
              stroke="#fff"
              strokeWidth={1.2}
            />
            {isViolated && (
              <title>{`${p.runDate} · z=${p.zScore.toFixed(2)} · ${p.violation}`}</title>
            )}
          </g>
        );
      })}

      {/* Eje Y */}
      {[3, 2, 1, 0, -1, -2, -3].map((z) => (
        <text
          key={z}
          x={PAD.left - 6}
          y={yFor(z) + 3}
          textAnchor="end"
          fontSize={9}
          fill="#71717a"
        >
          {z}
        </text>
      ))}
      <text x={PAD.left} y={H - 6} fontSize={10} fill="#71717a">
        {analyteName} ({points.length} corridas)
      </text>
      <text x={W - PAD.right} y={H - 6} textAnchor="end" fontSize={10} fill="#71717a">
        Media {mean} {unit ?? ""} · SD {sd.toFixed(2)}
      </text>
    </svg>
  );
}

const emptyMaterial = (analyzerId: number): QcMaterialInput => ({
  id: null,
  name: "",
  analyzerId,
  lot: null,
  expiresAt: null,
  notes: null,
  targets: [],
});

export function QcPage() {
  const { isVetOrAdmin } = usePermissions();
  const { data: materials = [], isLoading } = useQcMaterials();
  const { data: analyzers = [] } = useAnalyzers();
  const { data: analytes = [] } = useAnalytes();
  const { data: qcStatus = [] } = useQcAnalyzerStatus();
  const activeAnalyzers = analyzers.filter((a) => a.isActive && a.code !== "GENERAL");

  const [selectedMaterial, setSelectedMaterial] = useState<number | null>(null);
  const { data: targets = [] } = useQcTargets(selectedMaterial);
  const { data: runs = [] } = useQcRuns(selectedMaterial);

  // Diálogo de material
  const [materialOpen, setMaterialOpen] = useState(false);
  const [materialDraft, setMaterialDraft] = useState<QcMaterialInput>(() =>
    emptyMaterial(1),
  );
  const saveMaterial = useSaveQcMaterial();
  const deleteMaterial = useDeleteQcMaterial();

  // Diálogo de corrida
  const [runOpen, setRunOpen] = useState(false);
  const [runValues, setRunValues] = useState<Record<number, string>>({});
  const [runNotes, setRunNotes] = useState("");
  const recordRun = useRecordQcRun();
  const deleteRun = useDeleteQcRun();

  // Gráfico: primer analito objetivo del material seleccionado.
  const [chartAnalyte, setChartAnalyte] = useState<number | null>(null);
  useEffect(() => {
    if (targets.length > 0 && (chartAnalyte == null || !targets.some((t) => t.analyteId === chartAnalyte))) {
      setChartAnalyte(targets[0].analyteId);
    }
  }, [targets, chartAnalyte]);
  const { data: chart } = useQcChart(selectedMaterial, chartAnalyte);

  const qcStatusByAnalyzer = useMemo(
    () => new Map(qcStatus.map((s) => [s.analyzerId, s.latestStatus])),
    [qcStatus],
  );

  const openNewMaterial = () => {
    const first = activeAnalyzers[0]?.id ?? 1;
    setMaterialDraft(emptyMaterial(first));
    setMaterialOpen(true);
  };

  const openEditMaterial = (id: number) => {
    const m = materials.find((x) => x.id === id);
    if (!m) return;
    setMaterialDraft({
      id: m.id,
      name: m.name,
      analyzerId: m.analyzerId,
      lot: m.lot,
      expiresAt: m.expiresAt,
      notes: m.notes,
      targets: [],
    });
    setMaterialOpen(true);
  };

  const addTargetRow = () =>
    setMaterialDraft((d) => ({
      ...d,
      targets: [...d.targets, { analyteId: 0, mean: 0, sd: 0 }],
    }));

  const saveMaterialDraft = async () => {
    if (!materialDraft.name.trim()) {
      toast.error("Indica el nombre del material de control");
      return;
    }
    const validTargets = materialDraft.targets.filter(
      (t) => t.analyteId > 0 && t.sd > 0,
    );
    if (validTargets.length === 0) {
      toast.error("Agrega al menos un analito objetivo con media y SD > 0");
      return;
    }
    try {
      await saveMaterial.mutateAsync({ ...materialDraft, targets: validTargets });
      toast.success(
        materialDraft.id ? "Material actualizado" : "Material de control creado",
      );
      setMaterialOpen(false);
    } catch (err) {
      toast.error("No se pudo guardar el material", { description: getErrorMessage(err) });
    }
  };

  const openRecordRun = () => {
    setRunValues({});
    setRunNotes("");
    setRunOpen(true);
  };

  const submitRun = async () => {
    if (!selectedMaterial) return;
    const measurements = targets
      .map((t) => ({ analyteId: t.analyteId, value: Number((runValues[t.analyteId] ?? "").replace(",", ".")) }))
      .filter((m) => !Number.isNaN(m.value) && runValues[m.analyteId]?.trim() !== "");
    if (measurements.length === 0) {
      toast.error("Ingresa al menos un valor de control");
      return;
    }
    try {
      const run = await recordRun.mutateAsync({
        controlMaterialId: selectedMaterial,
        notes: runNotes.trim() || null,
        measurements,
      } as QcRunInput);
      toast.success(
        run.status === "RECHAZADO"
          ? "Corrida rechazada por reglas de Westgard — revisa el gráfico"
          : "Corrida de control aceptada",
        {
          description:
            run.status === "RECHAZADO"
              ? "Se detectaron violaciones (1_3s, 2_2s, R_4s, 4_1s, 10x). No proceses muestras de pacientes con este equipo."
              : "Todas las mediciones dentro de los límites aceptables.",
        },
      );
      setRunOpen(false);
    } catch (err) {
      toast.error("No se pudo registrar la corrida", { description: getErrorMessage(err) });
    }
  };

  const doDeleteMaterial = async (id: number, name: string) => {
    if (!window.confirm(`¿Eliminar el material de control "${name}" y todas sus corridas?`)) return;
    try {
      await deleteMaterial.mutateAsync(id);
      if (selectedMaterial === id) setSelectedMaterial(null);
      toast.success("Material eliminado");
    } catch (err) {
      toast.error("No se pudo eliminar", { description: getErrorMessage(err) });
    }
  };

  const doDeleteRun = async (id: number) => {
    if (!window.confirm("¿Eliminar esta corrida de control?")) return;
    try {
      await deleteRun.mutateAsync(id);
      toast.success("Corrida eliminada");
    } catch (err) {
      toast.error("No se pudo eliminar", { description: getErrorMessage(err) });
    }
  };

  const selected = materials.find((m) => m.id === selectedMaterial) ?? null;

  return (
    <div className="space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Control de calidad (QC)
          </h2>
          <p className="text-muted-foreground text-sm">
            Materiales de control por equipo, corridas evaluadas con las reglas
            multirregla de Westgard (1₂s, 1₃s, 2₂s, R₄s, 4₁s, 10ₓ) y gráficos
            Levey-Jennings.
          </p>
        </div>
        {isVetOrAdmin && (
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" onClick={openNewMaterial} className="gap-1.5">
              <Plus className="size-4" />
              Nuevo material
            </Button>
            <Button
              onClick={openRecordRun}
              disabled={!selectedMaterial || targets.length === 0}
              className="gap-1.5"
            >
              <FlaskConical className="size-4" />
              Registrar corrida
            </Button>
          </div>
        )}
      </div>

      <div className="grid gap-4 lg:grid-cols-[300px_1fr]">
        {/* Materiales */}
        <Card className="gap-0 p-0">
          <CardHeader className="border-b">
            <CardTitle className="text-base">Materiales de control</CardTitle>
            <CardDescription>
              {activeAnalyzers.some((a) => qcStatusByAnalyzer.get(a.id) === "RECHAZADO")
                ? "⚠ Hay equipos con la última corrida rechazada."
                : "Todos los equipos con QC aceptado."}
            </CardDescription>
          </CardHeader>
          <CardContent className="max-h-[60vh] space-y-2 overflow-y-auto p-3">
            {isLoading && <Skeleton className="h-16 w-full" />}
            {materials.length === 0 && !isLoading && (
              <p className="text-muted-foreground p-4 text-center text-sm">
                No hay materiales de control. Crea uno para empezar a registrar
                corridas QC.
              </p>
            )}
            {materials.map((m) => {
              const status = qcStatusByAnalyzer.get(m.analyzerId);
              return (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => setSelectedMaterial(m.id)}
                  className={cn(
                    "flex w-full items-center justify-between rounded-lg border px-3 py-2.5 text-left transition-colors",
                    selectedMaterial === m.id
                      ? "bg-primary/10 border-primary/40"
                      : "hover:bg-accent",
                  )}
                >
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">{m.name}</p>
                    <p className="text-muted-foreground truncate text-xs">
                      {m.analyzerName} · {m.targetCount} analito
                      {m.targetCount === 1 ? "" : "s"}
                      {m.lot ? ` · Lote ${m.lot}` : ""}
                    </p>
                  </div>
                  <div className="flex shrink-0 items-center gap-1.5">
                    {status === "RECHAZADO" && (
                      <Badge variant="destructive" className="animate-pulse">
                        QC ⚠
                      </Badge>
                    )}
                    {status === "ACEPTADO" && (
                      <Badge variant="success">QC ✓</Badge>
                    )}
                  </div>
                </button>
              );
            })}
          </CardContent>
        </Card>

        {/* Detalle del material seleccionado */}
        <div className="space-y-4">
          {!selectedMaterial && (
            <Card>
              <CardContent className="text-muted-foreground p-8 text-center text-sm">
                Selecciona un material de control para ver sus corridas y el
                gráfico Levey-Jennings.
              </CardContent>
            </Card>
          )}

          {selectedMaterial && selected && (
            <>
              <Card className="gap-0 p-0">
                <CardHeader className="flex-row items-center justify-between border-b">
                  <div>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <Activity className="size-4" />
                      {selected.name}
                      {isVetOrAdmin && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-6 px-2 text-xs"
                          onClick={() => openEditMaterial(selected.id)}
                        >
                          Editar
                        </Button>
                      )}
                    </CardTitle>
                    <CardDescription>
                      {selected.analyzerName} · {targets.length} objetivos
                      {selected.expiresAt ? ` · Vence ${selected.expiresAt}` : ""}
                    </CardDescription>
                  </div>
                  {isVetOrAdmin && (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-destructive hover:text-destructive"
                      onClick={() => doDeleteMaterial(selected.id, selected.name)}
                    >
                      <Trash2 className="size-4" />
                    </Button>
                  )}
                </CardHeader>
                <CardContent className="space-y-4 p-4">
                  {/* Gráfico L-J */}
                  <div>
                    <div className="mb-2 flex flex-wrap items-center gap-2">
                      <p className="text-sm font-semibold">Levey-Jennings</p>
                      <Select
                        value={chartAnalyte?.toString() ?? ""}
                        onValueChange={(v) => setChartAnalyte(Number(v))}
                      >
                        <SelectTrigger className="h-7 w-56 text-xs">
                          <SelectValue placeholder="Analito…" />
                        </SelectTrigger>
                        <SelectContent>
                          {targets.map((t) => (
                            <SelectItem key={t.analyteId} value={t.analyteId.toString()}>
                              {t.analyteName} {t.unit ? `(${t.unit})` : ""}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    {chart ? (
                      <div className="rounded-lg border p-2">
                        <LeveyJenningsChart data={chart} />
                      </div>
                    ) : (
                      <p className="text-muted-foreground text-xs">
                        Registra corridas para ver el gráfico.
                      </p>
                    )}
                  </div>

                  {/* Corridas */}
                  <div className="overflow-hidden rounded-lg border">
                    <div className="bg-muted/60 border-b px-3 py-2 text-sm font-semibold">
                      Corridas ({runs.length})
                    </div>
                    {runs.length === 0 ? (
                      <p className="text-muted-foreground px-4 py-6 text-center text-sm">
                        Sin corridas registradas.
                      </p>
                    ) : (
                      <div className="max-h-80 overflow-y-auto">
                        <Table>
                          <TableHeader>
                            <TableRow>
                              <TableHead>Fecha</TableHead>
                              <TableHead>Estado</TableHead>
                              <TableHead>Mediciones</TableHead>
                              <TableHead>Responsable</TableHead>
                              <TableHead className="text-right">Acción</TableHead>
                            </TableRow>
                          </TableHeader>
                          <TableBody>
                            {runs.map((run) => (
                              <TableRow key={run.id}>
                                <TableCell className="text-xs">
                                  {formatDateTime(run.runDate)}
                                </TableCell>
                                <TableCell>
                                  <Badge
                                    variant={run.status === "RECHAZADO" ? "destructive" : "success"}
                                    className={run.status === "RECHAZADO" ? "animate-pulse" : ""}
                                  >
                                    {run.status === "RECHAZADO" ? "Rechazada" : "Aceptada"}
                                  </Badge>
                                  {run.measurements.some((m) => m.violation) && (
                                    <p className="mt-1 max-w-52 text-destructive text-[11px]">
                                      {run.measurements
                                        .filter((m) => m.violation)
                                        .map((m) => `${m.analyteName}: ${m.violation}`)
                                        .join(" · ")}
                                    </p>
                                  )}
                                </TableCell>
                                <TableCell className="font-mono text-xs">
                                  {run.measurements.map((m) => (
                                    <div key={m.id}>
                                      {m.analyteName}: {m.value}{" "}
                                      <span className="text-muted-foreground">
                                        (z={m.zScore?.toFixed(2)})
                                      </span>
                                    </div>
                                  ))}
                                </TableCell>
                                <TableCell className="text-xs">{run.createdBy ?? "—"}</TableCell>
                                <TableCell className="text-right">
                                  {isVetOrAdmin && (
                                    <Button
                                      variant="ghost"
                                      size="sm"
                                      className="h-6 px-2 text-destructive hover:text-destructive"
                                      onClick={() => doDeleteRun(run.id)}
                                    >
                                      <Trash2 className="size-3.5" />
                                    </Button>
                                  )}
                                </TableCell>
                              </TableRow>
                            ))}
                          </TableBody>
                        </Table>
                      </div>
                    )}
                  </div>
                </CardContent>
              </Card>
            </>
          )}
        </div>
      </div>

      {/* Diálogo material de control */}
      <Dialog open={materialOpen} onOpenChange={setMaterialOpen}>
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>
              {materialDraft.id ? "Editar material de control" : "Nuevo material de control"}
            </DialogTitle>
            <DialogDescription>
              Define el equipo, el lote y los valores objetivo (media y SD) por
              analito. Las corridas se evalúan contra estos objetivos.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="grid gap-3 sm:grid-cols-2">
              <div>
                <label className="text-sm font-medium">Nombre *</label>
                <Input
                  className="mt-1"
                  placeholder="Ej. Control nivel 1 (MINDRAY B2800)"
                  value={materialDraft.name}
                  onChange={(e) =>
                    setMaterialDraft((d) => ({ ...d, name: e.target.value }))
                  }
                />
              </div>
              <div>
                <label className="text-sm font-medium">Equipo *</label>
                <Select
                  value={materialDraft.analyzerId.toString()}
                  onValueChange={(v) =>
                    setMaterialDraft((d) => ({ ...d, analyzerId: Number(v) }))
                  }
                >
                  <SelectTrigger className="mt-1">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {activeAnalyzers.map((a) => (
                      <SelectItem key={a.id} value={a.id.toString()}>
                        {a.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <div>
                <label className="text-sm font-medium">Lote (opcional)</label>
                <Input
                  className="mt-1"
                  placeholder="Ej. L-2026-001"
                  value={materialDraft.lot ?? ""}
                  onChange={(e) =>
                    setMaterialDraft((d) => ({ ...d, lot: e.target.value || null }))
                  }
                />
              </div>
              <div>
                <label className="text-sm font-medium">Vence (opcional)</label>
                <Input
                  type="date"
                  className="mt-1"
                  value={materialDraft.expiresAt ?? ""}
                  onChange={(e) =>
                    setMaterialDraft((d) => ({ ...d, expiresAt: e.target.value || null }))
                  }
                />
              </div>
            </div>

            <div>
              <div className="mb-2 flex items-center justify-between">
                <label className="text-sm font-medium">Objetivos (media / SD) *</label>
                <Button variant="outline" size="sm" onClick={addTargetRow} className="h-7 gap-1 px-2 text-xs">
                  <Plus className="size-3" />
                  Agregar analito
                </Button>
              </div>
              <div className="space-y-2">
                {materialDraft.targets.map((t, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <Select
                      value={t.analyteId > 0 ? t.analyteId.toString() : ""}
                      onValueChange={(v) =>
                        setMaterialDraft((d) => {
                          const targets = [...d.targets];
                          targets[i] = { ...targets[i], analyteId: Number(v) };
                          return { ...d, targets };
                        })
                      }
                    >
                      <SelectTrigger className="h-8 flex-1 text-xs">
                        <SelectValue placeholder="Analito…" />
                      </SelectTrigger>
                      <SelectContent>
                        {analytes.map((a) => (
                          <SelectItem key={a.id} value={a.id.toString()}>
                            {a.name} {a.unit ? `(${a.unit})` : ""}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <Input
                      type="number"
                      step="any"
                      className="h-8 w-24 font-mono text-xs"
                      placeholder="Media"
                      value={t.mean || ""}
                      onChange={(e) =>
                        setMaterialDraft((d) => {
                          const targets = [...d.targets];
                          targets[i] = { ...targets[i], mean: Number(e.target.value) };
                          return { ...d, targets };
                        })
                      }
                    />
                    <Input
                      type="number"
                      step="any"
                      className="h-8 w-20 font-mono text-xs"
                      placeholder="SD"
                      value={t.sd || ""}
                      onChange={(e) =>
                        setMaterialDraft((d) => {
                          const targets = [...d.targets];
                          targets[i] = { ...targets[i], sd: Number(e.target.value) };
                          return { ...d, targets };
                        })
                      }
                    />
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-8 w-8 px-0 text-destructive hover:text-destructive"
                      onClick={() =>
                        setMaterialDraft((d) => ({
                          ...d,
                          targets: d.targets.filter((_, j) => j !== i),
                        }))
                      }
                    >
                      <X className="size-3.5" />
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setMaterialOpen(false)}>
              Cancelar
            </Button>
            <Button onClick={saveMaterialDraft} disabled={saveMaterial.isPending}>
              {saveMaterial.isPending ? <Loader2 className="size-4 animate-spin" /> : null}
              Guardar material
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Diálogo registrar corrida */}
      <Dialog open={runOpen} onOpenChange={setRunOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <FlaskConical className="size-4" />
              Registrar corrida · {selected?.name ?? ""}
            </DialogTitle>
            <DialogDescription>
              Ingresa el valor medido de cada analito del control. El sistema
              calcula el z-score y evalúa las reglas de Westgard contra el
              historial reciente.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            {targets.map((t) => (
              <label key={t.analyteId} className="flex items-center gap-2 text-sm">
                <span className="min-w-0 flex-1 truncate">
                  {t.analyteName}
                  {t.unit ? (
                    <span className="text-muted-foreground text-xs"> ({t.unit})</span>
                  ) : null}
                </span>
                <span className="text-muted-foreground text-xs">
                  objetivo {t.mean} ± {t.sd}
                </span>
                <Input
                  type="number"
                  step="any"
                  inputMode="decimal"
                  autoFocus={t === targets[0]}
                  className="h-8 w-28 font-mono text-xs"
                  placeholder="Valor medido"
                  value={runValues[t.analyteId] ?? ""}
                  onChange={(e) =>
                    setRunValues((prev) => ({ ...prev, [t.analyteId]: e.target.value }))
                  }
                />
              </label>
            ))}
            <Textarea
              rows={2}
              className="resize-none text-xs"
              placeholder="Notas de la corrida (reactivo nuevo, calibración…) — opcional"
              value={runNotes}
              onChange={(e) => setRunNotes(e.target.value)}
            />
            <p className="flex items-start gap-1.5 text-muted-foreground text-[11px]">
              <AlertTriangle className="mt-0.5 size-3 shrink-0" />
              Si la corrida se rechaza, no proceses muestras de pacientes con
              ese equipo hasta corregir el problema (calibración, reactivo…).
            </p>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setRunOpen(false)}>
              Cancelar
            </Button>
            <Button onClick={submitRun} disabled={recordRun.isPending}>
              {recordRun.isPending ? <Loader2 className="size-4 animate-spin" /> : <CheckCircle2 className="size-4" />}
              Evaluar y guardar
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}