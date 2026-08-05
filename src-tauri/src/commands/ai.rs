use serde_json::json;
use tauri::State;

use crate::ai_cache::hash_results;
use crate::auth::require_session;
use crate::error::AppError;
use crate::models::consultation::Consultation;
use crate::models::patient::Patient;
use crate::models::sample::{LabResult, Sample};
use crate::models::vaccine::Vaccine;
use crate::repositories::clinical_history as ch_repo;
use crate::repositories::patient as patients_repo;
use crate::repositories::samples as samples_repo;
use crate::repositories::settings as settings_repo;
use crate::repositories::vaccines as vaccines_repo;
use crate::state::AppState;

/// Contexto clínico adicional para enriquecer el prompt de IA.
pub(crate) struct ClinicalContext {
    pub recent_consultations: Vec<Consultation>,
    pub vaccines: Vec<Vaccine>,
    pub previous_results: Vec<LabResult>,
}

/// Construye el prompt para la IA basado en el paciente, muestra, resultados
/// y contexto clínico adicional (consultas recientes, vacunas, resultados previos).
pub(crate) fn build_interpretation_prompt(
    patient: &Patient,
    sample: &Sample,
    results: &[LabResult],
    ctx: Option<&ClinicalContext>,
) -> String {
    let mut prompt = String::from(
        "Eres un veterinario experto en medicina interna y diagnóstico de laboratorio. \
        Analiza los siguientes resultados de laboratorio de un paciente y proporciona:\n\n\
        1. **Interpretación clínica**: Análisis de cada resultado anormal y su significado clínico\n\
        2. **Diagnósticos diferenciales**: Posibles causas de las alteraciones encontradas, ordenadas por probabilidad\n\
        3. **Recomendaciones**: Estudios complementarios, tratamiento sugerido y seguimiento\n\
        4. **Urgencia**: Nivel de urgencia (Baja/Media/Alta/Crítica) basado en los hallazgos\n\n\n"
    );

    // Información del paciente
    prompt.push_str("## Paciente\n");
    prompt.push_str(&format!("- **Nombre**: {}\n", patient.name));
    prompt.push_str(&format!("- **Especie/Raza**: {} / {}\n", 
        patient.species_name, 
        patient.breed_name.as_deref().unwrap_or("No especificada")
    ));
    prompt.push_str(&format!("- **Edad**: {} meses ({})\n", 
        patient.age_months,
        patient.birth_date.as_deref().unwrap_or("fecha de nacimiento desconocida")
    ));
    prompt.push_str(&format!("- **Sexo**: {}{}\n", 
        if patient.sex == "M" { "Macho" } else { "Hembra" },
        if patient.neutered { " (castrado/a)" } else { "" }
    ));
    if let Some(color) = &patient.color {
        prompt.push_str(&format!("- **Color**: {}\n", color));
    }
    if let Some(notes) = &patient.notes {
        if !notes.is_empty() {
            prompt.push_str(&format!("- **Notas clínicas**: {}\n", notes));
        }
    }
    prompt.push('\n');

    // Información de la muestra
    prompt.push_str("## Muestra\n");
    prompt.push_str(&format!("- **Tipo**: {}\n", sample.sample_type_name));
    prompt.push_str(&format!("- **Fecha de recepción**: {}\n", sample.received_at));
    if let Some(collector) = &sample.collected_by {
        prompt.push_str(&format!("- **Recogida por**: {}\n", collector));
    }
    if let Some(notes) = &sample.notes {
        if !notes.is_empty() {
            prompt.push_str(&format!("- **Notas**: {}\n", notes));
        }
    }
    prompt.push('\n');

    // Contexto clínico adicional
    if let Some(ctx) = ctx {
        // Consultas recientes
        if !ctx.recent_consultations.is_empty() {
            prompt.push_str("## Historial de consultas recientes\n");
            for c in ctx.recent_consultations.iter().take(3) {
                prompt.push_str(&format!(
                    "- **{}** ({}): {}\n",
                    c.consultation_date,
                    c.status,
                    c.reason
                ));
                if let Some(dx) = &c.diagnosis {
                    if !dx.is_empty() {
                        prompt.push_str(&format!("  - Diagnóstico previo: {}\n", dx));
                    }
                }
                if let Some(tx) = &c.treatment_plan {
                    if !tx.is_empty() {
                        prompt.push_str(&format!("  - Tratamiento: {}\n", tx));
                    }
                }
            }
            prompt.push('\n');
        }

        // Estado de vacunación
        if !ctx.vaccines.is_empty() {
            prompt.push_str("## Estado de vacunación\n");
            for v in ctx.vaccines.iter().take(5) {
                prompt.push_str(&format!(
                    "- {} ({}) - {}\n",
                    v.vaccine_name,
                    v.administered_at,
                    v.manufacturer.as_deref().unwrap_or("Fabricante desconocido")
                ));
            }
            prompt.push('\n');
        }

        // Resultados previos para comparación
        if !ctx.previous_results.is_empty() {
            prompt.push_str("## Resultados de laboratorio previos (para comparación)\n");
            for r in ctx.previous_results.iter().take(10) {
                let range = match (r.ref_min, r.ref_max) {
                    (Some(min), Some(max)) => format!(" (Ref: {}-{})", min, max),
                    _ => "".to_string(),
                };
                prompt.push_str(&format!(
                    "- {}: {} {}{} [{}]\n",
                    r.analyte_name,
                    r.value,
                    r.unit.as_deref().unwrap_or(""),
                    range,
                    r.status
                ));
            }
            prompt.push('\n');
        }
    }

    // Resultados actuales
    prompt.push_str("## Resultados actuales\n\n");
    prompt.push_str("| Analito | Resultado | Rango de referencia | Estado |\n");
    prompt.push_str("|---------|-----------|---------------------|--------|\n");
    for r in results {
        let range = match (r.ref_min, r.ref_max) {
            (Some(min), Some(max)) => format!("{} - {}", min, max),
            _ => "Sin rango".to_string(),
        };
        let status_emoji = match r.status.as_str() {
            "ALTO" => "🔴 ALTO",
            "BAJO" => "🟡 BAJO",
            "NORMAL" => "🟢 NORMAL",
            _ => "⚪ SIN_RANGO",
        };
        prompt.push_str(&format!(
            "| {} | {} {} | {} | {} |\n",
            r.analyte_name,
            r.value,
            r.unit.as_deref().unwrap_or(""),
            range,
            status_emoji
        ));
    }
    prompt.push('\n');

    // Instrucciones finales
    prompt.push_str(
        "Responde en español usando formato Markdown. Si hay resultados fuera de rango, "
    );
    prompt.push_str(
        "prioriza su interpretación. Si el paciente tiene un historial relevante, "
    );
    prompt.push_str(
        "considéralo en tu análisis. Sé preciso y conciso."
    );

    prompt
}

/// Extrae el contenido de texto de la respuesta de la API de Groq.
pub(crate) fn parse_groq_response(response: &serde_json::Value) -> Result<String, AppError> {
    response["choices"][0]["message"]["content"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| AppError::Internal("No se recibió respuesta de la IA.".into()))
}

#[tauri::command]
#[specta::specta]
pub fn interpret_lab_results(
    state: State<'_, AppState>,
    sample_id: i32,
) -> Result<String, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;

    // 1. Fetch settings to get Groq API key
    let settings = settings_repo::get(pooled.conn())?;
    let api_key = settings.groq_api_key.ok_or_else(|| {
        AppError::Validation("La clave de API de Groq no está configurada. Ve a Configuración para agregarla.".into())
    })?;

    // 2. Fetch sample and results
    let sample = samples_repo::get(pooled.conn(), sample_id)?.ok_or_else(|| {
        AppError::NotFound(format!("Muestra {sample_id} no encontrada"))
    })?;

    // 3. Check cache first
    let results_hash = hash_results(
        &sample.results.iter().map(|r| (r.value, r.status.clone())).collect::<Vec<_>>()
    );
    if let Some(cached) = state.ai_cache.get(sample_id, results_hash) {
        return Ok(cached);
    }

    // 4. Fetch patient info
    let patient = patients_repo::get(pooled.conn(), sample.patient_id)?.ok_or_else(|| {
        AppError::NotFound(format!("Paciente {} no encontrado", sample.patient_id))
    })?;

    // 5. Fetch clinical context (consultations, vaccines, previous results)
    let recent_consultations = ch_repo::get_clinical_history(pooled.conn(), sample.patient_id)
        .map(|h| h.consultations)
        .unwrap_or_default();
    
    let vaccines = vaccines_repo::by_patient(pooled.conn(), sample.patient_id)
        .unwrap_or_default();
    
    // Get previous sample results for trend comparison
    let previous_results = get_previous_results(pooled.conn(), sample.patient_id, sample_id)
        .unwrap_or_default();

    let ctx = ClinicalContext {
        recent_consultations,
        vaccines,
        previous_results,
    };

    // 6. Construct prompt with clinical context
    let prompt = build_interpretation_prompt(&patient, &sample, &sample.results, Some(&ctx));

    // 7. Call Groq API via HTTP blocking request
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": "llama3-8b-8192",
            "messages": [
                {
                    "role": "system",
                    "content": "Eres un veterinario experto en medicina interna y diagnóstico de laboratorio clínico. Proporciona interpretaciones precisas, basadas en evidencia y considerando el contexto completo del paciente."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3,
            "max_tokens": 1500
        }))
        .send()
        .map_err(|e| AppError::Internal(format!("Error conectando a Groq API: {}", e)))?;

    if !res.status().is_success() {
        let error_text = res.text().unwrap_or_default();
        return Err(AppError::Internal(format!("Error de Groq API: {}", error_text)));
    }

    let response_json: serde_json::Value = res
        .json()
        .map_err(|e| AppError::Internal(format!("Respuesta inválida de Groq API: {}", e)))?;

    let interpretation = parse_groq_response(&response_json)?;

    // 8. Cache the result
    state.ai_cache.set(sample_id, interpretation.clone(), results_hash);

    Ok(interpretation)
}

type PrevResultRow = (i32, i32, i32, String, Option<String>, f64, String, Option<f64>, Option<f64>, Option<String>);

/// Obtiene los resultados de laboratorio previos del paciente (excluyendo la muestra actual).
fn get_previous_results(
    conn: &mut rsfbclient::SimpleConnection,
    patient_id: i32,
    current_sample_id: i32,
) -> Result<Vec<LabResult>, AppError> {
    use rsfbclient::prelude::*;
    
    let rows: Vec<PrevResultRow> = conn
        .query(
            "SELECT r.ID, r.SAMPLE_ID, r.ANALYTE_ID, a.NAME, a.UNIT,
                    r.RESULT_VALUE, r.STATUS, rr.MIN_VALUE, rr.MAX_VALUE,
                    LEFT(CAST(r.ANALYZED_AT AS VARCHAR(60)), 19)
             FROM LAB_RESULTS r
             JOIN ANALYTES a ON a.ID = r.ANALYTE_ID
             JOIN SAMPLES s ON s.ID = r.SAMPLE_ID
             LEFT JOIN REFERENCE_RANGES rr ON rr.ID = r.REFERENCE_RANGE_ID
             WHERE s.PATIENT_ID = ? AND r.SAMPLE_ID <> ?
             ORDER BY r.ANALYZED_AT DESC
             ROWS 20",
            (&patient_id, &current_sample_id),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| LabResult {
            id: r.0,
            sample_id: r.1,
            analyte_id: r.2,
            analyte_name: r.3,
            unit: r.4,
            value: r.5,
            status: r.6,
            ref_min: r.7,
            ref_max: r.8,
            analyzed_at: r.9,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_patient() -> Patient {
        Patient {
            id: 1,
            owner_id: 1,
            species_id: 1,
            breed_id: Some(1),
            name: "Luna".to_string(),
            sex: "F".to_string(),
            birth_date: Some("2023-06-15".to_string()),
            neutered: false,
            color: Some("Marrón".to_string()),
            microchip: None,
            active: true,
            notes: None,
            species_name: "Canino".to_string(),
            breed_name: Some("Beagle".to_string()),
            owner_name: "Juan Pérez".to_string(),
            owner_phone: Some("+57 300 1234567".to_string()),
            age_months: 24,
        }
    }

    fn test_sample() -> Sample {
        Sample {
            id: 1,
            code: "M-2026-0001".to_string(),
            patient_id: 1,
            sample_type_id: 1,
            sample_type_name: "Sangre total (EDTA)".to_string(),
            received_at: "2026-08-04 10:00:00".to_string(),
            status: "FINALIZADA".to_string(),
            collected_by: Some("Dr. García".to_string()),
            notes: None,
            results: vec![],
        }
    }

    fn test_lab_result(status: &str) -> LabResult {
        LabResult {
            id: 1,
            sample_id: 1,
            analyte_id: 1,
            analyte_name: "Hematocrito".to_string(),
            unit: Some("%".to_string()),
            value: 45.0,
            status: status.to_string(),
            ref_min: Some(37.0),
            ref_max: Some(55.0),
            analyzed_at: Some("2026-08-04 10:30:00".to_string()),
        }
    }

    #[test]
    fn test_build_prompt_with_normal_results() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![test_lab_result("NORMAL")];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("veterinario experto"));
        assert!(prompt.contains("Canino / Beagle"));
        assert!(prompt.contains("24 meses"));
        assert!(prompt.contains("Hembra"));
        assert!(prompt.contains("Sangre total (EDTA)"));
        assert!(prompt.contains("Hematocrito"));
        assert!(prompt.contains("45"));
        assert!(prompt.contains("37 - 55"));
        assert!(prompt.contains("🟢 NORMAL"));
        assert!(prompt.contains("Markdown"));
    }

    #[test]
    fn test_build_prompt_with_high_result() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![test_lab_result("ALTO")];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("🔴 ALTO"));
    }

    #[test]
    fn test_build_prompt_with_low_result() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![test_lab_result("BAJO")];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("🟡 BAJO"));
    }

    #[test]
    fn test_build_prompt_with_no_reference_range() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![LabResult {
            ref_min: None,
            ref_max: None,
            ..test_lab_result("SIN_RANGO")
        }];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("Sin rango"));
        assert!(prompt.contains("⚪ SIN_RANGO"));
    }

    #[test]
    fn test_build_prompt_with_empty_results() {
        let patient = test_patient();
        let sample = test_sample();
        let results: Vec<LabResult> = vec![];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("Resultados actuales"));
        assert!(prompt.contains("Markdown"));
    }

    #[test]
    fn test_build_prompt_with_optional_patient_fields() {
        let mut patient = test_patient();
        patient.breed_name = None;
        patient.birth_date = None;
        patient.color = None;
        patient.notes = None;
        let sample = test_sample();
        let results = vec![];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("Canino / No especificada"));
        assert!(prompt.contains("fecha de nacimiento desconocida"));
    }

    #[test]
    fn test_build_prompt_multiple_results() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![
            LabResult {
                analyte_name: "Hematocrito".to_string(),
                value: 45.0,
                ..test_lab_result("NORMAL")
            },
            LabResult {
                id: 2,
                analyte_name: "Glucosa".to_string(),
                value: 120.0,
                unit: Some("mg/dL".to_string()),
                ..test_lab_result("ALTO")
            },
        ];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("Hematocrito"));
        assert!(prompt.contains("Glucosa"));
        assert!(prompt.contains("120"));
        assert!(prompt.contains("mg/dL"));
    }

    #[test]
    fn test_build_prompt_with_clinical_context() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![test_lab_result("ALTO")];

        let ctx = ClinicalContext {
            recent_consultations: vec![
                Consultation {
                    id: 1,
                    patient_id: 1,
                    veterinarian_id: Some(1),
                    consultation_date: "2026-07-15 10:00:00".to_string(),
                    reason: "Vacunación anual".to_string(),
                    anamnesis: None,
                    physical_exam: None,
                    diagnosis: Some("Paciente sana".to_string()),
                    treatment_plan: Some("Aplicar vacuna antirrábica".to_string()),
                    status: "COMPLETADA".to_string(),
                    veterinarian_name: Some("Dr. García".to_string()),
                },
            ],
            vaccines: vec![
                Vaccine {
                    id: 1,
                    patient_id: 1,
                    vaccine_type_id: Some(1),
                    vaccine_name: "Rabia".to_string(),
                    dose: Some("1ra".to_string()),
                    administered_at: "2026-07-15".to_string(),
                    next_dose_at: Some("2027-07-15".to_string()),
                    lot: Some("LOT123".to_string()),
                    manufacturer: Some("Zoetis".to_string()),
                    veterinarian_name: Some("Dr. García".to_string()),
                    notes: None,
                },
            ],
            previous_results: vec![],
        };

        let prompt = build_interpretation_prompt(&patient, &sample, &results, Some(&ctx));

        assert!(prompt.contains("Historial de consultas recientes"));
        assert!(prompt.contains("Vacunación anual"));
        assert!(prompt.contains("Paciente sana"));
        assert!(prompt.contains("Estado de vacunación"));
        assert!(prompt.contains("Rabia"));
        assert!(prompt.contains("Zoetis"));
    }

    #[test]
    fn test_build_prompt_with_previous_results() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![test_lab_result("ALTO")];

        let ctx = ClinicalContext {
            recent_consultations: vec![],
            vaccines: vec![],
            previous_results: vec![
                LabResult {
                    id: 10,
                    sample_id: 10,
                    analyte_id: 1,
                    analyte_name: "Hematocrito".to_string(),
                    unit: Some("%".to_string()),
                    value: 42.0,
                    status: "NORMAL".to_string(),
                    ref_min: Some(37.0),
                    ref_max: Some(55.0),
                    analyzed_at: Some("2026-06-01 10:00:00".to_string()),
                },
            ],
        };

        let prompt = build_interpretation_prompt(&patient, &sample, &results, Some(&ctx));

        assert!(prompt.contains("Resultados de laboratorio previos"));
        assert!(prompt.contains("42"));
    }

    #[test]
    fn test_parse_groq_response_success() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": "Los resultados son normales."
                }
            }]
        });

        let result = parse_groq_response(&response).unwrap();
        assert_eq!(result, "Los resultados son normales.");
    }

    #[test]
    fn test_parse_groq_response_empty_choices() {
        let response = json!({ "choices": [] });
        let result = parse_groq_response(&response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_groq_response_no_content() {
        let response = json!({
            "choices": [{
                "message": {}
            }]
        });
        let result = parse_groq_response(&response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_groq_response_null_content() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": null
                }
            }]
        });
        let result = parse_groq_response(&response);
        assert!(result.is_err());
    }
}
