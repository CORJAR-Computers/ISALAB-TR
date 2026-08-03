-- ============================================================================
-- ISALAB · Migración 0004 — Registro de Auditoría de Usuario (USER_AUDIT_LOG)
-- ============================================================================

CREATE TABLE USER_AUDIT_LOG (
    ID          D_PK NOT NULL PRIMARY KEY,
    USER_ID     D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    USERNAME    D_NAME NOT NULL,
    ACTION      VARCHAR(50) NOT NULL,
    DETAILS     D_NOTES,
    CREATED_AT  TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_USER_AUDIT_LOG_ID;

SET TERM ^ ;

CREATE TRIGGER BI_USER_AUDIT_LOG FOR USER_AUDIT_LOG BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_USER_AUDIT_LOG_ID, 1);
END^

SET TERM ; ^
