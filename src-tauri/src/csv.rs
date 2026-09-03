//! # Exportación CSV (muestras y resultados)
//!
//! Serializa los datos del laboratorio a CSV compatible con Excel (separador
//! `;`, punto decimal, BOM UTF-8) para informes gerenciales y entregables a
//! clientes. El separador `;` es el estándar de Excel en locales hispanas.

use crate::models::sample_list_item::SampleListItem;

/// Escapa un campo CSV: comillas dobles duplicadas y entrecomillado si contiene
/// separador, comillas o salto de línea.
///
/// Además, neutraliza la **inyección de fórmulas** de hojas de cálculo: si la
/// celda empieza por `=`, `+`, `-`, `@`, tabulación o retorno de carro, se le
/// antepone una comilla simple `'` para que Excel/LibreOffice la trate como
/// texto y no como una fórmula (p. ej. un propietario llamado `=cmd|...`).
pub fn esc(value: &str) -> String {
    const FORMULA_PREFIXES: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

    let needs_quotes = value.contains(';')
        || value.contains('"')
        || value.contains('\n')
        || value.starts_with(FORMULA_PREFIXES);

    // Neutraliza la inyección de fórmulas anteponiendo una comilla simple.
    let neutralized = if value.starts_with(FORMULA_PREFIXES) {
        format!("'{}", value)
    } else {
        value.to_string()
    };

    if !needs_quotes {
        return neutralized;
    }
    format!("\"{}\"", neutralized.replace('"', "\"\""))
}

/// BOM UTF-8 para que Excel detecte el encoding correctamente.
pub fn bom() -> &'static str {
    "\u{FEFF}"
}

/// Cabecera de la exportación de muestras.
const SAMPLES_HEADER: &str = "Código;Paciente;Propietario;Especie;Tipo de muestra;Recibida;Estado;Responsable;Resultados;Anormales;Notas";

/// Serializa la lista de muestras a CSV.
pub fn samples_to_csv(samples: &[SampleListItem]) -> String {
    let mut out = String::new();
    out.push_str(bom());
    out.push_str(SAMPLES_HEADER);
    out.push('\n');
    for s in samples {
        out.push_str(&esc(&s.code));
        out.push(';');
        out.push_str(&esc(&s.patient_name));
        out.push(';');
        out.push_str(&esc(&s.owner_name));
        out.push(';');
        out.push_str(&esc(&s.species_name));
        out.push(';');
        out.push_str(&esc(&s.sample_type_name));
        out.push(';');
        out.push_str(&esc(&s.received_at));
        out.push(';');
        out.push_str(&esc(&s.status));
        out.push(';');
        out.push_str(&esc(s.collected_by.as_deref().unwrap_or("")));
        out.push(';');
        out.push_str(&s.result_count.to_string());
        out.push(';');
        out.push_str(&s.abnormal_count.to_string());
        out.push(';');
        out.push_str(&esc(s.notes.as_deref().unwrap_or("")));
        out.push('\n');
    }
    out
}

/// Fila de resultado de laboratorio para exportación.
#[derive(Debug, Clone)]
pub struct ResultExportRow {
    pub code: String,
    pub patient_name: String,
    pub owner_name: String,
    pub species_name: String,
    pub sample_type_name: String,
    pub received_at: String,
    pub analyte_name: String,
    pub unit: Option<String>,
    pub value: f64,
    pub status: String,
    pub ref_min: Option<f64>,
    pub ref_max: Option<f64>,
    pub analyzed_at: Option<String>,
}

/// Cabecera de la exportación de resultados.
const RESULTS_HEADER: &str = "Código muestra;Paciente;Propietario;Especie;Tipo de muestra;Recibida;Analito;Valor;Unidad;Estado;Ref. mín;Ref. máx;Analizado";

/// Serializa los resultados de laboratorio a CSV. El valor se escribe con
/// punto decimal (formato de planillas).
pub fn results_to_csv(rows: &[ResultExportRow]) -> String {
    let mut out = String::new();
    out.push_str(bom());
    out.push_str(RESULTS_HEADER);
    out.push('\n');
    for r in rows {
        out.push_str(&esc(&r.code));
        out.push(';');
        out.push_str(&esc(&r.patient_name));
        out.push(';');
        out.push_str(&esc(&r.owner_name));
        out.push(';');
        out.push_str(&esc(&r.species_name));
        out.push(';');
        out.push_str(&esc(&r.sample_type_name));
        out.push(';');
        out.push_str(&esc(&r.received_at));
        out.push(';');
        out.push_str(&esc(&r.analyte_name));
        out.push(';');
        out.push_str(&format_value_csv(r.value));
        out.push(';');
        out.push_str(&esc(r.unit.as_deref().unwrap_or("")));
        out.push(';');
        out.push_str(&esc(&r.status));
        out.push(';');
        out.push_str(&r.ref_min.map(format_value_csv).unwrap_or_default());
        out.push(';');
        out.push_str(&r.ref_max.map(format_value_csv).unwrap_or_default());
        out.push(';');
        out.push_str(&esc(r.analyzed_at.as_deref().unwrap_or("")));
        out.push('\n');
    }
    out
}

/// Valor numérico con punto decimal (evita la coma del locale).
fn format_value_csv(v: f64) -> String {
    if v == v.round() {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(code: &str) -> SampleListItem {
        SampleListItem {
            id: 1,
            code: code.into(),
            patient_id: 1,
            patient_code: "P-2026-0001".into(),
            patient_name: "Luna".into(),
            owner_name: "Pérez, Juan".into(),
            species_name: "Canino".into(),
            sample_type_id: 1,
            sample_type_name: "Sangre total (EDTA)".into(),
            received_at: "2026-08-01 10:30:00".into(),
            status: "RECIBIDA".into(),
            collected_by: Some("Dr. Ramos".into()),
            notes: None,
            result_count: 0,
            abnormal_count: 0,
            critical_count: 0,
            quality_index: None,
            quality_severity: None,
            rejection_reason: None,
        }
    }

    #[test]
    fn test_esc_plain() {
        assert_eq!(esc("hola"), "hola");
    }

    #[test]
    fn test_esc_with_separator_quotes() {
        // El separador es `;` (Excel hispano); la coma no requiere entrecomillado.
        assert_eq!(esc("Pérez;Juan"), "\"Pérez;Juan\"");
        assert_eq!(esc("di \"hola\""), "\"di \"\"hola\"\"\"");
        assert_eq!(esc("Pérez, Juan"), "Pérez, Juan");
    }

    #[test]
    fn test_samples_csv_header_and_row() {
        let csv = samples_to_csv(&[sample("M-2026-0001")]);
        assert!(csv.starts_with('\u{FEFF}'));
        assert!(csv.contains("Código;Paciente"));
        assert!(csv.contains("M-2026-0001;Luna"));
        // El propietario con coma NO va entrecomillado (solo `;` y comillas).
        assert!(csv.contains("Pérez, Juan"));
    }

    #[test]
    fn test_results_csv_value_format() {
        let row = ResultExportRow {
            code: "M-2026-0001".into(),
            patient_name: "Luna".into(),
            owner_name: "Juan".into(),
            species_name: "Canino".into(),
            sample_type_name: "Sangre".into(),
            received_at: "2026-08-01 10:30:00".into(),
            analyte_name: "Hematocrito".into(),
            unit: Some("%".into()),
            value: 42.5,
            status: "NORMAL".into(),
            ref_min: Some(37.0),
            ref_max: Some(55.0),
            analyzed_at: Some("2026-08-01 11:00:00".into()),
        };
        let csv = results_to_csv(&[row]);
        assert!(csv.contains("42.50"));
        assert!(csv.contains("37")); // 37.0 se redondea a entero
        assert!(csv.contains("55"));
        assert!(csv.contains("Hematocrito"));
        assert!(csv.contains("Analito"));
    }
}
