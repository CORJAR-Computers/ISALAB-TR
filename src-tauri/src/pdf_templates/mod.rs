//! # Reportes médicos PDF (generación server-side en Rust)
//!
//! Implementado con `printpdf` 0.12 (API de operaciones: `Op`): compone el
//! informe Carta (encabezado institucional con datos centrados y logos, ficha del paciente,
//! tabla de resultados con valores fuera de rango resaltados y bloque de firma) y lo
//! guarda en `app_data/reports/<codigo>.pdf`.

pub mod builder;
pub mod clinical;
pub mod financial;
pub mod header;
pub mod layout;
pub mod signing;
pub mod surgical;
pub mod vaccines;

pub use builder::*;
pub use clinical::*;
pub use financial::*;
pub use header::*;
pub use layout::*;
pub use signing::*;
pub use surgical::*;
pub use vaccines::*;
