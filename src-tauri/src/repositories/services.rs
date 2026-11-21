use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::service::{
    SaveServiceCategoryInput, SaveServiceInput, Service, ServiceCategory,
};
use crate::repositories::next_id;

// ============================ CATEGORÍAS ====================================

type CategoryRow = (i32, String, String, i32, bool);

fn map_category(r: CategoryRow) -> ServiceCategory {
    ServiceCategory {
        id: r.0,
        code: r.1,
        name: r.2,
        sort_order: r.3,
        is_active: r.4,
    }
}

/// Lista las categorías del catálogo (por defecto solo activas; los
/// administradores pueden incluir las inactivas con `include_inactive`).
pub fn list_categories(
    conn: &mut SimpleConnection,
    include_inactive: bool,
) -> Result<Vec<ServiceCategory>, AppError> {
    let rows: Vec<CategoryRow> = conn
        .query(
            "SELECT ID, CODE, NAME, SORT_ORDER, IS_ACTIVE
             FROM SERVICE_CATEGORIES
             WHERE (? = TRUE OR IS_ACTIVE = TRUE)
             ORDER BY SORT_ORDER, NAME",
            (&include_inactive,),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_category).collect())
}

/// Crea o actualiza una categoría del catálogo.
pub fn save_category(
    conn: &mut SimpleConnection,
    input: &SaveServiceCategoryInput,
) -> Result<ServiceCategory, AppError> {
    let code = input.code.trim().to_ascii_uppercase();
    let name = input.name.trim();
    if code.is_empty() {
        return Err(AppError::Validation(
            "El código de la categoría es obligatorio".into(),
        ));
    }
    if name.is_empty() {
        return Err(AppError::Validation(
            "El nombre de la categoría es obligatorio".into(),
        ));
    }

    if let Some(id) = input.id {
        // Editar: code único salvo para la propia fila.
        let dup: Option<(i32,)> = conn
            .query_first(
                "SELECT ID FROM SERVICE_CATEGORIES WHERE CODE = ? AND ID <> ?",
                (&code, &id),
            )
            .map_err(AppError::from)?;
        if dup.is_some() {
            return Err(AppError::Validation(format!(
                "Ya existe una categoría con el código {code}"
            )));
        }
        conn.execute(
            "UPDATE SERVICE_CATEGORIES
                SET CODE = ?, NAME = ?, SORT_ORDER = ?
              WHERE ID = ?",
            (&code, &name, &input.sort_order, &id),
        )
        .map_err(AppError::from)?;
    } else {
        let dup: Option<(i32,)> = conn
            .query_first("SELECT ID FROM SERVICE_CATEGORIES WHERE CODE = ?", (&code,))
            .map_err(AppError::from)?;
        if dup.is_some() {
            return Err(AppError::Validation(format!(
                "Ya existe una categoría con el código {code}"
            )));
        }
        let nid = next_id(conn, "GEN_SERVICE_CATEGORIES_ID")?;
        conn.execute(
            "INSERT INTO SERVICE_CATEGORIES (ID, CODE, NAME, SORT_ORDER)
             VALUES (?, ?, ?, ?)",
            (&nid, &code, &name, &input.sort_order),
        )
        .map_err(AppError::from)?;
    }

    // Tras crear (id = None) el ID nuevo se recupera por código (único).
    let id = match input.id {
        Some(id) => id,
        None => {
            let (id,): (i32,) = conn
                .query_first("SELECT ID FROM SERVICE_CATEGORIES WHERE CODE = ?", (&code,))
                .map_err(AppError::from)?
                .ok_or_else(|| {
                    AppError::Internal("Categoría guardada pero ID no recuperado".into())
                })?;
            id
        }
    };

    get_category_by_id(conn, id)?
        .ok_or_else(|| AppError::Internal("Categoría guardada pero no recuperada".into()))
}

/// Recupera una categoría por su ID.
pub fn get_category_by_id(
    conn: &mut SimpleConnection,
    id: i32,
) -> Result<Option<ServiceCategory>, AppError> {
    let row: Option<CategoryRow> = conn
        .query_first(
            "SELECT ID, CODE, NAME, SORT_ORDER, IS_ACTIVE
             FROM SERVICE_CATEGORIES WHERE ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
    Ok(row.map(map_category))
}

/// Da de baja una categoría. Si tiene servicios, es una baja **lógica**
/// (se desactiva junto con sus servicios); si no, se elimina físicamente.
pub fn delete_category(conn: &mut SimpleConnection, id: i32) -> Result<(), AppError> {
    let count: Option<(i64,)> = conn
        .query_first(
            "SELECT COUNT(*) FROM SERVICES WHERE CATEGORY_ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
    if count.map(|(c,)| c).unwrap_or(0) > 0 {
        conn.execute(
            "UPDATE SERVICE_CATEGORIES SET IS_ACTIVE = FALSE WHERE ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
        conn.execute(
            "UPDATE SERVICES SET IS_ACTIVE = FALSE, UPDATED_AT = CURRENT_TIMESTAMP
              WHERE CATEGORY_ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
    } else {
        conn.execute("DELETE FROM SERVICE_CATEGORIES WHERE ID = ?", (&id,))
            .map_err(AppError::from)?;
    }
    Ok(())
}

// ============================== SERVICIOS ===================================

type ServiceRow = (
    i32,
    String,
    String,
    i32,
    String,
    Option<String>,
    i32,
    f64,
    Option<f64>,
    bool,
    bool,
    Option<i32>,
    i32,
    bool,
);

fn map_service(r: ServiceRow) -> Service {
    Service {
        id: r.0,
        code: r.1,
        name: r.2,
        category_id: r.3,
        category_name: r.4,
        description: r.5,
        duration_min: r.6,
        price: r.7,
        tax_rate: r.8,
        requires_vet: r.9,
        requires_groomer: r.10,
        species_id: r.11,
        sort_order: r.12,
        is_active: r.13,
    }
}

const SERVICE_SELECT: &str = "
    SELECT s.ID, s.CODE, s.NAME, s.CATEGORY_ID, c.NAME, s.DESCRIPTION,
           s.DURATION_MIN,
           CAST(s.PRICE AS DOUBLE PRECISION),
           CAST(s.TAX_RATE AS DOUBLE PRECISION),
           s.REQUIRES_VET, s.REQUIRES_GROOMER, s.SPECIES_ID, s.SORT_ORDER,
           s.IS_ACTIVE
    FROM SERVICES s
    JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID";

/// Lista servicios con filtros opcionales: categoría, texto (código o nombre)
/// e inclusión de inactivos (solo administración).
pub fn list_services(
    conn: &mut SimpleConnection,
    category_id: Option<i32>,
    search: Option<&str>,
    include_inactive: bool,
) -> Result<Vec<Service>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());

    let sql = format!(
        "{SERVICE_SELECT}
         WHERE (? IS NULL OR s.CATEGORY_ID = ?)
           AND (? IS NULL OR UPPER(s.CODE) LIKE UPPER(?) OR UPPER(s.NAME) LIKE UPPER(?))
           AND (? = TRUE OR s.IS_ACTIVE = TRUE)
         ORDER BY c.SORT_ORDER, s.SORT_ORDER, s.NAME"
    );
    let rows: Vec<ServiceRow> = conn
        .query(
            &sql,
            (
                &category_id,
                &category_id,
                &like,
                &like,
                &like,
                &include_inactive,
            ),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_service).collect())
}

/// Obtiene un servicio por ID.
pub fn get_service(conn: &mut SimpleConnection, id: i32) -> Result<Option<Service>, AppError> {
    let row: Option<ServiceRow> = conn
        .query_first(&format!("{SERVICE_SELECT} WHERE s.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_service))
}

/// Valida los datos de un servicio (común a crear/editar).
fn validate_service(
    conn: &mut SimpleConnection,
    input: &SaveServiceInput,
    exclude_id: Option<i32>,
) -> Result<(), AppError> {
    if input.code.trim().is_empty() {
        return Err(AppError::Validation(
            "El código del servicio es obligatorio".into(),
        ));
    }
    if input.name.trim().is_empty() {
        return Err(AppError::Validation(
            "El nombre del servicio es obligatorio".into(),
        ));
    }
    if input.duration_min <= 0 {
        return Err(AppError::Validation(
            "La duración debe ser mayor que cero minutos".into(),
        ));
    }
    if input.price < 0.0 {
        return Err(AppError::Validation(
            "El precio no puede ser negativo".into(),
        ));
    }
    if let Some(rate) = input.tax_rate {
        if !(0.0..=100.0).contains(&rate) {
            return Err(AppError::Validation(
                "El IVA debe estar entre 0 y 100 %".into(),
            ));
        }
    }

    let code = input.code.trim().to_ascii_uppercase();
    let dup_sql = match exclude_id {
        Some(_) => "SELECT ID FROM SERVICES WHERE CODE = ? AND ID <> ?".to_string(),
        None => "SELECT ID FROM SERVICES WHERE CODE = ?".to_string(),
    };
    let dup: Option<(i32,)> = match exclude_id {
        Some(id) => conn
            .query_first(&dup_sql, (&code, &id))
            .map_err(AppError::from)?,
        None => conn
            .query_first(&dup_sql, (&code,))
            .map_err(AppError::from)?,
    };
    if dup.is_some() {
        return Err(AppError::Validation(format!(
            "Ya existe un servicio con el código {code}"
        )));
    }

    let category: Option<(i32,)> = conn
        .query_first(
            "SELECT ID FROM SERVICE_CATEGORIES WHERE ID = ? AND IS_ACTIVE = TRUE",
            (&input.category_id,),
        )
        .map_err(AppError::from)?;
    if category.is_none() {
        return Err(AppError::Validation(
            "La categoría del servicio no existe o está inactiva".into(),
        ));
    }

    if let Some(species_id) = input.species_id {
        let species: Option<(i32,)> = conn
            .query_first("SELECT ID FROM SPECIES WHERE ID = ?", (&species_id,))
            .map_err(AppError::from)?;
        if species.is_none() {
            return Err(AppError::Validation("La especie indicada no existe".into()));
        }
    }

    Ok(())
}

/// Crea o actualiza un servicio del catálogo.
pub fn save_service(
    conn: &mut SimpleConnection,
    input: &SaveServiceInput,
) -> Result<Service, AppError> {
    validate_service(conn, input, input.id)?;

    let code = input.code.trim().to_ascii_uppercase();
    let name = input.name.trim();
    let id = match input.id {
        Some(id) => {
            conn.execute(
                "UPDATE SERVICES
                    SET CODE = ?, NAME = ?, CATEGORY_ID = ?, DESCRIPTION = ?,
                        DURATION_MIN = ?, PRICE = ?, TAX_RATE = ?,
                        REQUIRES_VET = ?, REQUIRES_GROOMER = ?, SPECIES_ID = ?,
                        SORT_ORDER = ?, UPDATED_AT = CURRENT_TIMESTAMP
                  WHERE ID = ?",
                (
                    &code,
                    &name,
                    &input.category_id,
                    &input.description,
                    &input.duration_min,
                    &input.price,
                    &input.tax_rate,
                    &input.requires_vet,
                    &input.requires_groomer,
                    &input.species_id,
                    &input.sort_order,
                    &id,
                ),
            )
            .map_err(AppError::from)?;
            id
        }
        None => {
            let nid = next_id(conn, "GEN_SERVICES_ID")?;
            conn.execute(
                "INSERT INTO SERVICES
                    (ID, CODE, NAME, CATEGORY_ID, DESCRIPTION, DURATION_MIN,
                     PRICE, TAX_RATE, REQUIRES_VET, REQUIRES_GROOMER,
                     SPECIES_ID, SORT_ORDER)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    &nid,
                    &code,
                    &name,
                    &input.category_id,
                    &input.description,
                    &input.duration_min,
                    &input.price,
                    &input.tax_rate,
                    &input.requires_vet,
                    &input.requires_groomer,
                    &input.species_id,
                    &input.sort_order,
                ),
            )
            .map_err(AppError::from)?;
            nid
        }
    };

    get_service(conn, id)?
        .ok_or_else(|| AppError::Internal("Servicio guardado pero no recuperado".into()))
}

/// Activa o desactiva un servicio (baja lógica: nunca se borra, la
/// facturación histórica referencia su ID).
pub fn set_service_active(
    conn: &mut SimpleConnection,
    id: i32,
    active: bool,
) -> Result<Service, AppError> {
    let exists: Option<(i32,)> = conn
        .query_first("SELECT ID FROM SERVICES WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    if exists.is_none() {
        return Err(AppError::NotFound(format!("Servicio {id} no encontrado")));
    }
    conn.execute(
        "UPDATE SERVICES SET IS_ACTIVE = ?, UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ?",
        (&active, &id),
    )
    .map_err(AppError::from)?;
    get_service(conn, id)?
        .ok_or_else(|| AppError::Internal("Servicio actualizado pero no recuperado".into()))
}

// ============================== TESTS =======================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    fn setup() -> (crate::test_helpers::TestDb, std::path::PathBuf) {
        test_helpers::setup_test_db()
    }

    fn category_input(code: &str, name: &str) -> SaveServiceCategoryInput {
        SaveServiceCategoryInput {
            id: None,
            code: code.into(),
            name: name.into(),
            sort_order: 1,
        }
    }

    fn service_input(category_id: i32, code: &str, name: &str) -> SaveServiceInput {
        SaveServiceInput {
            id: None,
            code: code.into(),
            name: name.into(),
            category_id,
            description: None,
            duration_min: 30,
            price: 50000.0,
            tax_rate: None,
            requires_vet: false,
            requires_groomer: false,
            species_id: None,
            sort_order: 1,
        }
    }

    #[test]
    fn test_seed_categories_and_services_exist() {
        let (mut conn, db_path) = setup();
        // La migración 0025 siembra 6 categorías y 17 servicios.
        let cats = list_categories(&mut conn, true).unwrap();
        assert_eq!(cats.len(), 6);
        assert_eq!(cats[0].code, "CONSULTA");

        let services = list_services(&mut conn, None, None, true).unwrap();
        assert_eq!(services.len(), 17);
        // Precio de referencia del baño.
        let bano = services.iter().find(|s| s.code == "PEL-BANO").unwrap();
        assert!((bano.price - 40000.0).abs() < 0.01);
        assert!(bano.requires_groomer);
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_and_update_service() {
        let (mut conn, db_path) = setup();
        let mut input = service_input(1, "CONS-TEST", "Consulta de prueba");
        let created = save_service(&mut conn, &input).unwrap();
        assert_eq!(created.code, "CONS-TEST");
        assert_eq!(created.category_name, "Consultas");
        assert!(created.is_active);

        input.id = Some(created.id);
        input.price = 75000.0;
        input.duration_min = 45;
        let updated = save_service(&mut conn, &input).unwrap();
        assert!((updated.price - 75000.0).abs() < 0.01);
        assert_eq!(updated.duration_min, 45);
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_duplicate_service_code_rejected() {
        let (mut conn, db_path) = setup();
        // 'CONS-GEN' viene de la semilla.
        let input = service_input(1, "CONS-GEN", "Duplicado");
        let err = save_service(&mut conn, &input).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_invalid_service_data_rejected() {
        let (mut conn, db_path) = setup();
        let mut input = service_input(1, "CONS-X", "Sin duración");
        input.duration_min = 0;
        assert!(save_service(&mut conn, &input).is_err());

        input.duration_min = 30;
        input.price = -1.0;
        assert!(save_service(&mut conn, &input).is_err());

        input.price = 0.0;
        input.category_id = 999;
        assert!(save_service(&mut conn, &input).is_err());
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_service_active_toggles() {
        let (mut conn, db_path) = setup();
        let svc = list_services(&mut conn, None, None, true)
            .unwrap()
            .remove(0);
        let off = set_service_active(&mut conn, svc.id, false).unwrap();
        assert!(!off.is_active);

        // Un servicio inactivo no aparece en el listado operativo.
        let active = list_services(&mut conn, None, None, false).unwrap();
        assert!(active.iter().all(|s| s.id != svc.id));

        let on = set_service_active(&mut conn, svc.id, true).unwrap();
        assert!(on.is_active);
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_category_soft_delete_with_services() {
        let (mut conn, db_path) = setup();
        // La categoría 5 (PELUQUERIA) tiene servicios: baja lógica.
        delete_category(&mut conn, 5).unwrap();
        let cats = list_categories(&mut conn, true).unwrap();
        let pel = cats.iter().find(|c| c.id == 5).unwrap();
        assert!(!pel.is_active);

        // Una categoría sin servicios se elimina físicamente.
        let cat = save_category(&mut conn, &category_input("TEST", "Temporal")).unwrap();
        delete_category(&mut conn, cat.id).unwrap();
        let cats = list_categories(&mut conn, true).unwrap();
        assert!(cats.iter().all(|c| c.id != cat.id));
        test_helpers::cleanup_test_db(&db_path);
    }
}
