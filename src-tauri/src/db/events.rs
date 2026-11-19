//! Puente entre los eventos nativos de Firebird y la UI.
//!
//! Los triggers `AI_SAMPLES` y `AI_LAB_RESULTS` insertan una fila en
//! `EVENT_LOG` y lanzan `POST_EVENT`. El evento nativo no transporta datos, así
//! que cada listener usa el `ID` de `EVENT_LOG` como **marca de agua**: en vez
//! de leer "la última fila" (que con varias PCs escribiendo puede ser de otro
//! cambio) lee todas las filas nuevas de su propio evento y emite un evento de
//! Tauri por fila, en orden.
//!
//! Con la base central el servidor puede reiniciarse o la red caerse, y
//! `listen_event` no reconecta: termina el hilo con error. Por eso cada evento
//! tiene un supervisor que vuelve a suscribirse con backoff exponencial. Si no
//! hay datos fiables (consulta fallida o primera notificación tras conectar) se
//! emite `db-changed`, una invalidación genérica para que la UI refetchee en
//! lugar de aplicar un payload potencialmente incorrecto.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use rsfbclient::prelude::*;
use rsfbclient::{FbError, RemoteEventsManager, SimpleConnection};
use tauri::{AppHandle, Emitter};

use crate::db::config::DbConfig;
use crate::models::sample::{LabResultChangedEvent, SampleChangedEvent};

/// Primer reintento tras perder la conexión con el servidor.
const RECONNECT_INITIAL: Duration = Duration::from_secs(2);
/// Tope del backoff: el servidor puede estar caído varios minutos.
const RECONNECT_MAX: Duration = Duration::from_secs(60);
/// Filas leídas como máximo por notificación (una ráfaga de 30 analitos cabe
/// de sobra; el tope evita bloquear el hilo si la app estuvo horas caída).
const DRAIN_LIMIT: i32 = 500;
/// Valor de la marca de agua cuando aún no se fijó (los `ID` son positivos).
const NO_WATERMARK: i32 = -1;

/// Evento genérico de invalidación, sin payload tipado.
pub const GENERIC_EVENT: &str = "db-changed";

/// Fila de `EVENT_LOG` reemitible: `ID`, `REF_ID`, `PATIENT_ID`, `STATUS`.
type EventRow = (i32, Option<i32>, Option<i32>, Option<String>);

#[derive(Clone, Copy)]
enum EventKind {
    Sample,
    LabResult,
}

/// Resultado de una lectura de `EVENT_LOG`.
struct Drained {
    rows: Vec<EventRow>,
    /// Hay que invalidar todo: primera lectura o lote truncado.
    generic: bool,
}

/// Arranca un supervisor por evento Firebird.
///
/// No falla al arrancar: si el servidor aún no está disponible, cada supervisor
/// reintenta con backoff hasta lograrlo. Los hilos viven mientras viva el
/// proceso.
pub fn start_firebird_listeners(app: AppHandle, cfg: &DbConfig) -> Vec<JoinHandle<()>> {
    [
        ("SAMPLE_CHANGED", EventKind::Sample),
        ("LAB_RESULT_CHANGED", EventKind::LabResult),
    ]
    .into_iter()
    .map(|(event_name, kind)| {
        let app = app.clone();
        let cfg = cfg.clone();
        let event_name = event_name.to_string();
        std::thread::spawn(move || supervise(app, cfg, event_name, kind))
    })
    .collect()
}

/// Bucle de vida de un listener: conectar, suscribirse, esperar a que la
/// conexión muera y reintentar con backoff exponencial (2 s → 60 s).
fn supervise(app: AppHandle, cfg: DbConfig, event_name: String, kind: EventKind) {
    let watermark = Arc::new(AtomicI32::new(NO_WATERMARK));
    let mut backoff = RECONNECT_INITIAL;
    // Solo se invalida la UI si había una suscripción viva: al arrancar con el
    // servidor caído no hay nada que refrescar, y no se bombardea al frontend.
    let mut was_subscribed = false;

    loop {
        match super::connect(&cfg) {
            Ok(conn) => {
                let app_handler = app.clone();
                let key = event_name.clone();
                let wm = Arc::clone(&watermark);

                let subscribed = conn.listen_event(event_name.clone(), move |c| {
                    match drain_new_rows(c, &key, &wm) {
                        Ok(drained) => {
                            let skipped = emit_rows(&app_handler, kind, drained.rows);
                            if drained.generic || skipped {
                                emit_generic(&app_handler);
                            }
                        }
                        Err(e) => {
                            eprintln!("[events] no se pudo leer EVENT_LOG para {key}: {e}");
                            emit_generic(&app_handler);
                        }
                    }
                    // El handler nunca corta el listener: un fallo de lectura se
                    // resuelve con una invalidación genérica, no cerrando el hilo.
                    Ok(true)
                });

                match subscribed {
                    Ok(handle) => {
                        backoff = RECONNECT_INITIAL;
                        was_subscribed = true;
                        match handle.join() {
                            Ok(Ok(())) => eprintln!("[events] el listener {event_name} terminó"),
                            Ok(Err(e)) => eprintln!(
                                "[events] el listener {event_name} perdió la conexión: {e}"
                            ),
                            Err(_) => {
                                eprintln!("[events] el listener {event_name} entró en pánico")
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[events] no se pudo suscribir a {event_name}: {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("[events] sin conexión para el listener {event_name}: {e}");
            }
        }

        // Al reconectar pueden haberse perdido notificaciones: una sola
        // invalidación genérica por caída deja la UI consistente.
        if was_subscribed {
            emit_generic(&app);
            was_subscribed = false;
        }

        std::thread::sleep(backoff);
        backoff = (backoff * 2).min(RECONNECT_MAX);
    }
}

/// Lee las filas nuevas de `EVENT_LOG` para `event_name` y avanza la marca de
/// agua. Es la pieza con lógica propia del listener, probada aparte.
fn drain_new_rows(
    conn: &mut SimpleConnection,
    event_name: &str,
    watermark: &AtomicI32,
) -> Result<Drained, FbError> {
    let last = watermark.load(Ordering::Relaxed);

    // Primera notificación tras conectar: fijar la marca en el máximo actual en
    // vez de reemitir el histórico (la UI ya carga el estado al montar).
    if last == NO_WATERMARK {
        let max: Option<(Option<i32>,)> = conn.query_first("SELECT MAX(ID) FROM EVENT_LOG", ())?;
        let max = max.and_then(|(v,)| v).unwrap_or(0);
        watermark.store(max, Ordering::Relaxed);
        return Ok(Drained {
            rows: Vec::new(),
            generic: true,
        });
    }

    let sql = format!(
        "SELECT FIRST {DRAIN_LIMIT} ID, REF_ID, PATIENT_ID, STATUS
         FROM EVENT_LOG
         WHERE EVENT_NAME = ? AND ID > ?
         ORDER BY ID"
    );
    let rows: Vec<EventRow> = conn.query(&sql, (event_name, last))?;
    let truncated = rows.len() as i32 >= DRAIN_LIMIT;
    if let Some((id, ..)) = rows.last() {
        watermark.store(*id, Ordering::Relaxed);
    }

    Ok(Drained {
        rows,
        generic: truncated,
    })
}

/// Reemite las filas como eventos de Tauri. Devuelve `true` si alguna fila no
/// se pudo correlacionar (sin `REF_ID` o `PATIENT_ID`), para que el llamador
/// caiga en la invalidación genérica.
fn emit_rows(app: &AppHandle, kind: EventKind, rows: Vec<EventRow>) -> bool {
    let mut skipped = false;

    for (_id, ref_id, patient_id, status) in rows {
        let (Some(ref_id), Some(patient_id)) = (ref_id, patient_id) else {
            skipped = true;
            continue;
        };

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

    skipped
}

/// Invalidación genérica: el frontend refresca todas sus queries.
fn emit_generic(app: &AppHandle) {
    let _ = app.emit(GENERIC_EVENT, ());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{cleanup_test_db, setup_test_db};

    fn insert_row(
        conn: &mut SimpleConnection,
        id: i32,
        event_name: &str,
        ref_id: i32,
        patient_id: i32,
    ) {
        conn.execute(
            "INSERT INTO EVENT_LOG (ID, EVENT_NAME, REF_ID, PATIENT_ID, STATUS)
             VALUES (?, ?, ?, ?, 'RECIBIDA')",
            (&id, &event_name, &ref_id, &patient_id),
        )
        .unwrap();
    }

    #[test]
    fn the_first_notification_only_sets_the_watermark() {
        let (mut conn, db_path) = setup_test_db();
        insert_row(&mut conn, 40, "SAMPLE_CHANGED", 7, 3);

        let watermark = AtomicI32::new(NO_WATERMARK);
        let drained = drain_new_rows(&mut conn, "SAMPLE_CHANGED", &watermark).unwrap();

        // No se reemiten filas viejas: solo se pide una invalidación global.
        assert!(drained.rows.is_empty());
        assert!(drained.generic);
        assert_eq!(watermark.load(Ordering::Relaxed), 40);

        drop(conn);
        cleanup_test_db(&db_path);
    }

    #[test]
    fn drains_only_new_rows_of_its_event_in_order() {
        let (mut conn, db_path) = setup_test_db();
        insert_row(&mut conn, 40, "SAMPLE_CHANGED", 7, 3);

        let watermark = AtomicI32::new(NO_WATERMARK);
        drain_new_rows(&mut conn, "SAMPLE_CHANGED", &watermark).unwrap();

        // Llegan dos cambios de muestra (con uno de laboratorio en medio, que
        // este listener debe ignorar) y otro de muestra al final.
        insert_row(&mut conn, 41, "SAMPLE_CHANGED", 8, 3);
        insert_row(&mut conn, 42, "LAB_RESULT_CHANGED", 8, 3);
        insert_row(&mut conn, 43, "SAMPLE_CHANGED", 9, 4);

        let drained = drain_new_rows(&mut conn, "SAMPLE_CHANGED", &watermark).unwrap();
        assert!(!drained.generic, "un lote parcial no invalida todo");
        assert_eq!(drained.rows.len(), 2, "solo las filas de su evento");
        assert_eq!(drained.rows[0].0, 41);
        assert_eq!(drained.rows[1].0, 43);
        assert_eq!(watermark.load(Ordering::Relaxed), 43);

        // Sin novedades no se reemite nada (evento duplicado del servidor).
        let again = drain_new_rows(&mut conn, "SAMPLE_CHANGED", &watermark).unwrap();
        assert!(again.rows.is_empty());
        assert!(!again.generic);

        drop(conn);
        cleanup_test_db(&db_path);
    }
}
