-- ============================================================================
-- ISALAB · Migración 0010 — Equipos analizadores y rangos por equipo
-- Permite cargar valores de referencia por marca/modelo (MINDRAY B2800, …)
-- y que el operario elija el equipo al recibir cada muestra. La validación
-- clínica usa los rangos del equipo de la muestra con respaldo al perfil
-- "GENERAL" (ID 1) cuando el equipo no tiene un rango específico.
-- ============================================================================

-- ========================= EQUIPOS ANALIZADORES =============================
CREATE TABLE ANALYZERS (
    ID           D_PK NOT NULL PRIMARY KEY,
    CODE         D_CODE NOT NULL UNIQUE,
    NAME         D_NAME NOT NULL,
    MANUFACTURER D_NAME,
    MODEL        D_NAME,
    IS_ACTIVE    BOOLEAN DEFAULT TRUE NOT NULL,
    NOTES        D_NOTES
);
CREATE GENERATOR GEN_ANALYZERS_ID;

SET TERM ^ ;

CREATE TRIGGER BI_ANALYZERS FOR ANALYZERS BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_ANALYZERS_ID, 1);
END^

SET TERM ; ^

-- Perfil estándar (ID 1): rangos por especie/sexo/edad sin equipo automatizado.
-- MINDRAY B2800 (ID 2): hematología automatizada, referencias precargadas.
INSERT INTO ANALYZERS (ID, CODE, NAME, MANUFACTURER, MODEL, NOTES) VALUES
(1, 'GENERAL', 'General (lectura manual)', NULL, NULL,
 'Rangos estándar por especie, sexo y edad. Aplica cuando la muestra no se procesa en un equipo automatizado.');
INSERT INTO ANALYZERS (ID, CODE, NAME, MANUFACTURER, MODEL, NOTES) VALUES
(2, 'MINDRAY-B2800', 'MINDRAY B2800', 'MINDRAY', 'B2800',
 'Hematología automatizada de 3 partes. Ajusta los rangos según el inserto del fabricante para cada especie.');

-- Avanza el generador más allá de los IDs sembrados (1, 2) para que los
-- equipos creados por la UI no colisionen.
SET GENERATOR GEN_ANALYZERS_ID TO 2;

-- ================= RANGOS POR EQUIPO (REFERENCE_RANGES) ====================
-- Los rangos existentes pasan a pertenecer al perfil GENERAL (ID 1).
ALTER TABLE REFERENCE_RANGES ADD ANALYZER_ID D_PK DEFAULT 1 NOT NULL;

-- Reemplaza la unicidad por especie/sexo/edad con una que incluye el equipo,
-- permitiendo un rango distinto por cada marca/modelo.
SET TERM ^ ;

EXECUTE BLOCK AS
    DECLARE VARIABLE CN VARCHAR(63);
BEGIN
    SELECT FIRST 1 rc.RDB$CONSTRAINT_NAME
    FROM RDB$RELATION_CONSTRAINTS rc
    WHERE rc.RDB$RELATION_NAME = 'REFERENCE_RANGES'
      AND rc.RDB$CONSTRAINT_TYPE = 'UNIQUE'
    INTO :CN;
    IF (:CN IS NOT NULL) THEN
        EXECUTE STATEMENT 'ALTER TABLE REFERENCE_RANGES DROP CONSTRAINT ' || :CN;
END^

SET TERM ; ^

ALTER TABLE REFERENCE_RANGES ADD CONSTRAINT FK_REFERENCE_RANGES_ANALYZER
    FOREIGN KEY (ANALYZER_ID) REFERENCES ANALYZERS (ID);

ALTER TABLE REFERENCE_RANGES ADD CONSTRAINT UQ_REFERENCE_RANGES_ANALYZER
    UNIQUE (ANALYZER_ID, ANALYTE_ID, SPECIES_ID, SEX, AGE_MIN_MONTHS, AGE_MAX_MONTHS);

-- Avanza el generador por encima de los rangos sembrados (IDs 1-47) **solo si**
-- todavía está por debajo de 47. Usar `SET GENERATOR ... TO 47` de forma
-- incondicional regresaría el generador en clínicas que ya crearon rangos
-- propios con ID >= 48 al actualizar desde un esquema anterior, lo que
-- provocaría colisiones de PK en el próximo INSERT. El EXECUTE BLOCK es
-- idempotente y seguro tanto para instalaciones nuevas como para upgrades.
SET TERM ^ ;

EXECUTE BLOCK AS
    DECLARE VARIABLE CURR INTEGER;
BEGIN
    CURR = GEN_ID(GEN_REFERENCE_RANGES_ID, 0);
    IF (CURR < 47) THEN
        CURR = GEN_ID(GEN_REFERENCE_RANGES_ID, 47 - CURR);
END^

SET TERM ; ^

-- ================ EQUIPO EN LA MUESTRA (SAMPLES) ============================
-- NULL = sin equipo seleccionado → se valida contra el perfil GENERAL.
ALTER TABLE SAMPLES ADD ANALYZER_ID D_PK;

ALTER TABLE SAMPLES ADD CONSTRAINT FK_SAMPLES_ANALYZER
    FOREIGN KEY (ANALYZER_ID) REFERENCES ANALYZERS (ID);

-- ============ STORED PROCEDURES CON EQUIPO + RESPALDO GENERAL ==============
SET TERM ^ ;

-- SP_VALIDATE depende de SP_FIND, así que se elimina primero.
DROP PROCEDURE SP_VALIDATE_ANALYTICAL_RESULT^
DROP PROCEDURE SP_FIND_REFERENCE_RANGE^

-- Busca el valor de referencia más específico para (analito, especie, sexo,
-- edad, equipo). Si el equipo no tiene rango para el caso, respalda con el
-- perfil GENERAL (ANALYZER_ID = 1). Prioriza el rango del equipo exacto,
-- luego el de sexo exacto y el de mayor edad aplicable.
CREATE PROCEDURE SP_FIND_REFERENCE_RANGE (
    P_ANALYTE_ID  INTEGER,
    P_SPECIES_ID  INTEGER,
    P_SEX         CHAR(1),
    P_AGE_MONTHS  INTEGER,
    P_ANALYZER_ID INTEGER
) RETURNS (
    RR_ID         INTEGER,
    MIN_VALUE     DOUBLE PRECISION,
    MAX_VALUE     DOUBLE PRECISION,
    CRITICAL_MIN  DOUBLE PRECISION,
    CRITICAL_MAX  DOUBLE PRECISION
) AS
BEGIN
    SELECT FIRST 1 rr.ID, rr.MIN_VALUE, rr.MAX_VALUE, rr.CRITICAL_MIN, rr.CRITICAL_MAX
    FROM REFERENCE_RANGES rr
    WHERE rr.ANALYTE_ID = :P_ANALYTE_ID
      AND rr.SPECIES_ID = :P_SPECIES_ID
      AND :P_AGE_MONTHS BETWEEN rr.AGE_MIN_MONTHS AND rr.AGE_MAX_MONTHS
      AND (rr.SEX IS NULL OR rr.SEX = :P_SEX)
      AND (:P_ANALYZER_ID IS NULL OR rr.ANALYZER_ID IN (1, :P_ANALYZER_ID))
    ORDER BY CASE WHEN rr.ANALYZER_ID = :P_ANALYZER_ID THEN 0 ELSE 1 END,
             CASE WHEN rr.SEX = :P_SEX THEN 0 ELSE 1 END,
             rr.AGE_MIN_MONTHS DESC
    INTO :RR_ID, :MIN_VALUE, :MAX_VALUE, :CRITICAL_MIN, :CRITICAL_MAX;
    SUSPEND;
END^

-- Valida un resultado contra los rangos del equipo de la muestra (con respaldo
-- GENERAL). Misma firma de siempre: el equipo se lee de la propia muestra.
CREATE PROCEDURE SP_VALIDATE_ANALYTICAL_RESULT (
    P_SAMPLE_ID   INTEGER,
    P_ANALYTE_ID  INTEGER,
    P_VALUE       DOUBLE PRECISION
) RETURNS (
    RR_ID   INTEGER,
    STATUS  VARCHAR(10)
) AS
DECLARE VARIABLE V_SPECIES_ID  INTEGER;
DECLARE VARIABLE V_SEX         CHAR(1);
DECLARE VARIABLE V_AGE_MONTHS  INTEGER;
DECLARE VARIABLE V_ANALYZER_ID INTEGER;
DECLARE VARIABLE V_MIN         DOUBLE PRECISION;
DECLARE VARIABLE V_MAX         DOUBLE PRECISION;
BEGIN
    SELECT FIRST 1 pa.SPECIES_ID, pa.SEX,
           COALESCE(CAST(DATEDIFF(MONTH, pa.BIRTH_DATE, CURRENT_TIMESTAMP) AS INTEGER), 0),
           sa.ANALYZER_ID
    FROM SAMPLES sa
    JOIN PATIENTS pa ON pa.ID = sa.PATIENT_ID
    WHERE sa.ID = :P_SAMPLE_ID
    INTO :V_SPECIES_ID, :V_SEX, :V_AGE_MONTHS, :V_ANALYZER_ID;

    IF (:V_SPECIES_ID IS NULL) THEN
    BEGIN
        RR_ID  = NULL;
        STATUS = 'SIN_RANGO';
        SUSPEND;
        EXIT;
    END

    SELECT RR_ID, MIN_VALUE, MAX_VALUE
    FROM SP_FIND_REFERENCE_RANGE(:P_ANALYTE_ID, :V_SPECIES_ID, :V_SEX, :V_AGE_MONTHS, :V_ANALYZER_ID)
    INTO :RR_ID, :V_MIN, :V_MAX;

    IF (:RR_ID IS NULL) THEN
    BEGIN
        STATUS = 'SIN_RANGO';
        SUSPEND;
        EXIT;
    END

    IF (:P_VALUE < :V_MIN) THEN
        STATUS = 'BAJO';
    ELSE IF (:P_VALUE > :V_MAX) THEN
        STATUS = 'ALTO';
    ELSE
        STATUS = 'NORMAL';

    SUSPEND;
END^

SET TERM ; ^
