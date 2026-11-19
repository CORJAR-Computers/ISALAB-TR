-- ============================================================================
-- ISALAB · Migración 0034 — Archivos como BLOB en Firebird (Fase 6)
--
-- La base central ya no apunta a archivos locales de una PC: adjuntos de
-- resultados, fotos/documentos de visitas, logos (principal y secundarios) y
-- el certificado PKCS#12 viven dentro del `.fdb`. Un solo protocolo (TCP 3050)
-- y un solo backup (`gbak`, que incluye los BLOB).
--
-- Las columnas de ruta se eliminan: la fuente de verdad es el BLOB. La app
-- guarda además `SIZE_BYTES` para mostrar el tamaño sin leer el contenido.
-- ============================================================================

-- Adjuntos de resultados: contenido binario + tamaño.
ALTER TABLE RESULT_ATTACHMENTS ADD CONTENT BLOB SUB_TYPE BINARY;
ALTER TABLE RESULT_ATTACHMENTS ADD SIZE_BYTES INTEGER DEFAULT 0 NOT NULL;
ALTER TABLE RESULT_ATTACHMENTS DROP FILE_PATH;

-- Adjuntos de visitas (peluquería y documentos): mismo tratamiento.
ALTER TABLE VISIT_ATTACHMENTS ADD CONTENT BLOB SUB_TYPE BINARY;
ALTER TABLE VISIT_ATTACHMENTS ADD SIZE_BYTES INTEGER DEFAULT 0 NOT NULL;
ALTER TABLE VISIT_ATTACHMENTS DROP FILE_PATH;

-- Logos secundarios: contenido binario + tipo MIME.
ALTER TABLE SECONDARY_LOGOS ADD CONTENT BLOB SUB_TYPE BINARY;
ALTER TABLE SECONDARY_LOGOS ADD MIME_TYPE VARCHAR(100) CHARACTER SET UTF8;
ALTER TABLE SECONDARY_LOGOS ADD SIZE_BYTES INTEGER DEFAULT 0 NOT NULL;
ALTER TABLE SECONDARY_LOGOS DROP LOGO_PATH;

-- Activos únicos de la clínica: logo principal y certificado PKCS#12.
-- Una sola fila por KIND (índice único): se hace upsert al importar.
CREATE TABLE CLINIC_ASSETS (
    ID         INTEGER NOT NULL PRIMARY KEY,
    -- LOGO | CERT_P12
    KIND       VARCHAR(20) CHARACTER SET ASCII NOT NULL,
    FILE_NAME  VARCHAR(255) CHARACTER SET UTF8,
    MIME_TYPE  VARCHAR(100) CHARACTER SET UTF8,
    CONTENT    BLOB SUB_TYPE BINARY,
    SIZE_BYTES INTEGER DEFAULT 0 NOT NULL,
    CREATED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX UX_CLINIC_ASSETS_KIND ON CLINIC_ASSETS (KIND);

CREATE GENERATOR GEN_CLINIC_ASSETS_SEQ;

SET TERM ^ ;
CREATE TRIGGER TR_CLINIC_ASSETS_BI FOR CLINIC_ASSETS ACTIVE BEFORE INSERT POSITION 0 AS
BEGIN
    IF (NEW.ID IS NULL) THEN
        NEW.ID = GEN_ID(GEN_CLINIC_ASSETS_SEQ, 1);
END^
SET TERM ; ^

-- Las rutas locales dejan de tener sentido en una base compartida. Las filas
-- de configuración se conservan por compatibilidad, pero la app ya no las lee.
DELETE FROM CLINIC_SETTINGS
 WHERE KEY_NAME IN ('clinic.logo_path', 'report.pkcs12_path');
