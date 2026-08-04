import { useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceArea,
} from "recharts";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useAnalytes } from "@/hooks/use-queries";
import { Loader2, TrendingUp } from "lucide-react";

export type TrendPoint = {
  date: string;
  value: number;
  refMin: number | null;
  refMax: number | null;
  status: string;
};

export function PatientTrendsChart({ patientId }: { patientId: number }) {
  const { data: analytes = [] } = useAnalytes();
  const [analyteId, setAnalyteId] = useState<number | null>(null);

  const { data: trends, isLoading } = useQuery<TrendPoint[]>({
    queryKey: ["patient-trends", patientId, analyteId],
    queryFn: async () => {
      if (!analyteId) return [];
      return await invoke("get_patient_lab_trends", {
        patientId,
        analyteId,
      });
    },
    enabled: analyteId !== null,
  });

  const selectedAnalyte = analytes.find((a) => a.id === analyteId);

  // Determinar los límites del rango normal para pintarlo de fondo
  const rangeAreas = useMemo(() => {
    if (!trends || trends.length === 0) return null;
    const firstWithRange = trends.find((t) => t.refMin != null && t.refMax != null);
    if (!firstWithRange) return null;
    return { min: firstWithRange.refMin, max: firstWithRange.refMax };
  }, [trends]);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base flex items-center gap-2">
          <TrendingUp className="size-5 text-primary" />
          Evolución y Tendencias
        </CardTitle>
        <CardDescription>
          Selecciona un analito para ver su comportamiento histórico en los exámenes de este paciente.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="w-full max-w-sm space-y-1.5">
          <Label>Analito a graficar</Label>
          <Select
            value={analyteId?.toString() ?? ""}
            onValueChange={(v) => setAnalyteId(Number(v))}
          >
            <SelectTrigger>
              <SelectValue placeholder="Seleccionar analito..." />
            </SelectTrigger>
            <SelectContent>
              {analytes.map((a) => (
                <SelectItem key={a.id} value={a.id.toString()}>
                  {a.name} {a.unit ? `(${a.unit})` : ""}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {isLoading ? (
          <div className="h-64 flex items-center justify-center border rounded-md bg-muted/20">
            <Loader2 className="animate-spin text-muted-foreground size-8" />
          </div>
        ) : analyteId && trends && trends.length === 0 ? (
          <div className="h-64 flex flex-col items-center justify-center border rounded-md bg-muted/20 text-muted-foreground text-sm">
            <p>No hay resultados históricos para este analito en muestras finalizadas.</p>
          </div>
        ) : analyteId && trends && trends.length > 0 ? (
          <div className="h-72 w-full">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart
                data={trends}
                margin={{ top: 20, right: 30, left: 20, bottom: 20 }}
              >
                <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#e5e7eb" />
                <XAxis
                  dataKey="date"
                  tick={{ fontSize: 12 }}
                  tickMargin={10}
                  stroke="#9ca3af"
                />
                <YAxis
                  tick={{ fontSize: 12 }}
                  stroke="#9ca3af"
                  domain={["auto", "auto"]}
                />
                <Tooltip
                  contentStyle={{ borderRadius: "8px", border: "1px solid #e5e7eb" }}
                  formatter={(value) => [`${value} ${selectedAnalyte?.unit ?? ""}`, "Resultado"]}
                  labelFormatter={(label) => `Fecha: ${label}`}
                />
                
                {/* Zona de referencia normal (verde claro) */}
                {rangeAreas?.min != null && rangeAreas?.max != null && (
                  <ReferenceArea
                    y1={rangeAreas.min}
                    y2={rangeAreas.max}
                    fill="#10b981"
                    fillOpacity={0.1}
                    strokeOpacity={0}
                  />
                )}
                
                <Line
                  type="monotone"
                  dataKey="value"
                  stroke="#3b82f6"
                  strokeWidth={3}
                  activeDot={{ r: 8 }}
                  dot={{ r: 4, strokeWidth: 2 }}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        ) : (
          <div className="h-64 flex flex-col items-center justify-center border rounded-md bg-muted/20 text-muted-foreground text-sm border-dashed">
            Selecciona un analito para cargar la gráfica
          </div>
        )}
      </CardContent>
    </Card>
  );
}
