import { useState } from "react";
import {
  ArrowDownUp,
  History,
  Package,
  Pencil,
  Plus,
  Power,
  Search,
  TriangleAlert,
} from "lucide-react";
import { toast } from "sonner";
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import {
  useProducts,
  useSetProductActive,
  useStockMovements,
} from "@/hooks/use-queries";
import { usePermissions } from "@/hooks/use-permissions";
import { cn, formatCOP, formatDateTime } from "@/lib/utils";
import {
  PRODUCT_CATEGORY,
  PRODUCT_UNIT,
  STOCK_MOVEMENT_KIND,
} from "@/lib/status";
import type { Product } from "@/bindings";
import { ProductDialog } from "./ProductDialog";
import { StockMovementDialog } from "./StockMovementDialog";

export function ProductsPage() {
  const { canInventory, isAdmin } = usePermissions();
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<string | null>(null);
  const [showInactive, setShowInactive] = useState(false);
  const [productDialogOpen, setProductDialogOpen] = useState(false);
  const [editing, setEditing] = useState<Product | null>(null);
  const [movementOpen, setMovementOpen] = useState(false);
  const [movementProductId, setMovementProductId] = useState<number | null>(null);
  const [historyProductId, setHistoryProductId] = useState<number | null>(null);

  const {
    data: products = [],
    isLoading,
    isError,
  } = useProducts(search, category, showInactive);
  const setActive = useSetProductActive();
  const { data: movements = [] } = useStockMovements(historyProductId, 100);

  const lowStock = products.filter((p) => p.lowStock);
  const stockValue = products.reduce(
    (acc, p) => acc + (p.stockValue ?? 0),
    0,
  );

  const openProductDialog = (product: Product | null) => {
    setEditing(product);
    setProductDialogOpen(true);
  };

  const openMovementDialog = (productId: number | null) => {
    setMovementProductId(productId);
    setMovementOpen(true);
  };

  const handleToggleActive = async (product: Product) => {
    try {
      await setActive.mutateAsync({
        id: product.id,
        active: !product.active,
      });
      toast.success(
        product.active ? "Producto desactivado" : "Producto activado",
        { description: product.name },
      );
    } catch (e) {
      toast.error("No se pudo cambiar el estado", { description: String(e) });
    }
  };

  if (!canInventory) {
    return (
      <Card>
        <CardContent className="py-10 text-center text-sm text-muted-foreground">
          Tu rol no tiene acceso al inventario de la clínica.
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 text-xl font-semibold tracking-tight">
            <Package className="text-primary size-5" />
            Inventario y productos
          </h2>
          <p className="text-muted-foreground text-sm">
            Alimentos, medicamentos, accesorios e insumos: cada movimiento
            suma o resta stock, y el histórico siempre cuadra con el
            inventario.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            type="button"
            variant="outline"
            onClick={() => openMovementDialog(null)}
          >
            <ArrowDownUp className="size-4" />
            Movimiento
          </Button>
          {isAdmin && (
            <Button type="button" onClick={() => openProductDialog(null)}>
              <Plus className="size-4" />
              Nuevo producto
            </Button>
          )}
        </div>
      </div>

      {/* Indicadores */}
      <div className="grid gap-3 sm:grid-cols-3">
        <Card>
          <CardContent className="pt-4">
            <p className="text-muted-foreground text-xs uppercase tracking-wide">
              Productos activos
            </p>
            <p className="text-2xl font-semibold tabular-nums">
              {products.filter((p) => p.product.active).length}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4">
            <p className="text-muted-foreground text-xs uppercase tracking-wide">
              Alertas de reposición
            </p>
            <p
              className={cn(
                "text-2xl font-semibold tabular-nums",
                lowStock.length > 0 && "text-warning",
              )}
            >
              {lowStock.length}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4">
            <p className="text-muted-foreground text-xs uppercase tracking-wide">
              Valor del inventario (costo)
            </p>
            <p className="text-2xl font-semibold tabular-nums">
              {formatCOP(stockValue)}
            </p>
          </CardContent>
        </Card>
      </div>

      {lowStock.length > 0 && (
        <Card className="border-warning/50 bg-warning/5">
          <CardContent className="flex items-start gap-3 pt-4">
            <TriangleAlert className="mt-0.5 size-4 shrink-0 text-warning" />
            <div className="min-w-0">
              <p className="text-sm font-semibold">Stock por reponer</p>
              <p className="text-muted-foreground text-xs">
                {lowStock
                  .map(
                    (p) =>
                      `${p.product.name} (${p.stock}/${p.product.minStock})`,
                  )
                  .join(" · ")}
              </p>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Filtros */}
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center">
        <div className="flex flex-wrap gap-1 rounded-lg border bg-card p-1">
          <button
            type="button"
            onClick={() => setCategory(null)}
            className={cn(
              "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
              category === null
                ? "bg-primary text-primary-foreground shadow-sm"
                : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
            )}
          >
            Todas
          </button>
          {Object.entries(PRODUCT_CATEGORY).map(([code, label]) => (
            <button
              key={code}
              type="button"
              onClick={() => setCategory(code)}
              className={cn(
                "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                category === code
                  ? "bg-primary text-primary-foreground shadow-sm"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
              )}
            >
              {label}
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
            No se pudo cargar el inventario. Reintenta o revisa la base de
            datos.
          </CardContent>
        </Card>
      )}

      {!isLoading && !isError && products.length === 0 && (
        <Card>
          <CardContent className="text-muted-foreground py-8 text-center text-sm">
            No hay productos con esos filtros. Crea el primero con «Nuevo
            producto».
          </CardContent>
        </Card>
      )}

      {!isLoading && !isError && products.length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">Catálogo</CardTitle>
            <CardDescription>
              El stock es la suma de los movimientos registrados.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Código</TableHead>
                  <TableHead>Producto</TableHead>
                  <TableHead>Categoría</TableHead>
                  <TableHead className="text-right">Stock</TableHead>
                  <TableHead className="text-right">Costo</TableHead>
                  <TableHead className="text-right">Venta</TableHead>
                  <TableHead className="text-right">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {products.map((item) => {
                  const p = item.product;
                  return (
                    <TableRow
                      key={p.id}
                      className={cn(!p.active && "opacity-60")}
                    >
                      <TableCell className="font-mono text-xs">{p.code}</TableCell>
                      <TableCell>
                        <span className="font-medium">{p.name}</span>
                        <p className="text-muted-foreground text-xs">
                          {PRODUCT_UNIT[p.unit] ?? p.unit}
                          {p.notes ? ` · ${p.notes}` : ""}
                        </p>
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">
                          {PRODUCT_CATEGORY[p.category] ?? p.category}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        <span
                          className={cn(
                            "font-medium",
                            item.lowStock && "text-warning",
                          )}
                        >
                          {item.stock}
                        </span>
                        {item.lowStock && (
                          <p className="text-warning text-xs">
                            mín. {p.minStock}
                          </p>
                        )}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        {formatCOP(p.costPrice ?? 0)}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">
                        {formatCOP(p.salePrice ?? 0)}
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => openMovementDialog(p.id)}
                            title="Registrar movimiento"
                            aria-label={`Movimiento de ${p.name}`}
                          >
                            <ArrowDownUp className="size-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => setHistoryProductId(p.id)}
                            title="Ver movimientos"
                            aria-label={`Movimientos de ${p.name}`}
                          >
                            <History className="size-4" />
                          </Button>
                          {isAdmin && (
                            <>
                              <Button
                                variant="ghost"
                                size="icon"
                                onClick={() => openProductDialog(p)}
                                title="Editar"
                                aria-label={`Editar ${p.name}`}
                              >
                                <Pencil className="size-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="icon"
                                onClick={() => handleToggleActive(p)}
                                disabled={setActive.isPending}
                                title={p.active ? "Desactivar" : "Activar"}
                                aria-label={
                                  p.active
                                    ? `Desactivar ${p.name}`
                                    : `Activar ${p.name}`
                                }
                              >
                                <Power
                                  className={cn(
                                    "size-4",
                                    p.active
                                      ? "text-success"
                                      : "text-muted-foreground",
                                  )}
                                />
                              </Button>
                            </>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}

      {/* Historial de movimientos */}
      <Dialog
        open={historyProductId != null}
        onOpenChange={(open) => !open && setHistoryProductId(null)}
      >
        <DialogContent className="max-h-[80vh] overflow-y-auto sm:max-w-3xl">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <History className="text-primary size-5" />
              Movimientos de inventario
            </DialogTitle>
            <DialogDescription>
              Entradas, salidas, consumos, devoluciones y ajustes del producto
              seleccionado.
            </DialogDescription>
          </DialogHeader>

          {movements.length === 0 ? (
            <p className="text-muted-foreground py-6 text-center text-sm">
              Sin movimientos registrados.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Fecha</TableHead>
                  <TableHead>Producto</TableHead>
                  <TableHead>Tipo</TableHead>
                  <TableHead className="text-right">Cantidad</TableHead>
                  <TableHead>Motivo</TableHead>
                  <TableHead>Registrado por</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {movements.map((m) => {
                  const meta = STOCK_MOVEMENT_KIND[m.kind] ?? {
                    label: m.kind,
                    variant: "secondary" as const,
                  };
                  return (
                    <TableRow key={m.id}>
                      <TableCell className="text-sm">
                        {formatDateTime(m.createdAt)}
                      </TableCell>
                      <TableCell className="text-sm">
                        {m.productName}
                      </TableCell>
                      <TableCell>
                        <Badge variant={meta.variant}>{meta.label}</Badge>
                      </TableCell>
                      <TableCell
                        className={cn(
                          "text-right font-medium tabular-nums",
                          m.quantity > 0
                            ? "text-success"
                            : "text-destructive",
                        )}
                      >
                        {m.quantity > 0 ? "+" : ""}
                        {m.quantity}
                      </TableCell>
                      <TableCell className="text-muted-foreground text-sm">
                        {m.reason ?? "—"}
                      </TableCell>
                      <TableCell className="text-sm">
                        {m.authorName ?? "—"}
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </DialogContent>
      </Dialog>

      <ProductDialog
        open={productDialogOpen}
        onOpenChange={setProductDialogOpen}
        editing={editing}
      />
      <StockMovementDialog
        open={movementOpen}
        onOpenChange={setMovementOpen}
        presetProductId={movementProductId}
      />
    </div>
  );
}
