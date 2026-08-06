import { invoke } from "@tauri-apps/api/core";
import type {
  Analyte,
  AppError,
  AuditLogEntry,
  Breed,
  ChangePasswordInput,
  ClinicalHistory,
  ClinicSettings,
  Consultation,
  ConsultationListItem,
  CreateConsultationInput,
  CreateInvoiceInput,
  CreatePatientInput,
  CreateSampleInput,
  CreateSurgeryInput,
  CreateUserInput,
  CreateVaccineInput,
  DashboardStats,
  DbHealth,
  Invoice,
  InvoiceListItem,
  LabResult,
  LoginInput,
  Owner,
  Patient,
  RegisterResultInput,
  ReportFile,
  ResultAttachment,
  Sample,
  SampleListItem,
  SampleType,
  SessionUser,
  Species,
  Surgery,
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
    if (typeof err.message === "string" && err.message) return err.message;
    if (err.type && "data" in err) {
      return `${err.type}: ${(err as { data: string }).data}`;
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
  listSamples: (status: string | null, search: string | null) =>
    invoke<SampleListItem[]>("list_samples", { status, search }),
  getSample: (id: number) => invoke<Sample | null>("get_sample", { id }),
  setSampleStatus: (id: number, status: string) =>
    invoke<Sample>("set_sample_status", { id, status }),
  attachResultFile: (resultId: number, sourcePath: string) =>
    invoke<ResultAttachment>("attach_result_file", { resultId, sourcePath }),
  deleteResultAttachment: (id: number) =>
    invoke<void>("delete_result_attachment", { id }),

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

  // ---- Dashboard ----
  getDashboardStats: () => invoke<DashboardStats>("get_dashboard_stats"),

  // ---- IA (Groq) ----
  interpretLabResults: (sampleId: number) =>
    invoke<string>("interpret_lab_results", { sampleId }),
  testGroqConnection: () => invoke<string>("test_groq_connection"),
};
