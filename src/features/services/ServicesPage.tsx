import { useMemo, useState } from "react";
import { Pencil, Plus, Power, Search } from "lucide-react";
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
  useAllServiceCategories,
  useServices,
  useSetServiceActive,
} from "@/hooks/use-queries";
import { cn, formatCOP } from "@/lib/utils";
import { ServiceDialog } from "./ServiceDialog";
import type { Service } from "@/bindings";
import { toast } from "sonner";

export function ServicesPage() {
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [search, setSearch] = useState("");
  const [showInactive, setShowInactive] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editing, setEditing] = useState<Service | null>(null);

  const { data: categories = [] } = useAllServiceCategories();
  const {
    data: services,
    isLoading,
    isError,
  } = useServices(categoryId, search, showInactive);
  const setActive = useSetServiceActive();

  // Filtro local por estado (activo/inactivo) sobre el resultado del backend.
  const filtered = useMemo(() => {
    const list = services ?? [];
    return showInactive ? list : list.filter((s) => s.isActive);
  }, [services, showInactive]);

  // Agrupa por categoría para tablas independientes.
  const grouped = useMemo(() => {
    const map = new Map<string, { name: string; items: Service[] }>();
    for (const cat of categories) {
      map.set(String(cat.id), { name: cat.name, items: [] });
    }
    for (const s of filtered) {
      const key = String(s.categoryId);
      const group = map.get(key);
      if (group) {
        group.items.push(s);
      } else {
        map.set(key, { name: s.categoryName, items: [s] });
      }
    }
    return [...map.entries()].filter(([, g]) => g.items.length > 0);
  }, [filtered, categories]);

  const handleToggleActive = async (service: Service) => {
    try {
      await setActive.mutateAsync({ id: service.id, active: !service.isActive });
      toast.success(
        service.isActive ? "Servicio desactivado" : "Servicio activado",
        { description: service.name },
      );
    } catch (e) {
      toast.error("No se pudo cambiar el estado", { description: String(e) });
    }
  };

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">
            Catálogo de servicios
          </h2>
          <p className="text-muted-foreground text-sm">
            Única fuente de precio, duración e IVA de toda la clínica:
            consultas, vacunación, cirugía, laboratorio, peluquería…
          </p>
        </div>
        <Button
          onClick={() => {
            setEditing(null);
            setDialogOpen(true);
          }}
        >
          <Plus className="size-4" />
          Nuevo servicio
        </Button>
      </div>

      {/* Filtros */}
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center">
        <div className="flex flex-wrap gap-1 rounded-lg border bg-card p-1">
          <button
            type="button"
            onClick={() => setCategoryId(null)}
            className={cn(
              "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
              categoryId === null
                ? "bg-primary text-primary-foreground shadow-sm"
                : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
            )}
          >
            Todas
          </button>
          {categories.map((c) => (
            <button
              key={c.id}
              type="button"
              onClick={() => setCategoryId(c.id)}
              className={cn(
                "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                categoryId === c.id
                  ? "bg-primary text-primary-foreground shadow-sm"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
              )}
            >
              {c.name}
            </button>
          ))}
        </div>

        <div className="relative lg:ml-auto lg:w-64">
          <Search className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Buscar por código o nombre…"
            className="pl-9"
          />
        </div>

        <button
          type="button"
          onClick={() => setShowInactive((v) => !v)}
          className={cn(
            "rounded-lg border px-3 py-1.5 text-sm font-medium transition-colors",
            showInactive
              ? "border-primary/40 bg-primary/10 text-foreground"
              : "text-muted-foreground hover:bg-accent",
          )}
        >
          Mostrar inactivos
        </button>
      </div>

      {isLoading && (
        <div className="space-y-3">
          {[1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-40 w-full" />
          ))}
        </div>
      )}

      {isError && (
        <Card>
          <CardContent className="py-8 text-center text-sm text-destructive">
            No se pudo cargar el catálogo. Reintenta o revisa la base de datos.
          </CardContent>
        </Card>
      )}

      {!isLoading && !isError && filtered.length === 0 && (
        <Card>
          <CardContent className="text-muted-foreground py-8 text-center text-sm">
            No hay servicios con esos filtros. Crea el primero con "Nuevo
            servicio".
          </CardContent>
        </Card>
      )}

      {!isLoading &&
        !isError &&
        grouped.map(([key, group]) => (
          <Card key={key}>
            <CardHeader>
              <CardTitle className="text-base">{group.name}</CardTitle>
              <CardDescription>
                {group.items.length} servicio{group.items.length === 1 ? "" : "s"}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Código</TableHead>
                    <TableHead>Nombre</TableHead>
                    <TableHead className="text-right">Duración</TableHead>
                    <TableHead className="text-right">Precio</TableHead>
                    <TableHead className="text-center">IVA</TableHead>
                    <TableHead className="text-center">Requiere</TableHead>
                    <TableHead className="text-center">Estado</TableHead>
                    <TableHead className="text-right">Acciones</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {group.items.map((s) => (
                    <TableRow key={s.id} className={cn(!s.isActive && "opacity-60")}>
                      <TableCell className="font-mono text-xs">{s.code}</TableCell>
                      <TableCell>
                        <span className="font-medium">{s.name}</span>
                        {s.description && (
                          <p className="text-muted-foreground line-clamp-1 text-xs">
                            {s.description}
                          </p>
                        )}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        {s.durationMin} min
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        {formatCOP(s.price ?? 0)}
                      </TableCell>
                      <TableCell className="text-center">
                        {s.taxRate != null ? `${s.taxRate}%` : "Global"}
                      </TableCell>
                      <TableCell className="text-center">
                        <div className="flex justify-center gap-1">
                          {s.requiresVet && <Badge variant="outline">MV</Badge>}
                          {s.requiresGroomer && <Badge variant="outline">Pel</Badge>}
                          {!s.requiresVet && !s.requiresGroomer && (
                            <span className="text-muted-foreground text-xs">—</span>
                          )}
                        </div>
                      </TableCell>
                      <TableCell className="text-center">
                        <Badge variant={s.isActive ? "default" : "secondary"}>
                          {s.isActive ? "Activo" : "Inactivo"}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => {
                              setEditing(s);
                              setDialogOpen(true);
                            }}
                            title="Editar"
                            aria-label={`Editar ${s.name}`}
                          >
                            <Pencil className="size-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => handleToggleActive(s)}
                            disabled={setActive.isPending}
                            title={s.isActive ? "Desactivar" : "Activar"}
                            aria-label={
                              s.isActive
                                ? `Desactivar ${s.name}`
                                : `Activar ${s.name}`
                            }
                          >
                            <Power
                              className={cn(
                                "size-4",
                                s.isActive ? "text-success" : "text-muted-foreground",
                              )}
                            />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        ))}

      <ServiceDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        editing={editing}
      />
    </div>
  );
}
