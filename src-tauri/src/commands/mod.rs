pub mod ai;
pub mod attachments;
pub mod auth;
pub mod catalog;
pub mod clinical_history;
pub mod dashboard;
pub mod db;
pub mod exports;
pub mod invoices;
pub mod patients;
pub mod reports;
pub mod samples;
pub mod settings;
pub mod surgeries;
pub mod users;
pub mod vaccines;

// Nota: `current_user()` fue consolidado en `crate::auth::require_session()`.
// Todos los comandos deben usar `crate::auth::require_session` o
// `crate::auth::require_admin` para verificación de acceso.
