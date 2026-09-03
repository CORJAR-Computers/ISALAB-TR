use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::csv_parse::{detect_delimiter, parse_csv, parse_number};
use crate::error::AppError;
use crate::models::import::{AnalyzerImportMapping, ImportPreview, ImportSkip, ImportSummary};
use crate::models::sample::RegisterResultInput;
use crate::repositories::clinical_history as history_repo;

/// Normaliza un nombre para comparar encabezados (sin espacios, mayúsculas).
fn norm(s: &str) -> String {
    s.trim()
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Detecta si una columna parece contener el código de muestra.
fn looks_like_sample_code(header: &str) -> bool {
    let h = norm(header);
    h.contains("codigo") || h.contains("code") || h.contains("muestra") || h.contains("sample")
}

/// Lee el archivo y lo parsea con detección de delimitador.
fn read_rows(path: &str) -> Result<(char, Vec<Vec<String>>), AppError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AppError::Validation(format!("No se pudo leer el archivo CSV: {e}")))?;
    // Quita el BOM UTF-8 si está presente.
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content);
    let delimiter = detect_delimiter(content);
    let rows = parse_csv(content, delimiter);
    if rows.is_empty() || rows[0].is_empty() {
        return Err(AppError::Validation(
            "El archivo CSV está vacío o no tiene encabezados".into(),
        ));
    }
    Ok((delimiter, rows))
}

/// Vista previa de la importación: encabezados, primeras filas y sugerencias
/// de mapeo (columna → analito por coincidencia de nombre).
pub fn preview(conn: &mut SimpleConnection, path: &str) -> Result<ImportPreview, AppError> {
    let (delimiter, rows) = read_rows(path)?;
    let headers = rows[0].clone();
    let data_rows = &rows[1..];
    let file_name = std::path::Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());

    // Analitos registrados para el emparejamiento por nombre.
    let analytes: Vec<(i32, String)> = conn
        .query("SELECT ID, NAME FROM ANALYTES ORDER BY NAME", ())
        .map_err(AppError::from)?;

    let mut suggested_analytes: Vec<Option<i32>> = Vec::with_capacity(headers.len());
    let mut suggested_sample_code_column: Option<i32> = None;
    for (i, header) in headers.iter().enumerate() {
        let hn = norm(header);
        // Coincidencia exacta primero; luego por contención del analito.
        let hit = analytes
            .iter()
            .find(|(_, name)| norm(name) == hn)
            .or_else(|| {
                if hn.len() >= 3 {
                    analytes.iter().find(|(_, name)| {
                        let an = norm(name);
                        an.len() >= 3 && (an.contains(&hn) || hn.contains(&an))
                    })
                } else {
                    None
                }
            });
        if hit.is_some() {
            suggested_analytes.push(hit.map(|(id, _)| *id));
        } else if looks_like_sample_code(header) && suggested_sample_code_column.is_none() {
            suggested_sample_code_column = Some(i as i32);
            suggested_analytes.push(None);
        } else {
            suggested_analytes.push(None);
        }
    }

    Ok(ImportPreview {
        file_name,
        delimiter: delimiter.to_string(),
        headers,
        sample_rows: data_rows.iter().take(5).cloned().collect(),
        suggested_sample_code_column,
        suggested_analytes,
        total_rows: data_rows.len() as i32,
    })
}

/// Ejecuta la importación con el mapeo confirmado. Por cada fila localiza la
/// muestra por código, valida su estado y carga los resultados mapeados.
pub fn import(
    conn: &mut SimpleConnection,
    path: &str,
    mapping: &AnalyzerImportMapping,
) -> Result<ImportSummary, AppError> {
    let (_, rows) = read_rows(path)?;
    let headers = &rows[0];
    let data_rows = &rows[1..];

    // Validación del mapeo contra el número real de columnas.
    for col in std::iter::once(mapping.sample_code_column)
        .chain(mapping.columns.iter().map(|c| c.column_index))
    {
        if col < 0 || col as usize >= headers.len() {
            return Err(AppError::Validation(format!(
                "La columna {col} no existe en el archivo (tiene {} columnas)",
                headers.len()
            )));
        }
    }

    let mut skipped: Vec<ImportSkip> = Vec::new();
    let mut samples_updated = 0;
    let mut results_imported = 0;
    let mut seen_samples: std::collections::HashSet<i32> = std::collections::HashSet::new();

    for (idx, row) in data_rows.iter().enumerate() {
        let row_no = (idx + 1) as i32;
        let code = row
            .get(mapping.sample_code_column as usize)
            .map(|c| c.trim().to_string())
            .unwrap_or_default();
        if code.is_empty() {
            skipped.push(ImportSkip {
                row: row_no,
                reason: "sin código de muestra".into(),
            });
            continue;
        }

        let sample: Option<(i32, String)> = conn
            .query_first("SELECT ID, STATUS FROM SAMPLES WHERE CODE = ?", (&code,))
            .map_err(AppError::from)?;
        let Some((sample_id, status)) = sample else {
            skipped.push(ImportSkip {
                row: row_no,
                reason: format!("muestra {code} no encontrada"),
            });
            continue;
        };
        if !matches!(status.as_str(), "RECIBIDA" | "EN_PROCESO") {
            skipped.push(ImportSkip {
                row: row_no,
                reason: format!("muestra {code} en estado {status}"),
            });
            continue;
        }

        let mut row_imported = 0;
        for col in &mapping.columns {
            let value_str = row
                .get(col.column_index as usize)
                .map(|c| c.trim().to_string())
                .unwrap_or_default();
            if value_str.is_empty() {
                continue;
            }
            let Some(value) = parse_number(&value_str) else {
                skipped.push(ImportSkip {
                    row: row_no,
                    reason: format!(
                        "valor \"{value_str}\" no numérico en columna {}",
                        headers[col.column_index as usize]
                    ),
                });
                continue;
            };
            history_repo::register_lab_result(
                conn,
                &RegisterResultInput {
                    sample_id,
                    analyte_id: col.analyte_id,
                    value,
                },
            )?;
            row_imported += 1;
            results_imported += 1;
        }

        if row_imported > 0 && seen_samples.insert(sample_id) {
            samples_updated += 1;
        }
    }

    Ok(ImportSummary {
        samples_updated,
        results_imported,
        skipped,
    })
}
