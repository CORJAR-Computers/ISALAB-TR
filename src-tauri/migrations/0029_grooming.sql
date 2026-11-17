-- ============================================================================
-- ISALAB · Migración 0029 — Peluquería (GROOMING_SESSIONS, VISIT_ATTACHMENTS)
--
-- Una sesión de peluquería es la ficha de una LÍNEA de servicio de categoría
-- PELUQUERIA dentro de una visita: una sesión por línea (UNIQUE), campos de
-- estilo/baño/pelaje/comportamiento y hora de inicio/fin. Al finalizar
-- (FINISHED_AT) la línea de la visita pasa a COMPLETADA y queda sugerido el
-- próximo turno (NEXT_SUGGESTED_AT) para el recordatorio del propietario.
--
-- VISIT_ATTACHMENTS guarda las fotos antes/después de la sesión (y documentos
-- de la visita) reutilizando el patrón de RESULT_ATTACHMENTS (0009).
-- ============================================================================

CREATE TABLE GROOMING_SESSIONS (
    ID               D_PK NOT NULL PRIMARY KEY,
    -- Una sesión por línea de servicio de peluquería.
    VISIT_SERVICE_ID D_PK NOT NULL UNIQUE
                     REFERENCES VISIT_SERVICES (ID) ON DELETE CASCADE,
    -- Peluquero asignado a la sesión (puede diferir del profesional de la línea).
    GROOMER_ID       D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    -- Corte de raza, higiénico, cachorro…
    STYLE            D_NAME,
    -- Medicado, antipulgas…
    BATH_TYPE        VARCHAR(40) CHARACTER SET UTF8,
    COAT_CONDITION   D_NOTES,
    MATTING_LEVEL    SMALLINT DEFAULT 0 NOT NULL CHECK (MATTING_LEVEL BETWEEN 0 AND 3),
    WEIGHT_KG        D_VALUE,
    -- Tranquilo, nervioso, agresivo…
    BEHAVIOR         VARCHAR(40) CHARACTER SET UTF8,
    PRODUCTS_USED    D_NOTES,
    STARTED_AT       TIMESTAMP,
    FINISHED_AT      TIMESTAMP,
    -- Próximo turno sugerido (fecha, sin hora) para el recordatorio.
    NEXT_SUGGESTED_AT D_DATE,
    NOTES            D_NOTES,
    CREATED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_GROOMING_SESSIONS_ID;

CREATE TABLE VISIT_ATTACHMENTS (
    ID         D_PK NOT NULL PRIMARY KEY,
    VISIT_ID   D_PK NOT NULL REFERENCES VISITS (ID) ON DELETE CASCADE,
    FILE_NAME  VARCHAR(255) CHARACTER SET UTF8 NOT NULL,
    FILE_PATH  VARCHAR(1000) CHARACTER SET UTF8 NOT NULL,
    MIME_TYPE  VARCHAR(100) CHARACTER SET UTF8,
    -- BEFORE | AFTER | DOCUMENT
    KIND       VARCHAR(20) CHARACTER SET UTF8,
    CREATED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_VISIT_ATTACHMENTS_ID;

CREATE INDEX IX_GROOMING_SESSIONS_SERVICE ON GROOMING_SESSIONS (VISIT_SERVICE_ID);
CREATE INDEX IX_GROOMING_SESSIONS_NEXT ON GROOMING_SESSIONS (NEXT_SUGGESTED_AT);
CREATE INDEX IX_VISIT_ATTACHMENTS_VISIT ON VISIT_ATTACHMENTS (VISIT_ID);
