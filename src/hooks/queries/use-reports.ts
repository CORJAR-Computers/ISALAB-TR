import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function useReports() {
  return useQuery({ queryKey: ["reports"], queryFn: api.listReports });
}

/**
 * Facturación de un período: facturado por emisión vs. cobrado por recepción.
 * Se consulta con fechas completas (AAAA-MM-DD) para no pegarle al backend en
 * cada tecla mientras el usuario edita el rango.
 */
export function useBillingReport(from: string, to: string) {
  const valid = (d: string) => /^\d{4}-\d{2}-\d{2}$/.test(d);
  return useQuery({
    queryKey: ["billing-report", from, to],
    queryFn: () => api.getBillingReport(from, to),
    enabled: valid(from) && valid(to),
  });
}

export function useGenerateReport() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ sampleId, overrideLogoId, saveLogoPreference }: { sampleId: number, overrideLogoId?: number | null, saveLogoPreference?: boolean | null }) =>
      api.generateReport(sampleId, overrideLogoId, saveLogoPreference),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["reports"] }),
  });
}

export function useGenerateSampleLabels() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (sampleIds: number[]) => api.generateSampleLabels(sampleIds),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["reports"] }),
  });
}

export function useGenerateFormulaMedica() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (consultationId: number) =>
      api.generateFormulaMedica(consultationId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["reports"] }),
  });
}

export function useGenerateConsentimiento() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (surgeryId: number) => api.generateConsentimiento(surgeryId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["reports"] }),
  });
}

export function useGenerateReciboInvoice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (invoiceId: number) => api.generateReciboInvoice(invoiceId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["reports"] }),
  });
}

export function useGenerateCertificadoCirugia() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (surgeryId: number) => api.generateCertificadoCirugia(surgeryId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["reports"] }),
  });
}

export function useGenerateCashClosePdf() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (sessionId: number) => api.generateCashClosePdf(sessionId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["reports"] }),
  });
}

export function useGenerateCarnetVacunacion() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patientId: number) => api.generateCarnetVacunacion(patientId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["reports"] }),
  });
}
