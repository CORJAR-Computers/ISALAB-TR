import { useState } from "react";
import { Plus, Search, Syringe } from "lucide-react";
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
import { useVaccines } from "@/hooks/use-queries";
import { formatDateTime } from "@/lib/utils";
import { NewVaccineDialog } from "./NewVaccineDialog";

export function VaccinesPage() {
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const { data: vaccines, isLoading, isError } = useVaccines(search);

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Vacunación y desparasitación
          </h2>
          <p className="text-muted-foreground text-sm">
            Esquemas de vacunación por paciente con control de refuerzos.
          </p>
        </div>
        <Button onClick={() => setDialogOpen(true)}>
          <Plus className="size-4" />
          Registrar vacuna
        </Button>
      </div>

      <div className="relative max-w-sm">
        <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
        <Input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Buscar por paciente, propietario o vacuna…"
          className="pl-9"
        />
      </div>

      <Card className="gap-0 p-0">
        <CardHeader className="border-b">
          <CardTitle className="flex items-center gap-2 text-base">
            <Syringe className="size-4 text-success" />
            Historial de vacunación
          </CardTitle>
          <CardDescription>
            {isLoading
              ? "Cargando…"
              : `${vaccines?.length ?? 0} registro${(vaccines?.length ?? 0) === 1 ? "" : "s"}`}
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Paciente</TableHead>
                <TableHead>Vacuna</TableHead>
                <TableHead>Aplicada</TableHead>
                <TableHead>Refuerzo</TableHead>
                <TableHead>Lote</TableHead>
                <TableHead>Registrado por</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading &&
                Array.from({ length: 5 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell colSpan={6} className="h-12">
                      <Skeleton className="h-8 w-full" />
                    </TableCell>
                  </TableRow>
                ))}

              {!isLoading && isError && (
                <TableRow>
                  <TableCell colSpan={6} className="text-muted-foreground h-16 text-center">
                    No se pudo cargar el historial de vacunación.
                  </TableCell>
                </TableRow>
              )}

              {!isLoading && vaccines?.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} className="text-muted-foreground h-24 text-center">
                    {search
                      ? "Ningún registro coincide con la búsqueda."
                      : "Sin vacunas registradas. Usa “Registrar vacuna” para iniciar el esquema."}
                  </TableCell>
                </TableRow>
              )}

              {vaccines?.map((v) => (
                <TableRow key={v.id}>
                  <TableCell>
                    <div className="flex flex-col">
                      <span className="font-medium">{v.patientName}</span>
                      <span className="text-muted-foreground text-xs">
                        {v.speciesName} · {v.ownerName}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <span className="font-medium">{v.vaccineName}</span>
                    {v.manufacturer && (
                      <span className="text-muted-foreground block text-xs">
                        {v.manufacturer}
                      </span>
                    )}
                  </TableCell>
                  <TableCell className="text-muted-foreground text-xs">
                    {formatDateTime(v.administeredAt)}
                  </TableCell>
                  <TableCell>
                    {v.nextDoseAt ? (
                      <Badge variant="warning">Refuerzo: {v.nextDoseAt}</Badge>
                    ) : (
                      <span className="text-muted-foreground text-xs">—</span>
                    )}
                  </TableCell>
                  <TableCell className="font-mono text-xs">
                    {v.lot ?? "—"}
                  </TableCell>
                  <TableCell className="text-muted-foreground text-xs">
                    {v.veterinarianName ?? "—"}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <NewVaccineDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
      />
    </div>
  );
}
