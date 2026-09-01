import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { CreateInvoiceInput } from "@/bindings";

export function useInvoices(status: string | null, search: string, enabled = true) {
  return useQuery({
    queryKey: ["invoices", status, search],
    queryFn: () => api.listInvoices(status, search.trim() || null),
    placeholderData: (prev) => prev,
    enabled,
  });
}

/** Contadores por estado (sin filtros) para las pestañas de facturación. */
export function useInvoiceCounts() {
  return useQuery({
    queryKey: ["invoice-counts"],
    queryFn: api.countInvoices,
  });
}

export function useInvoice(id: number | null) {
  return useQuery({
    queryKey: ["invoice", id],
    queryFn: () => api.getInvoice(id!),
    enabled: id != null,
  });
}

export function useCreateInvoice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateInvoiceInput) => api.createInvoice(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["invoices"] });
      qc.invalidateQueries({ queryKey: ["invoice-counts"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export function useSetInvoiceStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: string }) =>
      api.setInvoiceStatus(id, status),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["invoices"] });
      qc.invalidateQueries({ queryKey: ["invoice-counts"] });
      qc.invalidateQueries({ queryKey: ["invoice"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}
