-- ============================================================================
-- ISALAB · Migración 0023 — Mantenimiento: poda de EVENT_LOG e índices
--
-- Dos tareas del review de escalabilidad (docs/scalability-review-2026-09-10):
--
--   1. EVENT_LOG crece sin límite: los triggers AI_SAMPLES / AI_LAB_RESULTS
--      insertan una fila por cada insert/update de muestras y resultados y
--      nadie la borra. En 5 años de uso intensivo son millones de filas que
--      hinchan los backups locales (create_local_backup) y ralentizan la
--      lectura del último evento (events.rs hace `SELECT FIRST 1 ... ORDER BY
--      ID DESC`). Aquí se hace la PODA INICIAL (la recurrente vive en
--      db/maintenance.rs y corre en cada arranque).
--
--   2. Faltaban índices para las rutas de consulta más frecuentes. En
--      Firebird las FOREIGN KEY no crean índice automáticamente, así que
--      SAMPLES.PATIENT_ID (historial clínico) y LAB_RESULTS.ANALYTE_ID
--      (tendencias, delta check, historial previo del prompt IA) hacían
--      scan natural.
-- ============================================================================

-- ------------------------------ PODA INICIAL -------------------------------
-- Conserva los últimos 30 días. `CURRENT_TIMESTAMP - 30` resta 30 días en
-- Firebird (arithmética de fechas con enteros = días). Corre una sola vez:
-- el mantenimiento periódico posterior lo hace el bootstrap en Rust.
DELETE FROM EVENT_LOG WHERE CREATED_AT < CURRENT_TIMESTAMP - 30;

-- ------------------------------- ÍNDICES -----------------------------------
-- Tendencias por paciente/analito, delta check y "resultados previos" del
-- prompt de IA: filtran LAB_RESULTS por analito (junto con PATIENT_ID vía la
-- muestra) y ordenan por fecha de análisis.
CREATE INDEX IX_LAB_RESULTS_ANALYTE ON LAB_RESULTS (ANALYTE_ID);
CREATE INDEX IX_LAB_RESULTS_ANALYZED_AT ON LAB_RESULTS (ANALYZED_AT);

-- Historial clínico por paciente (list_samples de clinical_history.rs) y
-- delta check (busca el resultado previo del paciente). Las FK de Firebird
-- no crean índice: este es el índice real de la ruta más consultada.
CREATE INDEX IX_SAMPLES_PATIENT ON SAMPLES (PATIENT_ID);

-- La poda recurrente y cualquier diagnóstico futuro filtran por fecha.
CREATE INDEX IX_EVENT_LOG_CREATED_AT ON EVENT_LOG (CREATED_AT);
