use chrono::{DateTime, Local};
use rsfbclient::prelude::*;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::auth::{require_session, require_vet_or_admin};
use crate::error::AppError;
use crate::models::report::ReportFile;
use crate::models::settings::ClinicSettings;
use crate::pdf_templates::{
    CirugiaData, ClinicHeader, ClinicalReportData, ConsentimientoData, FormulaMedicaData,
    ReciboData, ReportSignature, VacunacionData,
};
use crate::repositories::{
    clinical_history as history_repo, invoices as invoices_repo, patient as patient_repo,
    samples as samples_repo, settings as settings_repo, surgeries as surgeries_repo,
    vaccines as vaccines_repo,
};
use crate::state::AppState;

fn reports_dir(app: &AppHandle) -> Result<std::path::PathBuf, AppError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("Sin carpeta de datos: {e}")))?
        .join("reports");
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Internal(format!("No se pudo crear la carpeta de reportes: {e}")))?;
    Ok(dir)
}

fn now_db(conn: &mut rsfbclient::SimpleConnection) -> Result<String, AppError> {
    conn.query_first(
        "SELECT LEFT(CAST(CURRENT_TIMESTAMP AS VARCHAR(60)), 19) FROM rdb$database",
        (),
    )
    .map_err(AppError::from)
    .map(|r| r.map(|(v,): (String,)| v).unwrap_or_default())
}

/// Encabezado de clínica a partir de la configuración.
fn clinic_header(s: &ClinicSettings) -> ClinicHeader {
    ClinicHeader {
        name: s.clinic_name.clone(),
        nit: s.clinic_nit.clone(),
        address: s.address.clone(),
        phone: s.phone.clone(),
        city: s.city.clone(),
        logo_path: s.logo_path.clone(),
    }
}

/// Bloque de firma del veterinario a partir de la configuración.
/// La contraseña PKCS#12 viene del estado en memoria (nunca de la BD).
fn report_signature(s: &ClinicSettings, pkcs12_password: Option<String>) -> ReportSignature {
    ReportSignature {
        mode: s.signature_mode.clone(),
        vet_name: s.vet_name.clone(),
        vet_license: s.vet_license.clone(),
        signature_image_path: None,
        pkcs12_path: s.pkcs12_path.clone(),
        pkcs12_password,
    }
}

/// Lee la contraseña PKCS#12 desde el estado en memoria.
fn pkcs12_password_from(state: &AppState) -> Option<String> {
    state.pkcs12_password.lock().ok().and_then(|g| g.clone())
}

/// Firma criptográficamente un PDF ya generado si la clínica usa modo DIGITAL
/// y tiene certificado + contraseña en memoria. Devuelve Ok sin hacer nada si
/// no aplica, o un error claro si falta la contraseña.
fn apply_digital_signature(
    state: &AppState,
    settings: &ClinicSettings,
    out_path: &std::path::Path,
    doc_name: &str,
) -> Result<(), AppError> {
    if !settings.signature_mode.eq_ignore_ascii_case("DIGITAL") {
        return Ok(());
    }
    let Some(ref p12) = settings.pkcs12_path else {
        return Ok(()); // sin certificado → se queda con firma gráfica/visible
    };
    if !std::path::Path::new(p12).exists() {
        return Err(AppError::Validation(
            "El certificado digital PKCS#12 ya no existe. Vuelve a importarlo en Configuración."
                .into(),
        ));
    }
    let password = pkcs12_password_from(state).ok_or_else(|| {
        AppError::Validation(
            "Para firmar digitalmente este reporte reingresa la contraseña del certificado PKCS#12 en Configuración.".into(),
        )
    })?;

    crate::pdf_templates::sign_pdf_with_pkcs12(
        out_path,
        out_path,
        std::path::Path::new(p12),
        &password,
        &settings.vet_name,
        doc_name,
    )
}

/// Genera el informe PDF de una muestra (con resultados) y devuelve la ruta.
#[tauri::command]
#[specta::specta]
pub fn generate_clinical_report(
    state: State<'_, AppState>,
    app: AppHandle,
    sample_id: i32,
    override_logo_path: Option<String>,
    save_logo_preference: Option<bool>,
) -> Result<ReportFile, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let conn = pooled.conn();

    let sample = samples_repo::get(conn, sample_id)?
        .ok_or_else(|| AppError::NotFound(format!("Muestra {sample_id} no encontrada")))?;
    if sample.status == "ANULADA" {
        return Err(AppError::Validation(
            "No se puede generar un informe de una muestra anulada".into(),
        ));
    }

    let mut patient = patient_repo::get(conn, sample.patient_id)?.ok_or_else(|| {
        AppError::NotFound(format!("Paciente {} no encontrado", sample.patient_id))
    })?;
    let settings = settings_repo::get(conn)?;

    // Handle logo override and preference
    let mut clinic = clinic_header(&settings);
    if let Some(ref path) = override_logo_path {
        clinic.logo_path = Some(path.clone());
    }

    if save_logo_preference.unwrap_or(false) {
        if let Some(ref path) = override_logo_path {
            if let Ok(logos) = crate::repositories::logos::list(conn) {
                if let Some(logo) = logos.iter().find(|l| l.logo_path == *path) {
                    // Actualizar en base de datos la preferencia
                    let _ = conn.execute(
                        "UPDATE PATIENTS SET PREFERRED_LOGO_ID = ? WHERE ID = ?",
                        (&logo.id, &patient.id),
                    );
                    patient.preferred_logo_id = Some(logo.id);
                }
            }
        } else {
            // Si elige el por defecto y pide guardar preferencia, limpiamos el campo
            let _ = conn.execute(
                "UPDATE PATIENTS SET PREFERRED_LOGO_ID = NULL WHERE ID = ?",
                (&patient.id,),
            );
            patient.preferred_logo_id = None;
        }
    }

    let data = ClinicalReportData {
        clinic,
        patient,
        sample_code: sample.code.clone(),
        sample_type: sample.sample_type_name.clone(),
        received_at: sample.received_at.clone(),
        results: sample.results,
        signature: report_signature(&settings, pkcs12_password_from(&state)),
    };

    let dir = reports_dir(&app)?;
    let file_name = format!("{}-resultados.pdf", sample.code);
    let out_path = dir.join(&file_name);
    crate::pdf_templates::generate_report(&data, &out_path).map_err(AppError::Internal)?;
    apply_digital_signature(
        &state,
        &settings,
        &out_path,
        "Informe de resultados de laboratorio",
    )?;

    let generated_at = now_db(conn)?;
    Ok(ReportFile {
        path: out_path.display().to_string(),
        file_name,
        sample_code: sample.code,
        generated_at,
    })
}

/// Genera la fórmula médica (receta) de una consulta y devuelve la ruta.
#[tauri::command]
#[specta::specta]
pub fn generate_formula_medica(
    state: State<'_, AppState>,
    app: AppHandle,
    consultation_id: i32,
) -> Result<ReportFile, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let conn = pooled.conn();

    let consultation = history_repo::get_consultation(conn, consultation_id)?
        .ok_or_else(|| AppError::NotFound(format!("Consulta {consultation_id} no encontrada")))?;
    let patient = patient_repo::get(conn, consultation.patient_id)?.ok_or_else(|| {
        AppError::NotFound(format!(
            "Paciente {} no encontrado",
            consultation.patient_id
        ))
    })?;
    let owner = patient_repo::get_owner(conn, patient.owner_id)?;
    let settings = settings_repo::get(conn)?;

    let data = FormulaMedicaData {
        clinic: clinic_header(&settings),
        patient,
        owner,
        date: consultation.consultation_date,
        reason: consultation.reason,
        diagnosis: consultation.diagnosis,
        medication: consultation.treatment_plan,
        signature: report_signature(&settings, pkcs12_password_from(&state)),
    };

    let dir = reports_dir(&app)?;
    let file_name = format!("formula-{consultation_id}.pdf");
    let out_path = dir.join(&file_name);
    crate::pdf_templates::generate_formula(&data, &out_path).map_err(AppError::Internal)?;
    apply_digital_signature(&state, &settings, &out_path, "Fórmula médica veterinaria")?;

    let generated_at = now_db(conn)?;
    Ok(ReportFile {
        path: out_path.display().to_string(),
        file_name,
        sample_code: format!("FÓRMULA {consultation_id}"),
        generated_at,
    })
}

/// Genera el consentimiento informado de una cirugía programada.
#[tauri::command]
#[specta::specta]
pub fn generate_consentimiento(
    state: State<'_, AppState>,
    app: AppHandle,
    surgery_id: i32,
) -> Result<ReportFile, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let conn = pooled.conn();

    let surgery = surgeries_repo::get(conn, surgery_id)?
        .ok_or_else(|| AppError::NotFound(format!("Cirugía {surgery_id} no encontrada")))?;
    if surgery.status == "CANCELADA" {
        return Err(AppError::Validation(
            "No se puede generar el consentimiento de una cirugía cancelada".into(),
        ));
    }
    let patient = patient_repo::get(conn, surgery.patient_id)?.ok_or_else(|| {
        AppError::NotFound(format!("Paciente {} no encontrado", surgery.patient_id))
    })?;
    let owner = patient_repo::get_owner(conn, patient.owner_id)?;
    let settings = settings_repo::get(conn)?;

    let data = ConsentimientoData {
        clinic: clinic_header(&settings),
        patient,
        owner,
        attention_code: format!("CIR-{}", surgery.id),
        procedure_type: surgery.surgery_type.clone(),
        procedure_date: surgery.scheduled_at.clone(),
        description: surgery.preoperative_notes.clone(),
        post_care: surgery.postoperative_notes.clone(),
        veterinarian: settings.vet_name.clone(),
    };

    let dir = reports_dir(&app)?;
    let file_name = format!("consentimiento-{surgery_id}.pdf");
    let out_path = dir.join(&file_name);
    crate::pdf_templates::generate_consentimiento(&data, &out_path).map_err(AppError::Internal)?;

    let generated_at = now_db(conn)?;
    Ok(ReportFile {
        path: out_path.display().to_string(),
        file_name,
        sample_code: format!("CONSENTIMIENTO {surgery_id}"),
        generated_at,
    })
}

/// Genera el comprobante de pago (recibo) de una factura.
#[tauri::command]
#[specta::specta]
pub fn generate_recibo_invoice(
    state: State<'_, AppState>,
    app: AppHandle,
    invoice_id: i32,
) -> Result<ReportFile, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let conn = pooled.conn();

    let invoice = invoices_repo::get(conn, invoice_id)?
        .ok_or_else(|| AppError::NotFound(format!("Factura {invoice_id} no encontrada")))?;
    if invoice.status == "ANULADA" {
        return Err(AppError::Validation(
            "No se puede generar el recibo de una factura anulada".into(),
        ));
    }
    let settings = settings_repo::get(conn)?;

    let data = ReciboData {
        clinic: clinic_header(&settings),
        invoice,
    };

    let dir = reports_dir(&app)?;
    let file_name = format!("recibo-{}.pdf", data.invoice.invoice_number);
    let out_path = dir.join(&file_name);
    crate::pdf_templates::generate_recibo(&data, &out_path).map_err(AppError::Internal)?;

    let generated_at = now_db(conn)?;
    Ok(ReportFile {
        path: out_path.display().to_string(),
        file_name,
        sample_code: data.invoice.invoice_number.clone(),
        generated_at,
    })
}

/// Genera el reporte/certificado quirúrgico de una cirugía.
#[tauri::command]
#[specta::specta]
pub fn generate_certificado_cirugia(
    state: State<'_, AppState>,
    app: AppHandle,
    surgery_id: i32,
) -> Result<ReportFile, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let conn = pooled.conn();

    let surgery = surgeries_repo::get(conn, surgery_id)?
        .ok_or_else(|| AppError::NotFound(format!("Cirugía {surgery_id} no encontrada")))?;
    let patient = patient_repo::get(conn, surgery.patient_id)?.ok_or_else(|| {
        AppError::NotFound(format!("Paciente {} no encontrado", surgery.patient_id))
    })?;
    let owner = patient_repo::get_owner(conn, patient.owner_id)?;
    let settings = settings_repo::get(conn)?;

    let data = CirugiaData {
        clinic: clinic_header(&settings),
        patient,
        owner,
        surgery,
        signature: report_signature(&settings, pkcs12_password_from(&state)),
    };

    let dir = reports_dir(&app)?;
    let file_name = format!("cirugia-{surgery_id}.pdf");
    let out_path = dir.join(&file_name);
    crate::pdf_templates::generate_cirugia(&data, &out_path).map_err(AppError::Internal)?;
    apply_digital_signature(&state, &settings, &out_path, "Certificado quirúrgico")?;

    let generated_at = now_db(conn)?;
    Ok(ReportFile {
        path: out_path.display().to_string(),
        file_name,
        sample_code: format!("CIRUGÍA {surgery_id}"),
        generated_at,
    })
}

/// Genera el certificado/carnet de vacunación de un paciente.
#[tauri::command]
#[specta::specta]
pub fn generate_carnet_vacunacion(
    state: State<'_, AppState>,
    app: AppHandle,
    patient_id: i32,
) -> Result<ReportFile, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let conn = pooled.conn();

    let patient = patient_repo::get(conn, patient_id)?
        .ok_or_else(|| AppError::NotFound(format!("Paciente {patient_id} no encontrado")))?;
    let owner = patient_repo::get_owner(conn, patient.owner_id)?;
    let vaccines = vaccines_repo::by_patient(conn, patient_id)?;
    if vaccines.is_empty() {
        return Err(AppError::Validation(
            "El paciente no tiene vacunas registradas; no se puede generar el carnet".into(),
        ));
    }
    let settings = settings_repo::get(conn)?;

    let issued_at = now_db(conn)?;
    let data = VacunacionData {
        clinic: clinic_header(&settings),
        patient,
        owner,
        vaccines,
        certificate_number: format!("CERT-{patient_id}"),
        issued_at: issued_at.clone(),
        veterinarian: settings.vet_name.clone(),
    };

    let dir = reports_dir(&app)?;
    let file_name = format!("vacunacion-{patient_id}.pdf");
    let out_path = dir.join(&file_name);
    crate::pdf_templates::generate_vacunacion(&data, &out_path).map_err(AppError::Internal)?;

    Ok(ReportFile {
        path: out_path.display().to_string(),
        file_name,
        sample_code: format!("VACUNACIÓN {patient_id}"),
        generated_at: issued_at,
    })
}

/// Genera una hoja de etiquetas imprimibles para los tubos de las muestras
/// indicadas (código de barras Code 128 + datos) y devuelve la ruta del PDF.
#[tauri::command]
#[specta::specta]
pub fn generate_sample_labels(
    state: State<'_, AppState>,
    app: AppHandle,
    sample_ids: Vec<i32>,
) -> Result<ReportFile, AppError> {
    require_vet_or_admin(&state)?;
    if sample_ids.is_empty() {
        return Err(AppError::Validation(
            "Selecciona al menos una muestra para etiquetar".into(),
        ));
    }
    if sample_ids.len() > 100 {
        return Err(AppError::Validation(
            "Máximo 100 etiquetas por hoja; selecciona menos muestras".into(),
        ));
    }
    let mut pooled = state.pool.acquire()?;
    let conn = pooled.conn();

    let samples = samples_repo::list_by_ids(conn, &sample_ids)?;
    if samples.is_empty() {
        return Err(AppError::NotFound("Ninguna muestra encontrada".into()));
    }

    let dir = reports_dir(&app)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let label = if samples.len() == 1 {
        format!("etiqueta-{}.pdf", samples[0].code)
    } else {
        format!("etiquetas-{stamp}.pdf")
    };
    let out_path = dir.join(&label);
    crate::pdf_templates::generate_sample_labels(&samples, &out_path)
        .map_err(AppError::Internal)?;

    let generated_at = now_db(conn)?;
    Ok(ReportFile {
        path: out_path.display().to_string(),
        file_name: label,
        sample_code: samples[0].code.clone(),
        generated_at,
    })
}

/// Lista los informes ya generados (carpeta app_data/reports), más reciente primero.
#[tauri::command]
#[specta::specta]
pub fn list_reports(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<ReportFile>, AppError> {
    require_session(&state)?;
    let dir = reports_dir(&app)?;

    let mut reports: Vec<ReportFile> = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| AppError::Internal(format!("No se pudo leer la carpeta de reportes: {e}")))?
        .flatten()
    {
        let path = entry.path();
        if path.extension().map(|e| e != "pdf").unwrap_or(true) {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        // Referencia legible según el prefijo del archivo.
        let reference = file_name
            .strip_suffix("-resultados.pdf")
            .map(|s| s.to_string())
            .or_else(|| file_name.strip_suffix(".pdf").map(|s| s.to_string()))
            .unwrap_or_else(|| file_name.clone());
        let generated_at = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let dt: DateTime<Local> = DateTime::from(t);
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_default();

        reports.push(ReportFile {
            path: path.display().to_string(),
            file_name,
            sample_code: reference,
            generated_at,
        });
    }

    // Orden estable: fecha desc (formato YYYY-MM-DD HH:MM:SS ordenable).
    reports.sort_by(|a, b| {
        b.generated_at
            .cmp(&a.generated_at)
            .then_with(|| b.file_name.cmp(&a.file_name))
    });
    Ok(reports)
}

/// Abre un archivo de reporte PDF en el visor por defecto del sistema operativo.
#[tauri::command]
#[specta::specta]
pub fn open_report_file(
    state: State<'_, AppState>,
    app: AppHandle,
    path: String,
) -> Result<(), AppError> {
    require_session(&state)?;
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err(AppError::Validation("El archivo PDF no existe".into()));
    }
    app.opener()
        .open_path(p.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::Internal(format!("No se pudo abrir el PDF: {e}")))?;
    Ok(())
}
