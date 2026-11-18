import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { CreateStockMovementInput, SaveProductInput } from "@/bindings";

/** Catálogo de productos con su stock actual. */
export function useProducts(
  search = "",
  category: string | null = null,
  includeInactive = false,
) {
  return useQuery({
    queryKey: ["products", search, category, includeInactive],
    queryFn: () =>
      api.listProducts(
        search.trim() ? search.trim() : null,
        category,
        includeInactive,
      ),
    placeholderData: (prev) => prev,
  });
}

/** Producto del catálogo por id. */
export function useProduct(id: number | null) {
  return useQuery({
    queryKey: ["product", id],
    queryFn: () => api.getProduct(id!),
    enabled: id != null,
  });
}

/** Histórico de movimientos de inventario. */
export function useStockMovements(
  productId: number | null = null,
  limit = 50,
) {
  return useQuery({
    queryKey: ["stock-movements", productId, limit],
    queryFn: () => api.listStockMovements(productId, limit),
    placeholderData: (prev) => prev,
  });
}
export function useSaveProduct() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: SaveProductInput) => api.saveProduct(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["products"] });
      qc.invalidateQueries({ queryKey: ["product"] });
    },
  });
}

export function useSetProductActive() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, active }: { id: number; active: boolean }) =>
      api.setProductActive(id, active),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["products"] });
      qc.invalidateQueries({ queryKey: ["product"] });
    },
  });
}

export function useAddStockMovement() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateStockMovementInput) =>
      api.addStockMovement(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["products"] });
      qc.invalidateQueries({ queryKey: ["stock-movements"] });
      qc.invalidateQueries({ queryKey: ["product"] });
    },
  });
}
