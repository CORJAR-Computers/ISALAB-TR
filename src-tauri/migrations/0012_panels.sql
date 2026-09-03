-- ============================================================================
-- ISALAB · Migración 0012 — Paneles de analitos (carga por lotes)
-- Un "panel" agrupa los analitos que se cargan juntos en una corrida
-- (p. ej. Hemograma completo, Perfil renal, Perfil hepático). Permite la
-- entrada en grilla y acelera la carga de resultados desde el analizador.
-- SAMPLE_TYPE_ID NULL = panel genérico disponible para cualquier tipo de muestra.
-- ============================================================================

CREATE TABLE PANELS (
    ID             D_PK NOT NULL PRIMARY KEY,
    NAME           D_NAME NOT NULL,
    SAMPLE_TYPE_ID D_PK REFERENCES SAMPLE_TYPES (ID) ON DELETE SET NULL,
    SORT_ORDER     INTEGER DEFAULT 0 NOT NULL,
    IS_ACTIVE      BOOLEAN DEFAULT TRUE NOT NULL,
    NOTES          D_NOTES,
    CREATED_AT     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_PANELS_ID;

CREATE TABLE PANEL_ANALYTES (
    ID         D_PK NOT NULL PRIMARY KEY,
    PANEL_ID   D_PK NOT NULL REFERENCES PANELS (ID) ON DELETE CASCADE,
    ANALYTE_ID D_PK NOT NULL REFERENCES ANALYTES (ID) ON DELETE CASCADE,
    SEQ        INTEGER DEFAULT 0 NOT NULL,
    UNIQUE (PANEL_ID, ANALYTE_ID)
);
CREATE GENERATOR GEN_PANEL_ANALYTES_ID;

-- El panel "Hemograma completo" agrupa los analitos hematológicos sembrados,
-- disponible para el tipo de muestra 1 (Sangre total EDTA) y como genérico.
INSERT INTO PANELS (ID, NAME, SAMPLE_TYPE_ID, SORT_ORDER, IS_ACTIVE, NOTES)
VALUES (1, 'Hemograma completo', 1, 10, TRUE,
        'Panel estándar de hematología: serie roja, serie blanca y plaquetas.');

INSERT INTO PANEL_ANALYTES (ID, PANEL_ID, ANALYTE_ID, SEQ)
SELECT 1, 1, a.ID, 10 FROM ANALYTES a WHERE UPPER(a.NAME) = 'HEMATOCRITO';
INSERT INTO PANEL_ANALYTES (ID, PANEL_ID, ANALYTE_ID, SEQ)
SELECT 2, 1, a.ID, 20 FROM ANALYTES a WHERE UPPER(a.NAME) = 'HEMOGLOBINA';
INSERT INTO PANEL_ANALYTES (ID, PANEL_ID, ANALYTE_ID, SEQ)
SELECT 3, 1, a.ID, 30 FROM ANALYTES a WHERE UPPER(a.NAME) = 'GLOBULOS ROJOS';
INSERT INTO PANEL_ANALYTES (ID, PANEL_ID, ANALYTE_ID, SEQ)
SELECT 4, 1, a.ID, 40 FROM ANALYTES a WHERE UPPER(a.NAME) = 'LEUCOCITOS';
INSERT INTO PANEL_ANALYTES (ID, PANEL_ID, ANALYTE_ID, SEQ)
SELECT 5, 1, a.ID, 50 FROM ANALYTES a WHERE UPPER(a.NAME) = 'PLAQUETAS';

-- Avanza los generadores por encima de los IDs sembrados.
SET GENERATOR GEN_PANELS_ID TO 1;
SET GENERATOR GEN_PANEL_ANALYTES_ID TO 5;