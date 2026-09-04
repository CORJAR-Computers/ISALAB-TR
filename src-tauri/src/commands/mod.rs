pub mod ai;
pub mod analyzer_sources;
pub mod analyzers;
pub mod attachments;
pub mod auth;
pub mod catalog;
pub mod clinical_history;
pub mod dashboard;
pub mod db;
pub mod exports;
pub mod import;
pub mod invoices;
pub mod lab_orders;
pub mod notifications;
pub mod panels;
pub mod patients;
pub mod qc;
pub mod reports;
pub mod samples;
pub mod search;
pub mod settings;
pub mod surgeries;
pub mod users;
pub mod vaccines;

// Nota: `current_user()` fue consolidado en `crate::auth::require_session()`.
// Todos los comandos deben usar `crate::auth::require_session` o
// `crate::auth::require_admin` para verificación de acceso.
