import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { AddCashMovementInput, CloseCashInput } from "@/bindings";

/** Resumen de la caja abierta (o de una sesión concreta). */
export function useCashSummary(
  sessionId: number | null = null,
  enabled = true,
) {
  return useQuery({
    queryKey: ["cash-summary", sessionId ?? "open"],
    queryFn: () => api.getCashSessionSummary(sessionId),
    enabled,
    refetchInterval: 30 * 1000,
  });
}

/** Movimientos de caja de la sesión abierta (o de una cerrada). */
export function useCashMovements(
  sessionId: number | null = null,
  enabled = true,
) {
  return useQuery({
    queryKey: ["cash-movements", sessionId ?? "open"],
    queryFn: () => api.listCashMovements(sessionId),
    enabled,
  });
}

/** Historial de sesiones de caja. */
export function useCashSessions(limit = 30) {
  return useQuery({
    queryKey: ["cash-sessions", limit],
    queryFn: () => api.listCashSessions(limit),
  });
}

function useInvalidateCash() {
  const qc = useQueryClient();
  return () => {
    qc.invalidateQueries({ queryKey: ["cash-summary"] });
    qc.invalidateQueries({ queryKey: ["cash-movements"] });
    qc.invalidateQueries({ queryKey: ["cash-sessions"] });
    qc.invalidateQueries({ queryKey: ["cash-session-open"] });
    qc.invalidateQueries({ queryKey: ["reports"] });
  };
}

/** Gasto, retiro o ajuste de caja. */
export function useAddCashMovement() {
  const invalidate = useInvalidateCash();
  return useMutation({
    mutationFn: (input: AddCashMovementInput) => api.addCashMovement(input),
    onSuccess: invalidate,
  });
}

/** Cierra la caja con el arqueo por denominación. */
export function useCloseCash() {
  const invalidate = useInvalidateCash();
  return useMutation({
    mutationFn: (input: CloseCashInput) => api.closeCashSession(input),
    onSuccess: invalidate,
  });
}
