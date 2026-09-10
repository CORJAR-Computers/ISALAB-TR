//! Mantenimiento periódico de la base de datos (corre en cada arranque).
//!
//! La poda inicial de EVENT_LOG vive en la migración 0023; esta rutina es la
//! recurrente: cada arranque borra la telemetría de eventos con más de 30
//! días. Los triggers AI_SAMPLES / AI_LAB_RESULTS insertan una fila por cada
//! cambio de muestra o resultado, así que sin poda la tabla crece sin límite
//! e hincha los backups locales (create_local_backup) y la lectura del
//! último evento (db/events.rs).
//!
//! Es best-effort: un fallo no impide arrancar la app (el bootstrap lo
//! ignora y se reintenta en el próximo arranque). El índice
//! IX_EVENT_LOG_CREATED_AT (migración 0023) hace que el DELETE filtre por
//! índice en vez de scan. Los días se interpolan en el SQL porque es una
//! constante de compilación: Firebird no resuelve bien el tipo de un
//! parámetro `?` dentro de aritmética de fechas (`CURRENT_TIMESTAMP - ?`).

use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;

/// Días de retención de la telemetría de eventos.
const EVENT_LOG_RETENTION_DAYS: i32 = 30;

/// SQL de poda compartido por el conteo y el borrado.
fn prune_sql() -> String {
    format!("CREATED_AT < CURRENT_TIMESTAMP - {EVENT_LOG_RETENTION_DAYS}")
}

/// Borra de EVENT_LOG las filas con más de `EVENT_LOG_RETENTION_DAYS` y
/// devuelve cuántas eliminó (informativo para pruebas/diagnóstico).
pub fn prune_event_log_conn(conn: &mut SimpleConnection) -> Result<u64, AppError> {
    let where_clause = prune_sql();

    let deleted: Option<(i32,)> = conn
        .query_first(
            &format!("SELECT COUNT(*) FROM EVENT_LOG WHERE {where_clause}"),
            (),
        )
        .map_err(AppError::from)?;

    conn.execute(&format!("DELETE FROM EVENT_LOG WHERE {where_clause}"), ())
        .map_err(AppError::from)?;

    Ok(deleted.map(|(n,)| n.max(0) as u64).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[test]
    fn test_prune_event_log_keeps_recent_and_removes_old() {
        let (mut conn, db_path) = setup_test_db();

        // El seed de demo puede haber dejado filas recientes (los INSERT de
        // muestras disparan AI_SAMPLES). Se toma como línea base.
        let (baseline,): (i32,) = conn
            .query_first("SELECT COUNT(*) FROM EVENT_LOG", ())
            .unwrap()
            .unwrap();

        // Fila reciente (hoy, vía DEFAULT CURRENT_TIMESTAMP) y una vieja
        // (hace 40 días). EVENT_LOG no tiene FK: no hacen falta fixtures.
        conn.execute(
            "INSERT INTO EVENT_LOG (EVENT_NAME, REF_ID, PATIENT_ID, STATUS)
             VALUES ('SAMPLE_CHANGED', 1, 1, 'RECIBIDA')",
            (),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO EVENT_LOG (EVENT_NAME, REF_ID, PATIENT_ID, STATUS, CREATED_AT)
             VALUES ('SAMPLE_CHANGED', 2, 1, 'RECIBIDA',
                     CURRENT_TIMESTAMP - 40)",
            (),
        )
        .unwrap();

        // La poda solo elimina lo anterior al corte (la fila de hace 40
        // días; las filas del seed y la reciente quedan).
        let removed = prune_event_log_conn(&mut conn).unwrap();
        assert_eq!(removed, 1);

        let (remaining,): (i32,) = conn
            .query_first("SELECT COUNT(*) FROM EVENT_LOG", ())
            .unwrap()
            .unwrap();
        assert_eq!(remaining, baseline + 1);
        cleanup_test_db(&db_path);
    }
}
