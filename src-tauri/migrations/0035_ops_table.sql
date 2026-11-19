-- ============================================================================
-- ISALAB · Migración 0035 — Tabla de estado operativo (Fase 6)
--
-- Tabla OPS para metadatos operativos de la clínica, en particular el estado
-- del backup del servidor Firebird (last_backup_at, last_backup_file). La app
-- no ejecuta gbak (eso lo hace el administrador o un planificado externo), pero
-- lee esta tabla para mostrar el estado del backup en Ajustes.
// ============================================================================

CREATE TABLE OPS (
    KEY_NAME   VARCHAR(100) CHARACTER SET UTF8 NOT NULL PRIMARY KEY,
    VALUE_TEXT VARCHAR(1000) CHARACTER SET UTF8,
    DESCRIPTION VARCHAR(300) CHARACTER SET UTF8,
    UPDATED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
