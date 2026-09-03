import { invoke } from "@tauri-apps/api/core";
import type {
  Analyte,
  StatusCount,
  Analyzer,
  AnalyzerImportMapping,
  AppError,
  AuditLogEntry,
  Breed,
  ChangePasswordInput,
  ClinicalHistory,
  ClinicSettings,
  Consultation,
  ConsultationListItem,
  CreateAnalyzerInput,
  CreateConsultationInput,
  CreateInvoiceInput,
  CreatePatientInput,
  CreateSampleInput,
  CreateSurgeryInput,
  CreateUserInput,
  CreateVaccineInput,
  DashboardStats,
  DbHealth,
  GlobalSearchResult,
  ImportPreview,
  ImportSummary,
  Invoice,
  InvoiceListItem,
  LabResult,
  LoginInput,
  Owner,
  Panel,
  PanelAnalyte,
  PanelInput,
  Patient,
  QcAnalyzerStatus,
  QcChartData,
  QcControlMaterial,
  QcMaterialInput,
  QcRun,
  QcRunInput,
  QcTarget,
  ReferenceRange,
  ReferenceRangeInput,
  RegisterResultInput,
  RegisterResultsInput,
  ReportFile,
  ResultAttachment,
  Sample,
  SampleListItem,
  SampleType,
  SessionUser,
  Species,
  Surgery,
  UpdateAnalyzerInput,
  UserListItem,
  Vaccine,
  VaccineListItem,
  VaccineType,
  SecondaryLogo,
  WorklistData,
} from "@/bindings";

/** Extrae el mensaje legible de un AppError serializado por Tauri. */
export function getErrorMessage(e: unknown): string {
  if (e && typeof e === "object") {
    const err = e as Partial<AppError> & { message?: string };
    if (typeof err.message === "string" && err.message.length > 0) return err.message;
    if (err.type && "data" in err) {
      const data = (err as { data: unknown }).data;
      if (typeof data === "string" && data.length > 0) return `${err.type}: ${data}`;
      return err.type;
    }
  }
  return String(e ?? "Error desconocido");
}

export const api = {
  // ---- Diagnóstico / setup ----
  dbHealth: () => invoke<DbHealth>("db_health"),

  // ---- Catálogos ----
  listSpecies: () => invoke<Species[]>("list_species"),
  listBreeds: (speciesId: number) =>
    invoke<Breed[]>("list_breeds", { speciesId }),
  listSampleTypes: () => invoke<SampleType[]>("list_sample_types"),
  listAnalytes: () => invoke<Analyte[]>("list_analytes"),
  listVaccineTypes: () => invoke<VaccineType[]>("list_vaccine_types"),
  listOwners: (search: string | null) =>
    invoke<Owner[]>("list_owners", { search }),

  // ---- Pacientes ----
  listPatients: (search?: string) =>
    invoke<Patient[]>("list_patients", { search: search || null }),
  getPatient: (id: number) => invoke<Patient | null>("get_patient", { id }),
  getPatientByCode: (code: string) =>
    invoke<Patient | null>("get_patient_by_code", { code }),
  createPatient: (input: CreatePatientInput) =>
    invoke<Patient>("create_patient", { input }),

  // ---- Historial clínico ----
  getClinicalHistory: (patientId: number) =>
    invoke<ClinicalHistory>("get_clinical_history", { patientId }),
  createConsultation: (input: CreateConsultationInput) =>
    invoke<Consultation>("create_consultation", { input }),

  // ---- Laboratorio ----
  getWorklist: () => invoke<WorklistData>("get_worklist"),
  createSample: (input: CreateSampleInput) =>
    invoke<Sample>("create_sample", { input }),
  registerLabResult: (input: RegisterResultInput) =>
    invoke<LabResult>("register_lab_result", { input }),
  registerLabResults: (input: RegisterResultsInput) =>
    invoke<LabResult[]>("register_lab_results", { input }),
  listSamples: (status: string | null, search: string | null) =>
    invoke<SampleListItem[]>("list_samples", { status, search }),
  getSample: (id: number) => invoke<Sample | null>("get_sample", { id }),
  setSampleStatus: (id: number, status: string) =>
    invoke<Sample>("set_sample_status", { id, status }),
  setSampleQuality: (
    id: number,
    qualityIndex: string | null,
    qualitySeverity: string | null,
    qualityNote: string | null,
  ) =>
    invoke<Sample>("set_sample_quality", {
      id,
      qualityIndex,
      qualitySeverity,
      qualityNote,
    }),
  rejectSample: (id: number, reason: string) =>
    invoke<Sample>("reject_sample", { id, reason }),
  reopenSample: (id: number) => invoke<Sample>("reopen_sample", { id }),

  // ---- Importación desde analizador (CSV) ----
  previewAnalyzerImport: (path: string) =>
    invoke<ImportPreview>("preview_analyzer_import", { path }),
  importAnalyzerResults: (path: string, mapping: AnalyzerImportMapping) =>
    invoke<ImportSummary>("import_analyzer_results", { path, mapping }),

  // ---- Paneles de analitos ----
  listPanels: () => invoke<Panel[]>("list_panels"),
  listPanelAnalytes: (panelId: number) =>
    invoke<PanelAnalyte[]>("list_panel_analytes", { panelId }),
  savePanel: (input: PanelInput) => invoke<Panel>("save_panel", { input }),
  deletePanel: (id: number) => invoke<void>("delete_panel", { id }),

  // ---- Control de calidad (QC) ----
  listQcMaterials: () => invoke<QcControlMaterial[]>("list_qc_materials"),
  listQcTargets: (controlMaterialId: number) =>
    invoke<QcTarget[]>("list_qc_targets", { controlMaterialId }),
  saveQcMaterial: (input: QcMaterialInput) =>
    invoke<QcControlMaterial>("save_qc_material", { input }),
  deleteQcMaterial: (id: number) =>
    invoke<void>("delete_qc_material", { id }),
  recordQcRun: (input: QcRunInput) => invoke<QcRun>("record_qc_run", { input }),
  listQcRuns: (controlMaterialId: number | null) =>
    invoke<QcRun[]>("list_qc_runs", { controlMaterialId }),
  deleteQcRun: (id: number) => invoke<void>("delete_qc_run", { id }),
  getQcChart: (controlMaterialId: number, analyteId: number) =>
    invoke<QcChartData | null>("get_qc_chart", {
      controlMaterialId,
      analyteId,
    }),
  listQcAnalyzerStatus: () =>
    invoke<QcAnalyzerStatus[]>("list_qc_analyzer_status"),
  attachResultFile: (resultId: number, sourcePath: string) =>
    invoke<ResultAttachment>("attach_result_file", { resultId, sourcePath }),
  deleteResultAttachment: (id: number) =>
    invoke<void>("delete_result_attachment", { id }),

  // ---- Equipos analizadores y rangos de referencia ----
  listAnalyzers: () => invoke<Analyzer[]>("list_analyzers"),
  createAnalyzer: (input: CreateAnalyzerInput) =>
    invoke<Analyzer>("create_analyzer", { input }),
  updateAnalyzer: (input: UpdateAnalyzerInput) =>
    invoke<Analyzer>("update_analyzer", { input }),
  setAnalyzerActive: (id: number, active: boolean) =>
    invoke<Analyzer>("set_analyzer_active", { id, active }),
  deleteAnalyzer: (id: number) => invoke<void>("delete_analyzer", { id }),
  listReferenceRanges: (analyzerId: number | null) =>
    invoke<ReferenceRange[]>("list_reference_ranges", { analyzerId }),
  createReferenceRange: (input: ReferenceRangeInput) =>
    invoke<ReferenceRange>("create_reference_range", { input }),
  updateReferenceRange: (id: number, input: ReferenceRangeInput) =>
    invoke<ReferenceRange>("update_reference_range", { id, input }),
  deleteReferenceRange: (id: number) =>
    invoke<void>("delete_reference_range", { id }),

  // ---- Configuración ----
  getClinicSettings: () => invoke<ClinicSettings>("get_clinic_settings"),
  saveClinicSettings: (input: ClinicSettings) =>
    invoke<ClinicSettings>("save_clinic_settings", { input }),
  importClinicLogo: (sourcePath: string) =>
    invoke<string>("import_clinic_logo", { sourcePath }),
  importPkcs12: (sourcePath: string, password: string) =>
    invoke<string>("import_pkcs12", { sourcePath, password }),
  listSecondaryLogos: () => invoke<SecondaryLogo[]>("list_secondary_logos"),
  importSecondaryLogo: (name: string, sourcePath: string) =>
    invoke<SecondaryLogo>("import_secondary_logo", { name, sourcePath }),
  deleteSecondaryLogo: (id: number) =>
    invoke<null>("delete_secondary_logo", { id }),

  // ---- Autenticación ----
  login: (input: LoginInput) => invoke<SessionUser>("login", { input }),
  logout: () => invoke<void>("logout"),
  getSession: () => invoke<SessionUser | null>("get_session"),

  // ---- Usuarios ----
  listUsers: () => invoke<UserListItem[]>("list_users"),
  createUser: (input: CreateUserInput) =>
    invoke<UserListItem>("create_user", { input }),
  changePassword: (input: ChangePasswordInput) =>
    invoke<SessionUser>("change_password", { input }),

  // ---- Auditoría ----
  listAuditLog: (
    limit: number | null,
    offset: number | null,
    username?: string,
    action?: string,
    dateFrom?: string,
    dateTo?: string,
  ) =>
    invoke<AuditLogEntry[]>("list_audit_log", {
      limit,
      offset,
      username: username || null,
      action: action || null,
      dateFrom: dateFrom || null,
      dateTo: dateTo || null,
    }),

  // ---- Exportación CSV ----
  exportSamplesCsv: (
    destPath: string,
    status: string | null,
    search: string | null,
  ) => invoke<string>("export_samples_csv", { destPath, status, search }),
  exportResultsCsv: (
    destPath: string,
    status: string | null,
    search: string | null,
  ) => invoke<string>("export_results_csv", { destPath, status, search }),

  // ---- Reportes PDF ----
  generateReport: (sampleId: number, overrideLogoPath?: string | null, saveLogoPreference?: boolean | null) =>
    invoke<ReportFile>("generate_clinical_report", { sampleId, overrideLogoPath, saveLogoPreference }),
  generateFormulaMedica: (consultationId: number) =>
    invoke<ReportFile>("generate_formula_medica", { consultationId }),
  generateConsentimiento: (surgeryId: number) =>
    invoke<ReportFile>("generate_consentimiento", { surgeryId }),
  generateReciboInvoice: (invoiceId: number) =>
    invoke<ReportFile>("generate_recibo_invoice", { invoiceId }),
  generateCertificadoCirugia: (surgeryId: number) =>
    invoke<ReportFile>("generate_certificado_cirugia", { surgeryId }),
  generateCarnetVacunacion: (patientId: number) =>
    invoke<ReportFile>("generate_carnet_vacunacion", { patientId }),
  listReports: () => invoke<ReportFile[]>("list_reports"),
  generateSampleLabels: (sampleIds: number[]) =>
    invoke<ReportFile>("generate_sample_labels", { sampleIds }),
  openReportFile: (path: string) => invoke<void>("open_report_file", { path }),

  // ---- Agenda de consultas ----
  listConsultations: (status: string | null, search: string | null) =>
    invoke<ConsultationListItem[]>("list_consultations", { status, search }),
  setConsultationStatus: (id: number, status: string) =>
    invoke<ConsultationListItem>("set_consultation_status", { id, status }),

  // ---- Vacunación ----
  createVaccine: (input: CreateVaccineInput) =>
    invoke<Vaccine>("create_vaccine", { input }),
  listVaccines: (search: string | null) =>
    invoke<VaccineListItem[]>("list_vaccines", { search }),

  // ---- Cirugías ----
  createSurgery: (input: CreateSurgeryInput) =>
    invoke<Surgery>("create_surgery", { input }),
  listSurgeries: (status: string | null, search: string | null) =>
    invoke<Surgery[]>("list_surgeries", { status, search }),
  setSurgeryStatus: (id: number, status: string) =>
    invoke<Surgery>("set_surgery_status", { id, status }),

  // ---- Facturación ----
  createInvoice: (input: CreateInvoiceInput) =>
    invoke<Invoice>("create_invoice", { input }),
  listInvoices: (status: string | null, search: string | null) =>
    invoke<InvoiceListItem[]>("list_invoices", { status, search }),
  getInvoice: (id: number) => invoke<Invoice | null>("get_invoice", { id }),
  setInvoiceStatus: (id: number, status: string) =>
    invoke<Invoice>("set_invoice_status", { id, status }),

  // ---- Búsqueda global (paleta Ctrl+K) ----
  globalSearch: (query: string) =>
    invoke<GlobalSearchResult[]>("global_search", { query }),

  // ---- Dashboard ----
  getDashboardStats: () => invoke<DashboardStats>("get_dashboard_stats"),

  // ---- IA (Groq) ----
  interpretLabResults: (sampleId: number) =>
    invoke<string>("interpret_lab_results", { sampleId }),
  testGroqConnection: () => invoke<string>("test_groq_connection"),

  // ---- Contadores por estado ----
  countSamples: () => invoke<StatusCount[]>("count_samples"),
  countConsultations: () => invoke<StatusCount[]>("count_consultations"),
  countSurgeries: () => invoke<StatusCount[]>("count_surgeries"),
  countInvoices: () => invoke<StatusCount[]>("count_invoices"),
};
