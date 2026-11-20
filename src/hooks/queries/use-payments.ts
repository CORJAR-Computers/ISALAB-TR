import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { RegisterPaymentInput } from "@/bindings";

/** Catálogo de formas de pago (efectivo, transferencia, tarjeta…). */
export function usePaymentMethods() {
  return useQuery({
    queryKey: ["payment-methods"],
    queryFn: () => api.listPaymentMethods(false),
    staleTime: 5 * 60 * 1000,
  });
}

/** Saldo derivado de una factura (total, abonado, saldo y estado de cobro). */
export function useInvoiceBalance(invoiceId: number | null) {
  return useQuery({
    queryKey: ["invoice-balance", invoiceId],
    queryFn: () => api.getInvoiceBalance(invoiceId!),
    enabled: invoiceId != null,
  });
}

/** Historial de abonos aplicados a una factura. */
export function useInvoicePayments(invoiceId: number | null) {
  return useQuery({
    queryKey: ["invoice-payments", invoiceId],
    queryFn: () => api.listInvoicePayments(invoiceId!),
    enabled: invoiceId != null,
  });
}

/** Caja abierta (null si no hay ninguna): cobrar exige una sesión abierta. */
export function useOpenCashSession() {
  return useQuery({
    queryKey: ["cash-session-open"],
    queryFn: api.getOpenCashSession,
  });
}

export function useOpenCash() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({
      openingFloat,
      notes,
    }: {
      openingFloat: number;
      notes: string | null;
    }) => api.openCashSession(openingFloat, notes),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["cash-session-open"] }),
  });
}

export function useRegisterPayment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: RegisterPaymentInput) => api.registerPayment(input),
    onSuccess: (_payment, vars) => {
      qc.invalidateQueries({ queryKey: ["invoices"] });
      qc.invalidateQueries({ queryKey: ["invoice-counts"] });
      qc.invalidateQueries({ queryKey: ["invoice"] });
      for (const allocation of vars.allocations) {
        qc.invalidateQueries({
          queryKey: ["invoice-balance", allocation.invoiceId],
        });
        qc.invalidateQueries({
          queryKey: ["invoice-payments", allocation.invoiceId],
        });
      }
      qc.invalidateQueries({ queryKey: ["dashboard"] });
      qc.invalidateQueries({ queryKey: ["cash-session-open"] });
      qc.invalidateQueries({ queryKey: ["cash-summary"] });
      qc.invalidateQueries({ queryKey: ["cash-movements"] });
      qc.invalidateQueries({ queryKey: ["billing-report"] });
    },
  });
}

/**
 * Anula una recepción con motivo. El backend revierte el efectivo en la caja
 * abierta y devuelve las facturas a su saldo real, así que se invalidan tanto
 * las facturas afectadas como la caja y el dashboard.
 */
export function useVoidPayment() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ paymentId, reason }: { paymentId: number; reason: string }) =>
      api.voidPayment(paymentId, reason),
    onSuccess: (payment) => {
      qc.invalidateQueries({ queryKey: ["invoices"] });
      qc.invalidateQueries({ queryKey: ["invoice-counts"] });
      qc.invalidateQueries({ queryKey: ["invoice"] });
      for (const allocation of payment.allocations) {
        qc.invalidateQueries({
          queryKey: ["invoice-balance", allocation.invoiceId],
        });
        qc.invalidateQueries({
          queryKey: ["invoice-payments", allocation.invoiceId],
        });
      }
      qc.invalidateQueries({ queryKey: ["dashboard"] });
      qc.invalidateQueries({ queryKey: ["cash-summary"] });
      qc.invalidateQueries({ queryKey: ["cash-movements"] });
      qc.invalidateQueries({ queryKey: ["billing-report"] });
    },
  });
}
