pub mod attachments;
pub mod auth;
pub mod clinical_history;
pub mod dashboard;
pub mod invoices;
pub mod logos;
pub mod patient;
pub mod samples;
pub mod search;
pub mod settings;
pub mod surgeries;
pub mod users;
pub mod vaccines;

use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;

/// Obtiene el siguiente ID de un GENERATOR (con CAST a INTEGER).
pub fn next_id(conn: &mut SimpleConnection, generator: &str) -> Result<i32, AppError> {
    let sql = format!("SELECT CAST(GEN_ID({generator}, 1) AS INTEGER) FROM rdb$database");
    conn.query_first(&sql, ())
        .map_err(AppError::from)?
        .map(|(v,): (i32,)| v)
        .ok_or_else(|| AppError::Internal("Generador sin valor".into()))
}
