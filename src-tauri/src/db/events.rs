use std::path::Path;
use std::thread::JoinHandle;

use rsfbclient::prelude::*;
use rsfbclient::{FbError, RemoteEventsManager};
use tauri::{AppHandle, Emitter};

use crate::error::AppError;
use crate::models::sample::{LabResultChangedEvent, SampleChangedEvent};

#[derive(Clone, Copy)]
enum EventKind {
    Sample,
    LabResult,
}

/// Suscribe un listener dedicado por evento Firebird.
///
/// Los eventos nativos de Firebird no transportan payload, por lo que el
/// handler lee la última fila de EVENT_LOG (escrita por los triggers
/// AI_SAMPLES / AI_LAB_RESULTS) y re-emite un evento de Tauri con el payload
/// real (id de muestra, paciente y estado) para la UI.
pub fn start_firebird_listeners(
    app: AppHandle,
    db_path: &Path,
    fbclient: &Path,
) -> Result<Vec<JoinHandle<Result<(), FbError>>>, AppError> {
    let mut handles = Vec::new();

    for (event_name, kind) in [
        ("SAMPLE_CHANGED", EventKind::Sample),
        ("LAB_RESULT_CHANGED", EventKind::LabResult),
    ] {
        let conn = super::new_connection(db_path, fbclient)?;
        let app = app.clone();
        let name = event_name.to_string();
        // El nombre vive también dentro del closure (move) y en el error.
        let event_key = name.clone();

        let handle = conn
            .listen_event(name.clone(), move |c| {
                let payload: Option<(i32, i32, Option<String>)> = c
                    .query_first(
                        "SELECT FIRST 1 REF_ID, PATIENT_ID, STATUS
                         FROM EVENT_LOG
                         WHERE EVENT_NAME = ?
                         ORDER BY ID DESC",
                        (&event_key,),
                    )
                    .ok()
                    .flatten();

                if let Some((ref_id, patient_id, status)) = payload {
                    match kind {
                        EventKind::Sample => {
                            let _ = app.emit(
                                "sample-changed",
                                SampleChangedEvent {
                                    sample_id: ref_id,
                                    patient_id,
                                    status: status.unwrap_or_default(),
                                },
                            );
                        }
                        EventKind::LabResult => {
                            let _ = app.emit(
                                "lab-result-changed",
                                LabResultChangedEvent {
                                    sample_id: ref_id,
                                    patient_id,
                                },
                            );
                        }
                    }
                }
                Ok(true)
            })
            .map_err(|e| {
                AppError::Db(format!(
                    "No se pudo suscribir al evento Firebird '{name}': {e}"
                ))
            })?;

        handles.push(handle);
    }

    Ok(handles)
}
