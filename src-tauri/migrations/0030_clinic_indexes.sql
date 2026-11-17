-- ============================================================================
-- ISALAB · Migración 0030 — Índices de clínica (fase 5)
--
-- Índices para el panel de control y los listados de recepción.
--   - APPOINTMENTS (SCHEDULED_AT) ya queda cubierto por
--     IX_APPOINTMENTS_SCHEDULED_PROF (SCHEDULED_AT, PROFESSIONAL_ID) de 0028
--     (misma columna líder), igual que GROOMING_SESSIONS (NEXT_SUGGESTED_AT)
--     desde 0029: no se duplican.
-- ============================================================================

CREATE INDEX IX_VISITS_ARRIVED ON VISITS (ARRIVED_AT);
CREATE INDEX IX_VISIT_SERVICES_STATUS ON VISIT_SERVICES (STATUS);
