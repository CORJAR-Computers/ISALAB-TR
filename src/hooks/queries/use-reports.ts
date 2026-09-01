import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function useReports() {
  return useQuery({ queryKey: ["reports"], queryFn: api.listReports });
}

export function useGenerateReport() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ sampleId, overrideLogoPath, saveLogoPreference }: { sampleId: number, overrideLogoPath?: string | null, saveLogoPreference?: boolean | null }) =>
      api.generateReport(sampleId, overrideLogoPath, saveLogoPreference),
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

export function useGenerateCarnetVacunacion() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patientId: number) => api.generateCarnetVacunacion(patientId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["reports"] }),
  });
}
