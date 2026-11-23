// ============================================================================
// Allowlist del mock IPC de los E2E.
//
// `scripts/check-e2e-mock.mjs` exige que TODO comando invocable por la UI
// (src/bindings.ts + src/lib/api.ts) tenga handler en e2e/ipc-mock.script.js o
// esté declarado aquí. Estar en esta lista NO hace que el mock responda: si un
// spec invoca el comando, el mock lanza un error legible y el test falla. La
// lista solo documenta la decisión de no modelarlo todavía.
//
// Criterio para entrar aquí: el comando es una mutación, un PDF, una
// exportación o una lectura de una vista que ningún spec recorre con datos.
// Si un spec empieza a necesitarlo, se implementa el handler y se borra la
// entrada (el script falla si una entrada sobra o si ya tiene handler).
// ============================================================================

export const allowlistedCommands = [
  // --- Conexión a Firebird / respaldo / exportaciones (solo escritorio) ----
  "get_db_connection",
  "save_db_connection",
  "test_db_connection",
  "get_server_backup_status",
  "export_local_config",
  "export_samples_csv",
  "export_results_csv",

  // --- Pacientes y consultas -------------------------------------------------
  "create_patient",
  "get_patient_by_code",
  "create_consultation",
  "set_consultation_status",

  // --- Citas -----------------------------------------------------------------
  "create_appointment",
  "get_appointment",
  "set_appointment_status",
  "cancel_appointment",

  // --- Muestras: calidad, rechazo, correo crítico ----------------------------
  "set_sample_quality",
  "reject_sample",
  "reopen_sample",
  "acknowledge_critical",
  "send_critical_email",
  "test_smtp_connection",

  // --- Órdenes de laboratorio y analizadores --------------------------------
  "create_lab_order",
  "get_lab_order",
  "set_lab_order_status",
  "accession_lab_order",
  "get_order_for_sample",
  "preview_analyzer_import",
  "import_analyzer_results",
  "save_analyzer_source",
  "delete_analyzer_source",
  "delete_analyzer_import_job",
  "create_analyzer",
  "update_analyzer",
  "set_analyzer_active",
  "delete_analyzer",

  // --- Paneles, QC y rangos --------------------------------------------------
  "save_panel",
  "delete_panel",
  "delete_reference_range",
  "save_qc_material",
  "delete_qc_material",
  "record_qc_run",
  "delete_qc_run",

  // --- Catálogo de servicios -------------------------------------------------
  "save_service_category",
  "delete_service_category",
  "get_service",
  "save_service",
  "set_service_active",

  // --- Documentos generados (PDFs con diálogo de guardar del SO) ------------
  "generate_clinical_report",
  "generate_formula_medica",
  "generate_consentimiento",
  "generate_certificado_cirugia",
  "generate_carnet_vacunacion",

  // --- Adjuntos y archivos ---------------------------------------------------
  "list_visit_attachments",
  "add_visit_attachment",
  "delete_visit_attachment",
  "get_visit_attachment_content",
  "open_visit_attachment",
  "attach_result_file",
  "delete_result_attachment",
  "get_result_attachment_content",

  // --- Marca y certificados PKCS#12 -----------------------------------------
  "delete_clinic_logo",
  "get_pkcs12_content",
  "import_pkcs12",
  "delete_pkcs12",

  // --- IA (Groq) -------------------------------------------------------------
  "test_groq_connection",
  "interpret_lab_results",
  "get_patient_lab_trends",

  // --- Usuarios --------------------------------------------------------------
  "create_user",
  "change_password",

  // --- Cirugías y vacunas (alta desde sus diálogos) --------------------------
  "create_surgery",
  "set_surgery_status",
  "create_vaccine",

  // --- Peluquería (sesiones) -------------------------------------------------
  "get_grooming_session",
  "upsert_grooming_session",
  "finish_grooming_session",

  // --- Estancias y hospedaje -------------------------------------------------
  "get_stay",
  "list_stay_events",
  "create_stay",
  "check_out_stay",
  "cancel_stay",
  "add_stay_event",

  // --- Recursos y productos --------------------------------------------------
  "save_resource",
  "get_product",
  "save_product",
  "set_product_active",
  "add_stock_movement",
];
