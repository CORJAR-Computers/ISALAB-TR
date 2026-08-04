use serde_json::json;
use tauri::State;

use crate::auth::require_session;
use crate::error::AppError;
use crate::repositories::patients as patients_repo;
use crate::repositories::samples as samples_repo;
use crate::repositories::settings as settings_repo;
use crate::state::AppState;

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

    // 3. Fetch patient info
    let patient = patients_repo::get(pooled.conn(), sample.patient_id)?.ok_or_else(|| {
        AppError::NotFound(format!("Paciente {} no encontrado", sample.patient_id))
    })?;

    // 4. Construct prompt
    let mut prompt = format!(
        "Eres un veterinario experto. Analiza los siguientes resultados de laboratorio de un paciente y proporciona una interpretación clínica breve, posibles diagnósticos diferenciales y recomendaciones.\n\n\
        Paciente:\n- Especie/Raza: {} / {}\n- Edad: {} (nacimiento)\n- Sexo: {}\n\n\
        Muestra: {}\nResultados:\n",
        patient.species_name,
        patient.breed_name.as_deref().unwrap_or("Mestizo"),
        patient.birth_date,
        patient.sex,
        sample.sample_type_name
    );

    for r in sample.results {
        let range = match (r.ref_min, r.ref_max) {
            (Some(min), Some(max)) => format!(" (Rango: {} - {})", min, max),
            _ => "".to_string(),
        };
        let status = if r.status != "NORMAL" && r.status != "SIN_RANGO" {
            format!(" [ALERTA: {}]", r.status)
        } else {
            "".to_string()
        };
        prompt.push_str(&format!(
            "- {}: {} {}{}{}\n",
            r.analyte_name,
            r.value,
            r.unit.as_deref().unwrap_or(""),
            range,
            status
        ));
    }

    prompt.push_str("\nProporciona tu respuesta en español, estructurada con títulos claros en Markdown.");

    // 5. Call Groq API via HTTP blocking request
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": "llama3-8b-8192",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3,
            "max_tokens": 1024
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

    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No se recibió respuesta de la IA.")
        .to_string();

    Ok(content)
}
