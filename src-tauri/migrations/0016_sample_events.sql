-- ============================================================================
-- ISALAB · Migración 0016 — Historial de eventos por muestra (SAMPLE_EVENTS)
-- Registra cada rechazo y reapertura de una muestra: quién, cuándo y motivo.
-- A diferencia de los campos REJECTED_AT/BY/REASON de SAMPLES (que solo
-- conservan el último ciclo y se limpian al reabrir), esta tabla conserva el
-- historial completo para la vista de auditoría de la ficha de la muestra.
-- ============================================================================

CREATE TABLE SAMPLE_EVENTS (
    ID          D_PK NOT NULL PRIMARY KEY,
    SAMPLE_ID   D_PK NOT NULL REFERENCES SAMPLES (ID) ON DELETE CASCADE,
    -- REJECTED | REOPENED
    EVENT_TYPE  VARCHAR(20) CHARACTER SET UTF8 NOT NULL,
    USERNAME    D_NAME NOT NULL,
    -- Motivo del rechazo (obligatorio en REJECTED, NULL en REOPENED).
    REASON      VARCHAR(200) CHARACTER SET UTF8,
    CREATED_AT  TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_SAMPLE_EVENTS_ID;

CREATE INDEX IX_SAMPLE_EVENTS_SAMPLE ON SAMPLE_EVENTS (SAMPLE_ID);

SET TERM ^ ;

CREATE TRIGGER BI_SAMPLE_EVENTS FOR SAMPLE_EVENTS BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_SAMPLE_EVENTS_ID, 1);
END^

SET TERM ; ^