-- ============================================================================
-- ISALAB · Migración 0031 — Claves de configuración nuevas (fase 5)
--
-- Tres claves adicionales en CLINIC_SETTINGS. Todas quedan con valor NULL:
-- son opcionales y la app usa sus valores de respaldo hasta que la clínica
-- las configure (el repo de settings ya trata claves ausentes con defaults).
--
--   clinic.tagline              Texto de marca en login, sidebar y About.
--   grooming.default_duration   Duración por defecto de la sesión de estética
--                               (minutos) cuando el servicio no la define.
--   reception.ticket_prefix     Prefijo del ticket de turno de recepción.
-- ============================================================================

UPDATE OR INSERT INTO CLINIC_SETTINGS (KEY_NAME, VALUE_TEXT, DESCRIPTION)
VALUES ('clinic.tagline', NULL, 'Texto de marca (login, sidebar y Acerca de)')
MATCHING (KEY_NAME);

UPDATE OR INSERT INTO CLINIC_SETTINGS (KEY_NAME, VALUE_TEXT, DESCRIPTION)
VALUES ('grooming.default_duration', NULL, 'Duración por defecto de sesión de estética (minutos)')
MATCHING (KEY_NAME);

UPDATE OR INSERT INTO CLINIC_SETTINGS (KEY_NAME, VALUE_TEXT, DESCRIPTION)
VALUES ('reception.ticket_prefix', NULL, 'Prefijo del ticket de turno de recepción')
MATCHING (KEY_NAME);
