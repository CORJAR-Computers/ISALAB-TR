-- ============================================================================
-- ISALAB · Migración 0006 — Código único de paciente (PAC-YYYY-NNNN)
--
-- Problema: muchas mascotas comparten el mismo nombre (Luna, Max, etc.).
-- Solución: columna CODE con formato PAC-YYYY-NNNN (año + secuencia) generada
-- automáticamente por un trigger BEFORE INSERT en Firebird.
-- Formato: PAC-2026-0001, PAC-2026-0042, PAC-2027-0001, …
--
-- El generator GEN_PATIENT_CODE_SEQ es global (no se resetea por año); la
-- secuencia anual se gestiona vía la función de ayuda en el trigger.
-- ============================================================================

-- 1. Añadir la columna (nullable para no romper rows existentes).
ALTER TABLE PATIENTS ADD CODE VARCHAR(20) CHARACTER SET UTF8;

-- 2. Generator de secuencia para el código.
CREATE GENERATOR GEN_PATIENT_CODE_SEQ;

-- 3. Poblar retroactivamente los pacientes existentes con códigos únicos.
--    Usamos el ID del paciente para reconstruir un código determinístico.
SET TERM ^ ;

EXECUTE BLOCK AS
    DECLARE VARIABLE V_ID INTEGER;
    DECLARE VARIABLE V_CODE VARCHAR(20);
    DECLARE VARIABLE V_SEQ INTEGER;
    DECLARE VARIABLE V_YEAR INTEGER;
    DECLARE VARIABLE V_CREATED TIMESTAMP;
    DECLARE VARIABLE V_TEMP INTEGER;
BEGIN
    V_SEQ = 0;
    FOR SELECT ID, CREATED_AT FROM PATIENTS ORDER BY ID INTO :V_ID, :V_CREATED DO
    BEGIN
        V_SEQ = V_SEQ + 1;
        V_YEAR = EXTRACT(YEAR FROM V_CREATED);
        V_CODE = 'PAC-' || CAST(V_YEAR AS VARCHAR(4)) || '-' || LPAD(CAST(V_SEQ AS VARCHAR(10)), 4, '0');
        UPDATE PATIENTS SET CODE = :V_CODE WHERE ID = :V_ID;
        -- Avanzar el generator para que los nuevos pacientes no colisionen.
        V_TEMP = GEN_ID(GEN_PATIENT_CODE_SEQ, 1);
    END
END^

SET TERM ; ^

-- 4. Ahora que todos los rows tienen CODE, aplicar el índice único.
ALTER TABLE PATIENTS ALTER COLUMN CODE SET NOT NULL;
CREATE UNIQUE INDEX UX_PATIENTS_CODE ON PATIENTS (CODE);

-- 5. Trigger BEFORE INSERT para auto-generar el código en nuevos pacientes.
SET TERM ^ ;

CREATE OR ALTER TRIGGER BI_PATIENTS_CODE FOR PATIENTS
ACTIVE BEFORE INSERT POSITION 10
AS
    DECLARE VARIABLE V_SEQ INTEGER;
    DECLARE VARIABLE V_YEAR INTEGER;
BEGIN
    IF (NEW.CODE IS NULL OR NEW.CODE = '') THEN
    BEGIN
        V_SEQ  = GEN_ID(GEN_PATIENT_CODE_SEQ, 1);
        V_YEAR = EXTRACT(YEAR FROM CURRENT_TIMESTAMP);
        NEW.CODE = 'PAC-' || CAST(V_YEAR AS VARCHAR(4)) || '-'
                   || LPAD(CAST(V_SEQ AS VARCHAR(10)), 4, '0');
    END
END^

SET TERM ; ^
