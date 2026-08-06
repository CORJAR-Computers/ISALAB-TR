use serde_json::json;
use tauri::State;

use crate::ai_cache::hash_results;
use crate::auth::{require_admin, require_vet_or_admin};
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

/// Nota clínica breve específica de la especie para orientar la interpretación.
/// Los rangos mostrados ya son específicos de la especie/sexo/edad del paciente;
/// esta nota añade particularidades fisiopatológicas conocidas por especie.
fn species_specific_note(species_name: &str) -> &'static str {
    let name = species_name.to_lowercase();
    if name.contains("felino") || name.contains("gato") {
        "Considera la hiperglucemia de estrés (puede elevar la glucosa sin ser patológica) y que los valores de eritrocitos/hematocrito suelen ser fisiológicamente menores que en caninos."
    } else if name.contains("canino") || name.contains("perro") {
        "Considera enfermedades frecuentes de la especie al interpretar (p. ej., ehrlichiosis, leptospirosis, enfermedad renal crónica en geriátricos, pancreatitis) y que la leucocitosis es un hallazgo inespecífico de inflamación."
    } else if name.contains("equino") || name.contains("caballo") {
        "Considera el estado de hidratación y la relación urea/creatinina al interpretar la azotemia, y que la bilirrubina y GGT se afectan por ayuno y ejercicio."
    } else if name.contains("bovino") || name.contains("vaca") {
        "Considera el estado de hidratación y que la hipocalcemia/hipomagnesemia son alteraciones frecuentes en el periparto; la relación urea/creatinina orienta causas prerrenales vs. renales."
    } else if name.contains("ovino")
        || name.contains("caprino")
        || name.contains("oveja")
        || name.contains("cabra")
    {
        "Considera que la anemia y la hipoproteinemia frecuentemente reflejan parasitismo gastrointestinal; valora la carga parasitaria y el estado nutricional al interpretar los resultados."
    } else {
        "No se dispone de particularidades específicas para esta especie; basa la interpretación en los rangos de referencia mostrados y en la literatura general."
    }
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
        "Eres un veterinario experto en medicina interna y diagnóstico de laboratorio clínico. \
        Analiza los siguientes resultados de laboratorio de un paciente y proporciona:\n\n\
        1. **Interpretación clínica**: análisis de cada resultado anormal, su magnitud y su significado clínico\n\
        2. **Diagnósticos diferenciales**: posibles causas de las alteraciones, ordenadas por probabilidad\n\
        3. **Recomendaciones**: estudios complementarios, tratamiento sugerido y seguimiento\n\
        4. **Nivel de urgencia**: clasifica en BAJA, MEDIA, ALTA o CRÍTICA con su justificación\n\n\
        IMPORTANTE: los rangos de referencia mostrados ya son específicos de la especie, sexo y \
        edad del paciente. Siempre indica la magnitud de cada desviación respecto a su rango.\n\n\n"
    );

    // Información del paciente
    prompt.push_str("## Paciente\n");
    prompt.push_str(&format!("- **Código**: {}\n", patient.code));
    prompt.push_str(&format!("- **Nombre**: {}\n", patient.name));
    prompt.push_str(&format!(
        "- **Especie/Raza**: {} / {}\n",
        patient.species_name,
        patient.breed_name.as_deref().unwrap_or("No especificada")
    ));
    prompt.push_str(&format!(
        "- **Edad**: {} meses ({})\n",
        patient.age_months,
        patient
            .birth_date
            .as_deref()
            .unwrap_or("fecha de nacimiento desconocida")
    ));
    prompt.push_str(&format!(
        "- **Sexo**: {}{}\n",
        if patient.sex == "M" {
            "Macho"
        } else {
            "Hembra"
        },
        if patient.neutered {
            " (castrado/a)"
        } else {
            ""
        }
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

    // Consideración fisiopatológica por especie
    prompt.push_str("## Consideración por especie\n");
    prompt.push_str(&format!(
        "- {}\n\n",
        species_specific_note(&patient.species_name)
    ));

    // Información de la muestra
    prompt.push_str("## Muestra\n");
    prompt.push_str(&format!("- **Tipo**: {}\n", sample.sample_type_name));
    prompt.push_str(&format!(
        "- **Fecha de recepción**: {}\n",
        sample.received_at
    ));
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
                    c.consultation_date, c.status, c.reason
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
                    v.manufacturer
                        .as_deref()
                        .unwrap_or("Fabricante desconocido")
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
    prompt.push_str("| Analito | Resultado | Rango de referencia | Estado | Desviación |\n");
    prompt.push_str("|---------|-----------|---------------------|--------|------------|\n");
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
            "| {} | {} {} | {} | {} | {} |\n",
            r.analyte_name,
            r.value,
            r.unit.as_deref().unwrap_or(""),
            range,
            status_emoji,
            deviation_label(r)
        ));
    }
    prompt.push('\n');

    // Formato de respuesta y criterios de urgencia
    prompt.push_str(
        "Responde en español usando formato Markdown. Estructura tu respuesta con estos encabezados:\n\
         ## Interpretación clínica\n\
         ## Diagnósticos diferenciales\n\
         ## Recomendaciones\n\
         ## Nivel de urgencia\n\n\
         En la sección **Nivel de urgencia** usa EXACTAMENTE este formato de tres líneas:\n\
         **Nivel**: BAJA | MEDIA | ALTA | CRÍTICA\n\
         **Criterio**: <motivo clínico en una frase>\n\
         **Acción sugerida**: <qué hacer y en qué plazo>\n\n\
         Criterios orientativos para clasificar la urgencia:\n\
         - BAJA: resultados dentro de rango o desviaciones mínimas sin relevancia clínica.\n\
         - MEDIA: una o dos desviaciones leves/moderadas (hasta ~25 % fuera del rango).\n\
         - ALTA: desviaciones marcadas (25-50 %) o varias alteraciones que sugieren enfermedad sistémica.\n\
         - CRÍTICA: desviaciones severas (>50 %) en analitos de riesgo vital (glucosa, potasio, urea/creatinina, hematocrito, plaquetas) o signos de insuficiencia orgánica.\n\n\
         Si hay resultados fuera de rango, prioriza su interpretación e indica la magnitud de la \
         desviación. Considera el historial clínico del paciente si es relevante. Sé preciso y \
         conciso; no inventes valores ni diagnósticos definitivos."
    );

    prompt
}

/// Etiqueta de la magnitud de la desviación para un resultado fuera de rango.
fn deviation_label(r: &LabResult) -> String {
    match r.status.as_str() {
        "ALTO" => {
            if let Some(max) = r.ref_max {
                if max > 0.0 {
                    let pct = ((r.value - max) / max) * 100.0;
                    return format!("🔺 +{:.1}% sobre máx", pct.abs());
                }
            }
            "🔺 Fuera de rango".to_string()
        }
        "BAJO" => {
            if let Some(min) = r.ref_min {
                if min > 0.0 {
                    let pct = ((min - r.value) / min) * 100.0;
                    return format!("🔻 -{:.1}% bajo mín", pct.abs());
                }
            }
            "🔻 Fuera de rango".to_string()
        }
        "NORMAL" => "Dentro de rango".to_string(),
        _ => "—".to_string(),
    }
}

/// Extrae el contenido de texto de la respuesta de la API de Groq.
pub(crate) fn parse_groq_response(response: &serde_json::Value) -> Result<String, AppError> {
    response["choices"][0]["message"]["content"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| AppError::Internal("No se recibió respuesta de la IA.".into()))
}

/// Devuelve la clave de API de Groq de la configuración o un error de
/// validación claro si no está configurada.
pub(crate) fn groq_api_key_or_error(
    settings: &crate::models::settings::ClinicSettings,
) -> Result<String, AppError> {
    settings
        .groq_api_key
        .clone()
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            AppError::Validation(
                "La clave de API de Groq no está configurada. Ve a Configuración para agregarla."
                    .into(),
            )
        })
}

/// Construye el mensaje de error legible ante una respuesta HTTP de error de
/// la API de Groq. Mapea el 401 (clave inválida/revocada) y el 429 (rate limit)
/// a mensajes claros para el usuario.
pub(crate) fn groq_error_message(status: u16, body: &str) -> String {
    match status {
        401 => "Clave de API de Groq inválida o revocada (401). Verifica la clave en Configuración."
            .to_string(),
        429 => "Límite de solicitudes de Groq alcanzado (429 Rate Limit). Espera unos segundos e intenta de nuevo."
            .to_string(),
        503 => "Servicio de Groq temporalmente no disponible (503). Intenta de nuevo en unos minutos."
            .to_string(),
        _ => format!("Error de Groq API ({}): {}", status, body),
    }
}

/// Cuerpo de la solicitud mínima de prueba de conexión (sin contexto clínico).
pub(crate) fn groq_test_request_body() -> serde_json::Value {
    json!({
        "model": "llama-3.3-70b-versatile",
        "messages": [
            { "role": "user", "content": "Responde únicamente: OK" }
        ],
        "max_tokens": 5,
        "temperature": 0.0
    })
}

#[tauri::command]
#[specta::specta]
pub fn interpret_lab_results(
    state: State<'_, AppState>,
    sample_id: i32,
) -> Result<String, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;

    // 1. Fetch settings to get Groq API key
    let settings = settings_repo::get(pooled.conn())?;
    let api_key = groq_api_key_or_error(&settings)?;

    // 2. Fetch sample and results
    let sample = samples_repo::get(pooled.conn(), sample_id)?
        .ok_or_else(|| AppError::NotFound(format!("Muestra {sample_id} no encontrada")))?;

    // 3. Check cache first
    let results_hash = hash_results(
        &sample
            .results
            .iter()
            .map(|r| (r.value, r.status.clone()))
            .collect::<Vec<_>>(),
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

    let vaccines = vaccines_repo::by_patient(pooled.conn(), sample.patient_id).unwrap_or_default();

    // Get previous sample results for trend comparison
    let previous_results =
        get_previous_results(pooled.conn(), sample.patient_id, sample_id).unwrap_or_default();

    let ctx = ClinicalContext {
        recent_consultations,
        vaccines,
        previous_results,
    };

    // 6. Construct prompt with clinical context
    let prompt = build_interpretation_prompt(&patient, &sample, &sample.results, Some(&ctx));

    // 7. Call Groq API via HTTP blocking request with timeout
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| AppError::Internal(format!("Error creando cliente HTTP: {}", e)))?;
    let res = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": "llama-3.3-70b-versatile",
            "messages": [
                {
                    "role": "system",
                    "content": "Eres un veterinario colegiado experto en medicina interna y diagnóstico de laboratorio clínico de animales de compañía y de producción. Proporciona interpretaciones precisas, basadas en evidencia, considerando la especie, sexo y edad del paciente y el contexto clínico completo. Al clasificar la urgencia sé conservador: ante la duda, prioriza la seguridad del paciente. Tu análisis es de apoyo al criterio del médico veterinario tratante, no un diagnóstico definitivo."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3,
            "max_tokens": 2000
        }))
        .send()
        .map_err(|e| AppError::Internal(format!("Error conectando a Groq API: {}", e)))?;

    if !res.status().is_success() {
        let status_code = res.status().as_u16();
        let error_text = res.text().unwrap_or_default();
        return Err(AppError::Internal(groq_error_message(
            status_code,
            &error_text,
        )));
    }

    let response_json: serde_json::Value = res
        .json()
        .map_err(|e| AppError::Internal(format!("Respuesta inválida de Groq API: {}", e)))?;

    let interpretation = parse_groq_response(&response_json)?;

    // 8. Cache the result
    state
        .ai_cache
        .set(sample_id, interpretation.clone(), results_hash);

    Ok(interpretation)
}

type PrevResultRow = (
    i32,
    i32,
    i32,
    String,
    Option<String>,
    f64,
    String,
    Option<f64>,
    Option<f64>,
    Option<String>,
);

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
            attachments: Vec::new(),
        })
        .collect())
}

/// Prueba la conexión con la API de Groq usando la clave configurada.
/// Devuelve `Ok` con un mensaje breve si la clave es válida, o un `AppError`
/// descriptivo si falta, es inválida o hay problemas de red.
/// Solo ADMIN (vive en la página de Configuración).
#[tauri::command]
#[specta::specta]
pub fn test_groq_connection(state: State<'_, AppState>) -> Result<String, AppError> {
    require_admin(&state)?;

    // Reutiliza la lectura de configuración: valida que exista clave.
    let mut pooled = state.pool.acquire()?;
    let settings = settings_repo::get(pooled.conn())?;
    let api_key = groq_api_key_or_error(&settings)?;

    // Solicitud mínima (modelo barato, sin contexto clínico) para validar la clave.
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::Internal(format!("Error creando cliente HTTP: {}", e)))?;
    let res = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&groq_test_request_body())
        .send()
        .map_err(|e| AppError::Internal(format!("Error conectando a Groq API: {}", e)))?;

    let status = res.status();
    if !status.is_success() {
        let error_text = res.text().unwrap_or_default();
        return Err(AppError::Internal(groq_error_message(
            status.as_u16(),
            &error_text,
        )));
    }

    let response_json: serde_json::Value = res
        .json()
        .map_err(|e| AppError::Internal(format!("Respuesta inválida de Groq API: {}", e)))?;

    let content = parse_groq_response(&response_json)?;
    Ok(format!("Conexión exitosa con Groq. Respuesta: {}", content))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_patient() -> Patient {
        Patient {
            id: 1,
            code: "PAC-2026-0001".into(),
            owner_id: 1,
            species_id: 1,
            breed_id: None,
            name: "Max".into(),
            sex: "M".into(),
            birth_date: Some("2020-01-01".into()),
            neutered: true,
            color: None,
            microchip: None,
            active: true,
            notes: None,
            preferred_logo_id: None,
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
        // Valor coherente con el estado clínico respecto al rango 37-55:
        // ALTO y BAJO deben quedar realmente fuera del rango para que la
        // columna de desviación del prompt sea matemáticamente correcta.
        let value = match status {
            "ALTO" => 62.0, // por encima de 55
            "BAJO" => 30.0, // por debajo de 37
            _ => 45.0,      // dentro de 37-55
        };
        LabResult {
            id: 1,
            sample_id: 1,
            analyte_id: 1,
            analyte_name: "Hematocrito".to_string(),
            unit: Some("%".to_string()),
            value,
            status: status.to_string(),
            ref_min: Some(37.0),
            ref_max: Some(55.0),
            analyzed_at: Some("2026-08-04 10:30:00".to_string()),
            attachments: Vec::new(),
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
        assert!(prompt.contains("Macho"));
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
            recent_consultations: vec![Consultation {
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
            }],
            vaccines: vec![Vaccine {
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
            }],
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
            previous_results: vec![LabResult {
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
                attachments: Vec::new(),
            }],
        };

        let prompt = build_interpretation_prompt(&patient, &sample, &results, Some(&ctx));

        assert!(prompt.contains("Resultados de laboratorio previos"));
        assert!(prompt.contains("42"));
    }

    #[test]
    fn test_build_prompt_includes_species_note() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("Consideración por especie"));
        assert!(prompt.contains("ehrlichiosis")); // nota canina
    }

    #[test]
    fn test_build_prompt_feline_species_note() {
        let mut patient = test_patient();
        patient.species_name = "Felino".to_string();
        let sample = test_sample();
        let results = vec![];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("hiperglucemia de estrés"));
    }

    #[test]
    fn test_build_prompt_ovine_species_note() {
        let mut patient = test_patient();
        patient.species_name = "Ovino".to_string();
        let sample = test_sample();
        let results = vec![];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("parasitismo gastrointestinal"));
    }

    #[test]
    fn test_build_prompt_unknown_species_note() {
        let mut patient = test_patient();
        patient.species_name = "Conejo".to_string();
        let sample = test_sample();
        let results = vec![];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("No se dispone de particularidades"));
    }

    #[test]
    fn test_build_prompt_includes_deviation_column() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![test_lab_result("ALTO")]; // 45 vs máx 55 → no puede ser ALTO

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("Desviación"));
    }

    #[test]
    fn test_build_prompt_includes_structured_urgency() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![test_lab_result("ALTO")];

        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);

        assert!(prompt.contains("## Nivel de urgencia"));
        assert!(prompt.contains("**Nivel**"));
        assert!(prompt.contains("BAJA | MEDIA | ALTA | CRÍTICA"));
        assert!(prompt.contains("**Criterio**"));
        assert!(prompt.contains("**Acción sugerida**"));
        assert!(prompt.contains("CRÍTICA: desviaciones severas"));
    }

    #[test]
    fn test_deviation_label_alto() {
        let r = LabResult {
            value: 80.0,
            ref_min: Some(37.0),
            ref_max: Some(55.0),
            status: "ALTO".to_string(),
            ..test_lab_result("ALTO")
        };
        assert!(deviation_label(&r).contains("🔺 +45.5%"));
    }

    #[test]
    fn test_deviation_label_bajo() {
        let r = LabResult {
            value: 20.0,
            ref_min: Some(37.0),
            ref_max: Some(55.0),
            status: "BAJO".to_string(),
            ..test_lab_result("BAJO")
        };
        assert!(deviation_label(&r).contains("🔻 -45.9%"));
    }

    #[test]
    fn test_deviation_label_normal() {
        let r = test_lab_result("NORMAL");
        assert_eq!(deviation_label(&r), "Dentro de rango");
    }

    #[test]
    fn test_deviation_label_sin_rango() {
        let r = LabResult {
            ref_min: None,
            ref_max: None,
            status: "SIN_RANGO".to_string(),
            ..test_lab_result("SIN_RANGO")
        };
        assert_eq!(deviation_label(&r), "—");
    }

    #[test]
    fn test_deviation_label_alto_sin_max() {
        let r = LabResult {
            ref_min: Some(37.0),
            ref_max: None,
            status: "ALTO".to_string(),
            ..test_lab_result("ALTO")
        };
        assert_eq!(deviation_label(&r), "🔺 Fuera de rango");
    }

    #[test]
    fn test_deviation_label_bajo_con_min_cero() {
        let r = LabResult {
            value: -5.0,
            ref_min: Some(0.0),
            ref_max: Some(10.0),
            status: "BAJO".to_string(),
            ..test_lab_result("BAJO")
        };
        assert_eq!(deviation_label(&r), "🔻 Fuera de rango");
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
    fn test_groq_api_key_or_error_ok() {
        let settings = crate::models::settings::ClinicSettings {
            groq_api_key: Some("gsk_abc123".into()),
            ..Default::default()
        };
        assert_eq!(groq_api_key_or_error(&settings).unwrap(), "gsk_abc123");
    }

    #[test]
    fn test_groq_api_key_or_error_none() {
        let settings = crate::models::settings::ClinicSettings::default();
        let err = groq_api_key_or_error(&settings).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("no está configurada"));
    }

    #[test]
    fn test_groq_api_key_or_error_empty() {
        let settings = crate::models::settings::ClinicSettings {
            groq_api_key: Some("   ".into()),
            ..Default::default()
        };
        assert!(groq_api_key_or_error(&settings).is_err());
    }

    #[test]
    fn test_groq_error_message_401() {
        let msg = groq_error_message(401, "{\"error\":\"invalid\"}");
        assert!(msg.contains("401"));
        assert!(msg.contains("inválida o revocada"));
        assert!(!msg.contains("invalid"));
    }

    #[test]
    fn test_groq_error_message_other_status() {
        let msg = groq_error_message(500, "internal error");
        assert!(msg.contains("500"));
        assert!(msg.contains("internal error"));
    }

    #[test]
    fn test_groq_error_message_429() {
        let msg = groq_error_message(429, "rate limited");
        assert!(msg.contains("429"));
        assert!(msg.contains("Rate Limit"));
        // No debe exponer el cuerpo raw de la respuesta
        assert!(!msg.contains("rate limited"));
    }

    #[test]
    fn test_groq_error_message_503() {
        let msg = groq_error_message(503, "service unavailable");
        assert!(msg.contains("503"));
        assert!(msg.contains("no disponible"));
    }

    #[test]
    fn test_build_prompt_includes_patient_code() {
        let patient = test_patient();
        let sample = test_sample();
        let results = vec![];
        let prompt = build_interpretation_prompt(&patient, &sample, &results, None);
        assert!(prompt.contains("**Código**"));
        assert!(prompt.contains("PAC-2026-0001"));
    }

    #[test]
    fn test_groq_test_request_body() {
        let body = groq_test_request_body();
        assert_eq!(body["model"], "llama-3.3-70b-versatile");
        assert_eq!(body["max_tokens"], 5);
        assert_eq!(body["temperature"], 0.0);
        assert_eq!(body["messages"][0]["content"], "Responde únicamente: OK");
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
