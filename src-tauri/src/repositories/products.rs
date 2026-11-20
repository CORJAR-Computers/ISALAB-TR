use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::db;
use crate::error::AppError;
use crate::models::product::{
    CreateStockMovementInput, Product, ProductStockItem, SaveProductInput, StockMovement,
};
use crate::repositories::next_id;

/// Categorías de producto admitidas.
pub const PRODUCT_CATEGORIES: [&str; 4] = ["ALIMENTO", "MEDICAMENTO", "ACCESORIO", "INSUMO"];
/// Tipos de movimiento de inventario.
pub const STOCK_MOVEMENT_KINDS: [&str; 5] =
    ["ENTRADA", "SALIDA", "CONSUMO", "DEVOLUCION", "AJUSTE"];

type ProductRow = (
    i32,            // id
    String,         // code
    String,         // name
    String,         // category
    String,         // unit
    f64,            // cost_price
    f64,            // sale_price
    Option<f64>,    // tax_rate
    i32,            // min_stock
    bool,           // active
    Option<String>, // notes
);

type ProductStockRow = (
    i32,            // id
    String,         // code
    String,         // name
    String,         // category
    String,         // unit
    f64,            // cost_price
    f64,            // sale_price
    Option<f64>,    // tax_rate
    i32,            // min_stock
    bool,           // active
    Option<String>, // notes
    i32,            // stock
);

type StockMovementRow = (
    i32,            // id
    i32,            // product_id
    String,         // product_name
    String,         // product_code
    i32,            // quantity
    String,         // kind
    Option<f64>,    // unit_cost
    Option<String>, // reason
    Option<i32>,    // stay_id
    Option<i32>,    // visit_id
    Option<String>, // author_name
    String,         // created_at
);

fn map_product(r: ProductRow) -> Product {
    Product {
        id: r.0,
        code: r.1,
        name: r.2,
        category: r.3,
        unit: r.4,
        cost_price: r.5,
        sale_price: r.6,
        tax_rate: r.7,
        min_stock: r.8,
        active: r.9,
        notes: r.10,
    }
}

fn map_movement(r: StockMovementRow) -> StockMovement {
    StockMovement {
        id: r.0,
        product_id: r.1,
        product_name: r.2,
        product_code: r.3,
        quantity: r.4,
        kind: r.5,
        unit_cost: r.6,
        reason: r.7,
        stay_id: r.8,
        visit_id: r.9,
        author_name: r.10,
        created_at: r.11,
    }
}

// =============================== PRODUCTOS ==================================

/// Lista el catálogo de productos con su stock actual (suma de movimientos).
pub fn list_products(
    conn: &mut SimpleConnection,
    search: Option<&str>,
    category: Option<&str>,
    include_inactive: bool,
) -> Result<Vec<ProductStockItem>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());
    let category_opt = category.map(|c| c.to_string());

    let sql = format!(
        "SELECT FIRST 200 p.ID, p.CODE, p.NAME, p.CATEGORY, p.UNIT,
                CAST(p.COST_PRICE AS DOUBLE PRECISION),
                CAST(p.SALE_PRICE AS DOUBLE PRECISION),
                CAST(p.TAX_RATE AS DOUBLE PRECISION),
                p.MIN_STOCK, p.ACTIVE, p.NOTES,
                COALESCE((SELECT SUM(m.QUANTITY)
                            FROM STOCK_MOVEMENTS m
                           WHERE m.PRODUCT_ID = p.ID), 0)
         FROM PRODUCTS p
         WHERE (? IS NULL OR UPPER(p.NAME) LIKE UPPER(?) OR UPPER(p.CODE) LIKE UPPER(?))
           AND (? IS NULL OR p.CATEGORY = ?)
           AND (? = TRUE OR p.ACTIVE = TRUE)
         ORDER BY p.ACTIVE DESC, p.CATEGORY, p.NAME"
    );
    let like_ref = like.clone();

    let rows: Vec<ProductStockRow> = conn
        .query(
            &sql,
            (
                &like_ref,
                &like_ref,
                &like_ref,
                &category_opt,
                &category_opt,
                &include_inactive,
            ),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let stock = r.11;
            let product = map_product((r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10));
            let low_stock = product.active && product.min_stock > 0 && stock <= product.min_stock;
            ProductStockItem {
                stock_value: stock as f64 * product.cost_price,
                low_stock,
                stock,
                product,
            }
        })
        .collect())
}

/// Producto del catálogo por id.
pub fn get_product(conn: &mut SimpleConnection, id: i32) -> Result<Option<Product>, AppError> {
    let row: Option<ProductRow> = conn
        .query_first(
            "SELECT ID, CODE, NAME, CATEGORY, UNIT,
                    CAST(COST_PRICE AS DOUBLE PRECISION),
                    CAST(SALE_PRICE AS DOUBLE PRECISION),
                    CAST(TAX_RATE AS DOUBLE PRECISION),
                    MIN_STOCK, ACTIVE, NOTES
             FROM PRODUCTS WHERE ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
    Ok(row.map(map_product))
}

/// Crea o actualiza un producto del catálogo.
pub fn save_product(
    conn: &mut SimpleConnection,
    input: &SaveProductInput,
) -> Result<Product, AppError> {
    let code = input.code.trim().to_uppercase();
    if code.is_empty() {
        return Err(AppError::Validation(
            "El código del producto es requerido".into(),
        ));
    }
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::Validation(
            "El nombre del producto es requerido".into(),
        ));
    }
    let category = input.category.trim().to_uppercase();
    if !PRODUCT_CATEGORIES.contains(&category.as_str()) {
        return Err(AppError::Validation(format!(
            "Categoría inválida. Usa una de: {}",
            PRODUCT_CATEGORIES.join(", ")
        )));
    }
    let unit = input
        .unit
        .as_deref()
        .map(|u| u.trim().to_uppercase())
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| "UNIDAD".to_string());
    let cost_price = input.cost_price.unwrap_or(0.0);
    let sale_price = input.sale_price.unwrap_or(0.0);
    if cost_price < 0.0 || sale_price < 0.0 {
        return Err(AppError::Validation(
            "Los precios no pueden ser negativos".into(),
        ));
    }
    let min_stock = input.min_stock.unwrap_or(0).max(0);
    let active = input.active.unwrap_or(true);

    let id = match input.id {
        Some(id) => {
            conn.execute(
                "UPDATE PRODUCTS SET CODE = ?, NAME = ?, CATEGORY = ?, UNIT = ?,
                    COST_PRICE = ?, SALE_PRICE = ?, TAX_RATE = ?, MIN_STOCK = ?,
                    ACTIVE = ?, NOTES = ?, UPDATED_AT = CURRENT_TIMESTAMP
                 WHERE ID = ?",
                (
                    &code,
                    &name,
                    &category,
                    &unit,
                    &cost_price,
                    &sale_price,
                    &input.tax_rate,
                    &min_stock,
                    &active,
                    &input.notes,
                    &id,
                ),
            )
            .map_err(AppError::from)?;
            id
        }
        None => {
            let id = next_id(conn, "GEN_PRODUCTS_ID")?;
            conn.execute(
                "INSERT INTO PRODUCTS (ID, CODE, NAME, CATEGORY, UNIT, COST_PRICE,
                                      SALE_PRICE, TAX_RATE, MIN_STOCK, ACTIVE, NOTES)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    &id,
                    &code,
                    &name,
                    &category,
                    &unit,
                    &cost_price,
                    &sale_price,
                    &input.tax_rate,
                    &min_stock,
                    &active,
                    &input.notes,
                ),
            )
            .map_err(AppError::from)?;
            id
        }
    };

    get_product(conn, id)?
        .ok_or_else(|| AppError::Internal("Producto guardado pero no recuperado".into()))
}

/// Activa o desactiva un producto del catálogo.
pub fn set_product_active(
    conn: &mut SimpleConnection,
    id: i32,
    active: bool,
) -> Result<Product, AppError> {
    conn.execute(
        "UPDATE PRODUCTS SET ACTIVE = ?, UPDATED_AT = CURRENT_TIMESTAMP WHERE ID = ?",
        (&active, &id),
    )
    .map_err(AppError::from)?;
    get_product(conn, id)?.ok_or_else(|| AppError::NotFound(format!("Producto {id} no encontrado")))
}

// ============================== MOVIMIENTOS ==================================

/// Registra un movimiento de inventario.
///
/// El signo se deriva del tipo: ENTRADA y DEVOLUCION suman, SALIDA y CONSUMO
/// restan y AJUSTE usa el signo de la cantidad enviada.
pub fn add_stock_movement(
    conn: &mut SimpleConnection,
    input: &CreateStockMovementInput,
    user_id: Option<i32>,
) -> Result<StockMovement, AppError> {
    let kind = input.kind.trim().to_uppercase();
    if !STOCK_MOVEMENT_KINDS.contains(&kind.as_str()) {
        return Err(AppError::Validation(format!(
            "Tipo de movimiento inválido. Usa uno de: {}",
            STOCK_MOVEMENT_KINDS.join(", ")
        )));
    }
    if input.quantity == 0 {
        return Err(AppError::Validation(
            "La cantidad del movimiento debe ser distinta de cero".into(),
        ));
    }

    // El producto debe existir.
    let product: Option<(String, f64)> = conn
        .query_first(
            "SELECT NAME, CAST(COST_PRICE AS DOUBLE PRECISION) FROM PRODUCTS WHERE ID = ?",
            (&input.product_id,),
        )
        .map_err(AppError::from)?;
    let Some((product_name, cost_price)) = product else {
        return Err(AppError::NotFound(format!(
            "Producto {} no encontrado",
            input.product_id
        )));
    };

    // Signo según el tipo de movimiento.
    let signed = match kind.as_str() {
        "ENTRADA" | "DEVOLUCION" => input.quantity.abs(),
        "SALIDA" | "CONSUMO" => -input.quantity.abs(),
        _ => input.quantity,
    };

    // Validación de stock + inserción en una transacción: dos PCs consumiendo
    // el mismo producto a la vez podrían pasar ambas la validación con el
    // stock leído y dejarlo en negativo.
    db::tx::in_transaction(conn, |conn| {
        // No se permite dejar el stock en negativo.
        if signed < 0 {
            let stock: Option<(i32,)> = conn
                .query_first(
                    "SELECT COALESCE(SUM(QUANTITY), 0) FROM STOCK_MOVEMENTS WHERE PRODUCT_ID = ?",
                    (&input.product_id,),
                )
                .map_err(AppError::from)?;
            let current = stock.map(|(v,)| v).unwrap_or(0);
            if current + signed < 0 {
                return Err(AppError::Validation(format!(
                    "Stock insuficiente de «{product_name}»: disponible {current}"
                )));
            }
        }

        let unit_cost = input.unit_cost.unwrap_or(cost_price);
        let id = next_id(conn, "GEN_STOCK_MOVEMENTS_ID")?;
        conn.execute(
            "INSERT INTO STOCK_MOVEMENTS (ID, PRODUCT_ID, QUANTITY, KIND, UNIT_COST, REASON,
                                          STAY_ID, VISIT_ID, CREATED_BY)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                &id,
                &input.product_id,
                &signed,
                &kind,
                &unit_cost,
                &input.reason,
                &input.stay_id,
                &input.visit_id,
                &user_id,
            ),
        )
        .map_err(AppError::from)?;

        let row: Option<StockMovementRow> = conn
            .query_first(
                "SELECT m.ID, m.PRODUCT_ID, p.NAME, p.CODE, m.QUANTITY, m.KIND,
                        CAST(m.UNIT_COST AS DOUBLE PRECISION), m.REASON,
                        m.STAY_ID, m.VISIT_ID, u.FULL_NAME,
                        LEFT(CAST(m.CREATED_AT AS VARCHAR(60)), 19)
                 FROM STOCK_MOVEMENTS m
                 JOIN PRODUCTS p ON p.ID = m.PRODUCT_ID
                 LEFT JOIN USERS u ON u.ID = m.CREATED_BY
                 WHERE m.ID = ?",
                (&id,),
            )
            .map_err(AppError::from)?;
        row.map(map_movement)
            .ok_or_else(|| AppError::Internal("Movimiento creado pero no recuperado".into()))
    })
}

/// Histórico de movimientos de un producto (más reciente primero).
pub fn list_stock_movements(
    conn: &mut SimpleConnection,
    product_id: Option<i32>,
    limit: i32,
) -> Result<Vec<StockMovement>, AppError> {
    let limit = limit.clamp(1, 500);
    let sql = format!(
        "SELECT FIRST {limit} m.ID, m.PRODUCT_ID, p.NAME, p.CODE, m.QUANTITY, m.KIND,
                CAST(m.UNIT_COST AS DOUBLE PRECISION), m.REASON,
                m.STAY_ID, m.VISIT_ID, u.FULL_NAME,
                LEFT(CAST(m.CREATED_AT AS VARCHAR(60)), 19)
         FROM STOCK_MOVEMENTS m
         JOIN PRODUCTS p ON p.ID = m.PRODUCT_ID
         LEFT JOIN USERS u ON u.ID = m.CREATED_BY
         WHERE (? IS NULL OR m.PRODUCT_ID = ?)
         ORDER BY m.CREATED_AT DESC, m.ID DESC"
    );
    let rows: Vec<StockMovementRow> = conn
        .query(&sql, (&product_id, &product_id))
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_movement).collect())
}

/// Stock actual de un producto (suma de movimientos).
pub fn get_stock(conn: &mut SimpleConnection, product_id: i32) -> Result<i32, AppError> {
    let stock: Option<(i32,)> = conn
        .query_first(
            "SELECT COALESCE(SUM(QUANTITY), 0) FROM STOCK_MOVEMENTS WHERE PRODUCT_ID = ?",
            (&product_id,),
        )
        .map_err(AppError::from)?;
    Ok(stock.map(|(v,)| v).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use std::path::PathBuf;

    fn setup() -> (SimpleConnection, PathBuf) {
        setup_test_db()
    }

    fn save_dog_food(conn: &mut SimpleConnection) -> Product {
        save_product(
            conn,
            &SaveProductInput {
                id: None,
                code: "ALIM-001".to_string(),
                name: "Alimento adulto 3 kg".to_string(),
                category: "ALIMENTO".to_string(),
                unit: Some("BOLSA".to_string()),
                cost_price: Some(28000.0),
                sale_price: Some(45000.0),
                tax_rate: Some(19.0),
                min_stock: Some(5),
                active: None,
                notes: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn test_save_product_and_list() {
        let (mut conn, db_path) = setup();
        let product = save_dog_food(&mut conn);

        assert_eq!(product.code, "ALIM-001");
        assert_eq!(product.category, "ALIMENTO");
        assert_eq!(product.unit, "BOLSA");
        assert!((product.cost_price - 28000.0).abs() < 0.01);
        assert!(product.active);

        // Sin movimientos, el stock es 0 y está por debajo del mínimo.
        let items = list_products(&mut conn, None, None, false).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].stock, 0);
        assert!(items[0].low_stock);

        // Búsqueda por nombre y por categoría.
        let found = list_products(&mut conn, Some("adulto"), None, false).unwrap();
        assert_eq!(found.len(), 1);
        let by_category = list_products(&mut conn, None, Some("ALIMENTO"), false).unwrap();
        assert_eq!(by_category.len(), 1);
        let other = list_products(&mut conn, None, Some("MEDICAMENTO"), false).unwrap();
        assert!(other.is_empty());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_save_product_validation() {
        let (mut conn, db_path) = setup();

        let err = save_product(
            &mut conn,
            &SaveProductInput {
                id: None,
                code: "".to_string(),
                name: "Producto".to_string(),
                category: "ALIMENTO".to_string(),
                unit: None,
                cost_price: None,
                sale_price: None,
                tax_rate: None,
                min_stock: None,
                active: None,
                notes: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let err = save_product(
            &mut conn,
            &SaveProductInput {
                id: None,
                code: "X-1".to_string(),
                name: "Producto".to_string(),
                category: "ROBOT".to_string(),
                unit: None,
                cost_price: None,
                sale_price: None,
                tax_rate: None,
                min_stock: None,
                active: None,
                notes: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_stock_movements_update_stock() {
        let (mut conn, db_path) = setup();
        let product = save_dog_food(&mut conn);

        // Entrada de 10 bolsas.
        let movement = add_stock_movement(
            &mut conn,
            &CreateStockMovementInput {
                product_id: product.id,
                quantity: 10,
                kind: "ENTRADA".to_string(),
                unit_cost: Some(28000.0),
                reason: Some("Compra".to_string()),
                stay_id: None,
                visit_id: None,
            },
            None,
        )
        .unwrap();
        assert_eq!(movement.quantity, 10);
        assert_eq!(get_stock(&mut conn, product.id).unwrap(), 10);

        // Salida de 3 bolsas (venta).
        add_stock_movement(
            &mut conn,
            &CreateStockMovementInput {
                product_id: product.id,
                quantity: 3,
                kind: "SALIDA".to_string(),
                unit_cost: None,
                reason: Some("Venta mostrador".to_string()),
                stay_id: None,
                visit_id: None,
            },
            None,
        )
        .unwrap();
        assert_eq!(get_stock(&mut conn, product.id).unwrap(), 7);

        // Consumo interno de 2 bolsas.
        add_stock_movement(
            &mut conn,
            &CreateStockMovementInput {
                product_id: product.id,
                quantity: 2,
                kind: "CONSUMO".to_string(),
                unit_cost: None,
                reason: None,
                stay_id: None,
                visit_id: None,
            },
            None,
        )
        .unwrap();
        assert_eq!(get_stock(&mut conn, product.id).unwrap(), 5);

        // El stock en el mínimo de reposición ya se marca como bajo: es el
        // punto en el que hay que reponer.
        let items = list_products(&mut conn, None, None, false).unwrap();
        assert_eq!(items[0].stock, 5);
        assert!(items[0].low_stock);
        assert!((items[0].stock_value - 5.0 * 28000.0).abs() < 0.01);

        // Por encima del mínimo, la alerta desaparece.
        add_stock_movement(
            &mut conn,
            &CreateStockMovementInput {
                product_id: product.id,
                quantity: 1,
                kind: "ENTRADA".to_string(),
                unit_cost: None,
                reason: None,
                stay_id: None,
                visit_id: None,
            },
            None,
        )
        .unwrap();
        let items = list_products(&mut conn, None, None, false).unwrap();
        assert_eq!(items[0].stock, 6);
        assert!(!items[0].low_stock);

        // El histórico conserva el orden.
        let movements = list_stock_movements(&mut conn, Some(product.id), 10).unwrap();
        assert_eq!(movements.len(), 4);
        assert_eq!(movements[0].kind, "ENTRADA");
        assert_eq!(movements[0].quantity, 1);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_stock_insufficient_and_invalid_movement() {
        let (mut conn, db_path) = setup();
        let product = save_dog_food(&mut conn);

        // Sin stock no se puede vender.
        let err = add_stock_movement(
            &mut conn,
            &CreateStockMovementInput {
                product_id: product.id,
                quantity: 1,
                kind: "SALIDA".to_string(),
                unit_cost: None,
                reason: None,
                stay_id: None,
                visit_id: None,
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Cantidad 0 no se admite.
        let err = add_stock_movement(
            &mut conn,
            &CreateStockMovementInput {
                product_id: product.id,
                quantity: 0,
                kind: "ENTRADA".to_string(),
                unit_cost: None,
                reason: None,
                stay_id: None,
                visit_id: None,
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Tipo de movimiento inválido.
        let err = add_stock_movement(
            &mut conn,
            &CreateStockMovementInput {
                product_id: product.id,
                quantity: 1,
                kind: "REGALO".to_string(),
                unit_cost: None,
                reason: None,
                stay_id: None,
                visit_id: None,
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Producto inexistente.
        let err = add_stock_movement(
            &mut conn,
            &CreateStockMovementInput {
                product_id: 9999,
                quantity: 1,
                kind: "ENTRADA".to_string(),
                unit_cost: None,
                reason: None,
                stay_id: None,
                visit_id: None,
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_product_active() {
        let (mut conn, db_path) = setup();
        let product = save_dog_food(&mut conn);

        let inactive = set_product_active(&mut conn, product.id, false).unwrap();
        assert!(!inactive.active);
        assert!(list_products(&mut conn, None, None, false)
            .unwrap()
            .is_empty());
        assert_eq!(list_products(&mut conn, None, None, true).unwrap().len(), 1);

        cleanup_test_db(&db_path);
    }
}
