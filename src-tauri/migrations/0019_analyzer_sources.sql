-- ============================================================================
-- ISALAB · Migración 0019 — Fuentes automáticas de resultados (carpeta vigilada)
-- Permite que cada analizador exporte resultados a una carpeta local y la app
-- los importe automáticamente con el mapeo columna → analito guardado.
-- El módulo `AnalyzerSource` (src/analyzer_sources/) abstrae la fuente: hoy
-- una carpeta vigilada (CSV), mañana ASTM/HL7 por red sin tocar la UI.
-- ============================================================================

-- ========================== FUENTES POR EQUIPO ==============================
-- Una fuente por analizador. SOURCE_TYPE queda listo para otros drivers
-- ('WATCHED_FOLDER' hoy; futuro: 'ASTM_SERIAL', 'HL7'...).
CREATE TABLE ANALYZER_SOURCES (
    ID             D_PK NOT NULL PRIMARY KEY,
    ANALYZER_ID    D_PK NOT NULL REFERENCES ANALYZERS (ID) ON DELETE CASCADE,
    SOURCE_TYPE    VARCHAR(30) DEFAULT 'WATCHED_FOLDER' NOT NULL,
    FOLDER_PATH    VARCHAR(500) CHARACTER SET UTF8,
    -- Índice de la columna CSV con el código de muestra (mapeo guardado).
    SAMPLE_CODE_COLUMN INTEGER,
    ENABLED        BOOLEAN DEFAULT TRUE NOT NULL,
    -- Última vez que el supervisor sondeó esta fuente.
    LAST_POLL_AT   TIMESTAMP,
    CREATED_AT     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE (ANALYZER_ID, SOURCE_TYPE)
);
CREATE GENERATOR GEN_ANALYZER_SOURCES_ID;

-- ==================== MAPEO COLUMNA → ANALITO (CSV) ========================
-- El mapeo vive en su propia tabla (no en JSON) para validarlo con FK contra
-- ANALYTES y mostrarlo en la UI como el diálogo de importación manual.
CREATE TABLE ANALYZER_SOURCE_COLUMNS (
    SOURCE_ID    D_PK NOT NULL REFERENCES ANALYZER_SOURCES (ID) ON DELETE CASCADE,
    COLUMN_INDEX INTEGER NOT NULL,
    ANALYTE_ID   D_PK NOT NULL REFERENCES ANALYTES (ID),
    PRIMARY KEY (SOURCE_ID, COLUMN_INDEX)
);

-- ==================== LOG DE IMPORTACIÓN POR ARCHIVO ========================
-- Cola/estado de cada archivo detectado en la carpeta vigilada: procesado o
-- fallido, con conteos y motivo. Permite reintentar sin reimportar duplicados.
CREATE TABLE ANALYZER_IMPORT_JOBS (
    ID                D_PK NOT NULL PRIMARY KEY,
    SOURCE_ID         D_PK NOT NULL REFERENCES ANALYZER_SOURCES (ID) ON DELETE CASCADE,
    FILE_NAME         VARCHAR(300) CHARACTER SET UTF8 NOT NULL,
    STATUS            VARCHAR(15) NOT NULL CHECK (STATUS IN ('IMPORTADO', 'FALLIDO')),
    SAMPLES_UPDATED   INTEGER DEFAULT 0 NOT NULL,
    RESULTS_IMPORTED  INTEGER DEFAULT 0 NOT NULL,
    SKIPPED_ROWS      INTEGER DEFAULT 0 NOT NULL,
    ERROR_MSG         D_NOTES,
    CREATED_AT        TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PROCESSED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE (SOURCE_ID, FILE_NAME)
);
CREATE GENERATOR GEN_ANALYZER_IMPORT_JOBS_ID;
CREATE INDEX IDX_ANALYZER_IMPORT_JOBS_SOURCE ON ANALYZER_IMPORT_JOBS (SOURCE_ID, CREATED_AT);

-- ============================= TRIGGERS =====================================
SET TERM ^ ;

CREATE TRIGGER BI_ANALYZER_SOURCES FOR ANALYZER_SOURCES BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_ANALYZER_SOURCES_ID, 1);
END^

CREATE TRIGGER BI_ANALYZER_IMPORT_JOBS FOR ANALYZER_IMPORT_JOBS BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_ANALYZER_IMPORT_JOBS_ID, 1);
END^

CREATE TRIGGER BU_ANALYZER_SOURCES FOR ANALYZER_SOURCES BEFORE UPDATE AS
BEGIN
    NEW.UPDATED_AT = CURRENT_TIMESTAMP;
END^

SET TERM ; ^
