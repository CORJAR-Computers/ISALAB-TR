-- ============================================================================
-- ISALAB · Migración 0003 — Cirugías y agenda quirúrgica (Hito 3)
-- Programación de intervenciones con tipo, anestesia y estados clínicos.
-- ============================================================================

CREATE TABLE SURGERIES (
    ID                  D_PK NOT NULL PRIMARY KEY,
    PATIENT_ID          D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    VETERINARIAN_ID     D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    SURGERY_TYPE        D_NAME NOT NULL,
    SCHEDULED_AT        TIMESTAMP NOT NULL,
    ANESTHESIA_TYPE     D_NAME,
    PREOPERATIVE_NOTES  D_NOTES,
    POSTOPERATIVE_NOTES D_NOTES,
    STATUS              D_STATUS DEFAULT 'PROGRAMADA' NOT NULL
                        CHECK (STATUS IN ('PROGRAMADA', 'EN_CURSO', 'COMPLETADA', 'CANCELADA')),
    CREATED_AT          TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT          TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_SURGERIES_ID;

SET TERM ^ ;

CREATE TRIGGER BI_SURGERIES FOR SURGERIES BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_SURGERIES_ID, 1);
END^

SET TERM ; ^
