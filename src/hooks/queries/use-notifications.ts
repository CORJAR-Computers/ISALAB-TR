import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { toast } from "sonner";
import { getErrorMessage } from "@/lib/api";

/** Historial de notificaciones de una muestra (valores críticos). */
export function useSampleNotifications(sampleId: number | null) {
  return useQuery({
    queryKey: ["sample-notifications", sampleId],
    queryFn: () => api.listSampleNotifications(sampleId!),
    enabled: sampleId != null,
  });
}

/** Persiste la confirmación del analista de valores críticos (CLSI GP47). */
export function useAcknowledgeCritical() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ sampleId, resultIds }: { sampleId: number; resultIds: number[] }) =>
      api.acknowledgeCritical(sampleId, resultIds),
    onSuccess: (_entries, vars) => {
      qc.invalidateQueries({ queryKey: ["sample-notifications", vars.sampleId] });
    },
  });
}

/** Envía el aviso de valor crítico por email al propietario. */
export function useSendCriticalEmail() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ sampleId, resultIds }: { sampleId: number; resultIds: number[] }) =>
      api.sendCriticalEmail(sampleId, resultIds),
    onSuccess: (_entries, vars) => {
      qc.invalidateQueries({ queryKey: ["sample-notifications", vars.sampleId] });
    },
  });
}

/** Prueba la conexión SMTP (Ajustes). */
export function useTestSmtpConnection() {
  return useMutation({
    mutationFn: () => api.testSmtpConnection(),
    onError: (err) => {
      toast.error("No se pudo conectar al servidor SMTP", {
        description: getErrorMessage(err),
      });
    },
  });
}