-- ============================================================================
-- ISALAB · Migración 0013 — Calidad preanalítica y rechazo de muestras
-- Añade la trazabilidad de interferencias (hemólisis, lipemia, ictericia,
-- coágulo, volumen insuficiente) y la información del tubo/anticoagulante por
-- tipo de muestra. Introduce el estado RECHAZADA en la máquina de estados.
-- ============================================================================

-- ===================== TUBO / ANTICOAGULANTE POR TIPO =======================
ALTER TABLE SAMPLE_TYPES ADD TUBE_TYPE VARCHAR(60) CHARACTER SET UTF8;
ALTER TABLE SAMPLE_TYPES ADD ANTICOAGULANT VARCHAR(60) CHARACTER SET UTF8;
ALTER TABLE SAMPLE_TYPES ADD MIN_VOLUME_ML DOUBLE PRECISION;

-- ================== CALIDAD DE LA MUESTRA EN SAMPLES ========================
-- QUALITY_INDEX: NORMAL | HEMOLISIS | LIPEMIA | ICTERICIA | COAGULO | INSUFICIENTE | CONTAMINADA
-- QUALITY_SEVERITY: LEVE | MODERADA | MARCADA
ALTER TABLE SAMPLES ADD QUALITY_INDEX VARCHAR(12) CHARACTER SET UTF8;
ALTER TABLE SAMPLES ADD QUALITY_SEVERITY VARCHAR(10) CHARACTER SET UTF8;
ALTER TABLE SAMPLES ADD QUALITY_NOTE VARCHAR(200) CHARACTER SET UTF8;

-- ==================== RECHAZO (ESTADO RECHAZADA) ============================
ALTER TABLE SAMPLES ADD REJECTED_AT TIMESTAMP;
ALTER TABLE SAMPLES ADD REJECTED_BY D_NAME;
ALTER TABLE SAMPLES ADD REJECTION_REASON VARCHAR(200) CHARACTER SET UTF8;

-- Reemplaza el CHECK de STATUS para admitir RECHAZADA (los CHECK no se
-- pueden alterar en Firebird; se elimina el anónimo y se recrea con nombre).
SET TERM ^ ;

EXECUTE BLOCK AS
    DECLARE VARIABLE CN VARCHAR(63);
BEGIN
    SELECT FIRST 1 rc.RDB$CONSTRAINT_NAME
    FROM RDB$RELATION_CONSTRAINTS rc
    WHERE rc.RDB$RELATION_NAME = 'SAMPLES'
      AND rc.RDB$CONSTRAINT_TYPE = 'CHECK'
    INTO :CN;
    IF (:CN IS NOT NULL) THEN
        EXECUTE STATEMENT 'ALTER TABLE SAMPLES DROP CONSTRAINT ' || :CN;
END^

SET TERM ; ^

ALTER TABLE SAMPLES ADD CONSTRAINT CK_SAMPLES_STATUS
    CHECK (STATUS IN ('RECIBIDA', 'EN_PROCESO', 'FINALIZADA', 'ANULADA', 'RECHAZADA'));

-- ==================== VALORES REFERENCIALES DE TUBOS ========================
-- Información orientativa de los tubos estándar para los tipos de muestra más
-- comunes (solo si aún no tienen valores; INSERT ... WHERE NOT EXISTS por tipo).
UPDATE SAMPLE_TYPES st
   SET st.TUBE_TYPE = 'Tubo lila (EDTA)',
       st.ANTICOAGULANT = 'EDTA K2/K3',
       st.MIN_VOLUME_ML = 1.0
 WHERE st.ID = 1;

UPDATE SAMPLE_TYPES st
   SET st.TUBE_TYPE = 'Tubo rojo (suero)',
       st.ANTICOAGULANT = 'Sin anticoagulante',
       st.MIN_VOLUME_ML = 2.0
 WHERE st.ID = 2;

UPDATE SAMPLE_TYPES st
   SET st.TUBE_TYPE = 'Tubo azul (citrato)',
       st.ANTICOAGULANT = 'Citrato de sodio 3.2%',
       st.MIN_VOLUME_ML = 1.8
 WHERE st.ID = 3;

UPDATE SAMPLE_TYPES st
   SET st.TUBE_TYPE = 'Tubo verde (heparina)',
       st.ANTICOAGULANT = 'Heparina de litio',
       st.MIN_VOLUME_ML = 2.0
 WHERE st.ID = 4;

UPDATE SAMPLE_TYPES st
   SET st.TUBE_TYPE = 'Frasco estéril (orina)',
       st.ANTICOAGULANT = 'Sin anticoagulante',
       st.MIN_VOLUME_ML = 5.0
 WHERE st.ID = 5;