-- ============================================================================
-- ISALAB · Migración 0017 — Notificaciones de valores críticos
-- Registro de auditoría de notificaciones (NOTIFICATION_LOG): cada envío por
-- WhatsApp/email y cada confirmación ("acknowledgment") del analista queda
-- persistido con quién, cuándo, canal y destinatario (CLSI GP47). Añade
-- también el correo electrónico a USERS para el enrutamiento de avisos.
-- La configuración SMTP vive en CLINIC_SETTINGS (claves planas, sin
-- migración): smtp.host, smtp.port, smtp.tls, smtp.username, smtp.password,
-- smtp.from.
-- ============================================================================

-- Correo del usuario (veterinario/administrador) para avisos.
ALTER TABLE USERS ADD EMAIL D_EMAIL;

CREATE TABLE NOTIFICATION_LOG (
    ID                D_PK NOT NULL PRIMARY KEY,
    -- Resultado asociado (NULL si el aviso es a nivel de muestra).
    RESULT_ID         D_PK REFERENCES LAB_RESULTS (ID) ON DELETE CASCADE,
    SAMPLE_ID         D_PK NOT NULL REFERENCES SAMPLES (ID) ON DELETE CASCADE,
    -- WHATSAPP | EMAIL | MANUAL (confirmación del analista)
    CHANNEL           VARCHAR(20) CHARACTER SET UTF8 NOT NULL,
    RECIPIENT_NAME    D_NAME,
    RECIPIENT_ADDRESS VARCHAR(200) CHARACTER SET UTF8,
    -- SENT | FAILED | ACKNOWLEDGED
    STATUS            VARCHAR(20) CHARACTER SET UTF8 NOT NULL,
    SENT_AT           TIMESTAMP,
    ACKED_AT          TIMESTAMP,
    ACKED_BY          D_NAME,
    NOTE              D_NOTES,
    CREATED_AT        TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_NOTIFICATION_LOG_ID;

CREATE INDEX IX_NOTIFICATION_LOG_SAMPLE ON NOTIFICATION_LOG (SAMPLE_ID);

SET TERM ^ ;

CREATE TRIGGER BI_NOTIFICATION_LOG FOR NOTIFICATION_LOG BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_NOTIFICATION_LOG_ID, 1);
END^

SET TERM ; ^