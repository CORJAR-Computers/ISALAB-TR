import { invoke } from "@tauri-apps/api/core";
import type {
  Analyte,
  CreateAnalyteInput,
  AccessionOrderInput,
  StatusCount,
  Analyzer,
  AnalyzerImportJob,
  AnalyzerImportMapping,
  AnalyzerSource,
  SaveAnalyzerSourceInput,
  AppError,
  AuditLogEntry,
  Breed,
  ChangePasswordInput,
  ClinicalHistory,
  ClinicBranding,
  ClinicSettings,
  Consultation,
  ConsultationListItem,
  CreateAnalyzerInput,
  CreateConsultationInput,
  CreateInvoiceInput,
  CreateLabOrderInput,
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
  LabOrder,
  LabResult,
  LabOrderListItem,
  NotificationLogEntry,
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
  SampleEvent,
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
  Service,
  ServiceCategory,
  SaveServiceCategoryInput,
  SaveServiceInput,
  Visit,
  VisitListItem,
  VisitService,
  CreateVisitInput,
  AddVisitServiceInput,
  UpdateVisitServiceStatusInput,
  Appointment,
  AppointmentListItem,
  CreateAppointmentInput,
  UpdateAppointmentStatusInput,
  GroomingSession,
  UpsertGroomingSessionInput,
  FinishGroomingSessionInput,
  GroomingDayLine,
  GroomingReminder,
  VisitAttachment,
  WorklistData,
  // Estancia e inventario (fase 6).
  Resource,
  ResourceBoardItem,
  Stay,
  StayListItem,
  StayEvent,
  CreateStayInput,
  CheckOutStayInput,
  CreateStayEventInput,
  SaveResourceInput,
  Product,
  ProductStockItem,
  StockMovement,
  SaveProductInput,
  CreateStockMovementInput,
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
  createAnalyte: (input: CreateAnalyteInput) =>
    invoke<Analyte>("create_analyte", { input }),
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
  deleteLabResult: (sampleId: number, analyteId: number) =>
    invoke<void>("delete_lab_result", { sampleId, analyteId }),
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
  listSampleEvents: (sampleId: number) =>
    invoke<SampleEvent[]>("list_sample_events", { sampleId }),
  listSampleNotifications: (sampleId: number) =>
    invoke<NotificationLogEntry[]>("list_sample_notifications", { sampleId }),
  acknowledgeCritical: (sampleId: number, resultIds: number[]) =>
    invoke<NotificationLogEntry[]>("acknowledge_critical", { sampleId, resultIds }),
  sendCriticalEmail: (sampleId: number, resultIds: number[]) =>
    invoke<NotificationLogEntry[]>("send_critical_email", { sampleId, resultIds }),
  testSmtpConnection: () => invoke<void>("test_smtp_connection"),

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

  // ---- Importación automática (carpetas vigiladas) ----
  listAnalyzerSources: () => invoke<AnalyzerSource[]>("list_analyzer_sources"),
  saveAnalyzerSource: (input: SaveAnalyzerSourceInput) =>
    invoke<AnalyzerSource | null>("save_analyzer_source", { input }),
  deleteAnalyzerSource: (id: number) =>
    invoke<void>("delete_analyzer_source", { id }),
  pollAnalyzerSource: (sourceId: number) =>
    invoke<AnalyzerImportJob[]>("poll_analyzer_source", { sourceId }),
  listAnalyzerImportJobs: (sourceId: number, limit: number) =>
    invoke<AnalyzerImportJob[]>("list_analyzer_import_jobs", {
      sourceId,
      limit,
    }),
  listFailedAnalyzerImports: (limit: number) =>
    invoke<AnalyzerImportJob[]>("list_failed_analyzer_imports", { limit }),
  deleteAnalyzerImportJob: (jobId: number) =>
    invoke<void>("delete_analyzer_import_job", { jobId }),

  // ---- Configuración ----
  getClinicBranding: () => invoke<ClinicBranding>("get_clinic_branding"),
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

  // ---- Órdenes de laboratorio ----
  createLabOrder: (input: CreateLabOrderInput) =>
    invoke<LabOrder>("create_lab_order", { input }),
  listLabOrders: (status: string | null, search: string | null) =>
    invoke<LabOrderListItem[]>("list_lab_orders", { status, search }),
  listPatientLabOrders: (patientId: number) =>
    invoke<LabOrderListItem[]>("list_patient_lab_orders", { patientId }),
  getLabOrder: (id: number) =>
    invoke<LabOrder | null>("get_lab_order", { id }),
  countLabOrders: () => invoke<StatusCount[]>("count_lab_orders"),
  setLabOrderStatus: (id: number, status: string) =>
    invoke<LabOrder>("set_lab_order_status", { id, status }),
  accessionLabOrder: (input: AccessionOrderInput) =>
    invoke<Sample>("accession_lab_order", { input }),
  getOrderForSample: (sampleId: number) =>
    invoke<LabOrder | null>("get_order_for_sample", { sampleId }),

  // ---- Catálogo de servicios (clínica) ----
  listServiceCategories: () =>
    invoke<ServiceCategory[]>("list_service_categories"),
  listAllServiceCategories: () =>
    invoke<ServiceCategory[]>("list_all_service_categories"),
  saveServiceCategory: (input: SaveServiceCategoryInput) =>
    invoke<ServiceCategory>("save_service_category", { input }),
  deleteServiceCategory: (id: number) =>
    invoke<void>("delete_service_category", { id }),
  listServices: (
    categoryId: number | null,
    search: string | null,
    includeInactive?: boolean,
  ) =>
    invoke<Service[]>("list_services", {
      categoryId,
      search,
      includeInactive: includeInactive ?? false,
    }),
  getService: (id: number) => invoke<Service | null>("get_service", { id }),
  saveService: (input: SaveServiceInput) =>
    invoke<Service>("save_service", { input }),
  setServiceActive: (id: number, active: boolean) =>
    invoke<Service>("set_service_active", { id, active }),

  // ---- Recepción de la clínica (visitas) ----
  listVisits: (date: string | null, status: string | null, search: string | null) =>
    invoke<VisitListItem[]>("list_visits", { date, status, search }),
  getVisit: (id: number) => invoke<Visit | null>("get_visit", { id }),
  createVisit: (input: CreateVisitInput) =>
    invoke<Visit>("create_visit", { input }),
  setVisitStatus: (id: number, status: string) =>
    invoke<Visit>("set_visit_status", { id, status }),
  addVisitService: (visitId: number, input: AddVisitServiceInput) =>
    invoke<VisitService>("add_visit_service", { visitId, input }),
  removeVisitService: (visitId: number, lineId: number) =>
    invoke<void>("remove_visit_service", { visitId, lineId }),
  updateVisitServiceStatus: (
    visitId: number,
    lineId: number,
    input: UpdateVisitServiceStatusInput,
  ) => invoke<VisitService>("update_visit_service_status", { visitId, lineId, input }),
  checkoutVisit: (visitId: number, paymentMethod: string | null, notes: string | null) =>
    invoke<Invoice>("checkout_visit", { visitId, paymentMethod, notes }),

  // ---- Agenda generalizada (citas) ----
  listAppointments: (
    date: string | null,
    status: string | null,
    professionalId: number | null,
  ) => invoke<AppointmentListItem[]>("list_appointments", { date, status, professionalId }),
  getAppointment: (id: number) => invoke<Appointment | null>("get_appointment", { id }),
  createAppointment: (input: CreateAppointmentInput) =>
    invoke<Appointment>("create_appointment", { input }),
  setAppointmentStatus: (
    id: number,
    input: UpdateAppointmentStatusInput,
  ) => invoke<Appointment>("set_appointment_status", { id, input }),
  cancelAppointment: (id: number) => invoke<Appointment>("cancel_appointment", { id }),
  checkInAppointment: (id: number) => invoke<Visit>("check_in_appointment", { id }),

  // ---- Peluquería (sesiones y adjuntos de visita) ----
  getGroomingSession: (visitServiceId: number) =>
    invoke<GroomingSession | null>("get_grooming_session", { visitServiceId }),
  upsertGroomingSession: (input: UpsertGroomingSessionInput) =>
    invoke<GroomingSession>("upsert_grooming_session", { input }),
  finishGroomingSession: (
    visitServiceId: number,
    input: FinishGroomingSessionInput,
  ) => invoke<GroomingSession>("finish_grooming_session", { visitServiceId, input }),
  listGroomingDay: (date: string) =>
    invoke<GroomingDayLine[]>("list_grooming_day", { date }),
  listGroomingReminders: () =>
    invoke<GroomingReminder[]>("list_grooming_reminders"),
  listVisitAttachments: (visitId: number) =>
    invoke<VisitAttachment[]>("list_visit_attachments", { visitId }),
  addVisitAttachment: (sourcePath: string, visitId: number, kind: string | null) =>
    invoke<VisitAttachment>("add_visit_attachment", {
      sourcePath,
      input: { visitId, kind },
    }),
  deleteVisitAttachment: (id: number) =>
    invoke<void>("delete_visit_attachment", { id }),

  // ---- Estancia (guardería, hospedaje, hospitalización) ----
  listResources: (includeInactive: boolean | null) =>
    invoke<Resource[]>("list_resources", { includeInactive }),
  listResourceBoard: () =>
    invoke<ResourceBoardItem[]>("list_resource_board"),
  saveResource: (input: SaveResourceInput) =>
    invoke<Resource>("save_resource", { input }),
  listStays: (
    status: string | null,
    kind: string | null,
    patientId: number | null,
  ) => invoke<StayListItem[]>("list_stays", { status, kind, patientId }),
  getStay: (id: number) => invoke<Stay | null>("get_stay", { id }),
  createStay: (input: CreateStayInput) => invoke<Stay>("create_stay", { input }),
  checkOutStay: (id: number, input: CheckOutStayInput) =>
    invoke<Stay>("check_out_stay", { id, input }),
  cancelStay: (id: number) => invoke<Stay>("cancel_stay", { id }),
  addStayEvent: (stayId: number, input: CreateStayEventInput) =>
    invoke<StayEvent>("add_stay_event", { stayId, input }),
  listStayEvents: (stayId: number) =>
    invoke<StayEvent[]>("list_stay_events", { stayId }),

  // ---- Catálogo de productos e inventario ----
  listProducts: (
    search: string | null,
    category: string | null,
    includeInactive: boolean | null,
  ) =>
    invoke<ProductStockItem[]>("list_products", {
      search,
      category,
      includeInactive,
    }),
  getProduct: (id: number) => invoke<Product | null>("get_product", { id }),
  saveProduct: (input: SaveProductInput) =>
    invoke<Product>("save_product", { input }),
  setProductActive: (id: number, active: boolean) =>
    invoke<Product>("set_product_active", { id, active }),
  addStockMovement: (input: CreateStockMovementInput) =>
    invoke<StockMovement>("add_stock_movement", { input }),
  listStockMovements: (productId: number | null, limit: number | null) =>
    invoke<StockMovement[]>("list_stock_movements", { productId, limit }),

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
