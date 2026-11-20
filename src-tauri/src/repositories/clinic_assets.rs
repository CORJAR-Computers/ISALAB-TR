use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::settings::ClinicAsset;

pub(crate) type ClinicAssetRow = (
    i32,            // id
    String,         // kind
    Option<String>, // file_name
    Option<String>, // mime_type
    i32,            // size_bytes (columna INTEGER)
    String,         // updated_at
);

pub(crate) fn map_clinic_asset(r: ClinicAssetRow) -> ClinicAsset {
    ClinicAsset {
        kind: r.1,
        file_name: r.2,
        mime_type: r.3,
        size_bytes: r.4,
        updated_at: r.5,
    }
}

/// Metadatos de todos los activos de la clínica (logo principal y certificado).
pub fn list(conn: &mut SimpleConnection) -> Result<Vec<ClinicAsset>, AppError> {
    let rows: Vec<ClinicAssetRow> = conn
        .query(
            "SELECT ID, KIND, FILE_NAME, MIME_TYPE, SIZE_BYTES,
                    CAST(UPDATED_AT AS VARCHAR(30))
             FROM CLINIC_ASSETS ORDER BY KIND",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_clinic_asset).collect())
}

pub fn get(conn: &mut SimpleConnection, kind: &str) -> Result<Option<ClinicAsset>, AppError> {
    let row: Option<ClinicAssetRow> = conn
        .query_first(
            "SELECT ID, KIND, FILE_NAME, MIME_TYPE, SIZE_BYTES,
                    CAST(UPDATED_AT AS VARCHAR(30))
             FROM CLINIC_ASSETS WHERE KIND = ?",
            (&kind,),
        )
        .map_err(AppError::from)?;
    Ok(row.map(map_clinic_asset))
}

/// Descarga el contenido (BLOB) de un activo por su KIND.
pub fn content(conn: &mut SimpleConnection, kind: &str) -> Option<Vec<u8>> {
    let row: Option<(Option<Vec<u8>>,)> = conn
        .query_first("SELECT CONTENT FROM CLINIC_ASSETS WHERE KIND = ?", (&kind,))
        .ok()
        .flatten();
    row.and_then(|(o,)| o)
}

/// Upsert de un activo: inserta si no existe la KIND, actualiza si ya existe.
pub fn upsert(
    conn: &mut SimpleConnection,
    kind: &str,
    file_name: Option<&str>,
    mime_type: Option<&str>,
    content: &[u8],
) -> Result<ClinicAsset, AppError> {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Firebird no tiene INSERT ... ON DUPLICATE KEY; usamos UPDATE OR INSERT.
    conn.execute(
        "UPDATE OR INSERT INTO CLINIC_ASSETS
             (KIND, FILE_NAME, MIME_TYPE, CONTENT, SIZE_BYTES, CREATED_AT, UPDATED_AT)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         MATCHING (KIND)",
        (
            kind,
            file_name,
            mime_type,
            content,
            content.len() as i32,
            now.clone(),
            now,
        ),
    )
    .map_err(AppError::from)?;

    get(conn, kind)?.ok_or_else(|| AppError::Internal("Activo no encontrado tras upsert".into()))
}

pub fn delete(conn: &mut SimpleConnection, kind: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM CLINIC_ASSETS WHERE KIND = ?", (&kind,))
        .map_err(AppError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    #[test]
    fn test_upsert_list_get_delete() {
        let (mut conn, db_path) = test_helpers::setup_test_db();

        assert!(get(&mut conn, "LOGO").unwrap().is_none());

        let asset = upsert(
            &mut conn,
            "LOGO",
            Some("logo.png"),
            Some("image/png"),
            b"logo-bytes",
        )
        .unwrap();
        assert_eq!(asset.kind, "LOGO");
        assert_eq!(asset.file_name.as_deref(), Some("logo.png"));
        assert_eq!(asset.size_bytes, 10);

        let all = list(&mut conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].kind, "LOGO");

        let content_bytes = content(&mut conn, "LOGO").unwrap();
        assert_eq!(content_bytes, b"logo-bytes");

        // Upsert sobre la misma KIND reemplaza (una sola fila).
        upsert(&mut conn, "LOGO", Some("logo2.png"), None, b"logo2-bytes").unwrap();
        let all = list(&mut conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(content(&mut conn, "LOGO").unwrap(), b"logo2-bytes");

        delete(&mut conn, "LOGO").unwrap();
        assert!(get(&mut conn, "LOGO").unwrap().is_none());

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_kind_unique_and_cert_p12() {
        let (mut conn, db_path) = test_helpers::setup_test_db();

        upsert(&mut conn, "LOGO", Some("logo.png"), None, b"a").unwrap();
        upsert(&mut conn, "CERT_P12", Some("cert.p12"), None, b"cert-bytes").unwrap();

        assert_eq!(list(&mut conn).unwrap().len(), 2);
        assert_eq!(content(&mut conn, "CERT_P12").unwrap(), b"cert-bytes");

        test_helpers::cleanup_test_db(&db_path);
    }
}
