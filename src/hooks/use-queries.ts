import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  ChangePasswordInput,
  ClinicSettings,
  CreateAnalyzerInput,
  CreateConsultationInput,
  CreateInvoiceInput,
  CreatePatientInput,
  CreateSampleInput,
  CreateSurgeryInput,
  CreateUserInput,
  CreateVaccineInput,
  LoginInput,
  ReferenceRangeInput,
  RegisterResultInput,
  UpdateAnalyzerInput,
} from "@/bindings";
import { useSessionStore } from "@/stores/session-store";
import { useUiStore } from "@/stores/ui-store";

export function useDbHealth() {
  return useQuery({
    queryKey: ["db-health"],
    queryFn: api.dbHealth,
    retry: false,
  });
}

export function useSpecies() {
  return useQuery({ queryKey: ["species"], queryFn: api.listSpecies });
}

export function useBreeds(speciesId: number | null) {
  return useQuery({
    queryKey: ["breeds", speciesId],
    queryFn: () => api.listBreeds(speciesId!),
    enabled: speciesId != null,
  });
}

export function useSampleTypes() {
  return useQuery({
    queryKey: ["sample-types"],
    queryFn: api.listSampleTypes,
  });
}

export function useAnalytes() {
  return useQuery({ queryKey: ["analytes"], queryFn: api.listAnalytes });
}

export function useVaccineTypes() {
  return useQuery({
    queryKey: ["vaccine-types"],
    queryFn: api.listVaccineTypes,
  });
}

export function useOwners(search: string) {
  return useQuery({
    queryKey: ["owners", search],
    queryFn: () => api.listOwners(search.trim() || null),
    placeholderData: (prev) => prev,
  });
}

export function usePatients(search: string, enabled = true) {
  return useQuery({
    queryKey: ["patients", search],
    queryFn: () => api.listPatients(search),
    placeholderData: (prev) => prev,
    enabled,
  });
}

export function usePatient(id: number | null) {
  return useQuery({
    queryKey: ["patient", id],
    queryFn: () => api.getPatient(id!),
    enabled: id != null,
  });
}

/** Busca un paciente por su código (p. ej. escáner de código de barras). */
export function usePatientByCode(code: string | null) {
  const normalized = code?.trim() ?? null;
  return useQuery({
    queryKey: ["patient-by-code", normalized],
    queryFn: () => api.getPatientByCode(normalized!),
    enabled: normalized != null && normalized.length > 0,
  });
}

export function useClinicalHistory(patientId: number | null) {
  return useQuery({
    queryKey: ["clinical-history", patientId],
    queryFn: () => api.getClinicalHistory(patientId!),
    enabled: patientId != null,
  });
}

export function useCreatePatient() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreatePatientInput) => api.createPatient(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["patients"] });
    },
  });
}

export function useCreateConsultation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateConsultationInput) =>
      api.createConsultation(input),
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({
        queryKey: ["clinical-history", vars.patientId],
      });
      qc.invalidateQueries({ queryKey: ["patient", vars.patientId] });
      qc.invalidateQueries({ queryKey: ["consultations"] });
      qc.invalidateQueries({ queryKey: ["consultation-counts"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export function useCreateSample() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateSampleInput) => api.createSample(input),
    onSuccess: (sample) => {
      qc.invalidateQueries({
        queryKey: ["clinical-history", sample.patientId],
      });
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export function useRegisterLabResult() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: RegisterResultInput) => api.registerLabResult(input),
    onSuccess: (result) => {
      qc.invalidateQueries({ queryKey: ["clinical-history"] });
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["sample", result.sampleId] });
    },
  });
}

// ==================== LABORATORIO (mesa de trabajo) ========================

export function useSamples(status: string | null, search: string, enabled = true) {
  return useQuery({
    queryKey: ["samples", status, search],
    queryFn: () => api.listSamples(status, search.trim() || null),
    placeholderData: (prev) => prev,
    enabled,
  });
}

/** Contadores por estado (sin filtros) para las pestañas de la mesa de trabajo. */
export function useSampleCounts() {
  return useQuery({
    queryKey: ["sample-counts"],
    queryFn: () => api.listSamples(null, null),
  });
}

/** Bandeja de trabajo diaria: pendientes por tipo con tiempo transcurrido.
 *  Se auto-refresca cada minuto porque el tiempo transcurrido avanza en vivo. */
export function useWorklist() {
  return useQuery({
    queryKey: ["worklist"],
    queryFn: api.getWorklist,
    refetchInterval: 60_000,
  });
}

export function useSample(id: number | null) {
  return useQuery({
    queryKey: ["sample", id],
    queryFn: () => api.getSample(id!),
    enabled: id != null,
  });
}

export function useSetSampleStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: string }) =>
      api.setSampleStatus(id, status),
    onSuccess: (sample) => {
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["sample", sample.id] });
      qc.invalidateQueries({
        queryKey: ["clinical-history", sample.patientId],
      });
    },
  });
}

/** Adjunta fotos (placas, frotis, electroforesis) a un resultado y refresca
 *  la ficha de la muestra y el historial del paciente. */
export function useAttachResultFile(sampleId: number | null) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ resultId, sourcePath }: { resultId: number; sourcePath: string }) =>
      api.attachResultFile(resultId, sourcePath),
    onSuccess: () => {
      if (sampleId != null) {
        qc.invalidateQueries({ queryKey: ["sample", sampleId] });
      }
      qc.invalidateQueries({ queryKey: ["clinical-history"] });
    },
  });
}

/** Elimina un adjunto (borra archivo + registro) y refresca la ficha. */
export function useDeleteResultAttachment(sampleId: number | null) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteResultAttachment(id),
    onSuccess: () => {
      if (sampleId != null) {
        qc.invalidateQueries({ queryKey: ["sample", sampleId] });
      }
      qc.invalidateQueries({ queryKey: ["clinical-history"] });
    },
  });
}

// ====================== EQUIPOS Y RANGOS ===================================

/** Equipos analizadores (para el selector de muestras y la gestión). */
export function useAnalyzers() {
  return useQuery({ queryKey: ["analyzers"], queryFn: api.listAnalyzers });
}

/** Rangos de referencia de un equipo (o todos si `analyzerId` es null). */
export function useReferenceRanges(analyzerId: number | null) {
  return useQuery({
    queryKey: ["reference-ranges", analyzerId],
    queryFn: () => api.listReferenceRanges(analyzerId),
    enabled: analyzerId != null,
  });
}

export function useCreateAnalyzer() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateAnalyzerInput) => api.createAnalyzer(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzers"] }),
  });
}

export function useUpdateAnalyzer() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdateAnalyzerInput) => api.updateAnalyzer(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzers"] }),
  });
}

export function useSetAnalyzerActive() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, active }: { id: number; active: boolean }) =>
      api.setAnalyzerActive(id, active),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzers"] }),
  });
}

export function useDeleteAnalyzer() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteAnalyzer(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["analyzers"] }),
  });
}

export function useCreateReferenceRange() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: ReferenceRangeInput) =>
      api.createReferenceRange(input),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: ["reference-ranges"] }),
  });
}

export function useUpdateReferenceRange() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: number; input: ReferenceRangeInput }) =>
      api.updateReferenceRange(id, input),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: ["reference-ranges"] }),
  });
}

export function useDeleteReferenceRange() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteReferenceRange(id),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: ["reference-ranges"] }),
  });
}

// ====================== CONFIGURACIÓN =======================================

export function useClinicSettings() {
  return useQuery({
    queryKey: ["clinic-settings"],
    queryFn: api.getClinicSettings,
  });
}

export function useSaveClinicSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: ClinicSettings) => api.saveClinicSettings(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["clinic-settings"] }),
  });
}

export function useImportClinicLogo() {
  return useMutation({
    mutationFn: (sourcePath: string) => api.importClinicLogo(sourcePath),
  });
}

export function useSecondaryLogos() {
  return useQuery({
    queryKey: ["secondary-logos"],
    queryFn: api.listSecondaryLogos,
  });
}

export function useImportSecondaryLogo() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ name, sourcePath }: { name: string; sourcePath: string }) =>
      api.importSecondaryLogo(name, sourcePath),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["secondary-logos"] }),
  });
}

export function useDeleteSecondaryLogo() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteSecondaryLogo(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["secondary-logos"] }),
  });
}

// ====================== AUTENTICACIÓN =======================================

export function useLogin() {
  const setSession = useSessionStore((s) => s.setSession);
  return useMutation({
    mutationFn: (input: LoginInput) => api.login(input),
    onSuccess: (user) => {
      setSession(user);
      // Tras autenticarse siempre se abre el Panel de control.
      useUiStore.getState().navigate("dashboard");
    },
  });
}

export function useLogout() {
  const setSession = useSessionStore((s) => s.setSession);
  return useMutation({
    mutationFn: () => api.logout(),
    onSuccess: () => setSession(null),
  });
}

// ====================== USUARIOS ===========================================

export function useUsers() {
  return useQuery({ queryKey: ["users"], queryFn: api.listUsers });
}

export function useCreateUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateUserInput) => api.createUser(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["users"] }),
  });
}

/** Cambia la contraseña del usuario con sesión activa y actualiza el store. */
export function useChangePassword() {
  const setSession = useSessionStore((s) => s.setSession);
  return useMutation({
    mutationFn: (input: ChangePasswordInput) => api.changePassword(input),
    onSuccess: (user) => setSession(user),
  });
}

// ====================== AGENDA DE CONSULTAS =================================

export function useConsultations(status: string | null, search: string, enabled = true) {
  return useQuery({
    queryKey: ["consultations", status, search],
    queryFn: () => api.listConsultations(status, search.trim() || null),
    placeholderData: (prev) => prev,
    enabled,
  });
}

/** Contadores por estado (sin filtros) para las pestañas de la agenda. */
export function useConsultationCounts() {
  return useQuery({
    queryKey: ["consultation-counts"],
    queryFn: () => api.listConsultations(null, null),
  });
}

export function useSetConsultationStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: string }) =>
      api.setConsultationStatus(id, status),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["consultations"] });
      qc.invalidateQueries({ queryKey: ["consultation-counts"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

// ====================== VACUNACIÓN ==========================================

export function useVaccines(search: string) {
  return useQuery({
    queryKey: ["vaccines", search],
    queryFn: () => api.listVaccines(search.trim() || null),
    placeholderData: (prev) => prev,
  });
}

export function useCreateVaccine() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateVaccineInput) => api.createVaccine(input),
    onSuccess: (v) => {
      qc.invalidateQueries({ queryKey: ["vaccines"] });
      qc.invalidateQueries({ queryKey: ["clinical-history", v.patientId] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

// ====================== CIRUGÍAS ============================================

export function useSurgeries(status: string | null, search: string, enabled = true) {
  return useQuery({
    queryKey: ["surgeries", status, search],
    queryFn: () => api.listSurgeries(status, search.trim() || null),
    placeholderData: (prev) => prev,
    enabled,
  });
}

/** Contadores por estado (sin filtros) para las pestañas de la agenda quirúrgica. */
export function useSurgeryCounts() {
  return useQuery({
    queryKey: ["surgery-counts"],
    queryFn: () => api.listSurgeries(null, null),
  });
}

export function useCreateSurgery() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateSurgeryInput) => api.createSurgery(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["surgeries"] });
      qc.invalidateQueries({ queryKey: ["surgery-counts"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

export function useSetSurgeryStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, status }: { id: number; status: string }) =>
      api.setSurgeryStatus(id, status),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["surgeries"] });
      qc.invalidateQueries({ queryKey: ["surgery-counts"] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}

// ====================== FACTURACIÓN =========================================

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
    queryFn: () => api.listInvoices(null, null),
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

// ====================== BÚSQUEDA GLOBAL =====================================

/** Búsqueda global (paleta Ctrl+K) con debounce aplicado en el componente. */
export function useGlobalSearch(query: string, enabled = true) {
  const trimmed = query.trim();
  return useQuery({
    queryKey: ["global-search", trimmed],
    queryFn: () => api.globalSearch(trimmed),
    enabled: enabled && trimmed.length > 0,
    placeholderData: (prev) => prev,
    staleTime: 30_000,
  });
}

// ====================== DASHBOARD ===========================================

export function useDashboardStats() {
  return useQuery({
    queryKey: ["dashboard"],
    queryFn: api.getDashboardStats,
    refetchInterval: 30_000,
  });
}

// ====================== REPORTES PDF ========================================

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

// ====================== AUDITORÍA ===========================================

export function useAuditLog(
  page: number,
  pageSize = 50,
  username?: string,
  action?: string,
  dateFrom?: string,
  dateTo?: string,
) {
  const offset = page * pageSize;
  return useQuery({
    queryKey: ["audit-log", page, pageSize, username, action, dateFrom, dateTo],
    queryFn: () => api.listAuditLog(pageSize, offset, username, action, dateFrom, dateTo),
    placeholderData: (prev) => prev,
  });
}
