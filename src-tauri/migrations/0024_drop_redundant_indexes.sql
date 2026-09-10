-- ============================================================================
-- ISALAB · Migración 0024 — Corrección: índices redundantes de la 0023
--
-- El perfilado con EXPLAIN PLAN sobre una base con ~100k resultados
-- (docs/scalability-review-2026-09-10, §5) demostró que Firebird 5
-- AUTO-INDEXA las FOREIGN KEY (cambio de comportamiento respecto a FB≤4):
-- el optimizador usó exclusivamente los índices de sistema RDB$FOREIGN36
-- (LAB_RESULTS.ANALYTE_ID), RDB$FOREIGN28 (SAMPLES.PATIENT_ID) y
-- RDB$FOREIGN35 (LAB_RESULTS.SAMPLE_ID); jamás eligió los homólogos de la
-- 0023, ni con ellos presentes ni al quitarlos (planes y tiempos idénticos).
--
-- Esta migración elimina los dos índices duplicados para no pagar su
-- mantenimiento en escritura. Se conservan:
--   * IX_EVENT_LOG_CREATED_AT — la poda de 30 días recorre el 90 % de la
--     tabla (scan igual de barato que un índice), pero el índice permite
--     planes indexados si el retenedor se acorta o se consulta por fecha.
--   * IX_LAB_RESULTS_ANALYZED_AT — única vía de ordenar por fecha de
--     análisis si el volume lo exige; el optimizador hoy prefiere SORT.
-- ============================================================================

DROP INDEX IX_LAB_RESULTS_ANALYTE;
DROP INDEX IX_SAMPLES_PATIENT;
