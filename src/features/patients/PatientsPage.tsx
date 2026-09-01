import { useEffect, useState } from "react";
import { HeartPulse, Plus, Search } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
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
import { usePatients } from "@/hooks/use-queries";
import { formatAge } from "@/lib/utils";
import { SEX_LABEL } from "@/lib/status";
import { useUiStore } from "@/stores/ui-store";
import { NewPatientDialog } from "./NewPatientDialog";
import { PatientScanner } from "./PatientScanner";

export function PatientsPage() {
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const { data, isLoading, isError } = usePatients(debouncedSearch);
  const setActivePatient = useUiStore((s) => s.setActivePatient);
  const navigate = useUiStore((s) => s.navigate);
  const newPatientRequest = useUiStore((s) => s.newPatientRequest);
  const consumeNewPatientRequest = useUiStore(
    (s) => s.consumeNewPatientRequest,
  );

  // Debounce real: la consulta a la BD se dispara 250 ms después de que el
  // usuario deja de escribir. El input se enlaza a `search` (sin trim), de
  // modo que se puedan escribir espacios (p. ej. nombres de varias palabras).
  useEffect(() => {
    const t = setTimeout(() => setDebouncedSearch(search.trim()), 250);
    return () => clearTimeout(t);
  }, [search]);

  // Abre el diálogo cuando llega una solicitud externa (p. ej. dashboard).
  useEffect(() => {
    if (newPatientRequest > 0) {
      setDialogOpen(true);
      consumeNewPatientRequest();
    }
  }, [newPatientRequest, consumeNewPatientRequest]);

  const openHistory = (id: number) => {
    setActivePatient(id);
    navigate("clinical-history");
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">Pacientes</h2>
          <p className="text-muted-foreground text-sm">
            Busca por código PAC-, nombre, propietario, microchip o documento.
          </p>
        </div>
        <Button onClick={() => setDialogOpen(true)}>
          <Plus className="size-4" />
          Nuevo paciente
        </Button>
      </div>

      <PatientScanner onFound={openHistory} />

      <div className="relative max-w-sm">
        <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
        <Input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="PAC-2026-0001 o buscar por nombre…"
          className="pl-9"
        />
      </div>

      <Card className="gap-0 p-0">
        <CardHeader className="border-b">
          <CardTitle className="text-base">Listado</CardTitle>
          <CardDescription>
            {data && data.length > 0
              ? `${data.length} resultado${data.length === 1 ? "" : "s"}`
              : "Sin resultados"}
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead className="w-36">Código</TableHead>
                <TableHead>Paciente</TableHead>
                <TableHead>Especie / Raza</TableHead>
                <TableHead>Sexo</TableHead>
                <TableHead>Edad</TableHead>
                <TableHead>Propietario</TableHead>
                <TableHead className="text-right">Historial</TableHead>
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
                    No se pudo cargar la lista de pacientes.
                  </TableCell>
                </TableRow>
              )}

              {!isLoading && data && data.length === 0 && (
                <TableRow>
                  <TableCell colSpan={7} className="text-muted-foreground h-24 text-center">
                    {search
                      ? "Ningún paciente coincide con la búsqueda."
                      : "Aún no hay pacientes registrados. Crea el primero."}
                  </TableCell>
                </TableRow>
              )}

              {data?.map((p) => (
                <TableRow
                  key={p.id}
                  className="cursor-pointer"
                  onClick={() => openHistory(p.id)}
                >
                  <TableCell>
                    <Badge
                      variant="outline"
                      className="font-mono text-xs tracking-wide"
                    >
                      {p.code}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex flex-col">
                      <span className="font-medium">{p.name}</span>
                      <span className="text-muted-foreground text-xs">
                        {p.active ? "Activo" : "Inactivo"}
                        {p.microchip ? ` · Chip ${p.microchip}` : ""}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary">{p.speciesName}</Badge>
                    {p.breedName && (
                      <span className="text-muted-foreground ml-1.5 text-xs">
                        {p.breedName}
                      </span>
                    )}
                  </TableCell>
                  <TableCell>{SEX_LABEL[p.sex] ?? p.sex}</TableCell>
                  <TableCell>{formatAge(p.birthDate)}</TableCell>
                  <TableCell>
                    <div className="flex flex-col">
                      <span>{p.ownerName}</span>
                      {p.ownerPhone && (
                        <span className="text-muted-foreground text-xs">
                          {p.ownerPhone}
                        </span>
                      )}
                    </div>
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        openHistory(p.id);
                      }}
                    >
                      <HeartPulse className="size-4" />
                      Ver
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <NewPatientDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onCreated={(id) => {
          setDialogOpen(false);
          setActivePatient(id);
          navigate("clinical-history");
        }}
      />
    </div>
  );
}
