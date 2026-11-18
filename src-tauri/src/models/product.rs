use serde::{Deserialize, Serialize};
use specta::Type;

/// Producto del catálogo de comercio (alimentos, medicamentos, accesorios).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: i32,
    /// Código interno (SKU) único.
    pub code: String,
    pub name: String,
    /// ALIMENTO | MEDICAMENTO | ACCESORIO | INSUMO
    pub category: String,
    /// UNIDAD | KG | LT | PAQUETE | CAJA | BOLSA
    pub unit: String,
    pub cost_price: f64,
    pub sale_price: f64,
    /// IVA del producto; NULL = usa el IVA global de la clínica.
    pub tax_rate: Option<f64>,
    /// Stock mínimo para la alerta de reposición.
    pub min_stock: i32,
    pub active: bool,
    pub notes: Option<String>,
}

/// Producto con su stock actual (suma de los movimientos).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProductStockItem {
    pub product: Product,
    /// Stock actual (entradas − salidas).
    pub stock: i32,
    /// true si el stock está por debajo del mínimo configurado.
    pub low_stock: bool,
    /// Valor del inventario a costo (stock × costo unitario).
    pub stock_value: f64,
}

/// Movimiento de inventario (entrada o salida).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StockMovement {
    pub id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub product_code: String,
    /// Positivo = entrada; negativo = salida.
    pub quantity: i32,
    /// ENTRADA | SALIDA | CONSUMO | DEVOLUCION | AJUSTE
    pub kind: String,
    pub unit_cost: Option<f64>,
    pub reason: Option<String>,
    pub stay_id: Option<i32>,
    pub visit_id: Option<i32>,
    pub author_name: Option<String>,
    /// YYYY-MM-DD HH:MM:SS
    pub created_at: String,
}

/// Alta o edición de un producto del catálogo.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveProductInput {
    /// NULL = crear; con ID = editar.
    pub id: Option<i32>,
    pub code: String,
    pub name: String,
    /// ALIMENTO | MEDICAMENTO | ACCESORIO | INSUMO
    pub category: String,
    /// UNIDAD | KG | LT | PAQUETE | CAJA | BOLSA
    pub unit: Option<String>,
    pub cost_price: Option<f64>,
    pub sale_price: Option<f64>,
    pub tax_rate: Option<f64>,
    pub min_stock: Option<i32>,
    pub active: Option<bool>,
    pub notes: Option<String>,
}

/// Movimiento de inventario (entrada, salida, consumo, devolución o ajuste).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateStockMovementInput {
    pub product_id: i32,
    /// Magnitud positiva siempre: el tipo define el signo.
    pub quantity: i32,
    /// ENTRADA | SALIDA | CONSUMO | DEVOLUCION | AJUSTE
    pub kind: String,
    pub unit_cost: Option<f64>,
    pub reason: Option<String>,
    /// Estancia que consumió el producto (opcional).
    pub stay_id: Option<i32>,
    /// Visita que consumió el producto (opcional).
    pub visit_id: Option<i32>,
}
