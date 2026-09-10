-- ============================================================================
-- ISALAB · Migración 0022 — Valores de referencia definidos por el veterinario
--
-- Cuando un analito no tiene rango en el catálogo (o el laboratorio quiere
-- usar el suyo), el veterinario puede capturar el rango junto al resultado.
-- El rango capturado se guarda en el propio LAB_RESULTS y el SP de validación
-- lo usa con PRECEDENCIA sobre el rango del catálogo (REFERENCE_RANGES).
--
-- Las columnas CUSTOM_REF_MIN / CUSTOM_REF_MAX son NULL = sin rango manual
-- (se valida contra el catálogo como siempre). REFERENCE_RANGE_ID sigue
-- apuntando al rango del catálogo usado; si hay rango manual queda NULL.
-- ============================================================================

ALTER TABLE LAB_RESULTS ADD CUSTOM_REF_MIN DOUBLE PRECISION;
ALTER TABLE LAB_RESULTS ADD CUSTOM_REF_MAX DOUBLE PRECISION;

-- Coherencia: si se capturan ambos, min <= max. (Uno solo puede ser NULL para
-- permitir rangos abiertos: solo mínimo o solo máximo.)
ALTER TABLE LAB_RESULTS ADD CONSTRAINT CK_LAB_RESULTS_CUSTOM_REF
    CHECK (CUSTOM_REF_MIN IS NULL OR CUSTOM_REF_MAX IS NULL
           OR CUSTOM_REF_MIN <= CUSTOM_REF_MAX);

-- ============================================================================
-- SP_VALIDATE_ANALYTICAL_RESULT con soporte de rango manual.
--
-- Se recrea (no se puede alterar el cuerpo en Firebird). Misma firma de
-- siempre + parámetros opcionales del rango capturado por el veterinario:
-- si llegan, determinan el estado directamente; si no, se usa el catálogo.
-- ============================================================================

SET TERM ^ ;

DROP PROCEDURE SP_VALIDATE_ANALYTICAL_RESULT^

CREATE PROCEDURE SP_VALIDATE_ANALYTICAL_RESULT (
    P_SAMPLE_ID     INTEGER,
    P_ANALYTE_ID    INTEGER,
    P_VALUE         DOUBLE PRECISION,
    P_CUSTOM_MIN    DOUBLE PRECISION DEFAULT NULL,
    P_CUSTOM_MAX    DOUBLE PRECISION DEFAULT NULL
) RETURNS (
    RR_ID   INTEGER,
    STATUS  VARCHAR(12)
) AS
DECLARE VARIABLE V_SPECIES_ID  INTEGER;
DECLARE VARIABLE V_SEX         CHAR(1);
DECLARE VARIABLE V_AGE_MONTHS  INTEGER;
DECLARE VARIABLE V_ANALYZER_ID INTEGER;
DECLARE VARIABLE V_MIN         DOUBLE PRECISION;
DECLARE VARIABLE V_MAX         DOUBLE PRECISION;
DECLARE VARIABLE V_CRIT_MIN    DOUBLE PRECISION;
DECLARE VARIABLE V_CRIT_MAX    DOUBLE PRECISION;
BEGIN
    SELECT FIRST 1 pa.SPECIES_ID, pa.SEX,
           COALESCE(CAST(DATEDIFF(MONTH, pa.BIRTH_DATE, CURRENT_TIMESTAMP) AS INTEGER), 0),
           sa.ANALYZER_ID
    FROM SAMPLES sa
    JOIN PATIENTS pa ON pa.ID = sa.PATIENT_ID
    WHERE sa.ID = :P_SAMPLE_ID
    INTO :V_SPECIES_ID, :V_SEX, :V_AGE_MONTHS, :V_ANALYZER_ID;

    -- Rango capturado por el veterinario (cualquiera de los dos puede ser
    -- NULL = límite abierto). Tiene precedencia sobre el catálogo y no se
    -- vincula a ningún rango de REFERENCE_RANGES.
    IF (:P_CUSTOM_MIN IS NOT NULL OR :P_CUSTOM_MAX IS NOT NULL) THEN
    BEGIN
        RR_ID = NULL;
        IF (:P_CUSTOM_MAX IS NOT NULL AND :P_VALUE > :P_CUSTOM_MAX) THEN
            STATUS = 'ALTO';
        ELSE IF (:P_CUSTOM_MIN IS NOT NULL AND :P_VALUE < :P_CUSTOM_MIN) THEN
            STATUS = 'BAJO';
        ELSE
            STATUS = 'NORMAL';
        SUSPEND;
        EXIT;
    END

    IF (:V_SPECIES_ID IS NULL) THEN
    BEGIN
        RR_ID  = NULL;
        STATUS = 'SIN_RANGO';
        SUSPEND;
        EXIT;
    END

    SELECT RR_ID, MIN_VALUE, MAX_VALUE, CRITICAL_MIN, CRITICAL_MAX
    FROM SP_FIND_REFERENCE_RANGE(:P_ANALYTE_ID, :V_SPECIES_ID, :V_SEX, :V_AGE_MONTHS, :V_ANALYZER_ID)
    INTO :RR_ID, :V_MIN, :V_MAX, :V_CRIT_MIN, :V_CRIT_MAX;

    IF (:RR_ID IS NULL) THEN
    BEGIN
        STATUS = 'SIN_RANGO';
        SUSPEND;
        EXIT;
    END

    -- Precedencia: crítico > fuera de rango > normal. Los umbrales críticos
    -- deben configurarse FUERA del rango de referencia (p. ej. K+ normal
    -- 3.5–5.5, crítico < 2.8 o > 7.5).
    IF (:V_CRIT_MAX IS NOT NULL AND :P_VALUE > :V_CRIT_MAX) THEN
        STATUS = 'CRITICO_ALTO';
    ELSE IF (:V_CRIT_MIN IS NOT NULL AND :P_VALUE < :V_CRIT_MIN) THEN
        STATUS = 'CRITICO_BAJO';
    ELSE IF (:P_VALUE < :V_MIN) THEN
        STATUS = 'BAJO';
    ELSE IF (:P_VALUE > :V_MAX) THEN
        STATUS = 'ALTO';
    ELSE
        STATUS = 'NORMAL';

    SUSPEND;
END^

SET TERM ; ^
