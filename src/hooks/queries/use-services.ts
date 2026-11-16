import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { SaveServiceInput } from "@/bindings";

/** Categorías activas del catálogo (lectura operativa). */
export function useServiceCategories(enabled = true) {
  return useQuery({
    queryKey: ["service-categories"],
    queryFn: api.listServiceCategories,
    enabled,
  });
}

/** Todas las categorías, incluidas inactivas (solo ADMIN). */
export function useAllServiceCategories(enabled = true) {
  return useQuery({
    queryKey: ["service-categories", "all"],
    queryFn: api.listAllServiceCategories,
    enabled,
  });
}

/**
 * Servicios del catálogo con filtros. `includeInactive` solo surte efecto
 * para ADMIN (el backend lo ignora para otros roles).
 */
export function useServices(
  categoryId: number | null,
  search: string,
  includeInactive = false,
  enabled = true,
) {
  return useQuery({
    queryKey: ["services", categoryId, search.trim(), includeInactive],
    queryFn: () =>
      api.listServices(categoryId, search.trim() || null, includeInactive),
    placeholderData: (prev) => prev,
    enabled,
  });
}

/** Servicio puntual (p. ej. para precargar un diálogo de edición). */
export function useService(id: number | null) {
  return useQuery({
    queryKey: ["service", id],
    queryFn: () => api.getService(id!),
    enabled: id != null,
  });
}

export function useSaveService() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: SaveServiceInput) => api.saveService(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["services"] });
      qc.invalidateQueries({ queryKey: ["service-categories"] });
    },
  });
}

export function useSaveServiceCategory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: {
      id: number | null;
      code: string;
      name: string;
      sortOrder: number;
    }) => api.saveServiceCategory(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["service-categories"] });
      qc.invalidateQueries({ queryKey: ["services"] });
    },
  });
}

export function useDeleteServiceCategory() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteServiceCategory(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["service-categories"] });
      qc.invalidateQueries({ queryKey: ["services"] });
    },
  });
}

export function useSetServiceActive() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, active }: { id: number; active: boolean }) =>
      api.setServiceActive(id, active),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["services"] });
      qc.invalidateQueries({ queryKey: ["service-categories"] });
    },
  });
}
